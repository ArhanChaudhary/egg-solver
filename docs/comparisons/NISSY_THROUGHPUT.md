# Nissy versus egg-solver: 500-state throughput, h6–h10

2026-11-08. Complete: five table sizes, three final 500-state batches per solver and size, on the same 48-core VM. These results describe this dataset, machine, source and configuration grid.

## Final throughput

Medians of three complete batch walls, after tuning. Each final process solves all 500 states. Higher solves/s is better.

| h | Egg batch wall | Nissy batch wall | Egg solves/s | Nissy solves/s | Nissy time / egg time |
|---:|---:|---:|---:|---:|---:|
| 6 | 14.405752 s | 102.252591 s | 34.71 | 4.89 | 7.10× |
| 7 | 7.187605 s | 51.486321 s | 69.56 | 9.71 | 7.16× |
| 8 | 3.407542 s | 34.367007 s | 146.73 | 14.55 | 10.09× |
| 9 | 1.722188 s | 15.977747 s | 290.33 | 31.29 | 9.28× |
| 10 | 0.891859 s | 8.560985 s | 560.63 | 58.40 | 9.60× |

Every selected egg configuration uses 48 workers, 24 lanes per worker, and concurrency zero (the complete input batch). Nissy uses 6 cube workers × 8 threads/cube for h6–h8, and 12 × 4 for h9–h10. Its selected build is retained PGO for h6/h8/h9/h10 and upstream default compiler flags for h7. PGO training overlap and the upstream rank issue are disclosed below.

## Running the measured egg configuration

On this 48-core VM, with an already resident table:

```bash
target/release/solve --table /path/to/table --load map -t 48 -l 24 -c 0 --json < testdata/random_state.txt
```

Pass the table path explicitly. The lane result is specific to this machine and batch. Existing global defaults are preserved.

## Every final raw output

The links contain unedited per-state results and the total line. The value shown is that run's complete batch wall. Three repeats are 1,500 solves of the same 500 distinct states, not 1,500 distinct states.

