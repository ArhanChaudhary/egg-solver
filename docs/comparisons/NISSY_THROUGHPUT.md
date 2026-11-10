# Nissy versus egg: 500-state batch throughput

[Benchmarks](README.md) · [Timing definitions](METHODS.md) · [Lane and scaling study](../performance/PARALLELISM.md)

Measured **November 8, 2026**, on the **48-core AMD EPYC 9V45 VM**. Each final run completes the same **500 distinct random states**, in repository order, with resident **1 GiB table pages**. The result is the median of **three fresh-process batches per solver and table**. Higher throughput is faster.

## Results

| Table | RAM each (GiB) | Egg (solves/s) | Nissy (solves/s) | Egg advantage |
|---|---:|---:|---:|---:|
| h6 | 2 | 34.71 | 4.89 | 7.10× |
| h7 | 4 | 69.56 | 9.71 | 7.16× |
| h8 | 8 | 146.73 | 14.55 | 10.09× |
| h9 | 15 | 290.33 | 31.29 | 9.28× |
| h10 | 29 | 560.63 | 58.40 | 9.60× |

“RAM each” is the mapped table allocation for either solver, rounded to 1 GiB pages; it excludes other process memory. “Egg advantage” is egg's solves/s divided by nissy's solves/s. It is specific to this host, dataset and tested configurations.

For example, h10 completes all 500 cubes in a median **0.891859 s for egg** and **8.560985 s for nissy**. Dividing 500 by those times gives the rates above. Batch time per cube is not the response time of an individual concurrent solve.

All 15,000 final solutions (30 batches × 500) replayed successfully in **both** independent cube models. Every accepted run also matched the previously cross-checked optimal lengths. This checks the recorded inputs and results, not every possible program execution.

## Selected configurations

Every egg row uses **48 worker threads, 24 lanes per worker, concurrency zero** (the whole batch). Nissy uses cube-worker processes sharing one read-only table; each process can use several threads.

| Tables | Nissy cube workers | Threads per cube | Total threads | Nissy build |
|---|---:|---:|---:|---|
| h6 | 6 | 8 | 48 | Retained PGO |
| h7 | 6 | 8 | 48 | Upstream compiler flags |
| h8 | 6 | 8 | 48 | Retained PGO |
| h9–h10 | 12 | 4 | 48 | Retained PGO |

On the measured VM, the egg command is:

```sh
target/release/solve --table /path/to/table --load map -t 48 -l 24 -c 0 --json < testdata/random_state.txt
```

The table path is explicit. Ordinary disk-file mapping is not equivalent to the benchmark's resident huge-page table. See [load modes](../reference/USER_GUIDE.md#table-files). Global defaults were not changed from one machine's screen.

## All final raw outputs

Each link opens the **untouched 500-result JSONL**, followed by its total. The displayed number is **batch wall seconds**, not seconds per cube. Three repeats are 1,500 solves of the same 500 states.

| Table | Solver | Run 1 (s) | Run 2 (s) | Run 3 (s) |
|---|---|---:|---:|---:|
| h6 | egg | [14.405751919](../data/nissy-throughput-20261108/raw/h6-egg-48_24_0-all500-final-r1.jsonl) | [14.429995963](../data/nissy-throughput-20261108/raw/h6-egg-48_24_0-all500-final-r2.jsonl) | [14.391931563](../data/nissy-throughput-20261108/raw/h6-egg-48_24_0-all500-final-r3.jsonl) |
| h6 | nissy | [102.445444972](../data/nissy-throughput-20261108/raw/h6-nissy-pgo_6_8-all500-final-r1.jsonl) | [102.252591182](../data/nissy-throughput-20261108/raw/h6-nissy-pgo_6_8-all500-final-r2.jsonl) | [102.101806420](../data/nissy-throughput-20261108/raw/h6-nissy-pgo_6_8-all500-final-r3.jsonl) |
| h7 | egg | [7.369067400](../data/nissy-throughput-20261108/raw/h7-egg-48_24_0-all500-final-r1.jsonl) | [7.165320625](../data/nissy-throughput-20261108/raw/h7-egg-48_24_0-all500-final-r2.jsonl) | [7.187604711](../data/nissy-throughput-20261108/raw/h7-egg-48_24_0-all500-final-r3.jsonl) |
| h7 | nissy | [51.221875536](../data/nissy-throughput-20261108/raw/h7-nissy-shipped_6_8-all500-final-r1.jsonl) | [51.486320916](../data/nissy-throughput-20261108/raw/h7-nissy-shipped_6_8-all500-final-r2.jsonl) | [51.545883987](../data/nissy-throughput-20261108/raw/h7-nissy-shipped_6_8-all500-final-r3.jsonl) |
| h8 | egg | [3.402620848](../data/nissy-throughput-20261108/raw/h8-egg-48_24_0-all500-final-r1.jsonl) | [3.454722068](../data/nissy-throughput-20261108/raw/h8-egg-48_24_0-all500-final-r2.jsonl) | [3.407541722](../data/nissy-throughput-20261108/raw/h8-egg-48_24_0-all500-final-r3.jsonl) |
| h8 | nissy | [34.338833650](../data/nissy-throughput-20261108/raw/h8-nissy-pgo_6_8-all500-final-r1.jsonl) | [34.367007481](../data/nissy-throughput-20261108/raw/h8-nissy-pgo_6_8-all500-final-r2.jsonl) | [34.396046823](../data/nissy-throughput-20261108/raw/h8-nissy-pgo_6_8-all500-final-r3.jsonl) |
| h9 | egg | [1.735110983](../data/nissy-throughput-20261108/raw/h9-egg-48_24_0-all500-final-r1.jsonl) | [1.707409881](../data/nissy-throughput-20261108/raw/h9-egg-48_24_0-all500-final-r2.jsonl) | [1.722188139](../data/nissy-throughput-20261108/raw/h9-egg-48_24_0-all500-final-r3.jsonl) |
| h9 | nissy | [15.984476809](../data/nissy-throughput-20261108/raw/h9-nissy-pgo_12_4-all500-final-r1.jsonl) | [15.977747455](../data/nissy-throughput-20261108/raw/h9-nissy-pgo_12_4-all500-final-r2.jsonl) | [15.919720810](../data/nissy-throughput-20261108/raw/h9-nissy-pgo_12_4-all500-final-r3.jsonl) |
| h10 | egg | [0.898266926](../data/nissy-throughput-20261108/raw/h10-egg-48_24_0-all500-final-r1.jsonl) | [0.891859069](../data/nissy-throughput-20261108/raw/h10-egg-48_24_0-all500-final-r2.jsonl) | [0.887208175](../data/nissy-throughput-20261108/raw/h10-egg-48_24_0-all500-final-r3.jsonl) |
| h10 | nissy | [8.551037397](../data/nissy-throughput-20261108/raw/h10-nissy-pgo_12_4-all500-final-r1.jsonl) | [8.560984671](../data/nissy-throughput-20261108/raw/h10-nissy-pgo_12_4-all500-final-r2.jsonl) | [8.561392890](../data/nissy-throughput-20261108/raw/h10-nissy-pgo_12_4-all500-final-r3.jsonl) |

