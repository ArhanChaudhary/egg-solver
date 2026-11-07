"""Audit the returned phack evidence and publish reproducible, scoped findings."""
import csv
import hashlib
import json
from pathlib import Path
import re
import statistics as stats

ROOT = Path('/data/agents/codex-resume')
RAW = ROOT / 'phack-returned/results-phack-20261106T194245Z'
OUT = ROOT / 'phack-analysis'
ARCHIVE = Path('/home/super/egg-solver-pmu-phack-20261106T194245Z.tar.gz')
OUT.mkdir(exist_ok=True)
runs = json.loads((RAW / 'runs.json').read_text())
status = json.loads((RAW / 'status.json').read_text())
manifest = json.loads((RAW / 'BUILD-MANIFEST.json').read_text())
assert status['complete'] and status['error'] is None
assert len(runs) == status['total_runs'] == status['valid_runs'] == 242
assert all(r['valid'] and r['exit_code'] == 0 for r in runs)
assert all(c['count'] is not None and c['running_percent'] == 100
           for r in runs for c in r['counters'])
for name in ('bench', 'gen', 'solve'):
    assert hashlib.file_digest((RAW / 'binaries' / name).open('rb'), 'sha256').hexdigest() == manifest['binaries'][name]
for run in runs:
    if run['record']:
        continue
    with (RAW / 'stat' / (run['name'] + '.csv')).open() as stream:
        rows = [r for r in csv.reader(stream, delimiter=';') if len(r) >= 5 and not r[0].startswith('#')]
    assert len(rows) == len(run['counters'])
    for row, c in zip(rows, run['counters']):
        assert float(row[0]) == c['count'] and row[2].strip() == c['event']
        assert float(row[4]) == c['running_percent']


def family(run):
    return re.sub(r'-r\d+$', '', run['name'])


def members(name):
    return [r for r in runs if family(r) == name]


def counter(run, event):
    return next(c['count'] for c in run['counters'] if c['event'] == event)


def summarize(name):
    rr = members(name)
    rates = [counter(r, 'instructions:u') / r['counted_window_seconds'] / 1e9 for r in rr]
    ipcs = [counter(r, 'instructions:u') / counter(r, 'cycles:u') for r in rr]
    throughput = [r['benchmark']['throughput'] for r in rr]
    return dict(n=len(rr), ipc=stats.median(ipcs), ginst_s=stats.median(rates),
                ginst_min=min(rates), ginst_max=max(rates), solves_s=stats.median(throughput),
                solves_min=min(throughput), solves_max=max(throughput),
                completed_cubes=[r['benchmark']['cubes'] for r in rr])


def event_summary(config, name):
    rr = [r for r in runs if r['name'].startswith('events-' + config + '-') and r['aux'] == name]
    assert rr
    def value(run):
        return run['counters'][-1]['count']
    return dict(n=len(rr), per_ki=stats.median(value(r) / counter(r, 'instructions:u') * 1000 for r in rr),
                per_cycle=stats.median(value(r) / counter(r, 'cycles:u') for r in rr),
                per_second=stats.median(value(r) / r['counted_window_seconds'] for r in rr),
                runs=[r['name'] for r in rr])


def table_mapping(run):
    blocks = []
    block = None
    for line in (RAW / 'placement' / (run['name'] + '.smaps')).read_text().splitlines():
        if re.match(r'^[0-9a-f]+-[0-9a-f]+ ', line):
            block = {'header': line}
            blocks.append(block)
        elif block is not None and ':' in line:
            name, value = line.split(':', 1)
            words = value.split()
            if words and words[0].isdigit():
                block[name] = int(words[0])
    block = max(blocks, key=lambda b: b.get('Size', 0))
    assert abs(block['Size'] * 1024 - 1945686016) < 4 * 2**20
    block['thp_percent'] = 100 * block.get('AnonHugePages', 0) / block['Size']
    return block


