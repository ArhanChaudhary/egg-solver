# Threads, lanes, empty slots, and available search parallelism

2026-11-08. Counter interpretation, the h6–h10 finite-batch matrix, and the single-cube scaling measurements are complete. Both final solution sets pass cross-implementation replay.

## What the numbers refer to

| Quantity | Meaning |
|---|---|
| 48 CPU workers | This VM exposes 48 CPU cores. This is a hardware allocation, not a software-lane limit. |
| 24 / 32 / 40 default lanes | Egg's default interleaved search contexts per worker: h11 uses 24; h9/h10 use 32; h0–h8 use 40. |
| 48 in `bench tune` | The largest candidate in the current built-in tuning list. It is a search-grid choice, not a limit in the search algorithm. Explicit `-l` accepts larger values. |
| 8 contexts in the supplied cube48opt10 | Its query and assignment functions loop over eight software-pipeline slots per worker. This is independent of its CPU thread count. |
| Data SIMD | Vector operations on a cube's pieces. These are distinct from both software contexts and OS threads. |

More software lanes provide more independent table loads to overlap. They also require state storage and bookkeeping. Filling more slots does not guarantee higher throughput. The new experiment explicitly tested 64, 96 and 128 lanes as well as smaller values.

## Chen Shuang's empty-slot counter

The supplied Linux binary prints `N/E node(s)`, where **N is occupied query-slot visits and E is empty query-slot visits**. In `step1_query`, addresses `0x5594–0x55af` increment the counter at statistics offset zero for a null current-node pointer and the counter at offset eight for a non-null pointer. The formatting code prints offset eight first and offset zero second. The loop terminates after eight slots.

Thus, for this format:

`slot occupancy = N / (N + E)`.

E is the number of empty positions, not a second total-node count. A future binary's print format must be checked before reusing this interpretation.

A separate controlled probe of the checksum-pinned current web Opt4 query function confirms the counter semantics. It uses the original generated table and valid one-move cubes. Negative budgets stop each assigned context after its first query; inactive slots are marked complete. Only exports are added to an in-memory module copy; function code and pruning data are unchanged.

| Assigned contexts | Counter at offset 8: occupied | Counter at offset 0: empty | Total slots |
|---:|---:|---:|---:|
| 0 | 0 | 8 | 8 |
| 1 | 1 | 7 | 8 |
| 4 | 4 | 4 | 8 |
| 8 | 8 | 0 | 8 |

Raw evidence: `cubeopt-slot-probe.json`, `cubeopt-counter-disassembly.txt`, and `src/probe-cubeopt-slots.mjs` in `../data/nissy-throughput-20261108/`.

The previous real runs give:

| Run | Occupied slot visits | Empty slot visits | Weighted query-slot occupancy |
|---|---:|---:|---:|
| Supplied native h10, 48 threads, all 500 states | 3,350,197,488 | 58,856,861 | 98.2735% |
| Current web Opt4, 14 threads, first 100 completed states | 42,957,800,426 | 730,518 | 99.9983% |

These use different tables and runtimes; the table is not a speed comparison. Counting empty positions in the native h10 total changes the occupied count by only about 1.76%. It does not turn its 6.70M occupied visits/cube into the reported Mac figure of 4.88M.

This occupancy is conditional on calls to the query pipeline. It does not include time spent building jobs, synchronizing, waiting for another phase, or not calling the query function. Occupied slots can also stall on memory. It is therefore a proxy for how densely a hypothetical parallel query step is populated, **not measured GPU utilization or a GPU speedup prediction**.

## Finite batches expose a different limit

An independent-cube lane needs its own input cube. With 500 inputs, 48 workers and 40 lanes per worker allocate 1,920 software slots, but at most 500 can initially hold separate cubes: a 26.0% ceiling before any complete. With 128 lanes per worker that ceiling is 8.1%. As cubes finish, a few hard cubes may remain on a few workers.