## Timing, inputs and checks

The timer starts after input parsing, table loading and validation. It includes worker creation/join, solution replay, output and the finishing tail. Throughput is **500 ÷ complete batch wall seconds**. Summed overlapping cube durations are not divided by a nominal worker count.

The input is `testdata/random_state.txt`. Its SHA-256 is:

```text
616810c31d090a78ea1357684750642ab017ff394bfa08ba131bb65cc6905de3
```

Runs hold exclusive benchmark locks, check unrelated CPU use, verify table integrity and record actual HugeTLB coverage. There are no hardware PMU counters on this VM. [Raw-index metadata](../data/nissy-throughput-20261108/run-index.csv) identifies all **305 accepted measurements**, including screens and separately labeled exploratory runs. [Cross-replay](../data/nissy-throughput-20261108/cross-replay.json) and [grid/hash/page audit](../data/nissy-throughput-20261108/evidence-audit.json) record acceptance checks.

## Tuning and source limits

Egg's measured CLI inputs match published source `1b243c0` except a generator help-text edit that is not linked into `solve`. The raw records cite that source as `947c20d`, its ID before the repository history was rebuilt. Binary and source hashes are in the [manifest](../data/nissy-throughput-20261108/manifest.json). The native release uses `target-cpu=native`.

Production egg screens used the shipped CLI at 8, 16, 24, 32, 40, 48, 64, 96 and 128 lanes, then tested 8–48 worker threads at the winning lane count. **Lanes 12 and 20 were not tested.** An ordinary `Pool::run` research caller had different timings and lane rankings from the CLI's fixed-stack-placement path. Its raw results are preserved as exploratory evidence and excluded from final configuration selection.

Nissy-core was pinned to `1f7e8e9`. Five builds were screened on a fixed 100-state pilot with 48 single-thread cube workers: upstream flags, native, retained PGO, rank-initialized native, and a rank-initialized compile-time single-thread build. The selected build's allocations were checked on all 500 states, including all factorizations of 48 threads. Builds and allocations were not exhaustively crossed. These are the best observed settings in the declared grid, not a global optimum.

**PGO overlap:** the retained nissy profile was trained on a subset of the legacy training file, which overlaps this 500-state evaluation. This is not a held-out result. Unmodified nissy builds also retain the identified uninitialized task-rank issue; patched controls and the one-line patch are included. Correct replay does not establish absence of undefined behavior in upstream C.

The implementations use their own table layouts and slightly different data sizes. Exact file bytes, logical data bytes, base values and page padding are retained in [table metadata](../data/nissy-throughput-20261108/status.json) and the [restoration record](../data/nissy-throughput-20261108/restored-tables.json).

## Recheck and scope

[Analysis JSON](../data/nissy-throughput-20261108/analysis.json) contains every final wall, range and rate. [Reproduction](../reproduce/README.md) explains how to recompute the table without rerunning solvers. [The evidence directory](../data/nissy-throughput-20261108/README.md) contains the harnesses, datasets, compiler records and rejected attempts.

These figures do not replace the differently measured historical latency/stream results. They are not a new measurement of closed-source cubeopt, the reported MacBook release or another CPU.