families = sorted({family(r) for r in runs if not r['record'] and not r['name'].startswith(('events-', 'share-events-'))})
summary = {f: summarize(f) for f in families}
events = list(dict.fromkeys(r['aux'] for r in runs if r['name'].startswith('events-one-')))
event_data = {config: {e: event_summary(config, e) for e in events} for config in ('one', 'physical')}
placement = {r['name']: table_mapping(r) for r in runs if family(r) in
             ('baseline-one', 'baseline-physical', 'baseline-smt', 'numa-node0', 'numa-node1', 'pages-4k-one', 'pages-4k-physical')}
profiles = {}
for run in runs:
    if not run['record']:
        continue
    text = (RAW / 'record' / (run['name'] + '.report.txt')).read_text()
    lost = int(re.search(r'Total Lost Samples:\s*(\d+)', text)[1])
    assert lost == 0
    hot = re.search(r'^\s*([\d.]+)%\s+\[.\]\s+cubeopt::lanes::run_lanes', text, re.M)
    samples_text = re.search(r'^# Samples:\s*(.*?)\s+of event', text, re.M)[1]
    profiles[run['name']] = dict(lost=lost, samples_label=samples_text,
                                search_loop_percent=float(hot[1]) if hot else None)
digest = hashlib.file_digest(ARCHIVE.open('rb'), 'sha256').hexdigest()
facts = dict(archive=str(ARCHIVE), sha256=digest, source=manifest['source'], runs=len(runs),
             stat_runs=sum(not r['record'] for r in runs), counter_rows=sum(len(r['counters']) for r in runs),
             profile_runs=len(profiles), all_raw_stat_values_match=True, binary_hashes_match=True,
             all_counters_running_percent=100, families=summary, events=event_data,
             table_placement=placement, profiles=profiles)
(OUT / 'audit.json').write_text(json.dumps(facts, indent=2) + '\n')
with (OUT / 'counter-ratios.csv').open('w') as stream:
    writer = csv.writer(stream)
    writer.writerow(['event', 'one_n', 'one_per_ki', 'one_per_cycle', 'physical_n', 'physical_per_ki', 'physical_per_cycle'])
    for e in events:
        a, b = event_data['one'][e], event_data['physical'][e]
        writer.writerow([e, a['n'], a['per_ki'], a['per_cycle'], b['n'], b['per_ki'], b['per_cycle']])


def pct(config, event):
    return 100 * event_data[config][event]['per_cycle']


def thp_range(name):
    values = [placement[r['name']]['thp_percent'] for r in members(name)]
    return f'{min(values):.2f}–{max(values):.2f}%'


