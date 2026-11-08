# Performance studies

[Documentation](../README.md) · [Solver comparisons](../comparisons/README.md) · [Experiment history](../history/README.md)

Each study answers a specific performance question. Historical improvements use different baselines and cannot be added together.

| Study | What it measures | Result / scope |
|---|---|---|
| [Lanes and parallelism](PARALLELISM.md) | Lane counts, cubeopt query slots, and scaling within one search | Latest production CLI screen; separate three-state scaling sample |
| [Nissy versus egg](../comparisons/NISSY_THROUGHPUT.md) | Complete 500-state batch throughput, h6–h10 | Matched 1 GiB table pages; three final runs per solver/table |
| [Xeon hardware counters](HARDWARE_COUNTERS.md) | Stalls, branches, memory and worker topology | Separate X5650 host and older source; not an EPYC timing comparison |
| [PMU-guided experiments](OPTIMIZATION_REVIEW.md) | Byte-sized table loads and an outlined lane step | Both rejected; historical paired throughput measurements |
| [h4 compression base](../comparisons/TABLE_ANALYSIS.md) | Base 8 versus base 9, and cstimer's displayed table size | Same data capacity; browser selector now uses the recommended base |
| [Compiler and layout history](../history/NEGATIVE_EXPERIMENTS.md) | PGO, LTO, code placement and other attempted changes | Includes neutral, rejected and noisy results |

The [catalog](../BENCHMARK_CATALOG.md) points to raw records. Use the [benchmark methods](../comparisons/METHODS.md) to distinguish rates, latency, normalized worker time and uncertainty. Current command usage belongs in the [user guide](../reference/USER_GUIDE.md), not in old experiment notes.
