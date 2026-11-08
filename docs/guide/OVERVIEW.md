# Tables, memory and solve modes

[Documentation](../README.md) · [Quick start](QUICKSTART.md) · [Glossary](GLOSSARY.md)

egg-solver searches for a shortest solution in the **half-turn metric**: a 90°, 180° or 270° turn of one face counts as one move. The native CLI reads scrambles. The JavaScript/browser interface can also read a 54-sticker facelet string.

A precomputed pruning table supplies lower bounds on the moves still needed. Larger tables retain more information and usually reduce search work. Optimal-search semantics do not require the largest table.

## Tables and memory

These are logical egg pruning-data sizes, rounded for display. Files add a 4 KiB trailer and may contain page padding. The process also needs memory for search state, auxiliary data and the operating system.

| Table | Pruning data |
|---|---:|
| h0 | 28.99 MiB |
| h1 | 57.99 MiB |
| h2 | 115.97 MiB |
| h3 | 231.94 MiB |
| h4 | 463.89 MiB |
| h5 | 927.77 MiB |
| h6 | 1.81 GiB |
| h7 | 3.62 GiB |
| h8 | 7.25 GiB |
| h9 | 14.50 GiB |
| h10 | 28.99 GiB |
| h11 | 57.99 GiB |

Choose a table that fits comfortably in **available** RAM. The same table data can be moved between supported machines and backends. Generation, loading and search are separate operations. The [user guide](../reference/USER_GUIDE.md) explains generation and loading options.

A 1 GiB-page benchmark rounds each table allocation to whole GiB pages. That is why the h6–h10 benchmark lists 2, 4, 8, 15 and 29 GiB of mapped table RAM. It is not a different data size. [Benchmark units](../comparisons/METHODS.md#units-and-configuration) explains GB versus GiB.

## One cube or many

- **One cube:** use `solve --fast --table PATH SCRAMBLE`, or `-c 1` for sequential stdin. Workers cooperate within each search.
- **A finite batch:** use the default concurrency (`-c 0`). The reported batch wall time includes waiting for the last cube.
- **A stream:** use the benchmark stream mode to measure completed work over a declared window. Its warm-up/window metrics differ from a fixed 500-state batch.

Lanes are interleaved search contexts, not extra CPU threads. More lanes are not always faster. The [parallelism study](../performance/PARALLELISM.md) records the measured tradeoff.

## Platforms and evidence

The project has native x86, ARM/NEON and WebAssembly paths. CPU features, browser memory limits and thread support affect which build can run. WebAssembly currently supports h0–h6; h7 exceeds its single-allocation limit.

[How the search works](HOW_IT_WORKS.md) explains the algorithm. [Safety](../safety/README.md) separates solution replay, optimality checks and undefined-behavior evidence. The [benchmark guide](../comparisons/README.md) keeps performance claims tied to their measured source and host.