Egg's independent-lane loop compacts active lanes, so it does not necessarily execute full-cost empty visits. Nevertheless, unused CPU workers cannot help a hard cube owned by another independent lane. Memory-latency hiding also shrinks when a worker has very few active contexts.

Work sharing changes the unit of scheduling to subtrees. A single input cube can supply multiple independent jobs, so a 500-cube batch can fill many more software contexts than it has cubes.

The initial **research-probe** h6, all-500-state runs show this distinction. They use ordinary `Pool::run`, not the shipped CLI's fixed-placement `run_pinned` path. The CLI was substantially faster and ranked lanes differently; these probe timings must not be quoted as current CLI performance:

| Mode | CPU threads | Lanes/thread | Batch wall | Mean nodes/cube | Process CPU seconds / batch wall |
|---|---:|---:|---:|---:|---:|
| Independent cubes | 48 | 8 | 216.826 s | 89,711,408 | 17.00 cores |
| Independent cubes | 48 | 16 | 236.493 s | 89,711,408 | 14.39 cores |
| Independent cubes | 48 | 40 | 342.684 s | 89,711,408 | Retained in raw resource record |
| Shared subtrees | 48 | 8 | 67.279 s | 103,273,616 | 47.95 cores |
| Shared subtrees | 48 | 40 | 70.092 s | 104,245,213 | 47.93 cores |
| Shared subtrees | 48 | 64 | 71.392 s | 104,066,798 | 47.95 cores |

These are one process per tuning configuration. Process CPU includes initialization and spinning; dividing it by the separately measured batch wall is an approximation of scheduled core use, not a count of useful cores. Every row's 500 solutions replayed correctly and matched the known optimal lengths. The pruning tables were resident on one-GiB pages; loading is excluded from batch wall.

Within this research-probe binary, shared scheduling finished about 3.22× faster than the eight-lane independent batch despite doing about 15% more counted search work. This is an end-to-end observation of those harness paths, not an isolated estimate of scheduling gain in the production CLI. Allocating 64 lanes instead of eight does not help this h6 case. This is evidence for scheduling and load balance, not an assertion that eight lanes is best for every table or workload.

The shared-mode `Pool::counters` statistic is the mean post-round active-lane count. Dividing it by configured lanes describes occupancy during counted rounds. It excludes maintenance/build/wait time and can differ slightly from an exact count of lane calls when a round ends jobs. It must not be presented as wall-time GPU occupancy.

## How much of one search can be parallelized?

The fixed-width software pipeline is not the limit of the tree. Prefix subtrees can be searched independently within an IDA* target depth. Nissy reserves space for 43,254 canonical four-move starting tasks before pruning; the number actually useful depends on the state and depth. Egg similarly builds and splits prefix jobs, and can donate remaining work.

The restrictions are elsewhere:

- A shortest-solution proof must finish all required shallower work. A found deeper solution cannot simply skip that proof.
- Pruning makes subtrees very uneven. The final few surviving branches can limit useful width.
- Job creation, sorting, distribution and synchronization have costs.
- The programs stop after an optimal solution is established; more workers may perform extra work before the stop reaches them.
- Memory bandwidth, dependency latency and the worker's state footprint can limit throughput even with occupied contexts.

A whole batch and one cube therefore need separate measurements. The matrix records the fastest observed complete-batch configuration, and separate one-cube-at-a-time scaling runs at 1–48 CPU threads. Measured speedup, efficiency and changed node counts are below. Neither high slot occupancy nor this 48-core VM establishes the maximum useful GPU width.

## Production CLI lane screen

These are actual complete-batch walls for all 500 states, one process per tuning configuration, 48 CPU workers. The shared-work CLI is used throughout this table. It fixes worker-state and stack-frame placement; `run_pinned` does not mean CPU-affinity pinning. The ordinary research-probe path above is excluded from production configuration selection.

| Lanes/worker | h6 seconds | h7 seconds | h8 seconds | h9 seconds | h10 seconds |
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

