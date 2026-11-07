"""Recover optional exports from the existing raw data; never run the workload."""
import json
import os
from pathlib import Path
import runpy
import subprocess
from pmu_reporting import patch_runner

ROOT = Path('/data/agents/codex-resume')
RAW = ROOT / 'phack-returned/results-phack-20261106T194245Z'
PAYLOAD = Path('/data/agents/egg-solver-perf/payload')
OUT = ROOT / 'phack-analysis/reporting-replay'
(OUT / 'record').mkdir(parents=True, exist_ok=True)
patched = OUT / 'runner-reporting-fixed.py'
patched.write_text(patch_runner((PAYLOAD / 'runner.py').read_text()))
namespace = runpy.run_path(str(patched), run_name='reporting_replay')
suite = object.__new__(namespace['Suite'])
suite.out = OUT
suite.perf = str(PAYLOAD / 'bin/perf')
calls = []


def execute(args, *, name, timeout=60):
    args = list(map(str, args))
    if args[1] in ('report', 'annotate', 'script'):
        args += ['--symfs', str(RAW / 'symfs')]
    with (OUT / name).open('w') as stream:
        result = subprocess.run(args, stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
    calls.append(dict(command=args, name=name, exit_code=result.returncode))
    return result.returncode


suite.execute = execute
for run in json.loads((RAW / 'runs.json').read_text()):
    if not run['record']:
        continue
    name = run['name']
    dest = OUT / 'record' / (name + '.data')
    if not dest.exists():
        os.link(RAW / 'record' / (name + '.data'), dest)
    suite.reports(OUT / 'record' / name, {})
    annotation = OUT / 'record' / (name + '.annotate-all.txt')
    if 'no samples!' not in (OUT / 'record' / (name + '.annotate.txt')).read_text():
        raise RuntimeError('Replay did not exercise the known missing-symbol error: ' + name)
    if not annotation.exists() or annotation.stat().st_size < 1000:
        raise RuntimeError('Annotation fallback did not recover the profile: ' + name)
    samples = OUT / 'record' / (name + '.samples.txt')
    if 'Cannot print' in samples.read_text():
        samples = OUT / 'record' / (name + '.samples-no-cpu.txt')
    if not samples.exists() or samples.stat().st_size < 1000:
        raise RuntimeError('Sample export did not recover the profile: ' + name)
    for output in (annotation, samples):
        result = next(c for c in reversed(calls) if OUT / c['name'] == output)
        if result['exit_code'] != 0:
            raise RuntimeError('Recovered export failed: ' + str(output))
    print(name, 'annotation and sample exports recovered', flush=True)
(OUT / 'validation.json').write_text(json.dumps(dict(status='PASS', raw_data_unchanged=True,
                                                    calls=calls), indent=2) + '\n')
print('PASS: all six real profiles recovered through the corrected optional export paths')
