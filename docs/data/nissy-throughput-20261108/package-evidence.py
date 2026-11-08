from pathlib import Path
import shutil,json,hashlib
R=Path('/data/agents/nissy-throughput-20261108');E=Path('/data/agents/cubeopt-current/egg');A=json.loads((R/'analysis.json').read_text())
if A['status']!='COMPLETE' or json.loads((R/'restored-tables.json').read_text())['status']!='PASS':raise SystemExit('Incomplete results or page restoration')
for name in ['throughput.rs','verify_solutions.rs']:
 if (E/'examples'/name).exists():shutil.copy2(E/'examples'/name,R/'src'/name)
for root in [E,Path('/home/super/egg-solver')]:
 dst=root/'docs/data/nissy-throughput-20261108';dst.mkdir(parents=True,exist_ok=True)
 for name in ['raw','meta','datasets','rejected']:shutil.copytree(R/name,dst/name,dirs_exist_ok=True)
 for p in R.iterdir():
  if p.is_file() and p.suffix in ['.json','.csv','.log','.py','.txt']:shutil.copy2(p,dst/p.name)
 src=dst/'src';src.mkdir(exist_ok=True)
 for p in (R/'src').iterdir():
  if p.is_file() and p.name not in ['NISSY_METHODS.md','PARALLELISM_DRAFT.md','probe-cubeopt-slots-inactive.mjs']:shutil.copy2(p,src/p.name)
 shutil.copy2(R/'src/nissy-fixed/LICENSE',src/'NISSY_LICENSE.txt')
 provenance=dst/'provenance';provenance.mkdir(exist_ok=True)
 for name in ['pgo-build.sh','pgo-train.sh','pgo-train2.sh','nissy-use.log','train-h48h7-1.log','train-h48h7-8.log']:
  shutil.copy2(Path('/data/agents/f1/pgo')/name,provenance/name)
 (dst/'README.md').write_text(f'''# November 8 finite-batch throughput and parallelism evidence

Read the [nissy comparison](../../comparisons/NISSY_THROUGHPUT.md) for the final table and links to every final raw output. [Parallelism](../../performance/PARALLELISM.md) explains the lane/worker distinction, cubeopt counters, lane screen and single-cube scaling.

This archive contains {A['accepted_measurements']} accepted measurements: production configuration screens, final triplicates, the single-cube scaling subset, and separately labeled exploratory probe runs. The [run index](run-index.csv) lists every accepted run; [analysis](analysis.json) retains the final aggregates. Labels `tune-cli-lanes`, `tune-cli-threads` and `final-rN` identify production CLI measurements. The ordinary `Pool::run` research probe is slower here and is excluded from production configuration selection.

- `raw/`: untouched per-state JSONL, stderr and total lines.
- `meta/`: exact commands, validation, resource accounting, page samples and output hashes.
- `datasets/`: the exact 500 inputs, pilot, pilot indices and three-state repeated scaling subset.
- `manifest.json`: source, compiler, host and binary hashes; includes the PGO training-overlap disclosure.
- `cross-replay.json`: all 15,000 final solutions pass both independent cube models.
- `restored-tables.json`: all five requested egg tables resident on 1 GiB pages; zero 2 MiB pages. H11 source and reconstruction remain on disk.
- `src/`: measured probe, linted research sources, C harness, metadata-preserving huge-page copy helper and the one-line nissy rank patch.
- `provenance/`: retained legacy nissy PGO build/training scripts and logs. Their old “disjoint” description referred to the former smaller evaluation, not the current all-500 dataset.
- `rejected/`: original setup failure and the subsecond run missed by the old page sampler, excluded from accepted results.

## Rechecking the evidence

Run `python3 analyze.py --require-complete` from this directory. It checks raw-output SHA-256 values before deriving the final aggregates. The controller, cross-replay and restoration scripts preserve this VM's absolute paths; they are research records, not portable install scripts. Tables, binaries, object files and PGO profile data are retained on the research VM and are not bundled here.

The C harness links to nissy's public C API. Match the nissy commit and build flags in the manifest/provenance records, compile with `-O3 -pthread` and the upstream header/object, then run `nissy-batch h48hN TABLE INPUT WORKERS THREADS_PER_CUBE`. The rank patch is explicit; do not silently apply it to a result labeled upstream/default or retained PGO. The compile-time single-thread control uses `THREADS=1` and `NISSY_COMPILED_THREADS=1`.

The Rust verifier/probe can be copied temporarily into a checkout's `examples/` and built with `cargo build --release --example NAME`. The CLI used for final timing is the ordinary shipped `solve` program. Its native measured binary and the research probe have separate hashes. Rebuilding a program or PGO profile can change generated code/layout and timing; matching source alone is not a promise of an identical binary or result.

The original cubeopt query-slot probe changes module exports only and uses the pinned original table. Its JSON/disassembly are counter-identity evidence, not a 1 GiB-page WASM timing benchmark. The original oracle helper lives at the recorded research path; it is not distributed as new solver code.
''')
 files=sorted(p for p in dst.rglob('*') if p.is_file() and p.name!='SHA256SUMS')
 (dst/'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+str(p.relative_to(dst))+'\n' for p in files))
 print(dst,len(files))
