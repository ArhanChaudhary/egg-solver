# Raw runs of the speed measurements of November 25, 2026

[How fast egg-solver is](../../performance/SPEED.md) · [Benchmark catalog](../../BENCHMARK_CATALOG.md)

Every run is one `solve --json` process: one line per cube (`moves`, `length`, `nodes`, `seconds`, `ok`), then a `total` line (`wall`, `cubes`, `nodes_per_cube`, `threads`, `lanes`, `concurrency`). Source: commit `17386a2`, built with `cargo build --release --locked` (Rust 1.99.1; for this CPU on the server, NEON on the M2). Input: `testdata/random_state.txt` (SHA-256 `616810c31d090a78ea1357684750642ab017ff394bfa08ba131bb65cc6905de3`).

| Path | Content |
|---|---|
| `epyc/hK-T-rR.jsonl` | The 48-core EPYC 9V45 server, table hK: one cube at a time at T = 1, 8, 14 or 48 threads, or `b48`, a batch at 48 threads; round R = 1 or 2 |
| `epyc/hK-setup.txt` | `solve -v`'s report of the table's load, pages and lanes |
| `epyc/progress.log` | When each run ended, in order |
| `m2/hK-tT-cC.jsonl` | The Apple M2: T threads, concurrency C (1: one cube at a time; 0: a batch) |
| `m2/hK-setup.txt`, `m2/log.txt` | The same for the M2 |
| `run-char.sh`, `m2-char.sh` | The scripts that made them (paths of the machines they ran on) |
| `summarize.py`, `tables.py` | `python3 tables.py epyc m2` prints the tables of the speed page from these files |
