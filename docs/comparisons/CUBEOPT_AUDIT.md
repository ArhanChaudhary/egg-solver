# Supplied Linux cubeopt: comparison audit

[Benchmarks](README.md) · [Methods](METHODS.md) · [Raw records](../data/cubeopt-audit-20261108/README.md)

**November 8, 2026.** The earlier 5.4× / 20.3× figures are supported by the retained arithmetic, but they describe **one supplied older x86 Linux executable on the EPYC VM**. They are not a comparison against an identified current cubeopt release on an M4 Pro.

## Fresh 500-state latency check

Both solvers used 48 threads, their own h10 tables, resident 1 GiB pages, and the same 500 repository states once. They solved one cube at a time. The timer excludes loading. **Lower mean latency is faster.** This is one fresh process per solver, not three process repeats.

| Solver | Mean latency (ms/cube) | Mean counted nodes/cube | Sum of per-cube solve times (s) |
|---|---:|---:|---:|
| Supplied Linux cube48opt10 | 47.654576 | 6,700,395 | 23.827288 |
| Egg h10, 32 lanes/worker | 2.363502 | 6,293,507 | 1.181751 |

The ratio of mean solve times is **20.163×** for this paired setup. All 500 returned lengths match; egg also replayed its solutions. Egg's complete batch wall was 1.185386 s. Nodes use each program's own instrumentation and are not a common unit of algorithmic work.

The input is `testdata/random_state.txt`, SHA-256 `616810c31d090a78ea1357684750642ab017ff394bfa08ba131bb65cc6905de3`. It is byte-identical to the dataset used by the historical harness. The runs were serialized under the benchmark locks on a quiet machine. Over the first 200 states, cubeopt averaged 48.177805 ms and 7,209,174.765 nodes/cube. Over all 500 it averaged 6.700M nodes/cube, so the longer dataset does not reproduce the reported 4.88M count described below.

[Cubeopt stdout](../data/cubeopt-audit-20261108/h10-cubeopt48.stdout.txt), [egg stdout](../data/cubeopt-audit-20261108/h10-egg-solver.stdout.txt), [input](../data/cubeopt-audit-20261108/h10-random_state.txt), [summary](../data/cubeopt-audit-20261108/h10-summary.json) and [checksums/commands](../data/cubeopt-audit-20261108/README.md) preserve the raw test.

## Executable and source identities

The supplied executable's release version is unverified. The historical harness ran the same file, byte for byte. Its SHA-256 is:

```text
6a20791d0b90ff946e6addc7e48565f0cb154260b38056b82052affffe94bf56
```

It is an x86-64 Linux ELF with GNU build ID `8169ccfa29e63d769214885e5e1c7c6191be74eb` and an embedded GCC 9.4.0 (Ubuntu 20.04) compiler string. Neither that string nor the file modification time establishes its release date. It prints `use sse, bmi2`, so the comparison includes implementation, compiler and instruction-set choices, not only search-algorithm differences. Its h10 table, `h48prun31h1.dat`, is 31,130,910,720 bytes. The supplied competitor was not rebuilt or retuned from source.

Egg's fresh build inputs (a 34-file build-source fingerprint) match published source `1b243c0`. The raw records cite it as `947c20d`, its ID before the repository history was rebuilt. The older matrix used egg `979dcf6`.

## What the historical ratios mean

The historical comparison was a fixed-configuration test: equal-sized tables, the same datasets, and runs at 1 and 48 threads. The 1-thread result used the first 50 states. The 48-thread result used the first 200 states and three process runs. The retained input file has 500 states.

The old harness parsed `tt=... ms` from each cube's output and averaged it. It did not divide cube48opt10's latency by its thread count. These binaries ignore `-g` ([finding #4](COMPETITOR_FINDINGS.md)), and the harness keeps one group, so that option did not introduce a division error either.

| Historical sample | Cubeopt time (ms/cube) | Cubeopt nodes/cube | Egg time (ms/cube) |
|---|---:|---:|---:|
| 1 thread, first 50 states | 368.82422 | 6,993,163.78 | 68.40798 |
| 48 threads, first 200 states, run 1 | 48.88043 | 7,112,389.26 | 2.707375 |
| 48 threads, first 200 states, run 2 | 48.89141 | 7,214,157.06 | 2.409495 |
| 48 threads, first 200 states, run 3 | 49.34675 | 7,183,479.15 | 2.414160 |

The 48-thread medians are 48.89141 ms for cubeopt and 2.414160 ms for egg. The recalculated ratios are therefore **5.39154×** at one thread and **20.25193×** at 48 threads (ratio of medians). The printed 5.4× and 20.3× figures match that arithmetic. Egg's 48-thread end-to-end batch wall time was 2.416–2.422 ms/cube in runs 2 and 3, so its per-cube timer did not hide an order-of-magnitude cleanup cost. Loading and process startup are excluded on both sides.

Those fixed-thread runs do not show that 48 threads was the best setting for either program. The raw [cubeopt](../data/comparison/raw/cube48opt10_48T_random_state_200_1g.log) and [egg](../data/comparison/raw/cubeoptrsh10_48T_random_state_200_1g.log) logs sit beside their `_r2`/`_r3` repeats and the one-thread logs; per-run summaries are in the matching `sum/` directory. The [historical run index](../data/comparison/run-index.csv) lists every run. The harness is [runtest.sh](../reproduce/harness/runtest.sh), function `cubeopt`. [The historical matrix](RESULTS.md) keeps all earlier numeric tables.

## Fresh 14-thread check

On November 8, the supplied Linux binary solved the first 200 random states three times in one process at 14 threads. Its table was verified resident on 29 GiB of 1 GiB pages. The three group means were **62.876945, 62.547540 and 62.847185 ms/cube**. Their median is **62.847185 ms/cube**, or **15.912 solves/s** under this serial-solve timing metric. Mean nodes/cube were 7.050M, 7.042M and 7.026M. These repeated groups are not independent process restarts. The raw output of this check is not among the retained records.

On the VM, this old binary was slower at 14 threads than at 48. It does not reproduce the reported Mac figure of 13.5176 ms/solve. This does not separate hardware from software differences, and the tested settings do not establish a globally best configuration.

## Reported M4 Pro result

The reported figures were 14 threads, h10, `dataset/random_state.txt`, 4.88M nodes/solve and 2.77 ns/node. Their product is 13.5176 ms/solve, equivalent to **73.98 solves/s** under that calculation. The exact binary, command, runtime, input prefix and concurrency mode were not established.

There is no paired egg measurement on that Mac. Dividing a VM time by the Mac time would not isolate a solver speedup. Matching dataset names or displayed h values does not establish matching versions, table contents or timed workloads. The fresh local runs do not explain the lower Mac node count.

## Reporting correction

Keep the executable identity, measured egg source, CPU, table bytes, threads, mode, sample and repeats beside each ratio. Label the competitor as the supplied x86-64 Linux cube48opt10 executable, with its checksum, rather than as cubeopt in general. Separate a fixed-thread benchmark from a best-configuration or best-solver claim. A Mac comparison needs both programs on that Mac, on identical input bytes, with equivalent timing boundaries and modes.

The old arithmetic is reproducible; the broader ranking was unsupported. The audit did not establish a numerical timing error. Use the [new nissy study](NISSY_THROUGHPUT.md) for the separately measured finite-batch throughput, and the [table study](TABLE_ANALYSIS.md) for the cstimer size/pruning question.
