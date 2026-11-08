# Supplied Linux cubeopt: comparison audit

[Benchmarks](README.md) · [Methods](METHODS.md) · [Complete dated audit](CUBEOPT_AUDIT_20261108.md)

**November 8, 2026.** The earlier 5.4× / 20.3× figures are supported by the retained arithmetic, but they describe **one supplied older x86 Linux executable on the EPYC VM**. They are not a comparison against an identified current cubeopt release on an M4 Pro.

## Fresh 500-state latency check

Both solvers used 48 threads, their own h10 tables, resident 1 GiB pages, and the same 500 repository states once. They solved one cube at a time. The timer excludes loading. **Lower mean latency is faster.** This is one fresh process per solver, not three process repeats.

| Solver | Mean latency (ms/cube) | Mean counted nodes/cube |
|---|---:|---:|
| Supplied Linux cube48opt10 | 47.654576 | 6,700,395 |
| Egg h10, 32 lanes/worker | 2.363502 | 6,293,507 |

The ratio of mean solve times is **20.163×** for this paired setup. All 500 returned lengths match; egg also replayed its solutions. Egg's complete batch wall was 1.185386 s. Nodes use each program's own instrumentation and are not a common unit of algorithmic work.

[Cubeopt stdout](../data/cubeopt-audit-20261108/h10-cubeopt48.stdout.txt), [egg stdout](../data/cubeopt-audit-20261108/h10-egg-solver.stdout.txt), [input](../data/cubeopt-audit-20261108/h10-random_state.txt), [summary](../data/cubeopt-audit-20261108/h10-summary.json) and [checksums/commands](../data/cubeopt-audit-20261108/README.md) preserve the raw test.

## Executable and source identities

The supplied executable's release version is unverified. Its SHA-256 is:

```text
6a20791d0b90ff946e6addc7e48565f0cb154260b38056b82052affffe94bf56
```

It is an x86-64 Linux ELF with an embedded GCC 9.4.0 compiler string; that string does not establish its release date. Egg's fresh build inputs match published source `947c20d`. The older matrix used egg `229b99f`. The supplied competitor was not rebuilt or retuned from source.

## What the historical ratios mean

| Historical sample | Cubeopt time | Egg time | Cubeopt / egg |
|---|---:|---:|---:|
| One thread, first 50 states | 368.82422 ms/cube | 68.40798 ms/cube | 5.39154× |
| 48 threads, first 200 states | 48.89141 ms/cube | 2.414160 ms/cube | 20.25193× |

The multi-thread row uses medians over three runs. Those historical configurations were fixed-thread comparisons, not a proof that either setting was globally optimal. The detailed audit retains individual runs, the 14-thread check, page evidence and harness interpretation. [The historical matrix](RESULTS.md) keeps all earlier numeric tables.

## Reported M4 Pro result

The reported figures were 14 threads, h10, 4.88M nodes/solve and 2.77 ns/node. Their product is 13.5176 ms/solve, equivalent to **73.98 solves/s** under that calculation. The exact binary, command, runtime, input prefix and concurrency mode were not established.

There is no paired egg measurement on that Mac. Dividing a VM time by the Mac time would not isolate a solver speedup. Matching dataset names or displayed h values does not establish matching versions, table contents or timed workloads. The fresh local run does not explain the lower Mac node count.

## Reporting correction

Keep the executable identity, measured egg source, CPU, table bytes, threads, mode, sample and repeats beside each ratio. The old arithmetic is reproducible; the broader ranking was unsupported. Use the [new nissy study](NISSY_THROUGHPUT.md) for the separately measured finite-batch throughput, and the [table study](TABLE_ANALYSIS.md) for the cstimer size/pruning question.
