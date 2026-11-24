# A third-party benchmark, and what it showed about lane counts (November 24, 2026)

[Lane study](../../../performance/PARALLELISM.md) · [nissy-core findings](../../../comparisons/COMPETITOR_FINDINGS.md) · [User guide: tune and benchmark](../../../reference/USER_GUIDE.md#tune-and-benchmark)

Sebastiano Tronto, the author of nissy-core, benchmarked egg-solver `c3eb909` against his H48 solver on a Ryzen 7 7700 (8 cores, 16 threads) and published the result in [nissy-core's benchmarks page](https://github.com/sebastianotronto/nissy-core/blob/master/benchmarks/benchmarks.md) (nissy-core `d0f311a`, November 22). An earlier run of his (November 14, egg-solver `28a1953`) had used other settings. Two things on the page asked for an explanation:

1. Some egg-solver rows are slower than in his earlier run, after the quarter-turn metric and the lists of all solutions were added.
2. H48 lists all optimal solutions of 20-move scrambles and of the superflip faster than egg-solver, and gains more from 16 threads on 8 cores.

This folder records the measurements made here to answer them, and the change that followed (`bench tune`, the default lane count). His machine was not available: what is said about it below is inferred from his published numbers and marked as such.

## Summary

- **The search code is not slower.** The loop that `solve` spends its time in (`lanes::share::Worker::rounds`) is the same machine code, instruction for instruction and at the same address, in `28a1953`, `c3eb909` and `dc153b3`, built for this CPU and for his (`-C target-cpu=znver4`). Timed at one thread, the three versions are within 2.3% of each other in every configuration, mostly within 1% and in both directions, on this x86 server and on an Apple M2.
- **The slower rows come from lane counts.** Between his two runs the settings changed: batch mode with the default lanes became `-c 1` with the lanes that `bench tune` saved. That `bench tune` counted the cubes a stream finished in six seconds, a handful on tables up to h9, so what it saved was close to chance. Run here, it saved 48, 24, 32 and 48 lanes for h6 to h9 at one thread, where 24 is the fastest on every one of them. His one-thread rows moved the way those lane counts predict: h6 and h9 slower, h7 and h8 faster, h10 and h11 the same.
- **The lists are not comparable as published.** nissy-core cuts the number of solutions it is asked for to 8 bits, so its "all solutions" runs stop at 63 per cube ([finding 12](../../../comparisons/COMPETITOR_FINDINGS.md)). Every 20-move scramble of the set has 154 to 335 optimal solutions and the superflip has 4,416. And `c3eb909` did not yet use a cube's symmetries for lists (added in `2ea70b9`). With both accounted for, egg-solver lists the same solutions 7.3 to 8.5 times faster than nissy-core on 16 cores of this server (18- to 20-move scrambles), and all 4,416 of the superflip in less than half the time nissy-core takes for 63.
- **Two threads per core gain egg-solver little.** A thread already keeps about 24 table lookups in flight, which is what a second thread on the same core would otherwise add. On his page, for the 18- to 20-move sets, H48 at 16 threads is 7.3 to 12.3 times as fast as at one thread and egg-solver 7.3 to 9.1 times; in seconds egg-solver is 4.1 to 7.6 times faster there at 16 threads.
- **Changed:** `bench tune` now measures the search speed of `solve` itself in nodes per second, which counts the cubes in progress when a measurement ends; records of the earlier command are ignored; and the untuned default is 24 lanes for every table (it was 40 up to h8 and 32 for h9 and h10).

## Machines and builds

| | This server | Apple M2 |
|---|---|---|
| CPU | AMD EPYC 9V45 (Zen 5), 48 cores given to the VM, one thread per core | Apple M2, 4 performance and 4 efficiency cores |
| Memory | 94 GB | 15 GB |
| System | Linux 7.0, transparent huge pages on request (`madvise`) | Fedora Asahi, Linux 7.1 |
| Compiler | rustc 1.99.1; GCC 13.3 for nissy-core | rustc 1.99.1 |

egg-solver was built at `28a1953` (his first run), `c3eb909` (his second) and `dc153b3` (main on November 24), each with `cargo build --release --locked` for this CPU (`native`) and for his (`znver4`), from one source path and one build path. Tables h6 to h9 were made by `gen K auto` and copied into memory on transparent huge pages. nissy-core is its master `d0f311a`, built for 16 threads (`-DTHREADS=16 -DAVX2 -O3 -mavx2`), once as it is and once with only the 8-bit cast of the solution limit removed (and a larger output buffer in the tool); its table is its own `h48h7`.

The scramble sets are those of nissy-core's `benchmarks/scrambles` (25 scrambles each, by optimal length). `scripts/s19-sub.txt` and `scripts/s20-sub.txt` are 6 scrambles of the 19-move set (indices 0, 1, 2, 3, 9, 24) and 3 of the 20-move set (indices 0, 1, 10), to keep nissy-core's list runs to minutes.

Timed runs held the machine's benchmark lock exclusively. In the first window five pinned measurements of this study ran at once on separate groups of cores (cores 2, 10 and 18; 24 to 31; 32 to 47), and nothing else. The second and third windows ran one program at a time on all 48 cores. On the M2 each unit held that machine's lock. Times are `solve`'s own `wall` of the whole set, which leaves out loading the table; nissy-core's are its "Total time".

## 1. The same code at three commits

`summarize.py` prints these tables from the raw files. Mean wall seconds per set, one thread pinned to a core; the percentage is against `28a1953` in the same row. The versions were run in the order A B C C B A, 12 times each for h6 and 6 times for h9.

| Run | 28a1953 | c3eb909 | dc153b3 |
|---|---:|---:|---:|
| h6, 17-move set, 40 lanes, `-c 1`, native | 5.000 | 5.034 (+0.7%) | 5.079 (+1.6%) |
| h6, 17-move set, 40 lanes, `-c 0`, native | 5.134 | 5.138 (+0.1%) | 5.145 (+0.2%) |
| h6, 17-move set, 40 lanes, `-c 1`, znver4 | 5.078 | 5.093 (+0.3%) | 5.078 (-0.0%) |
| h6, 17-move set, 40 lanes, `-c 0`, znver4 | 5.079 | 5.080 (+0.0%) | 5.080 (+0.0%) |
| h9, 18-move set, 32 lanes, `-c 1`, native | 5.279 | 5.347 (+1.3%) | 5.260 (-0.4%) |
| h9, 18-move set, 32 lanes, `-c 0`, native | 5.253 | 5.290 (+0.7%) | 5.227 (-0.5%) |
| h9, 18-move set, 32 lanes, `-c 1`, znver4 | 5.367 | 5.334 (-0.6%) | 5.335 (-0.6%) |
| h9, 18-move set, 32 lanes, `-c 0`, znver4 | 5.297 | 5.416 (+2.3%) | 5.402 (+2.0%) |

On the Apple M2 (40 lanes; one thread on a performance core, 4 threads on the performance cores, 8 on all):

| Run | 28a1953 | c3eb909 | dc153b3 |
|---|---:|---:|---:|
| h6, 17-move set, `-c 0`, 1 thread | 6.013 | 6.032 (+0.3%) | 6.011 (-0.0%) |
| h6, 17-move set, `-c 1`, 1 thread | 6.014 | 6.018 (+0.1%) | 6.030 (+0.3%) |
| h7, 18-move set, `-c 0`, 4 threads | 6.659 | 6.639 (-0.3%) | 6.595 (-1.0%) |
| h7, 18-move set, `-c 0`, 8 threads | 5.073 | 5.052 (-0.4%) | 5.033 (-0.8%) |
| h7, 18-move set, `-c 1`, 4 threads | 6.470 | 6.440 (-0.5%) | 6.353 (-1.8%) |
| h7, 18-move set, `-c 1`, 8 threads | 4.904 | 4.909 (+0.1%) | 4.859 (-0.9%) |

Every version searched the same number of nodes (15,523,314 per cube in the first row). The largest difference, 2.3% on h9 in batch mode for the znver4 build, is of 6 runs each and has the opposite sign in the native build; it was not followed up.

The machine code was compared by disassembling the binaries and comparing each function's instructions with addresses and the names' generic arguments removed. `Worker::rounds` is 1,246 instructions in the three native builds and 1,243 in the three znver4 builds, with none different between the commits, at the same offset within its page. Functions around it differ between `28a1953` and `c3eb909`: one field offset in `continue_local`, and `maintain` (once per round of the lanes), `restart` and `settle` (once per job) by 2 to 77 instructions. Between `c3eb909` and `dc153b3` the functions that differ are those of the lists (`solutions::Collector`, `lanes::share::all_solutions`) and of reading the input; nothing of the single-solution search does.

## 2. Lane counts

Mean wall seconds of two runs, `dc153b3`, one thread on this server (`window-1/lanes.txt`):

| Lanes | h6, 17-move, `-c 1` | h7, 17-move, `-c 1` | h8, 18-move, `-c 1` | h9, 18-move, `-c 1` | h6, `-c 0` | h7, `-c 0` | h8, `-c 0` | h9, `-c 0` |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 8 | 8.489 | 4.536 | 16.551 | 8.448 | 8.506 | 4.563 | 16.405 | 8.399 |
| 12 | 6.550 | 3.472 | 12.492 | 6.507 | 6.495 | 3.497 | 12.433 | 6.454 |
| 16 | 5.621 | 2.998 | 10.810 | 5.676 | 5.639 | 3.031 | 10.827 | 5.651 |
| 20 | 5.123 | 2.792 | 9.994 | 5.217 | 5.161 | 2.776 | 9.881 | 5.200 |
| 24 | 4.841 | 2.687 | 9.571 | 5.113 | 4.935 | 2.641 | 9.346 | 5.037 |
| 32 | 4.839 | 2.732 | 9.727 | 5.202 | 4.837 | 2.638 | 9.803 | 5.081 |
| 40 | 4.957 | 2.785 | 9.871 | 5.337 | 5.038 | 2.695 | 10.255 | 5.116 |
| 48 | 5.107 | 2.882 | 10.432 | 5.589 | 5.268 | 2.781 | 10.579 | 5.399 |

The node counts differ by less than 0.1% between lane counts, so the time differences are differences in speed. On the M2 (`m2/`), 16 to 48 lanes are within 1.5% of each other at 1, 4 and 8 threads, 40 lanes are 2 to 3% slower at 8 threads, and 8 lanes are 20% slower.

### What the earlier `bench tune` saved

`window-1/tune-old.txt` and `m2/m2-b.txt` hold its output (`dc153b3`). At one thread on h6 it measured between 0.17 and 1.83 solves per second over its eight lane counts: one to eleven cubes finished in each six-second window.

| Table | This server, 1 thread | This server, 8 threads | M2, 1 thread | M2, 8 threads |
|---|---:|---:|---:|---:|
| h6 | 48 (run twice) | 48 | 48 | 48 |
| h7 | 24 | 24 | 48 | 8 |
| h8 | 32 | 24 | | |
| h9 | 48 | 40 | | |

On the M2, 8 lanes for h7 are 20% slower than 16 to 32. That command also measured `bench stream`'s search, whose lanes hold one cube each and have other best counts than those of `solve`.

### His two runs, read with these lane counts (inferred)

His first run was `solve -t N` (batch mode, the default lanes: 40 for h6 to h8, 32 for h9 and h10, 24 for h11). His second was `solve -t N -c 1` after `bench tune` on every table. The stream that `bench tune` measured is the same on every machine, and one thread of his CPU is about as fast as one of this server, so its record at one thread was probably the one it made here. The change of his times from the first run to the second, against the change that those settings make here (first run's settings: `-c 0` with the default; second: `-c 1` with the lanes above):

| Table | His times, 1 thread (17-, 18-, 19-move sets) | Here, same change of settings |
|---|---:|---:|
| h6 | +2.1%, +4.7%, +5.3% | +1.4% (40 to 48 lanes) |
| h7 | -6.2%, -8.6%, -10.9% | -0.3% (40 to 24) |
| h8 | -6.2%, -6.7%, -6.7% | -5.1% (40 to 32) |
| h9 | +6.8%, +9.5%, +7.8% | +10.0% (32 to 48) |
| h10 | -2.9%, -1.6%, -1.5% | not measured |
| h11 | +0.1%, +0.3%, -0.2% | not measured |

The signs agree for h6, h8 and h9 and the sizes are close; h7 is flat here and 6 to 11% faster there. His tune records were not seen: this is an inference.

At 16 threads his second run is 33 to 40% faster than his first on h6 to h8, 26 to 36% on h9, 20 to 35% on h10, and from 11% faster to 9% slower on h11. The gain is where the default was 40 or 32 lanes and is absent where it was 24. That fits a CPU on which two threads share a core's caches (32 KB of first-level data cache on his, 48 KB here, and here one thread per core): 40 lanes for each of two threads no longer fit. This server has no second thread per core, so this could not be measured here.

## 3. Lists of all optimal solutions

h7, 16 threads on cores 32 to 47 of this server, one program at a time (`window-1/lists.txt`). "As published" is nissy-core master, which stops at 63 solutions per cube; "complete" is the copy without that cut.

| Scrambles | Solutions | egg-solver `dc153b3` | nissy-core, complete | nissy-core, as published | egg-solver, stopping at 63 per cube |
|---|---:|---:|---:|---:|---:|
| 18-move set (25) | 140 | 7.13 s | 60.74 s | 59.33 s (140) | |
| 19-move set, 6 scrambles | 326 | 20.83 s | 165.53 s | 141.42 s (296) | 18.53 s (296) |
| 20-move set, 3 scrambles | 609 | 100.47 s | 733.22 s | 242.00 s (189) | 34.40 s (189) |
| Superflip | 4,416 | 7.35 s | not run | 16.55 s (63) | |

`c3eb909`, the commit he benchmarked, took 7.19 s for the 18-move set and 268.19 s for the superflip: 36 times longer than `dc153b3`, which searches one job of each group of first moves that the superflip's 48 symmetries map onto each other. One optimal solution of each cube (no list) took `dc153b3` 1.35 s, 1.59 s and 7.17 s for the three sets.

His page has H48 ahead on 20-move scrambles (3.88 s against 5.08 s per cube on h11) and on the superflip (1.86 s against 15.38 s). Here nissy-core's time for the complete 20-move lists is 3.0 times its time for 63 solutions per cube, and egg-solver's 2.9 times; and the superflip's list is 36 times shorter with the symmetries.

## 4. The new `bench tune`

It measures, for each lane count, the nodes per second of the work-sharing search on a stream of random cubes, in a batch and with one cube at a time, at 1 thread, half the threads and all threads, in two passes (up and down the lane counts). The nodes of unfinished cubes count: `lanes::share::Pool::run` now returns its thread's node count. Of the lane counts within 1% of the fastest it saves the one nearest to the default.

On h7 with all 48 cores (`window-2/h7.txt`), against `solve` on the 19-move set (mean of two runs):

| Lanes | 24T `-c 1`: tune, M nodes/s | 24T `-c 1`: solve, s | 24T `-c 0`: tune | 24T `-c 0`: solve, s | 48T `-c 1`: tune | 48T `-c 1`: solve, s | 48T `-c 0`: tune | 48T `-c 0`: solve, s |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 8 | 1112 | 8.014 | 1107 | 7.582 | 2060 | 4.124 | 2115 | 4.086 |
| 12 | 1445 | 6.102 | 1447 | 5.696 | 2533 | 3.240 | 2725 | 3.085 |
| 16 | 1661 | 5.282 | 1668 | 4.969 | 2948 | 2.854 | 3084 | 2.781 |
| 20 | 1806 | 4.844 | 1825 | 4.494 | 3158 | 2.709 | 3284 | 2.557 |
| 24 | 1871 | 4.624 | 1905 | 4.351 | 3135 | 2.613 | 3337 | 2.536 |
| 32 | 1838 | 4.651 | 1872 | 4.457 | 3117 | 2.679 | 3232 | 2.553 |
| 40 | 1788 | 4.740 | 1828 | 4.458 | 3047 | 2.703 | 3128 | 2.622 |
| 48 | 1741 | 4.878 | 1772 | 4.704 | 2918 | 2.855 | 3023 | 2.641 |

The fastest `solve` is at 24 lanes in all four columns, and the measured speed follows the times. Before the 1% rule this run saved 24, 24, 24 for a batch (1, 24, 48 threads) and 32, 24, 20 for one cube at a time; the two that are not 24 were within 0.8% of 24 in the measurement, and 1.7% and 3.6% slower in `solve`, which is why the rule was added.

The final command (with the 1% rule) saved 24 lanes for h6, h7, h8 and h9 alike, for both modes at 1, 24 and 48 threads (`window-2/tune-final.txt`; about 3 minutes and 15 seconds per table). In its measurements 24 lanes were the fastest or within 0.4% of the fastest in every one of the 24 cases, 40 lanes 2.3 to 5.8% slower, 48 lanes 5.4 to 9.2%, 16 lanes 3.9 to 14.8% and 8 lanes 30 to 44%.

The fix does not change `solve`'s search code: every function of the search is the same machine code as in `dc153b3`, and the functions that differ are those that read the command line and the tune record. `bench`'s work-sharing thread differs at its end (it returns the node count); its frame and the loop it calls are as before. Mean wall seconds of 8 runs on h7 with 24 lanes (`window-2/h7.txt`):

| Run | dc153b3 | fix | Difference |
|---|---:|---:|---:|
| `bench batch --share=1`, 1 thread, 17-move set | 2.502 | 2.506 | +0.2% |
| `bench batch --share=0`, 48 threads, 19-move set | 2.493 | 2.516 | +0.9% |
| `bench batch --share=1`, 48 threads, 19-move set | 2.630 | 2.641 | +0.4% |
| `solve -c 1`, 1 thread, 17-move set | 2.498 | 2.499 | +0.0% |
| `solve -c 0`, 48 threads, 19-move set | 2.491 | 2.504 | +0.5% |
| `solve -c 1`, 48 threads, 19-move set | 2.622 | 2.610 | -0.4% |

## Miri

The change is `5bfcd2a`. Its library part is safe code: `Pool::run` returns the sum of its lanes' node and lookup counters, and tune records are read and written differently. The full Miri catalogues of [November 23](../miri-20261123/README.md) were not rerun on it. Its new unit tests were run under Miri (miri 0.1.0 of 2026-11-02, `-Zmiri-symbolic-alignment-check -Zmiri-num-cpus=4`) on the x86 backend (`-C target-cpu=x86-64-v3`) and the portable one (`-C target-cpu=x86-64`): `lanes::share::tests::share_run_returns_the_work_of_its_lanes` under 16 scheduler seeds, and `tables::file::tests::tune_files_are_read_by_mode` and `nearest_thread_count_is_by_ratio` once. All passed; `default_lanes_use_the_tune_file` uses files and is ignored under Miri. The logs are `miri/tune-tests-x86.log` and `miri/tune-tests-portable.log`. These are finite runs of unit tests in an interpreter, not a proof that the program has no undefined behavior.

## What this does not show

- Nothing was measured on his CPU or on any x86 CPU with two threads per core. The reading of his numbers in section 2 is an inference from them and from the lane counts measured here.
- The lists were timed on h7 only, on subsets of the 19- and 20-move sets, once each.
- The lane counts were measured on two machines. They are not a claim about others: that is what `bench tune` is for.

## Files

| Path | Content |
|---|---|
| `window-1/ab-h6.txt`, `ab-h9.txt` | The three commits, two target CPUs, one thread: table, set, mode, lanes, build, wall seconds, nodes per cube |
| `window-1/lanes.txt` | Lane counts at one thread, both modes |
| `window-1/tune-old.txt` | The earlier `bench tune` on h6 (twice), h7, h8, h9, with 8 cores |
| `window-1/lists.txt` | Lists against nissy-core |
| `window-2/h7.txt` | The new `bench tune` on h7 (before the 1% rule), `solve` over lane counts at 24 and 48 threads, the fix against `dc153b3` |
| `window-2/tune-final.txt` | The final `bench tune` on h6 to h9 (third window) |
| `m2/m2-a.txt`, `m2-b.txt` | Apple M2: the three commits, lane counts, the earlier `bench tune` |
| `miri/` | The targeted Miri runs of the new unit tests on `5bfcd2a` |
| `scripts/` | The scripts that made these files, and the two scramble subsets |
| `summarize.py` | Prints the tables of sections 1, 2 and 4 from the raw files |
