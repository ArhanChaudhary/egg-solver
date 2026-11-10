# Workers, lanes and search parallelism

[Performance studies](README.md) · [Batch throughput](../comparisons/NISSY_THROUGHPUT.md) · [How the search works](../guide/HOW_IT_WORKS.md)

Measured **November 8, 2026**, on the **48-core EPYC VM**. Forty-eight is the machine's worker budget, not a limit on search lanes. Explicit lane counts through 128 were tested; **24 lanes per worker won every h6–h10 finite-batch screen**.

## What is being parallelized?

| Quantity | Meaning |
|---|---|
| CPU worker | Operating-system thread executing search code |
| Software lane | An interleaved search context within a worker, helping overlap table accesses |
| SIMD operation | Vector computation on cube data; distinct from workers and lanes |
| Prefix job | A subtree that workers can claim and split independently |

A worker visits lanes in sequence while their memory accesses overlap. Increasing lanes does not add CPU cores, and each lane needs state storage and bookkeeping, so filling more slots does not guarantee higher throughput. A shared search can generate many jobs from one cube, so it is not limited to one independent input per lane.

The default lane counts are 40 for h0–h8, 32 for h9/h10 and 24 for h11. The built-in tuner currently stops its candidate list at 48; `-l` accepts larger values. Those defaults and candidates are heuristics, not tree-width limits. They were not changed from a single host's new batch screen.

## Production CLI lane screen

Every cell is **complete-batch wall seconds for 500 states**, at **48 workers** and concurrency zero. **Lower is faster.** Each tuning cell is one process, not a repeated confidence estimate. All tables use resident 1 GiB pages.

| Lanes per worker | h6 (s) | h7 (s) | h8 (s) | h9 (s) | h10 (s) |
|---:|---:|---:|---:|---:|---:|
| 8 | 23.6840 | 12.1135 | 5.6396 | 2.7792 | 1.4491 |
| 16 | 16.0036 | 8.1643 | 3.8440 | 1.8881 | 0.9904 |
| 24 | 14.4613 | 7.3027 | 3.4157 | 1.7165 | 0.8918 |
| 32 | 14.6744 | 7.3976 | 3.5035 | 1.7338 | 0.9134 |
| 40 | 14.9207 | 7.6410 | 3.5987 | 1.7976 | 0.9410 |
| 48 | 15.5517 | 7.8780 | 3.7333 | 1.8625 | 0.9717 |
| 64 | 16.0705 | 8.1591 | 3.8336 | 1.9486 | 1.0103 |
| 96 | 16.2510 | 8.3361 | 3.8817 | 1.9754 | 1.0488 |
| 128 | 16.6385 | 8.1849 | 3.9488 | 1.9705 | 1.0363 |

The declared grid did not test 12 or 20 lanes. More than 48 lanes was supported but slower here. This does not determine the best lane count for every CPU or workload. [The throughput report](../comparisons/NISSY_THROUGHPUT.md) gives the three final repeated batches at the selected setting.

## Parallelism within one cube

These runs use the **production CLI**, concurrency one and a fixed 24 lanes per worker, the count selected by the throughput screen and not retuned for each row. They solve indices 0, 8 and 10 (optimal lengths 18, 16 and 19) ten times in the same order: **30 solves of three distinct states**. Each row is one process. This is a small scaling sample, not the complete 500-state benchmark or a confidence interval.

Speedup is the one-worker batch time divided by the row's batch time. Efficiency is speedup divided by worker count. “Extra nodes” is the change in counted work relative to one worker. Time/cube is the 30-solve wall time divided by 30; lower is faster.

### h6

| Workers | Time/cube (ms) | Speedup | Efficiency | Extra nodes |
|---:|---:|---:|---:|---:|
| 1 | 2547.695 | 1.00× | 100.0% | +0.0% |
| 2 | 1282.842 | 1.99× | 99.3% | +0.7% |
| 4 | 647.304 | 3.94× | 98.4% | +0.2% |
| 8 | 328.917 | 7.75× | 96.8% | +2.1% |
| 16 | 172.266 | 14.79× | 92.4% | +5.7% |
| 24 | 120.665 | 21.11× | 88.0% | +9.2% |
| 32 | 98.366 | 25.90× | 80.9% | +12.2% |
| 48 | 76.554 | 33.28× | 69.3% | +19.2% |

