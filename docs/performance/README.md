# Performance studies: what was tried and what was learned

[Documentation home](../README.md) · [Benchmark catalog](../BENCHMARK_CATALOG.md)

The main comparison answers how the solver performed against other implementations. The experiments here answer why it behaves that way and which changes were worth keeping.

## Follow a topic

| Topic | Main evidence | How to interpret it |
|---|---|---|
| Lanes, empty slots and parallel search width | [Parallelism study](PARALLELISM.md) and [fresh nissy throughput](../comparisons/NISSY_THROUGHPUT.md) | Production CLI versus exploratory probe; 1–48-worker scaling is a CPU measurement, not GPU utilization |
| Table lookup cost, stalls, branches and caches | [Physical-host counters](HARDWARE_COUNTERS.md) | Measured on the Xeon X5650 with its compatible build; no cross-machine speed inference |
| Two PMU-motivated native changes | [Optimization review](OPTIMIZATION_REVIEW.md) | Both rejected; full 1T and 48T tables retained |
| Lane scheduling, pruning, job ordering and sharing | [Optimization history](../history/README.md) | Dated baseline-to-candidate results, often with node and lookup identity |
| Compiler flags, LTO, PGO and BOLT | [Performance Book audit](../reference/inventories/perf-book-audit.md) and the [negative index](../history/NEGATIVE_EXPERIMENTS.md) | Includes neutral results and cases dominated by code placement |
| Huge pages, THP, ordinary pages, NUMA and SMT | [Hardware study](HARDWARE_COUNTERS.md), [full comparison](../comparisons/RESULTS.md), and historical shared-memory records | Page coverage and worker topology matter; startup and search costs are separate |
| WebAssembly, browsers and Bun | [User guide](../reference/USER_GUIDE.md#webassembly) and historical WASM commits | Includes warm-up, lane/prefetch and worker studies; NEON emulation is correctness evidence only |
| Table generation and memory | [Generation records](../data/comparison/ours-gen/README.md), [overview](../guide/OVERVIEW.md), and [table analysis](../reference/inventories/cube48opt10-table.md) | Similar file sizes do not imply identical indexing or table contents |

## The latest optimization decision

Byte-sized packed table loads measured −0.83% at 1T and −0.28% at 48T. Moving lane advancement out of the stream loop measured −7.44% and −10.72%. Both remain rejected. The 1T values summarize 12 paired samples across four padding settings; the 48T values summarize five paired rounds at settings chosen by that screen.

The smaller byte-load effect is near the scale of layout and run noise. These short-window tests did not justify adoption. The [full review](OPTIMIZATION_REVIEW.md) records the methods, source hashes, per-run values and archival commits.

## Reading older experiments

Older measurements were taken at different stages of the implementation. Some early small wins were later traced to code placement; some measurements were rejected after an interference check. Use the notes in each record. A label such as “negative” preserves useful evidence about a tested idea, not a guarantee that it can never work on another CPU or workload.

The [history index](../history/README.md) contains all surviving commit-message records, with separate indexes for adopted history, negative branch tips and archived branches. The [technique inventories](../reference/inventories/README.md) connect ideas in other solvers to the experiments that investigated them.
