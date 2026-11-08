# Solver benchmarks

[Documentation](../README.md) · [How to read a benchmark](METHODS.md) · [Raw-record catalog](../BENCHMARK_CATALOG.md)

Start with the study that matches your question. Results from different CPUs, table sizes, source versions or timing methods cannot be combined into one ranking.

## Latest matched throughput study

[Nissy versus egg-solver, h6–h10](NISSY_THROUGHPUT.md) measures the time to finish **all 500 repository random states**, on the 48-core EPYC VM with **1 GiB table pages**. It includes the tested configurations, three final batches per solver and table, and every final raw output. This is the clearest starting point for batch throughput.

The [parallelism study](../performance/PARALLELISM.md) explains the lane screen and the separate one-cube-at-a-time scaling measurements. Neither study measures performance on a GPU or on a Mac.

## Other comparisons

| Study | Question answered | Important boundary |
|---|---|---|
| [Supplied Linux cubeopt binary](CUBEOPT_AUDIT.md) | What did the old cubeopt speedups actually measure? | Identified x86 Linux executable; no paired measurement of the reported M4 Pro release |
| [Large nxopt table](NXOPT32.md) | How did nxopt's 62.474 GB table perform? | Nxopt's reported time is normalized from overlapping cube durations, not complete batch wall time |
| [cstimer table study](TABLE_ANALYSIS.md) | Is “486 MB” more pruning data than “464 MiB”? | Same h4 data capacity; semantic bounds and compression bases were tested separately |
| [Historical multi-solver matrix](RESULTS.md) | What was measured against cubeopt30, cube48opt-rust, nissy, nxopt and vcube earlier? | Earlier egg build, several timing formulas and table sizes; dated results, not a current release ranking |
| [Competitor findings](COMPETITOR_FINDINGS.md) | Which implementation and harness issues affected interpretation? | Findings apply to the pinned source versions or supplied binaries |

## Timing conventions

- **Batch throughput:** completed cubes divided by the time to finish the entire batch. Higher is faster.
- **Single-cube latency:** time to solve one cube. Lower is faster.
- **Normalized worker time:** summed cube durations divided by workers and cube count. It omits an uneven finishing tail and is not measured batch wall time.

A ratio always states its direction. Counts of nodes and lookups use each solver's instrumentation and are not interchangeable. [Methods](METHODS.md) defines units, repeats and the historical table notation.