### h10

| Workers | Time/cube (ms) | Speedup | Efficiency | Extra nodes |
|---:|---:|---:|---:|---:|
| 1 | 164.672 | 1.00× | 100.0% | +0.0% |
| 2 | 83.805 | 1.96× | 98.2% | -0.0% |
| 4 | 42.276 | 3.90× | 97.4% | +0.3% |
| 8 | 21.537 | 7.65× | 95.6% | +0.8% |
| 16 | 11.146 | 14.77× | 92.3% | +2.0% |
| 24 | 7.707 | 21.37× | 89.0% | +3.1% |
| 32 | 6.544 | 25.16× | 78.6% | +4.3% |
| 48 | 5.423 | 30.37× | 63.3% | +6.7% |

At 48 workers the measured speedups are **33.28× (h6)** and **30.37× (h10)**. Additional speculative work, imbalance and coordination limit scaling. Because node counts change too, these figures do not identify a universal parallel fraction with an Amdahl fit.

Twenty-four lanes × 48 workers configures 1,152 contexts; it does not establish that all are continuously active. Both implementations can prepare tens of thousands of canonical prefix jobs (43,254 before pruning) and split work further. Useful width depends on pruning, target depth, the remaining tail and scheduling cost. This CPU experiment does not measure a GPU's useful width.

The fixed-width software pipeline is not the limit of the tree: prefix subtrees can be searched independently within an IDA* target depth. The restrictions are elsewhere:

- A shortest-solution proof must finish all required shallower work. A found deeper solution cannot simply skip that proof.
- Pruning makes subtrees very uneven. The final few surviving branches can limit useful width.
- Job creation, sorting, distribution and synchronization have costs.
- The programs stop after an optimal solution is established; more workers may perform extra work before the stop reaches them.
- Memory bandwidth, dependency latency and the worker's state footprint can limit throughput even with occupied contexts.

[Analysis JSON](../data/nissy-throughput-20261108/analysis.json) retains every speedup and node ratio. Raw scaling files are indexed as `*-single30-parallelism` in [the run index](../data/nissy-throughput-20261108/run-index.csv).

## Cubeopt's occupied and empty slots

The supplied Linux cube48opt10 has **eight software query slots per worker**, independent of its CPU thread count. Its printed `N/E node(s)` counters are **occupied / empty slot visits**, not occupied / total. In the binary's `step1_query`, addresses `0x5594–0x55af` increment the counter at statistics offset zero for a null current-node pointer and the counter at offset eight for a non-null pointer; the formatting code prints offset eight first and offset zero second, and the loop terminates after eight slots. Query-slot occupancy is:

```text
occupied / (occupied + empty)
```

A future binary's print format must be checked before reusing this interpretation.

A controlled probe of the checksum-pinned original web Opt4 query function confirmed the interpretation. It uses the original generated table and valid one-move cubes. Negative budgets stop each assigned context after its first query; inactive slots are marked complete. Only exports are added to an in-memory module copy; function code and pruning data are unchanged.

| Assigned slots | Occupied visits | Empty visits |
|---:|---:|---:|
| 0 | 0 | 8 |
| 1 | 1 | 7 |
| 4 | 4 | 4 |
| 8 | 8 | 0 |

The recorded runs give:

| Run | Occupied slot visits | Empty slot visits | Weighted query-slot occupancy |
|---|---:|---:|---:|
| Supplied native h10, 48 threads, all 500 states | 3,350,197,488 | 58,856,861 | 98.2735% |
| Web Opt4, 14 threads, first 100 completed states | 42,957,800,426 | 730,518 | 99.9983% |

Their tables, runtimes and samples differ; these percentages are not a speed comparison.

Occupied slots can still wait on memory. The counter excludes job construction, synchronization, idle phases and any time the query function is not running. It is not measured wall-time GPU utilization or a GPU speedup prediction. Including empty visits changes the native occupied count by only about 1.76%; it does not turn the native run's 6.70M occupied visits/cube into the reported Mac figure of 4.88M.

[Counter probe](../data/nissy-throughput-20261108/cubeopt-slot-probe.json), [probe source](../data/nissy-throughput-20261108/src/probe-cubeopt-slots.mjs), [native disassembly](../data/nissy-throughput-20261108/cubeopt-counter-disassembly.txt) and [aggregated occupancy](../data/nissy-throughput-20261108/cubeopt-occupancy.json) retain the evidence.

