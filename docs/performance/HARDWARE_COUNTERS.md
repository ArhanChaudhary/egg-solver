# phack hardware-counter analysis

Archive: `egg-solver-pmu-phack-20261106T194245Z.tar.gz`.
SHA256: `e4367ce0595fffacadda8647a22024761a4be542e4aae1040a0514c587fa33a3`. Analysis is reproducible with
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
24 logical CPUs. The tested source is **935cb7fa369248161244507b7c13aa9c405f9ff1**, plus the included
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
`run_lanes` symbol are also in `phack-analysis/cycles-{one,physical}.annotate.txt`.
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
| Baseline median IPC |1.762|1.767|Instructions per counted logical-thread cycle|
| Instruction length decoder stall event / cycles |32.08%|30.90%|Front-end signal; overlaps other stalls|
| Reservation station full event / cycles |20.82%|19.76%|Scheduling/dependency pressure signal|
| Any resource stall event / cycles |21.51%|20.53%|Includes overlapping resource conditions|
| Load resource stall event / cycles |0.26%|0.25%|Different event from all memory-related delay|
| DTLB page-walk event / cycles |6.43%|6.54%|Activity while page walks occur, not exclusive stall time|
| Branch misses per 1000 instructions |1.331|1.342|Median of three within-run ratios|
| Generic LLC-load misses per 1000 instructions |1.198|1.201|Request-level observation|
| Retired-load LLC misses per 1000 instructions |0.00102|0.00152|Retired-load event; not interchangeable with generic LLC misses|
| Offcore demand-read occupancy / cycles |0.592|0.650|Mean outstanding reads, averaged over all counted cycles|
| Offcore demand-read busy event / cycles |53.77%|58.61%|At least one such request outstanding|

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
|---|---:|---:|---:|
|1 worker, local memory|0.15 [0.15–0.15]|1.762|5.35 [5.33–5.36]|
|6 physical workers, one socket|1.05 [0.85–1.05]|1.790|22.50 [21.24–22.75]|
|12 physical workers, interleaved memory|1.55 [1.55–1.60]|1.767|52.07 [51.66–52.17]|
|24 SMT workers, interleaved memory|1.40 [1.30–1.60]|1.022|55.95 [55.94–56.25]|

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
|---|---:|
|baseline-one|97.55–99.92%|
|baseline-physical|99.92–99.92%|
|pages-4k-one|0.00–0.00%|
|pages-4k-physical|0.00–0.00%|
|numa-node1|99.92–99.92%|
|numa-node0|89.37–90.55%|

