# Benchmark catalog

[Documentation home](README.md) · [Comparison guide](comparisons/README.md) · [Reproduction](reproduce/README.md)

This catalog separates the studies by the question they answer. It includes negative and rejected results so that the reader can inspect how the final conclusions were chosen.

| Study | Coverage | Where to start |
|---|---|---|
| November 8 throughput and parallelism | 305 accepted measurements; h6–h10, 500 states, three final batches per solver/size, 1 GiB pages; separate single-cube scaling | [Nissy comparison](comparisons/NISSY_THROUGHPUT.md), [parallelism](performance/PARALLELISM.md), [raw index](data/nissy-throughput-20261108/run-index.csv) |
| Main solver comparison | 230 completed cells: 86 headline, 64 distance-20, 80 wiki; 1T/48T, table-size groups, page modes | [Full report](comparisons/RESULTS.html) |
| Raw comparisons and build screening | 552 retained run groups with raw output, summary and metadata; frozen compiler/PGO choices | [Run index](data/comparison/run-index.csv), [build selection](data/comparison/build-selection.json) |
| Distance-20 and interference checks | Separate harder-state datasets; serial nissy fallback after the interference check | Full report's Distance-20 section; [datasets](data/datasets/README.md) |
| Table contents and generation | h9/h10 generation and equality checks; cube48opt10/h10 mapping and byte differences | [Generation records](data/comparison/ours-gen/README.md), [table inventory](reference/inventories/cube48opt10-table.md) |
| Final source versus measured source | 8 paired 1T streams, 5 paired 48T streams, 6 paired latency rounds for each of 3 datasets | [Final validation JSON](data/final-build/final-main-validation.json), [comparison guide](comparisons/README.md) |
| Physical-host PMU | 242 runs, 621 counter rows, 6 sampled profiles; workers/SMT, lanes, pages, NUMA and event groups | [Analysis](performance/HARDWARE_COUNTERS.md), [raw-data index](data/phack/README.md) |
| PMU-guided native changes | 12 paired 1T results and 5 paired 48T results per candidate; two rejected ideas | [Optimization review](performance/OPTIMIZATION_REVIEW.md) |
| Search and scheduling optimizations | Lane interleaving, bounds/pruning, job ordering, symmetry, work sharing and prefetch studies | [History](history/README.md) |
| Compiler and layout experiments | Code placement, flags, LTO, PGO, BOLT, instruction/branch studies, neutral results | [Performance Book review](reference/inventories/perf-book-audit.md), [negative experiments](history/NEGATIVE_EXPERIMENTS.md) |
| Startup, shared memory and page size | Loading/attachment, search with page configurations, allocation/page-fault and placement studies | [User guide](reference/USER_GUIDE.md), historical shared-memory and placement records |
| WebAssembly and Bun | Engine warm-up, prefetch/lane behavior, worker scaling and table generation | [User guide](reference/USER_GUIDE.md#webassembly), historical WASM records |
| Correctness and safety | Optimal-length cross-checks, Miri, sanitizers, lint review and source identity | [Safety guide](safety/README.md) |

## Accepted, historical, and rejected

`data/comparison/raw`, `sum`, and `meta` retain the collected comparison run groups. The run index marks which pass the report's eligibility checks and which use the selected build. Eligible does not mean every row contributes to every published aggregate: dataset coverage, sample alignment and report grouping also apply. `data/comparison/rejected` retains failed/contaminated attempts and reasons. Use the published tables or the reproduction script to obtain the headline aggregates.

The historical commit archive is deliberately broader than successful performance changes. It includes missing/limited measurements and noisy results with their recorded qualifications. No universal speed ranking across machines is claimed. The physically collected PMU results use their own older source and compatible build, stated in their manifest.