text = [f'''# phack hardware-counter analysis

Archive: `egg-solver-pmu-phack-20261106T194245Z.tar.gz`.
SHA256: `{digest}`. Analysis is reproducible with
`/data/agents/codex-resume/analyze-phack.py`; checked aggregates are in
`/data/agents/codex-resume/phack-analysis/audit.json` and `counter-ratios.csv`.

All **242/242 collection runs succeeded**: **236 counted runs**, **621 counter rows**,
and **six sampled profiles**. Every raw stat value matches `runs.json` and every
counter reports **100% running**. All six profiles report **zero lost samples**.
The returned bench/gen/solve SHA256 hashes match the included build manifest.
These checks validate collection consistency; they do not establish that every
derived comparison has enough completed solves or that every CPU event is free of hardware errata.

## Scope and measurement quality

This is the dual-socket Xeon X5650, family 6/model 44, with 12 physical cores and
24 logical CPUs. The tested source is **{manifest['source']}**, plus the included
profiling-control patch, compiled for Westmere. The table is **h6,
1,945,686,016 bytes (1.946 GB / 1.812 GiB)**. This is a separate host/build/table
dataset from the EPYC comparison matrix. It predates the rename and the current
Miri-driven safety fixes; those fixes cannot be credited as measured here.

Each run loads the table, warms for eight seconds, then enables counting through
acknowledged perf control FIFOs for approximately 20 seconds. Observed controlled
windows are 20.0001–20.0037 seconds. Table load/generation are excluded. Hardware
events count user execution. IPC and event/instruction ratios use counters from
the **same run and event group**. Different auxiliary events were measured in
separate windows, usually once; critical events have three repeats. Ratios across
different events are estimates, not simultaneous observations.

Two optional text exports failed: instruction annotations selected the wrong
hot symbol, and sample listings requested a CPU field absent from five profiles.
The raw profiles contain samples. All six annotations and sample listings were
recovered locally using the returned `symfs`; outputs and replay checks are in
`phack-analysis/reporting-replay/`. Focused cycle annotations of the actual
`run_lanes` symbol are also in `phack-analysis/cycles-{{one,physical}}.annotate.txt`.
No host rerun was needed. The collector's “valid” status describes measurement,
not successful generation of every optional postprocessing file.

## Cycle profile and counter evidence

The search loop accounts for **97.34% of one-thread cycle samples** and
**97.36% at 12 physical workers**. Ordinary cycle sampling can skid, so an individual
annotated instruction's percentage is not its exclusive cost. Both recovered
annotations concentrate on the same hot regions, with integer/vector unpacking,
stack traffic and loop control visible. This locates work; it does not prove an
optimization or its gain.

| Metric | One physical worker |12 physical workers | Interpretation |
|---|---:|---:|---|
| Baseline median IPC |{summary['baseline-one']['ipc']:.3f}|{summary['baseline-physical']['ipc']:.3f}|Instructions per counted logical-thread cycle|
| Instruction length decoder stall event / cycles |{pct('one','ILD_STALL.ANY'):.2f}%|{pct('physical','ILD_STALL.ANY'):.2f}%|Front-end signal; overlaps other stalls|
| Reservation station full event / cycles |{pct('one','RESOURCE_STALLS.RS_FULL'):.2f}%|{pct('physical','RESOURCE_STALLS.RS_FULL'):.2f}%|Scheduling/dependency pressure signal|
| Any resource stall event / cycles |{pct('one','RESOURCE_STALLS.ANY'):.2f}%|{pct('physical','RESOURCE_STALLS.ANY'):.2f}%|Includes overlapping resource conditions|
| Load resource stall event / cycles |{pct('one','RESOURCE_STALLS.LOAD'):.2f}%|{pct('physical','RESOURCE_STALLS.LOAD'):.2f}%|Different event from all memory-related delay|
| DTLB page-walk event / cycles |{pct('one','DTLB_LOAD_MISSES.WALK_CYCLES'):.2f}%|{pct('physical','DTLB_LOAD_MISSES.WALK_CYCLES'):.2f}%|Activity while page walks occur, not exclusive stall time|
| Branch misses per 1000 instructions |{event_data['one']['branch-misses:u']['per_ki']:.3f}|{event_data['physical']['branch-misses:u']['per_ki']:.3f}|Median of three within-run ratios|
| Generic LLC-load misses per 1000 instructions |{event_data['one']['LLC-load-misses:u']['per_ki']:.3f}|{event_data['physical']['LLC-load-misses:u']['per_ki']:.3f}|Request-level observation|
| Retired-load LLC misses per 1000 instructions |{event_data['one']['MEM_LOAD_RETIRED.LLC_MISS']['per_ki']:.5f}|{event_data['physical']['MEM_LOAD_RETIRED.LLC_MISS']['per_ki']:.5f}|Retired-load event; not interchangeable with generic LLC misses|
| Offcore demand-read occupancy / cycles |{event_data['one']['OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA']['per_cycle']:.3f}|{event_data['physical']['OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA']['per_cycle']:.3f}|Mean outstanding reads, averaged over all counted cycles|
| Offcore demand-read busy event / cycles |{pct('one','OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA_NOT_EMPTY'):.2f}%|{pct('physical','OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA_NOT_EMPTY'):.2f}%|At least one such request outstanding|

Intel defines these model-specific events in its [Westmere catalogue](https://github.com/intel/perfmon/blob/main/WSM-EP-DP/events/WestmereEP-DP_core.json).
The stall events **must not be added** into a pie chart or interpreted as a
complete partition of execution time. These are older PMU events, not a modern
Top-Down analysis. The very large difference between generic request misses and
retired-load misses means the latter cannot stand for all memory traffic. This
dataset alone does not distinguish speculative requests, prefetch timing and
other event-definition or hardware effects behind that difference.

Dividing separately measured normalized outstanding-read occupancy by busy cycles
gives roughly 1.1 outstanding reads while busy. This is an approximate demand-read
concurrency metric; it excludes other traffic and is **not** total system MLP or
DRAM bandwidth. The interleaved 12-worker run shows remote DRAM responses near half
of the two captured local/remote response classes, consistent with placement.
No memory-controller bandwidth measurement was collected, so bandwidth saturation
is not established.

**Inference:** the strongest optimization leads on this Westmere build are the
search loop's instruction delivery and scheduling/dependency pressure, with page
translation also material. The measurements do not justify treating DRAM bandwidth
as the sole bottleneck or predicting gains on the EPYC build.

## Worker scaling and SMT

Each row is three 20-second measurements. Solves/s is the completion count divided
by the window; Ginst/s is counted instructions divided by the acknowledged window.
Ginst/s measures instruction execution, not useful search progress.

| Configuration | Median solves/s [range] | Median IPC | Median Ginst/s [range] |
|---|---:|---:|---:|''']
for f, label in [('baseline-one','1 worker, local memory'),('baseline-socket','6 physical workers, one socket'),('baseline-physical','12 physical workers, interleaved memory'),('baseline-smt','24 SMT workers, interleaved memory')]:
    s = summary[f]
    text.append(f"|{label}|{s['solves_s']:.2f} [{s['solves_min']:.2f}–{s['solves_max']:.2f}]|{s['ipc']:.3f}|{s['ginst_s']:.2f} [{s['ginst_min']:.2f}–{s['ginst_max']:.2f}]|")