The 4 KiB intervention really disabled THP for the table. Median IPC changed from
1.762 to 1.610 at 1 worker and
from 1.767 to 1.617 at 12 workers.
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
|---:|---|---:|---:|
|0|0, 0, 0|0.00|2.072|
|1|0, 0, 0|0.00|1.352|
|2|1, 1, 1|0.05|1.945|
|4|1, 1, 1|0.05|1.956|
|8|5, 5, 5|0.25|1.945|
|12|5, 5, 5|0.25|1.916|
|16|4, 4, 4|0.20|1.867|
|24|3, 3, 3|0.15|1.763|
|32|2, 2, 2|0.10|1.679|
|40|4, 4, 4|0.20|1.630|
|48|4, 4, 4|0.20|1.609|
|64|4, 4, 4|0.20|1.595|

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
|---|---:|---:|---:|
|`branches:u`|1|43.287750|43.414955|
|`branch-misses:u`|3|1.331145|1.341987|
|`cache-references:u`|1|2.905948|2.950581|
|`cache-misses:u`|1|1.198571|1.203458|
|`ref-cycles:u`|1|493.950659|611.331675|
|`L1-dcache-loads:u`|1|250.916012|251.220429|
|`L1-dcache-load-misses:u`|1|14.002615|13.917177|
|`LLC-loads:u`|1|2.851037|2.873600|
|`LLC-load-misses:u`|1|1.197693|1.201234|
|`dTLB-loads:u`|1|250.918788|251.225814|
|`dTLB-load-misses:u`|1|2.169763|2.168326|
|`task-clock`|1|0.000187|0.000233|
|`context-switches`|1|0.000000|0.000000|
|`cpu-migrations`|1|0.000000|0.000000|
|`page-faults`|1|0.000043|0.000055|
|`UOPS_EXECUTED.PORT0`|1|271.465967|271.659104|
|`UOPS_EXECUTED.PORT1`|1|284.407529|284.140108|
|`UOPS_EXECUTED.PORT5`|1|312.457111|311.760828|
|`L1I.MISSES`|1|0.799519|1.188991|
|`ARITH.CYCLES_DIV_BUSY`|1|0.005003|0.072243|
|`SSEX_UOPS_RETIRED.VECTOR_INTEGER`|1|253.328904|253.157948|
|`MEM_INST_RETIRED.LOADS`|1|250.917139|251.233587|
|`MEM_INST_RETIRED.STORES`|1|81.185833|81.347010|
|`MEM_LOAD_RETIRED.L1D_HIT`|1|237.451658|237.818922|
|`MEM_LOAD_RETIRED.L2_HIT`|1|6.155979|6.183236|
|`MEM_LOAD_RETIRED.LLC_UNSHARED_HIT`|1|1.438021|1.464296|
|`MEM_LOAD_RETIRED.LLC_MISS`|3|0.001023|0.001524|
|`MEM_LOAD_RETIRED.HIT_LFB`|1|5.625567|5.666890|
|`MEM_LOAD_RETIRED.DTLB_MISS`|1|2.162382|2.156685|
|`MEM_LOAD_RETIRED.OTHER_CORE_L2_HIT_HITM`|1|0.000000|0.000001|
|`L1D.REPL`|1|13.901934|13.910860|
|`L2_RQSTS.LD_HIT`|1|10.370175|10.395427|
|`L2_RQSTS.LD_MISS`|1|2.834606|2.878541|
|`L2_RQSTS.PREFETCH_HIT`|1|0.184530|0.218566|
|`L2_RQSTS.PREFETCH_MISS`|1|0.060993|0.073986|
|`LOAD_HIT_PRE`|1|0.016838|0.015475|
|`DTLB_LOAD_MISSES.ANY`|1|2.169292|2.170434|
|`DTLB_LOAD_MISSES.STLB_HIT`|1|0.016882|0.138268|
|`DTLB_LOAD_MISSES.WALK_COMPLETED`|1|2.167479|2.163726|
|`DTLB_LOAD_MISSES.WALK_CYCLES`|3|36.587808|37.001671|
|`ITLB_MISSES.ANY`|1|0.000102|0.000530|
|`ITLB_MISSES.WALK_CYCLES`|3|0.006612|0.011572|
|`OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA`|3|335.758720|368.080178|
|`OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA_NOT_EMPTY`|3|304.814752|331.532392|
|`RESOURCE_STALLS.ANY`|1|121.979431|116.148142|
|`RESOURCE_STALLS.LOAD`|3|1.474936|1.431617|
|`RESOURCE_STALLS.STORE`|1|0.351922|0.443178|
|`RESOURCE_STALLS.ROB_FULL`|1|2.507491|3.268369|
|`RESOURCE_STALLS.RS_FULL`|1|118.169838|111.995745|
|`UOPS_ISSUED.ANY`|1|1186.148549|1185.964810|
|`UOPS_RETIRED.ANY`|1|1221.435167|1221.344321|
|`UOPS_RETIRED.RETIRE_SLOTS`|1|1120.801986|1120.554166|
|`ILD_STALL.ANY`|1|182.341239|174.828068|
|`ILD_STALL.LCP`|1|0.214666|0.214995|
|`OFFCORE_RESPONSE.DEMAND_DATA_RD.ALL_LOCAL_DRAM_AND_REMOTE_CACHE_HIT`|1|1.197336|0.605355|
|`OFFCORE_RESPONSE.DEMAND_DATA_RD.REMOTE_DRAM`|1|0.000156|0.595690|