Twenty-four lanes won each screen. Neither a 48-lane limit nor larger defaults are justified by these measurements: 64–128 lanes are supported but slower on this batch. The existing built-in tune grid still ends at 48; explicit larger values remain usable. Other hardware and workload sizes need their own tuning.

The probe/CLI timing gap is also a useful warning: an algorithmically similar research caller is not interchangeable with the shipped program. Fixed placement and different generated code are possible contributors; this VM has no PMU evidence that isolates their contributions. We do not attribute the entire difference to one mechanism.

## Measured single-cube scaling

This study solves one input at a time (`-c 1`) with the production CLI. It uses three declared states (indices 0, 8 and 10, optimal lengths 18, 16 and 19), repeated ten times in the same order: 30 solves, three distinct states. The 24-lane count is fixed across worker counts; it was selected by the throughput screen, not retuned for every scaling row. Each row is one process, so small differences are exploratory rather than repeated confidence intervals.

| CPU workers | h6 ms/cube | h6 speedup | h6 efficiency | h10 ms/cube | h10 speedup | h10 efficiency |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2547.695 | 1.00× | 100.0% | 164.672 | 1.00× | 100.0% |
| 2 | 1282.842 | 1.99× | 99.3% | 83.805 | 1.96× | 98.2% |
| 4 | 647.304 | 3.94× | 98.4% | 42.276 | 3.90× | 97.4% |
| 8 | 328.917 | 7.75× | 96.8% | 21.537 | 7.65× | 95.6% |
| 16 | 172.266 | 14.79× | 92.4% | 11.146 | 14.77× | 92.3% |
| 24 | 120.665 | 21.11× | 88.0% | 7.707 | 21.37× | 89.0% |
| 32 | 98.366 | 25.90× | 80.9% | 6.544 | 25.16× | 78.6% |
| 48 | 76.554 | 33.28× | 69.3% | 5.423 | 30.37× | 63.3% |

At 48 workers, the measured speedup was **33.28× for h6** and **30.37× for h10**: 69.3% and 63.3% efficiency against the one-worker configuration. The searches counted 19.2% and 6.7% more nodes respectively than at one worker. This extra work, imbalance and coordination mean the figures cannot identify a universal parallel fraction by an Amdahl fit.

This establishes useful parallelism within one cube through 48 CPU workers on these states. Twenty-four lanes × 48 workers configures 1,152 interleaved contexts; it is not a claim that all are continuously occupied or executed simultaneously. Both implementations can generate tens of thousands of prefix jobs, and split work further. Their usefulness depends on pruning, target depth, the remaining tail and scheduling cost. This CPU study neither measures a GPU nor determines its maximum useful width.

The nissy allocation screen makes the batch issue concrete too: h6's pilot took 60.624 s at 48 × 1 versus 21.796 s at 6 × 8. The latter finished the all-500 confirmation in 102.621 s and won the complete allocation screen. Allocating all cores to independent inputs is not always the best way to finish a finite batch.

Raw single-cube outputs are indexed by `*-single30-parallelism` in [the run index](../data/nissy-throughput-20261108/run-index.csv); [analysis JSON](../data/nissy-throughput-20261108/analysis.json) retains every speedup and node ratio. Final h6–h10 throughput and all 30 raw final outputs are in [the nissy comparison](../comparisons/NISSY_THROUGHPUT.md).

## Benchmark scope

The research probe uses the existing public Rust lane and pool APIs. Every primary egg throughput screen, final batch, and single-cube scaling measurement uses the shipped `egg-solve` CLI. Research-probe timings are excluded from production configuration selection. Nissy is driven through its public C API with one shared, read-only, one-GiB-page table and independently scheduled cube processes. Its build and worker/thread allocation are selected explicitly. Both final rows use measured complete-batch wall time per cube, not summed per-cube latency divided by a nominal worker count.

The source and raw records are retained under `../data/nissy-throughput-20261108/`. No hardware PMU counters are available on this VM; this study uses wall clocks, process CPU accounting, and software search counters.