text.append('''
The separate thread sweep observed 1.60 solves/s at 12 workers and 1.20 at 24 workers.
The baseline observed 1.55 and 1.40 respectively. SMT increases counted instruction
execution modestly but does not improve completed-solve throughput in these
windows. Aggregate SMT IPC uses the sum of both logical threads' cycles; it
cannot be read as physical-core IPC.

Use 12 physical workers as the candidate for a longer matched validation, not as
a proven universal setting. One-socket scaling also varies: counted cycles per
worker per wall second are lower at 6 workers than at 1 or 12. The archive does not
include per-window CPU temperature/frequency telemetry or enough controls to
identify the cause. Do not attribute this confidently to NUMA, throttling or
software contention.

## Pages and NUMA placement

Actual large table mapping coverage from the start-of-window `smaps` snapshots:

| Family | Table bytes backed by THP |
|---|---:|''')
for f in ('baseline-one','baseline-physical','pages-4k-one','pages-4k-physical','numa-node1','numa-node0'):
    text.append(f'|{f}|{thp_range(f)}|')
text.append(f'''
The 4 KiB intervention really disabled THP for the table. Median IPC changed from
{summary['baseline-one']['ipc']:.3f} to {summary['pages-4k-one']['ipc']:.3f} at 1 worker and
from {summary['baseline-physical']['ipc']:.3f} to {summary['pages-4k-physical']['ipc']:.3f} at 12 workers.
That is an instruction-execution signal in favor of THP for this workload, but
not a measured percentage improvement in completed solves: one-thread solves/s
remained quantized at 0.15, and 12-worker completion rates overlap.

CPU 1 belongs to node 1 on this host. The local test put the table on node 1; the
remote test put it on node 0. The latter also had only 89–91% THP coverage versus
99.92% locally, so it changes two conditions. Both completed 0.15 solves/s and IPC
was 1.760 local versus 1.743 remote. This cannot establish a clean NUMA latency
penalty. The 12-worker baseline table was split approximately equally between
the nodes, as requested. No 1 GiB hugetlb measurements exist in this archive:
the optional intervention was skipped because suitable existing pages were
unavailable. No kernel huge-page pool was changed.

Linux documents `AnonHugePages` as the amount of anonymous memory backed by THP
in [its THP guide](https://docs.kernel.org/admin-guide/mm/transhuge.html).
These are placement snapshots, not continuous residency measurements.

## Lane sweep: exploratory only

| Lanes | Completed cubes in each 20-second run | Observed median solves/s | Median IPC |
|---:|---|---:|---:|''')
for lanes in (0,1,2,4,8,12,16,24,32,40,48,64):
    s = summary[f'lanes-{lanes}']
    text.append(f"|{lanes}|{', '.join(map(str,s['completed_cubes']))}|{s['solves_s']:.2f}|{s['ipc']:.3f}|")
