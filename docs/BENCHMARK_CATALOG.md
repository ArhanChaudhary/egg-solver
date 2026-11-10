# Benchmark and evidence catalog

[Documentation](README.md) · [Benchmark guide](comparisons/README.md) · [Reproduction](reproduce/README.md)

Choose a study by its metric and source. Record counts below are not distinct cube counts or interchangeable measures of coverage.

## Solver comparisons

| Study | Date / measurement | Reader report | Raw records |
|---|---|---|---|
| Nissy, h6–h10 | Nov 8; complete 500-state batch wall, 1 GiB pages | [Throughput](comparisons/NISSY_THROUGHPUT.md) | [305 accepted measurements](data/nissy-throughput-20261108/run-index.csv) |
| Supplied Linux cubeopt h10 | Nov 8; 500 one-cube latency solves, 48 threads | [Audit](comparisons/CUBEOPT_AUDIT.md) | [Commands, logs and metadata](data/cubeopt-audit-20261108/README.md) |
| Large nxopt EO3/EP2 | Nov 7; first 50/200 states, normalized worker timing | [62.474 GB study](comparisons/NXOPT32.md) | [Experiment directory](data/nxopt32-20261107/README.md) |
| Earlier multi-solver matrix | Nov 7 snapshot; egg `979dcf6`, 230 report cells, several metrics | [Historical overview](comparisons/RESULTS.md) | [Run index](data/comparison/run-index.csv), [build choices](data/comparison/build-selection.json) |

The newest throughput study has **30 final batches × 500 states = 15,000 solves** of **500 distinct states**. Its 305 accepted measurements also include configuration screens, scaling and exploratory research calls. The historical matrix's report cells and retained run groups are different quantities.

## Performance mechanisms

| Study | Scope | Evidence |
|---|---|---|
| Worker/lane scaling and cubeopt slots | Nov 8 CPU measurements; three-state single-cube sample | [Parallelism](performance/PARALLELISM.md), [analysis](data/nissy-throughput-20261108/analysis.json) |
| h4 compression and cstimer table capacity | Nov 8 semantic bounds, native base-8/base-9 batches and browser generation | [Table study](comparisons/TABLE_ANALYSIS.md), [raw records](data/cubeopt-audit-20261108/README.md) |
| Physical Xeon PMU | Separate host and older source; 242 collection runs, 621 counter rows, 6 profiles | [Counter analysis](performance/HARDWARE_COUNTERS.md), [data](data/phack/README.md) |
| PMU-motivated native changes | Two rejected experiments; paired 1/48-worker throughput | [Review](performance/OPTIMIZATION_REVIEW.md) |
| Generation, pages, startup and compiler/layout work | Each recorded source and machine | [Generation](data/comparison/ours-gen/README.md), [history](history/DELETED_EXPERIMENTS.md) |
| Earlier WebAssembly timings | Historical engines, source and workloads | [Historical measurements](reference/USER_GUIDE.md#speed) |

## Correctness and validation

[Safety](safety/README.md) separates replay, optimal-length evidence and UB checks. [The detailed assessment](safety/NO_UB.md) identifies the audited source and Miri/sanitizer limits. [Final-build validation](data/final-build/final-main-validation.json) records the older source A/B check; it is not a fresh timing of every later commit.

## Historical and rejected work

Adopted changes and their measurements are in the git history (`git log`). [Deleted experiments](history/DELETED_EXPERIMENTS.md) lists the 213 unmerged experiment branches and tags that were deleted when the history was rebuilt, including [151 rejected or neutral experiments](history/DELETED_EXPERIMENTS.md#rejected-and-neutral-experiments) and [59 archived experiments](history/DELETED_EXPERIMENTS.md#archived-experiments). Old pending statements and file paths describe their snapshot date. Gains have different baselines and are not additive.

Raw rejected/incomplete attempts remain separated from accepted results. An accepted run can still be a tuning or exploratory measurement rather than a headline result. Use each report's inclusion rules and [timing definitions](comparisons/METHODS.md), not a raw record count, to reconstruct the published tables.