| h | Solver | Configuration | Repeat 1, raw | Repeat 2, raw | Repeat 3, raw |
|---:|---|---|---:|---:|---:|
| 6 | egg | `48_24_0` | [14.405751919 s](../data/nissy-throughput-20261108/raw/h6-egg-48_24_0-all500-final-r1.jsonl) | [14.429995963 s](../data/nissy-throughput-20261108/raw/h6-egg-48_24_0-all500-final-r2.jsonl) | [14.391931563 s](../data/nissy-throughput-20261108/raw/h6-egg-48_24_0-all500-final-r3.jsonl) |
| 6 | nissy | `pgo_6_8` | [102.445444972 s](../data/nissy-throughput-20261108/raw/h6-nissy-pgo_6_8-all500-final-r1.jsonl) | [102.252591182 s](../data/nissy-throughput-20261108/raw/h6-nissy-pgo_6_8-all500-final-r2.jsonl) | [102.101806420 s](../data/nissy-throughput-20261108/raw/h6-nissy-pgo_6_8-all500-final-r3.jsonl) |
| 7 | egg | `48_24_0` | [7.369067400 s](../data/nissy-throughput-20261108/raw/h7-egg-48_24_0-all500-final-r1.jsonl) | [7.165320625 s](../data/nissy-throughput-20261108/raw/h7-egg-48_24_0-all500-final-r2.jsonl) | [7.187604711 s](../data/nissy-throughput-20261108/raw/h7-egg-48_24_0-all500-final-r3.jsonl) |
| 7 | nissy | `shipped_6_8` | [51.221875536 s](../data/nissy-throughput-20261108/raw/h7-nissy-shipped_6_8-all500-final-r1.jsonl) | [51.486320916 s](../data/nissy-throughput-20261108/raw/h7-nissy-shipped_6_8-all500-final-r2.jsonl) | [51.545883987 s](../data/nissy-throughput-20261108/raw/h7-nissy-shipped_6_8-all500-final-r3.jsonl) |
| 8 | egg | `48_24_0` | [3.402620848 s](../data/nissy-throughput-20261108/raw/h8-egg-48_24_0-all500-final-r1.jsonl) | [3.454722068 s](../data/nissy-throughput-20261108/raw/h8-egg-48_24_0-all500-final-r2.jsonl) | [3.407541722 s](../data/nissy-throughput-20261108/raw/h8-egg-48_24_0-all500-final-r3.jsonl) |
| 8 | nissy | `pgo_6_8` | [34.338833650 s](../data/nissy-throughput-20261108/raw/h8-nissy-pgo_6_8-all500-final-r1.jsonl) | [34.367007481 s](../data/nissy-throughput-20261108/raw/h8-nissy-pgo_6_8-all500-final-r2.jsonl) | [34.396046823 s](../data/nissy-throughput-20261108/raw/h8-nissy-pgo_6_8-all500-final-r3.jsonl) |
| 9 | egg | `48_24_0` | [1.735110983 s](../data/nissy-throughput-20261108/raw/h9-egg-48_24_0-all500-final-r1.jsonl) | [1.707409881 s](../data/nissy-throughput-20261108/raw/h9-egg-48_24_0-all500-final-r2.jsonl) | [1.722188139 s](../data/nissy-throughput-20261108/raw/h9-egg-48_24_0-all500-final-r3.jsonl) |
| 9 | nissy | `pgo_12_4` | [15.984476809 s](../data/nissy-throughput-20261108/raw/h9-nissy-pgo_12_4-all500-final-r1.jsonl) | [15.977747455 s](../data/nissy-throughput-20261108/raw/h9-nissy-pgo_12_4-all500-final-r2.jsonl) | [15.919720810 s](../data/nissy-throughput-20261108/raw/h9-nissy-pgo_12_4-all500-final-r3.jsonl) |
| 10 | egg | `48_24_0` | [0.898266926 s](../data/nissy-throughput-20261108/raw/h10-egg-48_24_0-all500-final-r1.jsonl) | [0.891859069 s](../data/nissy-throughput-20261108/raw/h10-egg-48_24_0-all500-final-r2.jsonl) | [0.887208175 s](../data/nissy-throughput-20261108/raw/h10-egg-48_24_0-all500-final-r3.jsonl) |
| 10 | nissy | `pgo_12_4` | [8.551037397 s](../data/nissy-throughput-20261108/raw/h10-nissy-pgo_12_4-all500-final-r1.jsonl) | [8.560984671 s](../data/nissy-throughput-20261108/raw/h10-nissy-pgo_12_4-all500-final-r2.jsonl) | [8.561392890 s](../data/nissy-throughput-20261108/raw/h10-nissy-pgo_12_4-all500-final-r3.jsonl) |

All 305 accepted measurements, including the exploratory probe and configuration screens, appear in [the run index](../data/nissy-throughput-20261108/run-index.csv). The [analysis JSON](../data/nissy-throughput-20261108/analysis.json), [manifest](../data/nissy-throughput-20261108/manifest.json) and [cross-replay result](../data/nissy-throughput-20261108/cross-replay.json) are machine-readable.

## Workload and metric

Both solvers receive the same 500 states, in the original order of `testdata/random_state.txt`. The SHA-256 is `616810c31d090a78ea1357684750642ab017ff394bfa08ba131bb65cc6905de3`.

The primary measurement is the wall time to finish the entire finite batch. Throughput is `500 / batch wall seconds`; milliseconds per cube is `1000 × batch wall / 500`. This includes the idle tail, worker startup and shutdown, verification, and output. Input parsing, table validation and loading occur before the batch timer. Summed overlapping cube latencies are not divided by a nominal thread count.

Every benchmark pruning table is mapped from **1 GiB hugetlbfs**. The runner records actual HugeTLB mapping coverage for each accepted run. The 2 MiB huge-page pool is zero. Tables remain resident in RAM throughout a run.

The machine is the 48-core AMD EPYC VM identified in the retained manifest. There are no hardware PMU counters here. Timing runs and builds hold the same exclusive locks; unrelated CPU use is checked before and around each measurement.

## Source and tuning

Egg's published upstream is `https://github.com/ArhanChaudhary/egg-solver`, checked at `947c20d93564f150aad9c292486ab730a329b86c`. All recorded Rust/build inputs for the retained native CLI match the published-base worktree except `src/bin/gen.rs`, whose help text changed and is not linked into `solve`. The CLI and research-probe binary hashes are retained separately.

