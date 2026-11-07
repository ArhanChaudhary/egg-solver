# egg-solver: learn, compare, investigate

egg-solver is a Rust program that finds a shortest solution to a 3×3×3 Rubik's cube. This documentation explains how it works, how to use it, and what the measurements show when it is compared with other optimal solvers.

## Start here

1. **Understand the project:** [What egg-solver does](guide/OVERVIEW.md) explains optimal solutions, pruning tables, and the memory tradeoff.
2. **Try a solve:** [Quick start](guide/QUICKSTART.md) builds the source and runs a small, reproducible example.
3. **Read the results:** [Comparing the solvers](comparisons/README.md) explains the headline numbers and the conditions that make them meaningful.
4. **Look inside:** [How the search works](guide/HOW_IT_WORKS.md) connects the algorithm to the main source files.
5. **Check the evidence:** [Performance studies](performance/README.md) and [Correctness and safety](safety/README.md) explain the experiments and their limits.

For browsing offline, open **[index.html](index.html)**. The [complete comparison report](comparisons/RESULTS.html) has a contents menu and a table-row filter. Markdown versions work in a repository browser.

New to the terminology? Keep the [short glossary](guide/GLOSSARY.md) nearby while reading the results.

## Choose a question

| What would you like to know? | Read |
|---|---|
| How much RAM do I need? | [Tables and memory](guide/OVERVIEW.md#tables-and-memory) |
| How do I solve one cube or a batch? | [Quick start](guide/QUICKSTART.md) |
| How does it compare with nissy, nxopt, vcube and cubeopt? | [Comparison guide](comparisons/README.md), then the [full tables](comparisons/RESULTS.md) |
| What do latency, throughput, 1T, 48T and page sizes mean? | [Reading the measurements](comparisons/README.md#reading-the-measurements) |
| Which optimizations helped, failed, or were too noisy to call? | [Performance guide](performance/README.md) and [experiment history](history/README.md) |
| What did the physical-machine counters show? | [Hardware-counter analysis](performance/HARDWARE_COUNTERS.md) |
| Are the answers optimal, and what safety checks were run? | [Correctness and safety](safety/README.md) |
| Can I inspect or reprocess the measurements? | [Benchmark catalog](BENCHMARK_CATALOG.md) and [reproduction guide](reproduce/README.md) |

## What is included

The bundle contains **230 completed comparison cells**, **552 comparison run groups** with raw outputs and metadata, **242 physical-host PMU runs**, six sampled-profile text exports, final-build A/B checks, and the surviving **478 commit records**, including all **151 negative/neutral branch tips** and **59 archived branch tips**. The catalog maps these records to their purpose.

The main competitor matrix measured source **229b99f**. The source ZIP contains the later audited release **842a504**. Their separate performance comparison is disclosed in the results; the old measurements are not relabeled as new ones.

## Using this with the source ZIP

The source archive extracts to `egg-solver/`. Extract this documentation ZIP inside that directory to add `egg-solver/docs/`:

```sh
unzip egg-solver-source.zip
unzip egg-solver-docs.zip -d egg-solver
```

The documentation also works as a standalone folder. [Provenance and scope](PROVENANCE.md) explains which records were copied, which links were adapted, and which generated artifacts are intentionally outside a documentation archive.

[Publish the browser solver and documentation on GitHub Pages](guide/PUBLISHING.md).