text.append('''
Eight and 12 lanes show the highest observed completion counts, but there are
only 0–5 completions per window. Zero completions does not mean zero work: those
runs have billions of counted instructions and unfinished searches. Lane count
also changes the instruction mix. Neither IPC nor Ginst/s can replace an exact
completed-work denominator here. Warm-up and end-of-window in-flight work can
change which cubes finish. **Do not claim a 67% speedup or change the default lane
count from this sweep.** A longer, interleaved, same-cube validation is required
before adopting a performance change.

The `share=1` runs use cooperative latency semantics and are not a directly
equivalent throughput comparison with independent stream workers. Their counters
are retained in the raw dataset; no cross-mode speedup is claimed.

## Precise sampling and useful next experiments

All DWARF cycle, LBR, branch PEBS and load PEBS captures succeeded. The two load
profiles are sparse: 55 retired-LLC-miss samples and 41 latency samples above 128 cycles.
The latter have observed weights 129–534 cycles, median 193. This is a threshold-
selected tail with only 41 samples, not the mean or distribution of all loads.
The 55-sample LLC profile is dominated by setup paths and cannot establish their
share of total cache-miss cost accurately.

The existing data supports investigating hot-loop instruction footprint, unpacking
and register/stack pressure first, while retaining THP. It supports testing 8/12
lanes and 12 physical workers with longer, matched solves. Changes still need
correctness/Miri checks and measured 1T and full-machine gains before adoption.
No performance code was changed on the strength of this archive. A complete
repeat of all 242 runs is unnecessary to recover annotations or to retain these
counter findings; any follow-up should target the unresolved throughput ranking
and, if needed, a NUMA test with matched page coverage.

## Full auxiliary counter ratios

The table below contains all 56 auxiliary events. Entries are median within-run
event counts per 1000 instructions; this normalization is only a numerical rate.
Software `task-clock` retains milliseconds as its original unit. Counts of cycles,
bytes and instructions are not interchangeable. The accompanying CSV also contains
event/cycle ratios and repeat counts. Each configuration has the listed number
of independent windows.

| Event | Repeats each |1 worker /1000 insn |12 workers /1000 insn |
|---|---:|---:|---:|''')
for e in events:
    a, b = event_data['one'][e], event_data['physical'][e]
    assert a['n'] == b['n']
    text.append(f"|`{e}`|{a['n']}|{a['per_ki']:.6f}|{b['per_ki']:.6f}|")
content = '\n'.join(text) + '\n'
(OUT / 'PMU_PHACK_ANALYSIS.md').write_text(content)
Path('/home/super/PMU_PHACK_ANALYSIS.md').write_text(content)
Path('/data/agents/f1/final/pmu-phack-analysis.md').write_text(content)
print('Audited 242 runs; wrote /home/super/PMU_PHACK_ANALYSIS.md')