## Research caller versus shipped CLI

The exploratory Rust probe used ordinary `Pool::run`; the shipped CLI uses `run_pinned`, which fixes worker-state and stack-frame placement. **Pinned here does not mean CPU affinity.** Their timings and lane rankings differed substantially, so probe times were excluded from production configuration selection and must not be quoted as CLI performance.

The initial research-probe runs, recorded in the November 8 study, solved all 500 states with h6:

| Mode | CPU threads | Lanes/thread | Batch wall | Mean nodes/cube | Process CPU seconds / batch wall |
|---|---:|---:|---:|---:|---:|
| Independent cubes | 48 | 8 | 216.826 s | 89,711,408 | 17.00 cores |
| Independent cubes | 48 | 16 | 236.493 s | 89,711,408 | 14.39 cores |
| Independent cubes | 48 | 40 | 342.684 s | 89,711,408 | Retained in raw resource record |
| Shared subtrees | 48 | 8 | 67.279 s | 103,273,616 | 47.95 cores |
| Shared subtrees | 48 | 40 | 70.092 s | 104,245,213 | 47.93 cores |
| Shared subtrees | 48 | 64 | 71.392 s | 104,066,798 | 47.95 cores |

These are one process per tuning configuration. Process CPU includes initialization and spinning; dividing it by the separately measured batch wall approximates scheduled core use, not a count of useful cores. Every row's 500 solutions replayed correctly and matched the known optimal lengths. The pruning tables were resident on one-GiB pages; loading is excluded from batch wall.

Within this research-probe binary, shared scheduling finished about 3.22× faster than the eight-lane independent batch despite doing about 15% more counted search work. This is an end-to-end observation of those harness paths, not an isolated estimate of scheduling gain in the production CLI. Allocating 64 lanes instead of eight does not help this h6 case; this is evidence for scheduling and load balance, not an assertion that eight lanes is best for every table or workload.

Independent-cube scheduling has a finite-input limit. Each independent-cube lane needs its own input cube: with 500 inputs, 48 workers and 40 lanes per worker allocate 1,920 software slots, but at most 500 can initially hold separate cubes, a 26.0% ceiling before any complete. With 128 lanes per worker that ceiling is 8.1%. As cubes finish, a few hard cubes may remain on a few workers. Egg's independent-lane loop compacts active lanes, so it does not necessarily execute full-cost empty visits; nevertheless, unused CPU workers cannot help a hard cube owned by another independent lane, and memory-latency hiding shrinks when a worker has very few active contexts. Shared-subtree scheduling makes subtrees the unit of scheduling, so a 500-cube batch can fill many more software contexts than it has cubes and workers can help a remaining hard cube. This explains the scheduling distinction; it does not turn the research probe's timing ratio into a production-CLI optimization claim.

The nissy allocation screen shows the same batch issue: h6's pilot took 60.624 s at 48 × 1 versus 21.796 s at 6 × 8. The latter finished the all-500 confirmation in 102.621 s and won the complete allocation screen. Allocating all cores to independent inputs is not always the best way to finish a finite batch.

Fixed placement and generated-code differences may contribute to the probe/CLI timing gap, but the VM has no PMU counters to isolate their effects, and the difference is not attributed to one mechanism. The ordinary pool's shared-mode `Pool::counters` statistic is the mean post-round active-lane count. Dividing it by configured lanes describes occupancy during counted rounds; it excludes maintenance/build/wait time and can differ slightly from an exact count of lane calls when a round ends jobs. It is not an exact query-slot or wall-time utilization counter.

## Benchmark scope

Every primary egg throughput screen, final batch and single-cube scaling measurement uses the shipped `egg-solve` CLI; the research probe uses the existing public Rust lane and pool APIs. Nissy is driven through its public C API with one shared, read-only, one-GiB-page table and independently scheduled cube processes; its build and worker/thread allocation are selected explicitly. The final throughput figures in [the nissy comparison](../comparisons/NISSY_THROUGHPUT.md) use measured complete-batch wall time per cube, not summed per-cube latency divided by a nominal worker count. The VM has no hardware PMU counters; this study uses wall clocks, process CPU accounting and software search counters. Sources and raw records are under [`data/nissy-throughput-20261108/`](../data/nissy-throughput-20261108/README.md).
