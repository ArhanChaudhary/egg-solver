# Comparing egg-solver with other solvers

[Documentation home](../README.md) · [All tables](RESULTS.html) · [Benchmark catalog](../BENCHMARK_CATALOG.md)

The comparison covers nxopt, vcube, nissy-core H48, cubeopt30, cube48opt10 and cube48opt-rust. The full report groups results by table size, page type, worker count and solve mode. Start with the conditions, then read the speedups.

## Reading the measurements

| Term | Meaning here |
|---|---|
| 1T / 48T | One worker / the all-thread test configuration on the 48-core VM; cube48opt-rust latency has an explicit 18-thread cap |
| Latency | Time for one cube with workers cooperating on it |
| Throughput | Rate across many cubes; batch and steady-state stream details are stated separately |
| 1G / THP | 1 GiB explicit huge pages / transparent huge pages; actual coverage was sampled |
| Table GB | Decimal table-data size; compare similar sizes and note remaining differences |
| sec/solv | Seconds per solved cube under the report's stated formula; lower is better |
| Speedup | Competitor seconds per solve divided by egg-solver seconds per solve for the matched row |
| 95% interval at 1T | Variation over sampled cubes, not a guarantee about all future machines or states |

The host was an AMD EPYC 9V45 VM with 48 cores and about 94 GiB of RAM. It has no hardware PMU counters. The physical Xeon counter study is a separate experiment and should not be used as a direct speed comparison with this VM.

## November 8 follow-up

- [Nissy versus egg: all 500 states, h6–h10](NISSY_THROUGHPUT.md): complete batch walls, throughput tuning, all final raw outputs, and explicit PGO/source limits.
- [Lanes and parallelism](../performance/PARALLELISM.md): why 48 is a tuning candidate, exact cubeopt empty-slot counters, and measured scaling within one cube.
- [Cubeopt comparison audit](CUBEOPT_AUDIT.html): exact old binary identity, historical arithmetic, fresh 14-thread and full 500-state h10 runs.
- [Exact cstimer Opt4 table study](TABLE_ANALYSIS.html): 486 MB versus 464 MiB, semantic pruning queries, and the browser base-selection fix.
- [Large nxopt PS=16 table](NXOPT32.html): the requested 62.474 GB experiment and its measurement limits.
- [Raw evidence](../data/cubeopt-audit-20261108/README.md).

## Selected results

Selected results on this VM using **1 GiB pages**, with matching latency or throughput semantics. Speedups below are from the immutable comparison build **229b99f**, not newly attributed to current main. Table sizes are decimal GB; cubeopt30 uses a smaller table than egg-solver h7. The full report includes all tested sizes, both page modes for small/medium tables, build selection, sample sizes and variation.

| Competitor / egg-solver table | Mode | Table GB, competitor / egg | 1T speedup | 48T speedup |
|---|---|---:|---:|---:|
| cubeopt30 / h7 | latency | 2.60 / 3.89 | 10.1× | 13.8× |
| cube48opt10 / h10 | latency | 31.13 / 31.13 | 5.4× | 20.3× |
| nissy H48 h11 / h11 | latency | 60.67 / 62.26 | 10.4× | 31.4× |
| nxopt22 / h8 | throughput | 7.81 / 7.78 | 21.3× | 18.0× |
| vcube208 / h8 | throughput | 7.81 / 7.78 | 14.2× | 12.3× |
| cube48opt-rust Opt8 / h8 | throughput | 7.78 / 7.78 | 3.9× | 3.2× |

## Final solver validation

The recorded release source `842a5043abfd550313a86a3d9bb5254f4b457540` had the same audited Rust-source fingerprint as the fully checked safety candidate. Its search node/lookup counts and solution-length histograms match the verification datasets. Its separate h11 timing checks measured:

| Final build versus comparison build | Paired runs | Measured change |
|---|---:|---:|
| 1T stream throughput | 8 | -1.28% |
| 48T stream throughput | 5 | -1.01% |
| 48T latency: random_state | 6 | +0.16% time |
| 48T latency: random_move_17f | 6 | +2.84% time |
| 48T latency: random_state_depth17 | 6 | +1.20% time |

Higher throughput and lower latency are better. The final build's measured throughput and latency regressions are retained explicitly. Competitor timing tables continue to identify their original measured build.

Both complete Miri catalogues passed with documented exclusions, along with native/backend checks, focused strict-provenance Miri, AddressSanitizer and ThreadSanitizer. [NO_UB.md](../safety/NO_UB.md) records the identified fixes, evidence and limits. Universal absence of undefined behavior remains unproven. Correctness and optimality cross-checks are described in the full comparison below.


## What makes the comparison fair—and what still differs

The report matches page types, comparable table sizes, the cube sample, and latency/throughput semantics. Available source builds were screened for compiler and PGO choices. Prebuilt closed binaries could not be retuned. Large tables were tested on 1 GiB pages only. Some THP coverage is partial and is reported. Each solver's node count follows its own instrumentation and is not directly comparable across implementations.

For nxopt and vcube, the historical harness divides summed per-cube times by the number of workers; that can favor them relative to a batch wall-time measure with an idle tail. The report describes this explicitly. Small effects also require care because code placement and interference were measured sources of noise.

Read the **Fairness**, **Build screening**, **Distance-20**, and **Solution lengths** sections of the [complete report](RESULTS.md) before reusing a number. [Competitor findings](COMPETITOR_FINDINGS.md) documents the investigated issues and their evidence.

## Which source do the numbers describe?

The competitor matrix measured `229b99f`. The downloadable source is `842a504`, after the later safety changes and rename. The final-source A/B table above discloses its regressions. Historical optimization gains apply to their own baselines and cannot be added together or substituted for a current-source competitor run.

All 230 required matrix cells are complete. The [raw run index](../data/comparison/run-index.csv), [build-selection record](../data/comparison/build-selection.json), and [reproduction check](../data/comparison/reproduction-check.json) make the accepted evidence inspectable. Rejected and preliminary runs are explicitly separated in the data and methods.

## Larger nxopt tables

The historical matrix measured nxopt31 (2.60 GB) and nxopt22 (7.81 GB).
The later [EO3/EP2 follow-up](NXOPT32.md) measured the requested 62.474 GB
table and records its source, raw timings and limits. EO3/EP3 (182 GB) and
EO2/EP4 (546 GB) remain unmeasured.
The larger configurations were considered in the [source inventory](../reference/inventories/x-nxopt-inventory.md),
which is not a timing comparison. The 182 GB and 546 GB tables exceed this
VM's RAM. Results for the measured configurations do not establish that
egg-solver is faster than those larger configurations.

The current command-line cleanup is later than the recorded release and has
not been substituted into the historical timing or Miri results.