The native build uses the project's release profile and `target-cpu=native`. All primary egg configuration screens and final rows use the shipped CLI with explicit table, worker count, lane count and concurrency zero. The screen tests 8–128 lanes per worker and worker counts from 8 to 48. The CLI fixes the placement of worker state and stack frames; this is not CPU-affinity pinning. A separate research probe uses the public APIs for scheduling/counter evidence. Its ordinary `Pool::run` path produced substantially different timing and rankings from the CLI's `run_pinned` path, so its times are retained as exploratory evidence and excluded from production throughput selection.

Nissy-core is checked at `1f7e8e937d0e71e7eb3b5deab38b34d0a0159d88`. A C harness calls its public API, sharing one read-only table between cube-worker processes. It screens upstream `-O3 -mavx2`, native, retained PGO, rank-initialized native, and a rank-initialized compile-time single-thread build. Build screening uses a declared 100-state pilot. Worker-allocation candidates for the selected build are then checked on all 500 states: independent workers and all factorizations of 48 CPU workers into cube workers × threads per cube. CPU allocation never exceeds 48.

Two limitations are explicit. The retained PGO profile was trained on a subset of the legacy training file, which overlaps the present 500-state evaluation; it is not a held-out PGO result. The unmodified Nissy builds also retain the previously identified uninitialized task-rank issue. Rank-initialized controls and the one-line patch are included. Output replay does not establish that upstream C has no undefined behavior.

After configuration selection, three fresh processes per solver and table run with alternating solver order. Reported final values are medians of those three complete batches, with all individual walls and raw outputs linked. These are the best observed configurations within the declared grid, not a proof of a global optimum or a prediction for another CPU.

## Tables and correctness

| h | Egg pruning-data bytes | Nissy table file bytes | Resident allocation per solver | Page size |
|---:|---:|---:|---:|---:|
| 6 | 1,945,681,920 | 1,897,951,528 | 2 GiB | 1 GiB |
| 7 | 3,891,363,840 | 3,793,842,344 | 4 GiB | 1 GiB |
| 8 | 7,782,727,680 | 7,585,624,040 | 8 GiB | 1 GiB |
| 9 | 15,565,455,360 | 15,169,187,432 | 15 GiB | 1 GiB |
| 10 | 31,130,910,720 | 30,336,314,216 | 29 GiB | 1 GiB |

Egg's persistent h7/h8 working-copy files contain alignment padding; the exact file sizes and logical pruning-data sizes are recorded separately in the per-table metadata and `restored-tables.json`. Mapped copies are rounded to whole 1 GiB pages, with the egg metadata trailer relocated to the end. Every table is validated before use. These are each implementation's native H48 tables at the same h; their layouts and byte sizes differ.

Each accepted run must contain every requested input index exactly once, replay all solutions successfully, and match the previously cross-checked optimal lengths. All 15,000 final solutions additionally replayed in both independent cube models: [cross-replay evidence](../data/nissy-throughput-20261108/cross-replay.json). Matching lengths and replays provide dataset-specific correctness evidence; timing is not a universal optimality or safety proof.

## Interpreting this comparison

The historical reports contain different source builds, sample sizes and latency/stream formulas. Their ratios must not be substituted for this finite-batch result. In particular, this is a fresh nissy comparison, not a new measurement of closed-source cubeopt or the reported MacBook release. These numbers do not establish a cross-machine or universal solver ranking.

The build screen and allocation screen are separate: builds were screened with 48 independent single-thread workers, then the selected build's allocations were screened on all 500 states. Every compiler build was not exhaustively crossed with every allocation. Lane counts 12 and 20 were not in this experiment's explicit lane grid. “Best observed” means exactly the recorded search, not every possible parameter combination.

## Raw evidence

The live evidence directory is `../data/nissy-throughput-20261108/`:

- `raw/`: unedited per-state JSONL and stderr, including configuration screens.
- `meta/`: exact commands, validity decisions, raw-output hashes, resource use, and page samples.
- `datasets/`: all 500 states, the fixed pilot, pilot indices, and the separate scaling subset.
- `manifest.json`: machine, compiler, source identities, binary hashes and PGO provenance.
- `analysis.json` and `run-index.csv`: derived summaries; `analyze.py` verifies raw hashes before summarizing.
- `src/`: research harnesses, the measured Rust probe source, rank-initialization patch, and counter probe.

Setup failures, when present, remain in `rejected/` and are excluded from results. A controller checkpoint preserves a fully completed child measurement without changing its raw output or batch timer; such metadata explicitly omits the unavailable parent process wall.
