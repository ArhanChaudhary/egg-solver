# November 8 finite-batch throughput and parallelism evidence

Read the [nissy comparison](../../comparisons/NISSY_THROUGHPUT.md) for the final table and links to every final raw output. [Parallelism](../../performance/PARALLELISM.md) explains the lane/worker distinction, cubeopt counters, lane screen and single-cube scaling.

This archive contains 305 accepted measurements: production configuration screens, final triplicates, the single-cube scaling subset, and separately labeled exploratory probe runs. The [run index](run-index.csv) lists every accepted run; [analysis](analysis.json) retains the final aggregates. Labels `tune-cli-lanes`, `tune-cli-threads` and `final-rN` identify production CLI measurements. The ordinary `Pool::run` research probe is slower here and is excluded from production configuration selection.

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
