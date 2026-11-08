# egg-solver documentation

Start with the quick start to run the program, or the overview to learn about optimal solving and pruning tables.

- [Quick start](guide/QUICKSTART.md): build, generate a table, and solve a cube or batch.
- [Project overview](guide/OVERVIEW.md) and [glossary](guide/GLOSSARY.md): concepts and memory requirements.
- [How the search works](guide/HOW_IT_WORKS.md): lanes, bounds, prefetching, and work sharing.
- [Solver comparisons](comparisons/README.md): matched configurations, results, and limitations.
- [Performance studies](performance/README.md): measured optimizations and rejected ideas.
- [Correctness and safety](safety/README.md): optimality checks, Miri, sanitizers, and their limits.

The [benchmark catalog](BENCHMARK_CATALOG.md) indexes the raw evidence. [History](history/README.md) keeps the dated experiments, and [reproduction](reproduce/README.md) explains how to rerun them. Historical results retain their original source and machine identities.

The documentation is included in the source project under `docs/`. Open `index.html` for offline browsing. See [publishing](guide/PUBLISHING.md) for the GitHub Pages build and [provenance](PROVENANCE.md) for the retained archives.
