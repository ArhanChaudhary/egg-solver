# CUBEOPT comparison: all currently known results

The historical matrix below retains its original source and measurement conditions. The [November 8 all-500-state nissy comparison](NISSY_THROUGHPUT.md) uses complete finite-batch wall times on 1 GiB pages; its final results and raw outputs are maintained there. See [parallelism](../performance/PARALLELISM.md) for the new lane and single-cube scaling analysis.

Snapshot: **2026-11-07 15:18 UTC**. Final collection and validation are complete.

`egg-solver` is the Rust project previously named `cube-opt` / `cubeopt`.
`cubeopt30` and `cube48opt10` are separate prebuilt competitors and have their own rows.

The comparison measurements use commit **229b99f**. Current renamed source is
**842a5043abfd550313a86a3d9bb5254f4b457540**; its full native/scalar/ARM/WASM checks, hot-code comparisons and
legacy h8 search verification passed. Final h11 identity and performance comparisons are complete; their results are in the report.
Claude's completed ordinary nissy h7–h11 comparisons are retained. Replacements
for the separate distance-20 interference failure completed serially.

Accepted measurement records: **551**. Required report cells complete:
**230/230 (100.0%)**.
Percentages count complete cells; partial distance-20 samples do not produce aggregate
headline timings. All times exclude table generation and loading. Table sizes use
decimal GB; page sizes use GiB/MiB. Hardware-counter measurements are a separate dataset
from the portable phack kit and are not part of these VM timing tables.

This document includes both page types, 1T/48T, latency/batch/stream semantics,
wiki metrics, complete distance-20 samples, build screening,
fairness qualifications, and the complete coverage ledger below.

egg-solver: 229b99f Merge opt/u1-cli: clap, anyhow, thiserror, log/env_logger, --json, indicatif, clap_complete, criterion (user request). Machine: AMD EPYC 9V45, 48 cores (Zen 5), 94 GiB; THP enabled=madvise, defrag=madvise. Harness: cube_solver_test runtest.sh (extended), driven by final-compare.sh.

Cells: seconds per cube (sec/solv); at 48T the median over repeats [min-max, repeats]; at 1T (one run of 50 cubes, or two 25-cube chunks) ± the 95% interval of the mean over the cubes (this is the cube-to-cube spread; every solver gets the same cubes, see 'per cube' in the headline). 1G = 1 GiB hugetlb pages, THP = 2 MiB transparent huge pages: every solver is measured on both, and a speedup is only stated between equal page types, equal semantics and the same size group. 'size' is the table size relative to the egg-solver table it is compared with. GB·s = table GB x sec/solv at 48T. nodes/cube is each solver's own count (not comparable across solvers).

No-panic integration: in first-party sources (dependencies excluded), the same source scanner found 127 forbidden explicit calls at `0b5b5a6` and zero at `935cb7f` (including shipped backends, binaries and build scripts; tests and compile-time contexts excluded). The full backend checks enforce the policy. [Source inventory and remaining implicit-check diagnostics](#no-panic-source-inventory).


## Final source validation

The comparison tables measure commit `229b99f`. The later final source `842a5043abfd` adds
preflight, shared-memory support, the no-panic integration, the measured stream follow-up
and the completed egg-solver rename.
Its performance was checked separately against the immutable comparison binaries on h11
(1 GiB pages, 40 lanes; stream warmup/window 10/60 seconds at 1T and 20/30 at 48T).
The timing tables below retain their original measured provenance. Search node/lookup
counts and solution-length histograms match exactly on the verification datasets.

| check | paired runs | final / measured | change |
|---|---:|---:|---:|
| Stream throughput 1T | 8 | 0.987217 | -1.28% |
| Stream throughput 48T | 5 | 0.989851 | -1.01% |
| Latency 48T random_state | 6 | 1.001574 | +0.16% time |
| Latency 48T random_move_17f | 6 | 1.028380 | +2.84% time |
| Latency 48T random_state_depth17 | 6 | 1.012014 | +1.20% time |

Ratios are medians of paired ratios; higher throughput and lower latency are better.

**Final stream throughput regressed by more than 0.5%:** 1T (-1.28%), 48T (-1.01%).
This measured difference remains disclosed; the competitor tables were not measured on this later build.

The project rename passed full native/scalar/ARM/WASM checks and was separately
validated for identical hot instructions, required placement and stack frames.
Existing environment names remain fallback aliases and legacy tables remain readable.

Final renamed main: `842a5043abfd550313a86a3d9bb5254f4b457540` in `~/egg-solver`. Full native/scalar/ARM/WASM checks, legacy h8/h11 search identity and hot-code comparisons passed. The temporary old repository-path alias has been removed.


## Fresh Miri validation

The x86 AVX2/BMI2 and portable solver paths passed Miri with symbolic-alignment checks and four virtual CPUs. Each path also passed all catalogued shared-search tests under scheduling seeds 0 through 15. The staged-sort tests execute disjoint tasks concurrently with up to three workers and join before every stage barrier. A contention test also exercises the mutex park/wake path.

| Path | Catalogued unit tests | Executed unique tests | Explicitly ignored | Shared-search seeds |
|---|---:|---:|---:|---:|
| x86 | 56 | 50 | 6 | 16 |
| portable | 54 | 48 | 6 | 16 |

Explicit Miri exclusions:

- `solver::tests::hot_index_matches_reference_index`: Cocsep::generate is too slow under Miri.
- `tables::file::tests::trailer_write_keeps_data`: files.
- `tables::tests::cocsep_classes_and_distribution`: Cocsep::generate is too slow under Miri.
- `tables::tests::opened_table_file_is_read_only`: files.
- `tables::tests::table_from_file_memory_is_checked`: a 29 MiB checksum is too slow under Miri.
- `tables::tests::table_meta_and_size_are_checked`: files.

Linux SysV shared-memory calls, native inline assembly, AVX-512, NEON and WASM-specific code were not interpreted by these runs. The native, portable, ARM/QEMU, WASI/wasmtime and Bun/WASM checks validate those supported paths separately; full table-generation and solver integration tests run natively. Miri checks the executions and schedules exercised by the tests; it does not prove safety for every input or interleaving.

The initial optional strict-provenance attempt stopped at pointer tagging in `parking_lot_core 0.9.12` (`word_lock.rs:320`, seed14): an unsupported operation in that mode, not a reported undefined-behavior finding. The accepted runs use default provenance handling, retain the pointer-cast warnings, and keep aliasing, alignment, bounds, validity and race checks enabled. The original diagnostic is preserved separately.

A subsequent run rejected `parking_lot_core 0.9.12` Linux futex c-variadic pointer arguments under seed15. The production dependency contains a minimal backport of upstream explicit raw-pointer conversions; the accepted Miri runs use that same dependency source.

Toolchain: `miri 0.1.0 (1c3a650696 2026-11-02)`. Source SHA-256: `93478bc5c45ba267255574f8cb466f02f006f1c7bf558fb8f9d088ec4307a4c9`. Completed: 2026-11-07 14:47:20 UTC. Raw logs and catalogues are retained in `/data/agents/codex-resume/miri-current/` and `miri-portable/`. Build, individual-worker and catalogue-coverage failures all fail the runner; failure-injection probes verify those guards.


## Latency: one cube at a time on all threads (egg-solver solve -c 1 vs nissy H48, cubeopt30, cube48opt10)


### ~2.6-3.9 GB tables (latency)

| solver | build | table GB (size) | 1T 1G | 1T THP | 48T 1G | 48T THP | GB·s 48T 1G / THP | nodes/cube | pages measured (1G / THP rows) | egg-solver faster by (same pages) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| egg-solver h7 | SIMD cube | 3.89 (1.00x) | 0.604 (±36%) | 0.632 (±37%) | 0.0178 [0.0169-0.0180, 3] | 0.0181 [0.0175-0.0186, 3] | 0.069 / 0.070 | 51.80M | 1G 100% / THP 100% | - |
| cubeopt30 | prebuilt (no PGO possible) | 2.60 (0.67x) | 6.120 (±38%) | 6.409 (±38%) | 0.2458 [0.2458-0.2518, 3] | 0.2467 [0.2434-0.2519, 4] | 0.640 / 0.642 | 64.53M | 1G 100% / THP 100% | 1G 48T 13.8x, 1G 1T 10.1x, THP 48T 13.6x, THP 1T 10.1x |
| h48h7 | shipped | 3.79 (0.97x) | 4.275 (±37%) | 4.597 (±30%) | 0.1655 [0.1596-0.1670, 3] | 0.1631 [0.1624-0.1682, 4] | 0.628 / 0.619 | 72.43M | 1G 100% / THP 100% | 1G 48T 9.3x, 1G 1T 7.1x, THP 48T 9.0x, THP 1T 7.3x |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) | pfpgo | 3.89 (1.00x) | 2.693 (±38%) | 2.751 (±38%) | 0.1701 [0.1696-0.1713, 3] | 0.1702 [0.1695-0.1703, 3] | 0.662 / 0.662 | 49.61M | 1G 100% / THP 100% | 1G 48T 9.6x, 1G 1T 4.5x, THP 48T 9.4x, THP 1T 4.4x |

### ~7.6-7.8 GB tables (latency)

| solver | build | table GB (size) | 1T 1G | 1T THP | 48T 1G | 48T THP | GB·s 48T 1G / THP | nodes/cube | pages measured (1G / THP rows) | egg-solver faster by (same pages) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| egg-solver h8 | SIMD cube | 7.78 (1.00x) | 0.267 (±36%) | 0.285 (±36%) | 0.0079 [0.0079-0.0083, 3] | 0.0085 [0.0083-0.0090, 3] | 0.062 / 0.066 | 23.35M | 1G 100% / THP 100% | - |
| h48h8 | shipped | 7.59 (0.97x) | 2.913 (±32%) | 2.953 (±32%) | 0.1205 [0.1198-0.1224, 3] | 0.1229 [0.1224-0.1233, 3] | 0.914 / 0.932 | 45.89M | 1G 100% / THP 100% | 1G 48T 15.2x, 1G 1T 10.9x, THP 48T 14.5x, THP 1T 10.4x |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) | native | 7.78 (1.00x) | 1.048 (±38%) | 1.054 (±38%) | 0.0774 [0.0764-0.0776, 3] | 0.0773 [0.0768-0.0792, 3] | 0.603 / 0.602 | 18.44M | 1G 100% / THP 85% | 1G 48T 9.8x, 1G 1T 3.9x, THP 48T 9.1x, THP 1T 3.7x |

### large tables (1 GiB pages only) (latency)

| solver | build | table GB (size) | 1T 1G | 1T THP | 48T 1G | 48T THP | GB·s 48T 1G / THP | nodes/cube | pages measured (1G / THP rows) | egg-solver faster by (same pages) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| egg-solver h9 | SIMD cube | 15.57 (1.00x) | 0.133 (±35%) | - | 0.0041 [0.0041-0.0042, 3] | - | 0.064 / - | 11.86M | 1G 100% | - |
| egg-solver h10 | SIMD cube | 31.13 (1.00x) | 0.068 (±34%) | - | 0.0024 [0.0024-0.0027, 3] | - | 0.075 / - | 6.29M | 1G 100% | - |
| egg-solver h11 | SIMD cube | 62.26 (1.00x) | 0.041 (±33%) | - | 0.0017 [0.0016-0.0018, 3] | - | 0.104 / - | 3.70M | 1G 100% | - |
| cube48opt10 | prebuilt (no PGO possible) | 31.13 (1.00x) | 0.369 (±37%) | - | 0.0489 [0.0489-0.0493, 3] | - | 1.522 / - | 7.17M | 1G 100% | 1G 48T 20.3x, 1G 1T 5.4x |
| h48h9 | shipped | 15.17 (0.97x) | 1.278 (±33%) | - | 0.0765 [0.0763-0.0773, 3] | - | 1.160 / - | 20.67M | 1G 100% | 1G 48T 18.6x, 1G 1T 9.6x |
| h48h10 | shipped | 30.34 (0.97x) | 0.714 (±32%) | - | 0.0594 [0.0592-0.0609, 3] | - | 1.802 / - | 11.13M | 1G 100% | 1G 48T 24.6x, 1G 1T 10.4x |
| h48h11 | shipped | 60.67 (0.97x) | 0.423 (±31%) | - | 0.0524 [0.0513-0.0524, 3] | - | 3.179 / - | 6.84M | 1G 100% | 1G 48T 31.4x, 1G 1T 10.4x |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) | pfnative | 31.13 (1.00x) | 0.293 (±39%) | - | 0.0209 [0.0209-0.0211, 3] | - | 0.652 / - | 4.92M | 1G 100% | 1G 48T 8.7x, 1G 1T 4.3x |

## Throughput: many cubes in flight (egg-solver solve -c 0 / bench stream vs nxopt -t, vcube -w, which solve one cube per thread)


### ~2.6-3.9 GB tables (throughput)

| solver | build | table GB (size) | 1T 1G | 1T THP | 48T 1G | 48T THP | GB·s 48T 1G / THP | nodes/cube | pages measured (1G / THP rows) | egg-solver faster by (same pages) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| egg-solver h7 batch | SIMD cube | 3.89 (1.00x) | 0.602 | 0.624 | 0.0156 [0.0154-0.0168, 3] | 0.0164 [0.0162-0.0172, 3] | 0.061 / 0.064 | 51.36M | 1G 100% / THP 100% | - |
| nxopt31 | pgo (-march=native) | 2.60 (0.67x) | 13.938 (±38%) | 13.927 (±38%) | 0.3030 [0.2941-0.3061, 3] | 0.2973 [0.2958-0.3105, 3] | 0.789 / 0.774 | 67.45M | 1G 100% / THP 100% | 1G 48T 19.4x, 1G 1T 23.2x, THP 48T 18.2x, THP 1T 22.3x |
| vcube112 | shipped | 2.60 (0.67x) | 8.743 (±38%) | 8.963 (±31%) | 0.1943 [0.1916-0.1946, 3] | 0.1922 [0.1903-0.1975, 4] | 0.506 / 0.500 | 66.87M | 1G 100% / THP 100% | 1G 48T 12.4x, 1G 1T 14.5x, THP 48T 11.7x, THP 1T 14.4x |
| cube48opt-rust Opt7 batch | pfpgo | 3.89 (1.00x) | 2.703 (±38%) | 2.729 (±38%) | 0.0590 [0.0590-0.0592, 3] | 0.0595 [0.0594-0.0599, 3] | 0.229 / 0.232 | 48.73M | 1G 100% / THP 98% | 1G 48T 3.8x, 1G 1T 4.5x, THP 48T 3.6x, THP 1T 4.4x |

### ~7.6-7.8 GB tables (throughput)

| solver | build | table GB (size) | 1T 1G | 1T THP | 48T 1G | 48T THP | GB·s 48T 1G / THP | nodes/cube | pages measured (1G / THP rows) | egg-solver faster by (same pages) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| egg-solver h8 batch | SIMD cube | 7.78 (1.00x) | 0.271 | 0.284 | 0.0076 [0.0076-0.0077, 3] | 0.0081 [0.0080-0.0085, 3] | 0.059 / 0.063 | 24.13M | 1G 100% / THP 100% | - |
| nxopt22 | pgo (-march=native) | 7.81 (1.00x) | 5.760 (±39%) | 5.956 (±39%) | 0.1370 [0.1337-0.1401, 3] | 0.1426 [0.1408-0.1427, 3] | 1.070 / 1.114 | 27.87M | 1G 100% / THP 77% | 1G 48T 18.0x, 1G 1T 21.3x, THP 48T 17.6x, THP 1T 21.0x |
| vcube208 | shipped | 7.81 (1.00x) | 3.858 (±37%) | 4.012 (±30%) | 0.0939 [0.0930-0.0949, 3] | 0.0954 [0.0953-0.0965, 3] | 0.733 / 0.745 | 27.98M | 1G 100% / THP 77% | 1G 48T 12.3x, 1G 1T 14.2x, THP 48T 11.8x, THP 1T 14.1x |
| cube48opt-rust Opt8 batch | native | 7.78 (1.00x) | 1.044 (±38%) | 1.057 (±38%) | 0.0242 [0.0239-0.0243, 3] | 0.0243 [0.0243-0.0253, 3] | 0.189 / 0.189 | 17.50M | 1G 100% / THP 100% | 1G 48T 3.2x, 1G 1T 3.9x, THP 48T 3.0x, THP 1T 3.7x |

### large tables (1 GiB pages only) (throughput)

| solver | build | table GB (size) | 1T 1G | 1T THP | 48T 1G | 48T THP | GB·s 48T 1G / THP | nodes/cube | pages measured (1G / THP rows) | egg-solver faster by (same pages) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| egg-solver h9 batch | SIMD cube | 15.57 (1.00x) | 0.133 | - | 0.0036 [0.0035-0.0036, 3] | - | 0.056 / - | 11.73M | 1G 100% | - |
| egg-solver h10 batch | SIMD cube | 31.13 (1.00x) | 0.067 | - | 0.0019 [0.0019-0.0019, 3] | - | 0.058 / - | 6.16M | 1G 100% | - |
| egg-solver h11 batch | SIMD cube | 62.26 (1.00x) | 0.041 | - | 0.0012 [0.0011-0.0012, 3] | - | 0.072 / - | 3.52M | 1G 100% | - |
| egg-solver h11 stream | SIMD cube | 62.26 (1.00x) | 0.040 | - | 0.0009 [0.0009-0.0010, 3] | - | 0.058 / - | 3.05M | 1G 100% | - |
| vcube308 | shipped | 22.78 (1.46x) | 1.412 (±36%) | - | 0.0287 [0.0285-0.0287, 3] | - | 0.653 / - | 9.19M | 1G 100% | 1G 48T 7.9x, 1G 1T 10.6x |
| cube48opt-rust Opt10 batch | pfnative | 31.13 (1.00x) | 0.292 (±39%) | - | 0.0067 [0.0067-0.0067, 3] | - | 0.208 / - | 4.81M | 1G 100% | 1G 48T 3.6x, 1G 1T 4.4x |

## Headline (same page type, same semantics, same size group)

egg-solver uses the default SIMD build, compared against the same competitor rows.

| semantics | group | competitor | size vs egg-solver | pages | threads | egg-solver faster by (sec/solv) | per cube |
|---|---|---|---:|---|---:|---:|---|
| latency | ~2.6-3.9 GB tables | cubeopt30 (prebuilt binary: not profile-tunable) vs egg-solver h7 | 0.67x | 1G | 48 | 13.8x | 200 cubes: geo-mean 18.0x, 10-90% 8.7-37.6x |
| latency | ~2.6-3.9 GB tables | cubeopt30 (prebuilt binary: not profile-tunable) vs egg-solver h7 | 0.67x | 1G | 1 | 10.1x | 50 cubes: geo-mean 9.7x, 10-90% 5.1-18.7x |
| latency | ~2.6-3.9 GB tables | cubeopt30 (prebuilt binary: not profile-tunable) vs egg-solver h7 | 0.67x | THP | 48 | 13.6x | 200 cubes: geo-mean 17.4x, 10-90% 8.2-36.3x |
| latency | ~2.6-3.9 GB tables | cubeopt30 (prebuilt binary: not profile-tunable) vs egg-solver h7 | 0.67x | THP | 1 | 10.1x | 50 cubes: geo-mean 9.7x, 10-90% 4.7-18.4x |
| latency | ~2.6-3.9 GB tables | h48h7 vs egg-solver h7 | 0.97x | 1G | 48 | 9.3x | 200 cubes: geo-mean 12.8x, 10-90% 6.2-30.3x |
| latency | ~2.6-3.9 GB tables | h48h7 vs egg-solver h7 | 0.97x | 1G | 1 | 7.1x | 50 cubes: geo-mean 7.3x, 10-90% 4.1-12.6x |
| latency | ~2.6-3.9 GB tables | h48h7 vs egg-solver h7 | 0.97x | THP | 48 | 9.0x | 200 cubes: geo-mean 12.5x, 10-90% 6.0-29.5x |
| latency | ~2.6-3.9 GB tables | h48h7 vs egg-solver h7 | 0.97x | THP | 1 | 7.3x | 50 cubes: geo-mean 7.4x, 10-90% 4.1-13.0x |
| latency | ~2.6-3.9 GB tables | cube48opt-rust Opt7 (latency: 18 threads, its cap) vs egg-solver h7 | 1.00x | 1G | 48 | 9.6x | 200 cubes: geo-mean 10.0x, 10-90% 6.0-17.4x |
| latency | ~2.6-3.9 GB tables | cube48opt-rust Opt7 (latency: 18 threads, its cap) vs egg-solver h7 | 1.00x | 1G | 1 | 4.5x | 50 cubes: geo-mean 4.2x, 10-90% 2.6-7.1x |
| latency | ~2.6-3.9 GB tables | cube48opt-rust Opt7 (latency: 18 threads, its cap) vs egg-solver h7 | 1.00x | THP | 48 | 9.4x | 200 cubes: geo-mean 9.8x, 10-90% 5.6-16.4x |
| latency | ~2.6-3.9 GB tables | cube48opt-rust Opt7 (latency: 18 threads, its cap) vs egg-solver h7 | 1.00x | THP | 1 | 4.4x | 50 cubes: geo-mean 4.1x, 10-90% 2.5-7.1x |
| latency | ~7.6-7.8 GB tables | h48h8 vs egg-solver h8 | 0.97x | 1G | 48 | 15.2x | 200 cubes: geo-mean 19.7x, 10-90% 10.5-42.4x |
| latency | ~7.6-7.8 GB tables | h48h8 vs egg-solver h8 | 0.97x | 1G | 1 | 10.9x | 50 cubes: geo-mean 11.9x, 10-90% 7.9-20.1x |
| latency | ~7.6-7.8 GB tables | h48h8 vs egg-solver h8 | 0.97x | THP | 48 | 14.5x | 200 cubes: geo-mean 19.0x, 10-90% 10.5-39.0x |
| latency | ~7.6-7.8 GB tables | h48h8 vs egg-solver h8 | 0.97x | THP | 1 | 10.4x | 50 cubes: geo-mean 11.3x, 10-90% 7.5-18.8x |
| latency | ~7.6-7.8 GB tables | cube48opt-rust Opt8 (latency: 18 threads, its cap) vs egg-solver h8 | 1.00x | 1G | 48 | 9.8x | 200 cubes: geo-mean 9.9x, 10-90% 5.3-18.2x |
| latency | ~7.6-7.8 GB tables | cube48opt-rust Opt8 (latency: 18 threads, its cap) vs egg-solver h8 | 1.00x | 1G | 1 | 3.9x | 50 cubes: geo-mean 3.7x, 10-90% 2.0-6.1x |
| latency | ~7.6-7.8 GB tables | cube48opt-rust Opt8 (latency: 18 threads, its cap) vs egg-solver h8 | 1.00x | THP | 48 | 9.1x | 200 cubes: geo-mean 9.5x, 10-90% 5.2-17.7x |
| latency | ~7.6-7.8 GB tables | cube48opt-rust Opt8 (latency: 18 threads, its cap) vs egg-solver h8 | 1.00x | THP | 1 | 3.7x | 50 cubes: geo-mean 3.5x, 10-90% 1.9-5.8x |
| latency | large tables (1 GiB pages only) | cube48opt10 (prebuilt binary: not profile-tunable) vs egg-solver h10 | 1.00x | 1G | 48 | 20.3x | 200 cubes: geo-mean 23.6x, 10-90% 13.3-36.2x |
| latency | large tables (1 GiB pages only) | cube48opt10 (prebuilt binary: not profile-tunable) vs egg-solver h10 | 1.00x | 1G | 1 | 5.4x | 50 cubes: geo-mean 4.8x, 10-90% 3.3-8.4x |
| latency | large tables (1 GiB pages only) | h48h9 vs egg-solver h9 | 0.97x | 1G | 48 | 18.6x | 200 cubes: geo-mean 25.6x, 10-90% 13.1-55.6x |
| latency | large tables (1 GiB pages only) | h48h9 vs egg-solver h9 | 0.97x | 1G | 1 | 9.6x | 50 cubes: geo-mean 10.9x, 10-90% 6.0-19.1x |
| latency | large tables (1 GiB pages only) | h48h10 vs egg-solver h10 | 0.97x | 1G | 48 | 24.6x | 200 cubes: geo-mean 33.1x, 10-90% 15.7-72.0x |
| latency | large tables (1 GiB pages only) | h48h10 vs egg-solver h10 | 0.97x | 1G | 1 | 10.4x | 50 cubes: geo-mean 11.9x, 10-90% 6.7-30.6x |
| latency | large tables (1 GiB pages only) | h48h11 vs egg-solver h11 | 0.97x | 1G | 48 | 31.4x | 200 cubes: geo-mean 41.5x, 10-90% 19.6-86.4x |
| latency | large tables (1 GiB pages only) | h48h11 vs egg-solver h11 | 0.97x | 1G | 1 | 10.4x | 50 cubes: geo-mean 12.4x, 10-90% 7.4-33.0x |
| latency | large tables (1 GiB pages only) | cube48opt-rust Opt10 (latency: 18 threads, its cap) vs egg-solver h10 | 1.00x | 1G | 48 | 8.7x | 200 cubes: geo-mean 7.6x, 10-90% 4.4-13.3x |
| latency | large tables (1 GiB pages only) | cube48opt-rust Opt10 (latency: 18 threads, its cap) vs egg-solver h10 | 1.00x | 1G | 1 | 4.3x | 50 cubes: geo-mean 3.7x, 10-90% 2.1-6.9x |
| throughput | ~2.6-3.9 GB tables | nxopt31 vs egg-solver h7 | 0.67x | 1G | 48 | 19.4x | - |
| throughput | ~2.6-3.9 GB tables | nxopt31 vs egg-solver h7 | 0.67x | 1G | 1 | 23.2x | - |
| throughput | ~2.6-3.9 GB tables | nxopt31 vs egg-solver h7 | 0.67x | THP | 48 | 18.2x | - |
| throughput | ~2.6-3.9 GB tables | nxopt31 vs egg-solver h7 | 0.67x | THP | 1 | 22.3x | - |
| throughput | ~2.6-3.9 GB tables | vcube112 vs egg-solver h7 | 0.67x | 1G | 48 | 12.4x | - |
| throughput | ~2.6-3.9 GB tables | vcube112 vs egg-solver h7 | 0.67x | 1G | 1 | 14.5x | - |
| throughput | ~2.6-3.9 GB tables | vcube112 vs egg-solver h7 | 0.67x | THP | 48 | 11.7x | - |
| throughput | ~2.6-3.9 GB tables | vcube112 vs egg-solver h7 | 0.67x | THP | 1 | 14.4x | - |
| throughput | ~2.6-3.9 GB tables | cube48opt-rust Opt7 batch vs egg-solver h7 | 1.00x | 1G | 48 | 3.8x | - |
| throughput | ~2.6-3.9 GB tables | cube48opt-rust Opt7 batch vs egg-solver h7 | 1.00x | 1G | 1 | 4.5x | - |
| throughput | ~2.6-3.9 GB tables | cube48opt-rust Opt7 batch vs egg-solver h7 | 1.00x | THP | 48 | 3.6x | - |
| throughput | ~2.6-3.9 GB tables | cube48opt-rust Opt7 batch vs egg-solver h7 | 1.00x | THP | 1 | 4.4x | - |
| throughput | ~7.6-7.8 GB tables | nxopt22 vs egg-solver h8 | 1.00x | 1G | 48 | 18.0x | - |
| throughput | ~7.6-7.8 GB tables | nxopt22 vs egg-solver h8 | 1.00x | 1G | 1 | 21.3x | - |
| throughput | ~7.6-7.8 GB tables | nxopt22 vs egg-solver h8 | 1.00x | THP | 48 | 17.6x | - |
| throughput | ~7.6-7.8 GB tables | nxopt22 vs egg-solver h8 | 1.00x | THP | 1 | 21.0x | - |
| throughput | ~7.6-7.8 GB tables | vcube208 vs egg-solver h8 | 1.00x | 1G | 48 | 12.3x | - |
| throughput | ~7.6-7.8 GB tables | vcube208 vs egg-solver h8 | 1.00x | 1G | 1 | 14.2x | - |
| throughput | ~7.6-7.8 GB tables | vcube208 vs egg-solver h8 | 1.00x | THP | 48 | 11.8x | - |
| throughput | ~7.6-7.8 GB tables | vcube208 vs egg-solver h8 | 1.00x | THP | 1 | 14.1x | - |
| throughput | ~7.6-7.8 GB tables | cube48opt-rust Opt8 batch vs egg-solver h8 | 1.00x | 1G | 48 | 3.2x | - |
| throughput | ~7.6-7.8 GB tables | cube48opt-rust Opt8 batch vs egg-solver h8 | 1.00x | 1G | 1 | 3.9x | - |
| throughput | ~7.6-7.8 GB tables | cube48opt-rust Opt8 batch vs egg-solver h8 | 1.00x | THP | 48 | 3.0x | - |
| throughput | ~7.6-7.8 GB tables | cube48opt-rust Opt8 batch vs egg-solver h8 | 1.00x | THP | 1 | 3.7x | - |
| throughput | large tables (1 GiB pages only) | vcube308 vs egg-solver h9 | 1.46x | 1G | 48 | 7.9x | - |
| throughput | large tables (1 GiB pages only) | vcube308 vs egg-solver h9 | 1.46x | 1G | 1 | 10.6x | - |
| throughput | large tables (1 GiB pages only) | cube48opt-rust Opt10 batch vs egg-solver h10 | 1.00x | 1G | 48 | 3.6x | - |
| throughput | large tables (1 GiB pages only) | cube48opt-rust Opt10 batch vs egg-solver h10 | 1.00x | 1G | 1 | 4.4x | - |

## cube_solver_test wiki metrics (this machine)

The wiki's three tables, measured here: first 100 cubes of random_move_15f/16f/17f and of random_state ('inf'; fewer where marked), nodes from 1-thread runs (`solve -t 1 -c 1` for egg-solver). **Node definitions differ between solvers** (what one node is: a move generated, an evaluation, a lookup...), so node counts and space-node products compare search-plus-table efficiency only roughly; nissy's counts also vary run to run (competitor-bugs.md #7), so its values are means over the runs of each cube.

### Space-node product (table bytes x nodes visited, in 1e15 bytes; less is better)

| solver | table MB | 15f | 16f | 17f | inf |
|---|---:|---:|---:|---:|---:|
| egg-solver h7 | 3891.36 | 0.318 | 2.108 | 24.926 | 197.633 |
| egg-solver h8 | 7782.73 | 0.345 | 3.824 | 27.560 | 177.544 |
| egg-solver h9 | 15565.46 | 0.369 | 4.000 | 26.512 | 179.027 |
| egg-solver h10 | 31130.91 | 0.424 | 4.320 | 28.147 | 191.665 |
| egg-solver h11 | 62261.82 | 0.505 | 4.813 | 29.216 | 220.061 |
| nxopt31 | 2603.09 | 0.316 | 2.248 | 20.307 | 183.400 |
| nxopt22 | 7809.27 | 0.661 | 2.937 | 27.801 | 216.594 |
| vcube112 | 2603.09 | 0.278 | 2.308 | 20.400 | 182.080 |
| vcube208 | 7809.27 | 0.583 | 2.871 | 28.185 | 217.275 |
| vcube308 | 22777.04 | 0.504 | 3.444 | 28.040 | 221.600 |
| h48h7 | 3793.84 | 2.676 | 6.014 | 38.722 | 276.653 |
| h48h8 | 7585.62 | 5.077 | 10.014 | 55.752 | 341.650 |
| h48h9 | 15169.19 | 8.794 | 14.341 | 56.709 | 312.032 |
| h48h10 | 30336.31 | 16.269 | 23.360 | 69.160 | 345.212 |
| h48h11 | 60670.57 | 31.156 | 40.772 | 96.834 | 428.438 |
| cubeopt30 | 2603.09 | 0.186 | 1.859 | 18.324 | 175.939 |
| cube48opt10 | 31130.91 | 0.454 | 3.043 | 29.984 | 223.038 |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) | 3891.36 | 0.240 | 1.961 | 23.802 | 199.606 |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) | 7782.73 | 0.218 | 1.737 | 18.439 | 141.069 |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) | 31130.91 | 0.275 | 1.974 | 19.605 | 153.950 |

### Number of nodes visited (millions per cube; less is better)

| solver | 15-move | 16-move | 17-move | inf-move |
|---|---:|---:|---:|---:|
| egg-solver h7 | 0.082 | 0.542 | 6.405 | 50.788 |
| egg-solver h8 | 0.044 | 0.491 | 3.541 | 22.813 |
| egg-solver h9 | 0.024 | 0.257 | 1.703 | 11.502 |
| egg-solver h10 | 0.014 | 0.139 | 0.904 | 6.157 |
| egg-solver h11 | 0.008 | 0.077 | 0.469 | 3.534 |
| nxopt31 | 0.121 | 0.864 | 7.801 | 70.455 |
| nxopt22 | 0.085 | 0.376 | 3.560 | 27.736 |
| vcube112 | 0.107 | 0.886 | 7.837 | 69.947 |
| vcube208 | 0.075 | 0.368 | 3.609 | 27.823 |
| vcube308 | 0.022 | 0.151 | 1.231 | 9.729 |
| h48h7 | 0.705 | 1.585 | 10.207 | 72.922 |
| h48h8 | 0.669 | 1.320 | 7.350 | 45.039 |
| h48h9 | 0.580 | 0.945 | 3.738 | 20.570 |
| h48h10 | 0.536 | 0.770 | 2.280 | 11.380 |
| h48h11 | 0.514 | 0.672 | 1.596 | 7.062 |
| cubeopt30 | 0.071 | 0.714 | 7.039 | 67.588 |
| cube48opt10 | 0.015 | 0.098 | 0.963 | 7.165 |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) | 0.062 | 0.504 | 6.117 | 51.295 |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) | 0.028 | 0.223 | 2.369 | 18.126 |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) | 0.009 | 0.063 | 0.630 | 4.945 |

### Average time per node (random_state; harness tt, ns/node = sec/solv / nodes per cube of the same run)

1 GiB pages, chosen builds; THP where no 1 GiB row exists (marked). 1T timings: the same first 50 cubes, 48T: 200 cubes, median of 3 repeats. 48T sec/solv is a latency for egg-solver, nissy and cubeopt, a throughput (one cube per thread) for nxopt and vcube, as in the wiki.

Single-thread results

| solver | ns/node | sec/solv | GB*sec |
|---|---:|---:|---:|
| egg-solver h7 | 11.81 | 0.6039 | 2.350 |
| egg-solver h8 | 11.96 | 0.2673 | 2.080 |
| egg-solver h9 | 11.55 | 0.1325 | 2.063 |
| egg-solver h10 | 11.67 | 0.0684 | 2.130 |
| egg-solver h11 | 12.16 | 0.0405 | 2.524 |
| nxopt31 | 206.48 | 13.9384 | 36.283 |
| nxopt22 | 225.98 | 5.7604 | 44.985 |
| vcube112 | 127.93 | 8.7433 | 22.760 |
| vcube208 | 148.98 | 3.8579 | 30.127 |
| vcube308 | 143.98 | 1.4121 | 32.163 |
| h48h7 | 59.89 | 4.2749 | 16.218 |
| h48h8 | 63.50 | 2.9134 | 22.100 |
| h48h9 | 61.94 | 1.2780 | 19.387 |
| h48h10 | 64.30 | 0.7136 | 21.649 |
| h48h11 | 58.96 | 0.4232 | 25.675 |
| cubeopt30 | 96.34 | 6.1203 | 15.932 |
| cube48opt10 | 52.74 | 0.3688 | 11.482 |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) | 53.20 | 2.6933 | 10.481 |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) | 60.87 | 1.0484 | 8.160 |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) | 61.53 | 0.2925 | 9.107 |

All-thread (48) results

| solver | ns/node | sec/solv | GB*sec |
|---|---:|---:|---:|
| egg-solver h7 | 0.34 | 0.0178 | 0.069 |
| egg-solver h8 | 0.34 | 0.0079 | 0.062 |
| egg-solver h9 | 0.35 | 0.0041 | 0.064 |
| egg-solver h10 | 0.38 | 0.0024 | 0.075 |
| egg-solver h11 | 0.45 | 0.0017 | 0.104 |
| nxopt31 | 4.49 | 0.3030 | 0.789 |
| nxopt22 | 4.92 | 0.1370 | 1.070 |
| vcube112 | 2.91 | 0.1943 | 0.506 |
| vcube208 | 3.36 | 0.0939 | 0.733 |
| vcube308 | 3.12 | 0.0287 | 0.653 |
| h48h7 | 2.28 | 0.1655 | 0.628 |
| h48h8 | 2.63 | 0.1205 | 0.914 |
| h48h9 | 3.70 | 0.0765 | 1.160 |
| h48h10 | 5.33 | 0.0594 | 1.802 |
| h48h11 | 7.66 | 0.0524 | 3.179 |
| cubeopt30 | 3.81 | 0.2458 | 0.640 |
| cube48opt10 | 6.82 | 0.0489 | 1.522 |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) | 3.43 | 0.1701 | 0.662 |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) | 4.20 | 0.0774 | 0.603 |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) | 4.25 | 0.0209 | 0.652 |

### Reference: the wiki's published values (Intel Core i7-10750H, 6 cores / 12 threads, THP always; NOT this machine)

| solver | table MB | space-node 15f / 16f / 17f / inf | nodes (M) 15 / 16 / 17 / inf |
|---|---:|---|---|
| nxopt31 | 2603.08 | 0.315 / 2.246 / 20.307 / 183.431 | 0.121 / 0.863 / 7.801 / 70.467 |
| nxopt22 | 7809.26 | 0.664 / 2.936 / 28.020 / 216.542 | 0.085 / 0.376 / 3.588 / 27.729 |
| vcube112 | 2603.08 | 0.279 / 2.306 / 20.400 / 182.078 | 0.107 / 0.886 / 7.837 / 69.947 |
| vcube208 | 7809.26 | 0.586 / 2.874 / 28.184 / 217.277 | 0.075 / 0.368 / 3.609 / 27.823 |
| vcube308 | 22777.0 | 0.501 / 3.439 / 28.038 / 221.597 | 0.022 / 0.151 / 1.231 / 9.729 |
| h48h7 | 3793.84 | 2.713 / 5.990 / 37.688 / 277.246 | 0.715 / 1.579 / 9.934 / 73.078 |
| h48h8 | 7585.62 | 5.158 / 10.491 / 54.859 / 343.333 | 0.680 / 1.383 / 7.232 / 45.261 |
| h48h9 | 15169.1 | 8.904 / 14.881 / 56.217 / 312.741 | 0.587 / 0.981 / 3.706 / 20.617 |
| cubeopt30 | 2603.08 | 0.185 / 1.859 / 18.323 / 175.937 | 0.071 / 0.714 / 7.039 / 67.588 |
| cube48opt7 | 3891.36 | 0.241 / 1.965 / 23.780 / 199.615 | 0.062 / 0.505 / 6.111 / 51.297 |
| cube48opt8 | 7782.72 | 0.335 / 2.483 / 26.780 / 197.300 | 0.043 / 0.319 / 3.441 / 25.351 |
| cube48opt9 | 15565.4 | 0.342 / 2.755 / 27.722 / 200.622 | 0.022 / 0.177 / 1.781 / 12.889 |

Time per node on the wiki's machine (ns/node, sec/solv, GB*sec):

| solver | 1 thread | 12 threads |
|---|---|---|
| nxopt31 | 162.57 / 11.456 / 29.821 | 20.747 / 1.462 / 3.806 |
| nxopt22 | 220.52 / 6.115 / 47.752 | 28.762 / 0.798 / 6.228 |
| vcube112 | 106.64 / 7.459 / 19.417 | 17.904 / 1.252 / 3.260 |
| vcube208 | 147.54 / 4.105 / 32.057 | 22.424 / 0.624 / 4.872 |
| vcube308 | 210.14 / 2.044 / 46.566 | 30.253 / 0.294 / 6.704 |
| h48h7 | 76.95 / 5.624 / 21.335 | 14.883 / 1.088 / 4.126 |
| h48h8 | 80.10 / 3.625 / 27.500 | 15.462 / 0.700 / 5.309 |
| h48h9 | 98.18 / 2.024 / 30.706 | 16.353 / 0.337 / 5.114 |
| cubeopt30 | 70.74 / 4.781 / 12.445 | 11.392 / 0.770 / 2.004 |
| cube48opt7 | 41.02 / 2.104 / 8.188 | 6.927 / 0.355 / 1.383 |
| cube48opt8 | 46.45 / 1.178 / 9.164 | 6.788 / 0.172 / 1.339 |
| cube48opt9 | 48.26 / 0.622 / 9.681 | 7.445 / 0.096 / 1.494 |

### Node counts vs the wiki (same datasets, first 100 cubes; nodes do not depend on the machine)

| solver | 15-move here / wiki | 16-move | 17-move | inf-move |
|---|---|---|---|---|
| nxopt31 | 0.121 / 0.121 | 0.864 / 0.863 | 7.801 / 7.801 | 70.455 / 70.467 |
| nxopt22 | 0.085 / 0.085 | 0.376 / 0.376 | 3.560 / 3.588 | 27.736 / 27.729 |
| vcube112 | 0.107 / 0.107 | 0.886 / 0.886 | 7.837 / 7.837 | 69.947 / 69.947 |
| vcube208 | 0.075 / 0.075 | 0.368 / 0.368 | 3.609 / 3.609 | 27.823 / 27.823 |
| vcube308 | 0.022 / 0.022 | 0.151 / 0.151 | 1.231 / 1.231 | 9.729 / 9.729 |
| h48h7 | 0.705 / 0.715 | 1.585 / 1.579 | 10.207 / 9.934 | 72.922 / 73.078 |
| h48h8 | 0.669 / 0.680 | 1.320 / 1.383 | 7.350 / 7.232 | 45.039 / 45.261 |
| h48h9 | 0.580 / 0.587 | 0.945 / 0.981 | 3.738 / 3.706 | 20.570 / 20.617 |
| cubeopt30 | 0.071 / 0.071 | 0.714 / 0.714 | 7.039 / 7.039 | 67.588 / 67.588 |

## Distance-20 positions (cube20.org)

Positions from the known list of distance-20 positions at https://cube20.org/distance20s/ (list of 25 July 2025; random samples of 10 and 100, /data/agents/d20, converted to the harness format). Every position is a proven distance-20 position in the half-turn metric: a solver must exhaust depth 19 before it finds a 20-move solution, so these are the hardest cases. 1 GiB pages only; chosen builds. 48T: the 100-cube sample, one run per solver (chunked into holds of <= 15 min where needed: the per-cube times and, for nxopt/vcube, the harness formula sum / cubes / 48 do not depend on the chunking); 1T: the 10-cube sample for every solver (and the 100-cube sample for egg-solver where cheap). Semantics as above: latency rows (all threads on one cube) vs throughput rows (one cube per thread).

1T method (agreed with the coordinator): to fit ~10 h of 1T work, the 1T rows ran in parallel blocks, each inside one exclusive machine hold: up to 5 of these 1T rows at once, one per L3 domain (CPUs 8-15, 16-23, 24-31, 32-39, 40-47; CPUs 0-7 left to the harness), every row next to 4 busy neighbours (idle slots ran an unreported filler, egg-solver h8 on random states). Interference check, same cubes, alone on an idle machine vs inside a full block: egg-solver h8 -6.5% (#0 18.49 -> 17.23 s, #1 15.00 -> 13.90 s, #2 17.17 -> 16.27 s; nodes identical); nissy h48h7 +2.9% (#0 231.27 -> 234.27 s, #1 190.47 -> 199.86 s; nodes identical). (Threshold: > 2% would have meant serial 1T rows for that solver. Failed families: h48; their parallel timings are excluded and replaced by serial reruns.)

A timing cell is shown only when all 10 or 100 requested cubes are present. Probes and incomplete chunks are withheld from timings and speedup claims.

Length check: all 3904 solutions returned have length 20.

nissy h48h11 (its generator is slightly inadmissible, competitor-bugs.md #1): 20 moves on every cube here, so the inadmissible entries did not cost optimality on these positions.


### ~2.6-3.9 GB tables (distance 20)

| solver | semantics | table GB | 48T 1G (100 cubes) | 1T 1G (10 cubes) | 1T 1G (100 cubes) | nodes/cube | egg-solver faster by (same semantics) |
|---|---|---:|---:|---:|---:|---:|---|
| egg-solver h7 | latency | 3.89 | 0.897 | 32.56 (±12%) | 34.32 (±2%) | 3039.5M | - |
| cubeopt30 | latency | 2.60 | 9.678 | 300.19 (±11%) | - | 3154.0M | 48T 10.8x, 1T 9.2x (vs egg-solver h7) |
| h48h7 | latency | 3.79 | 6.041 | 201.63 (±11%) | - | 3528.2M | 48T 6.7x, 1T 6.2x (vs egg-solver h7) |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) | latency | 3.89 | 9.035 | 139.83 (±11%) | - | 2761.4M | 48T 10.1x, 1T 4.3x (vs egg-solver h7) |
| egg-solver h7 batch | throughput | 3.89 | 0.880 | 32.30 | - | 3016.5M | - |
| nxopt31 | throughput | 2.60 | 14.135 | 643.18 (±3%) | - | 3252.0M | 48T 16.1x, 1T 19.9x (vs egg-solver h7) |
| vcube112 | throughput | 2.60 | 9.285 | 424.68 (±3%) | - | 3251.9M | 48T 10.6x, 1T 13.1x (vs egg-solver h7) |
| cube48opt-rust Opt7 batch | throughput | 3.89 | 3.302 | 139.87 (±11%) | - | 2748.1M | 48T 3.8x, 1T 4.3x (vs egg-solver h7) |

### ~7.6-7.8 GB tables (distance 20)

| solver | semantics | table GB | 48T 1G (100 cubes) | 1T 1G (10 cubes) | 1T 1G (100 cubes) | nodes/cube | egg-solver faster by (same semantics) |
|---|---|---:|---:|---:|---:|---:|---|
| egg-solver h8 | latency | 7.78 | 0.412 | 15.65 (±12%) | 15.50 (±2%) | 1367.8M | - |
| h48h8 | latency | 7.59 | 3.208 | 104.75 (±12%) | - | 1822.1M | 48T 7.8x, 1T 6.7x (vs egg-solver h8) |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) | latency | 7.78 | 3.541 | 53.84 (±11%) | - | 944.8M | 48T 8.6x, 1T 3.4x (vs egg-solver h8) |
| egg-solver h8 batch | throughput | 7.78 | 0.400 | 14.60 | - | 1364.8M | - |
| nxopt22 | throughput | 7.81 | 7.979 | 358.73 (±4%) | - | 1630.4M | 48T 19.9x, 1T 24.6x (vs egg-solver h8) |
| vcube208 | throughput | 7.81 | 5.319 | 237.20 (±4%) | - | 1633.5M | 48T 13.3x, 1T 16.2x (vs egg-solver h8) |
| cube48opt-rust Opt8 batch | throughput | 7.78 | 1.277 | 53.65 (±11%) | - | 942.7M | 48T 3.2x, 1T 3.7x (vs egg-solver h8) |

### large tables (1 GiB pages only) (distance 20)

| solver | semantics | table GB | 48T 1G (100 cubes) | 1T 1G (10 cubes) | 1T 1G (100 cubes) | nodes/cube | egg-solver faster by (same semantics) |
|---|---|---:|---:|---:|---:|---:|---|
| egg-solver h9 | latency | 15.57 | 0.211 | 7.50 (±11%) | 7.71 (±2%) | 688.2M | - |
| egg-solver h10 | latency | 31.13 | 0.099 | 3.59 (±12%) | 3.68 (±2%) | 341.0M | - |
| egg-solver h11 | latency | 62.26 | 0.054 | 1.94 (±11%) | 2.03 (±2%) | 181.7M | - |
| cube48opt10 | latency | 31.13 | 0.644 | 15.51 (±11%) | - | 324.4M | 48T 6.5x, 1T 4.3x (vs egg-solver h10) |
| h48h9 | latency | 15.17 | 1.520 | 49.19 (±12%) | - | 858.7M | 48T 7.2x, 1T 6.6x (vs egg-solver h9) |
| h48h10 | latency | 30.34 | 0.769 | 23.99 (±12%) | - | 419.0M | 48T 7.8x, 1T 6.7x (vs egg-solver h10) |
| h48h11 | latency | 60.67 | 0.431 | 12.37 (±12%) | - | 215.4M | 48T 8.0x, 1T 6.4x (vs egg-solver h11) |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) | latency | 31.13 | 0.815 | 13.23 (±11%) | - | 229.7M | 48T 8.3x, 1T 3.7x (vs egg-solver h10) |
| egg-solver h9 batch | throughput | 15.57 | 0.211 | 7.51 | 7.46 | 683.2M | - |
| egg-solver h10 batch | throughput | 31.13 | 0.095 | 3.58 | 3.69 | 336.6M | - |
| egg-solver h11 batch | throughput | 62.26 | 0.052 | 1.94 | 2.04 | 180.4M | - |
| vcube308 | throughput | 22.78 | 1.598 | 68.99 (±5%) | - | 529.0M | 48T 7.6x, 1T 9.2x (vs egg-solver h9) |
| cube48opt-rust Opt10 batch | throughput | 31.13 | 0.313 | 13.24 (±11%) | - | 228.5M | 48T 3.3x, 1T 3.7x (vs egg-solver h10) |

## Build screening (shipped flags vs -O3 -march=native and vs PGO)

Frozen selection: nxopt pgo, vcube shipped, h48 shipped, cube48opt-rust Opt7 pfpgo, cube48opt-rust Opt8 native, cube48opt-rust Opt10 pfnative (an alternative build replaces the shipped one only when its runs are > 1% faster). nxopt's makefile already uses -O3 -march=native. cubeopt30/cube48opt10 are prebuilt closed-source binaries: they cannot be rebuilt or profile-tuned.

| run | shipped | alternative | vs shipped |
|---|---:|---:|---:|
| nxopt22 1g 1T (first chunk) | 5.7291 | pgo 5.7604 | +0.5% |
| nxopt22 1g 48T | 0.1415 | pgo 0.1370 | -3.2% |
| nxopt22 thp 1T (first chunk) | 6.2227 | pgo 6.0697 | -2.5% |
| nxopt22 thp 48T | 0.1435 | pgo 0.1426 | -0.6% |
| nxopt31 1g 1T (first chunk) | 14.4882 | pgo 14.3014 | -1.3% |
| nxopt31 1g 48T | 0.3076 | pgo 0.3030 | -1.5% |
| nxopt31 thp 1T (first chunk) | 14.0016 | pgo 14.0657 | +0.5% |
| nxopt31 thp 48T | 0.3066 | pgo 0.2973 | -3.1% |
| vcube112 1g 1T (first chunk) | 8.9932 | native 8.9864 | -0.1% |
| vcube112 thp 1T (first chunk) | 9.0164 | pgo 9.1078 | +1.0% |
| vcube112 thp 48T | 0.1922 | pgo 0.1947 | +1.3% |
| vcube208 thp 1T (first chunk) | 4.2427 | pgo 4.1976 | -1.1% |
| vcube208 thp 48T | 0.0954 | pgo 0.0969 | +1.5% |
| h48h7 1g 1T (first chunk) | 4.1646 | native 4.5074 | +8.2% |
| h48h7 thp 1T (first chunk) | 4.2181 | pgo 4.2409 | +0.5% |
| h48h7 thp 48T | 0.1631 | pgo 0.1616 | -0.9% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk) | 3.1800 | native 3.2048 | +0.8% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T | 0.2044 | native 0.2065 | +1.0% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk) | 3.1800 | pgo 3.2826 | +3.2% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T | 0.2044 | pgo 0.2173 | +6.3% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk) | 3.1800 | pfnative 2.6924 | -15.3% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T | 0.2044 | pfnative 0.1726 | -15.6% |
| cube48opt-rust Opt7 batch 1g 48T | 0.0688 | pfpgo 0.0590 | -14.2% |
| cube48opt-rust Opt7 batch thp 48T | 0.0690 | pfpgo 0.0595 | -13.8% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk) | 3.1800 | pfpgo 2.6933 | -15.3% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T | 0.2044 | pfpgo 0.1701 | -16.8% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) thp 1T (first chunk) | 3.1822 | pfpgo 2.7505 | -13.6% |
| cube48opt-rust Opt7 (latency: 18 threads, its cap) thp 48T | 0.2069 | pfpgo 0.1702 | -17.7% |
| cube48opt-rust Opt8 batch 1g 48T | 0.0247 | native 0.0242 | -2.1% |
| cube48opt-rust Opt8 batch thp 48T | 0.0250 | native 0.0243 | -2.6% |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 1T (first chunk) | 1.0859 | native 1.0484 | -3.4% |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 48T | 0.0781 | native 0.0774 | -0.9% |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) thp 1T (first chunk) | 1.0862 | native 1.0543 | -2.9% |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) thp 48T | 0.0788 | native 0.0773 | -1.8% |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 1T (first chunk) | 1.0859 | pgo 1.0520 | -3.1% |
| cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 48T | 0.0781 | pgo 0.0783 | +0.2% |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 1T (first chunk) | 0.3896 | native 0.3494 | -10.3% |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 48T | 0.0258 | native 0.0249 | -3.2% |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 1T (first chunk) | 0.3896 | pf 0.2992 | -23.2% |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 48T | 0.0258 | pf 0.0217 | -15.9% |
| cube48opt-rust Opt10 batch 1g 48T | 0.0083 | pfnative 0.0067 | -19.3% |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 1T (first chunk) | 0.3896 | pfnative 0.2925 | -24.9% |
| cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 48T | 0.0258 | pfnative 0.0209 | -18.8% |

## Solution lengths (optimality cross-check)

Competitors among themselves: 612 cubes, 0 with different lengths. egg-solver: 613 cubes.

| run | cubes vs egg-solver | equal | different |
|---|---:|---:|---:|
| c48rsbatchopt10_1T_distance20_random10_10_1g_pfnative | 10 | 10 | 0 |
| c48rsbatchopt10_1T_random_state_50_1g_pfnative | 50 | 50 | 0 |
| c48rsbatchopt10_48T_distance20_random100_10_1g_pfnative | 10 | 10 | 0 |
| c48rsbatchopt10_48T_distance20_random100_from10_90_1g_pfnative | 90 | 90 | 0 |
| c48rsbatchopt10_48T_random_state_200_1g | 200 | 200 | 0 |
| c48rsbatchopt10_48T_random_state_200_1g_pfnative | 200 | 200 | 0 |
| c48rsbatchopt10_48T_random_state_200_1g_pfnative_r2 | 200 | 200 | 0 |
| c48rsbatchopt10_48T_random_state_200_1g_pfnative_r3 | 200 | 200 | 0 |
| c48rsbatchopt10_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| c48rsbatchopt10_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| c48rsbatchopt7_1T_distance20_random10_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsbatchopt7_1T_distance20_random10_from2_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsbatchopt7_1T_distance20_random10_from4_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsbatchopt7_1T_distance20_random10_from6_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsbatchopt7_1T_distance20_random10_from8_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsbatchopt7_1T_random_state_50_1g_pfpgo | 50 | 50 | 0 |
| c48rsbatchopt7_1T_random_state_50_thp_pfpgo | 50 | 50 | 0 |
| c48rsbatchopt7_48T_distance20_random100_10_1g_pfpgo | 10 | 10 | 0 |
| c48rsbatchopt7_48T_distance20_random100_from10_90_1g_pfpgo | 90 | 90 | 0 |
| c48rsbatchopt7_48T_random_state_200_1g | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_1g_pfpgo | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_1g_pfpgo_r2 | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_1g_pfpgo_r3 | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_thp | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_thp_pfpgo | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_thp_pfpgo_r2 | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_thp_pfpgo_r3 | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| c48rsbatchopt7_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| c48rsbatchopt8_1T_distance20_random10_5_1g_native | 5 | 5 | 0 |
| c48rsbatchopt8_1T_distance20_random10_from5_5_1g_native | 5 | 5 | 0 |
| c48rsbatchopt8_1T_random_state_50_1g_native | 50 | 50 | 0 |
| c48rsbatchopt8_1T_random_state_50_thp_native | 50 | 50 | 0 |
| c48rsbatchopt8_48T_distance20_random100_10_1g_native | 10 | 10 | 0 |
| c48rsbatchopt8_48T_distance20_random100_from10_90_1g_native | 90 | 90 | 0 |
| c48rsbatchopt8_48T_random_state_200_1g | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_1g_native | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_1g_native_r2 | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_1g_native_r3 | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_thp | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_thp_native | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_thp_native_r2 | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_thp_native_r3 | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| c48rsbatchopt8_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| c48rsopt10_1T_distance20_random10_10_1g_pfnative | 10 | 10 | 0 |
| c48rsopt10_1T_random_move_15f_100_1g_pfnative | 100 | 100 | 0 |
| c48rsopt10_1T_random_move_16f_100_1g_pfnative | 100 | 100 | 0 |
| c48rsopt10_1T_random_move_17f_100_1g_pfnative | 100 | 100 | 0 |
| c48rsopt10_1T_random_state_50_1g | 50 | 50 | 0 |
| c48rsopt10_1T_random_state_50_1g_native | 50 | 50 | 0 |
| c48rsopt10_1T_random_state_50_1g_pf | 50 | 50 | 0 |
| c48rsopt10_1T_random_state_50_1g_pfnative | 50 | 50 | 0 |
| c48rsopt10_1T_random_state_from50_50_1g_pfnative | 50 | 50 | 0 |
| c48rsopt10_48T_distance20_random100_10_1g_pfnative | 10 | 10 | 0 |
| c48rsopt10_48T_distance20_random100_from10_90_1g_pfnative | 90 | 90 | 0 |
| c48rsopt10_48T_random_state_200_1g | 200 | 200 | 0 |
| c48rsopt10_48T_random_state_200_1g_native | 200 | 200 | 0 |
| c48rsopt10_48T_random_state_200_1g_pf | 200 | 200 | 0 |
| c48rsopt10_48T_random_state_200_1g_pfnative | 200 | 200 | 0 |
| c48rsopt10_48T_random_state_200_1g_pfnative_r2 | 200 | 200 | 0 |
| c48rsopt10_48T_random_state_200_1g_pfnative_r3 | 200 | 200 | 0 |
| c48rsopt10_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| c48rsopt10_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| c48rsopt7_1T_distance20_random10_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsopt7_1T_distance20_random10_from2_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsopt7_1T_distance20_random10_from4_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsopt7_1T_distance20_random10_from6_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsopt7_1T_distance20_random10_from8_2_1g_pfpgo | 2 | 2 | 0 |
| c48rsopt7_1T_random_move_15f_100_1g_pfpgo | 100 | 100 | 0 |
| c48rsopt7_1T_random_move_16f_100_1g_pfpgo | 100 | 100 | 0 |
| c48rsopt7_1T_random_move_17f_100_1g_pfpgo | 100 | 100 | 0 |
| c48rsopt7_1T_random_state_50_1g | 50 | 50 | 0 |
| c48rsopt7_1T_random_state_50_1g_native | 50 | 50 | 0 |
| c48rsopt7_1T_random_state_50_1g_pfnative | 50 | 50 | 0 |
| c48rsopt7_1T_random_state_50_1g_pfpgo | 50 | 50 | 0 |
| c48rsopt7_1T_random_state_50_1g_pgo | 50 | 50 | 0 |
| c48rsopt7_1T_random_state_50_thp | 50 | 50 | 0 |
| c48rsopt7_1T_random_state_50_thp_pfpgo | 50 | 50 | 0 |
| c48rsopt7_1T_random_state_from50_50_1g_pfpgo | 50 | 50 | 0 |
| c48rsopt7_48T_distance20_random100_10_1g_pfpgo | 10 | 10 | 0 |
| c48rsopt7_48T_distance20_random100_from10_30_1g_pfpgo | 30 | 30 | 0 |
| c48rsopt7_48T_distance20_random100_from40_30_1g_pfpgo | 30 | 30 | 0 |
| c48rsopt7_48T_distance20_random100_from70_30_1g_pfpgo | 30 | 30 | 0 |
| c48rsopt7_48T_random_state_200_1g | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_native | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_pfnative | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_pfpgo | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_pfpgo_r2 | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_pfpgo_r3 | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_pgo | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_thp | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_thp_pfpgo | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_thp_pfpgo_r2 | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_thp_pfpgo_r3 | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| c48rsopt7_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| c48rsopt8_1T_distance20_random10_5_1g_native | 5 | 5 | 0 |
| c48rsopt8_1T_distance20_random10_from5_5_1g_native | 5 | 5 | 0 |
| c48rsopt8_1T_random_move_15f_100_1g_native | 100 | 100 | 0 |
| c48rsopt8_1T_random_move_16f_100_1g_native | 100 | 100 | 0 |
| c48rsopt8_1T_random_move_17f_100_1g_native | 100 | 100 | 0 |
| c48rsopt8_1T_random_state_50_1g | 50 | 50 | 0 |
| c48rsopt8_1T_random_state_50_1g_native | 50 | 50 | 0 |
| c48rsopt8_1T_random_state_50_1g_pgo | 50 | 50 | 0 |
| c48rsopt8_1T_random_state_50_thp | 50 | 50 | 0 |
| c48rsopt8_1T_random_state_50_thp_native | 50 | 50 | 0 |
| c48rsopt8_1T_random_state_from50_50_1g_native | 50 | 50 | 0 |
| c48rsopt8_48T_distance20_random100_10_1g_native | 10 | 10 | 0 |
| c48rsopt8_48T_distance20_random100_from10_90_1g_native | 90 | 90 | 0 |
| c48rsopt8_48T_random_state_200_1g | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_1g_native | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_1g_native_r2 | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_1g_native_r3 | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_1g_pgo | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_thp | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_thp_native | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_thp_native_r2 | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_thp_native_r3 | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| c48rsopt8_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| cube48opt10_1T_distance20_random10_7_1g | 7 | 7 | 0 |
| cube48opt10_1T_distance20_random10_from7_3_1g | 3 | 3 | 0 |
| cube48opt10_1T_random_move_15f_100_1g | 100 | 100 | 0 |
| cube48opt10_1T_random_move_16f_100_1g | 100 | 100 | 0 |
| cube48opt10_1T_random_move_17f_100_1g | 100 | 100 | 0 |
| cube48opt10_1T_random_state_50_1g | 50 | 50 | 0 |
| cube48opt10_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| cube48opt10_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| cube48opt10_48T_distance20_random100_from10_90_1g | 90 | 90 | 0 |
| cube48opt10_48T_random_state_200_1g | 200 | 200 | 0 |
| cube48opt10_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| cube48opt10_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| cubeopt30_1T_distance20_random10_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from1_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from2_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from3_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from4_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from5_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from6_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from7_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from8_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_distance20_random10_from9_1_1g | 1 | 1 | 0 |
| cubeopt30_1T_random_move_15f_100_thp | 100 | 100 | 0 |
| cubeopt30_1T_random_move_16f_100_thp | 100 | 100 | 0 |
| cubeopt30_1T_random_move_17f_100_thp | 100 | 100 | 0 |
| cubeopt30_1T_random_state_25_1g | 25 | 25 | 0 |
| cubeopt30_1T_random_state_25_thp | 25 | 25 | 0 |
| cubeopt30_1T_random_state_from25_25_1g | 25 | 25 | 0 |
| cubeopt30_1T_random_state_from25_25_thp | 25 | 25 | 0 |
| cubeopt30_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| cubeopt30_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| cubeopt30_48T_distance20_random100_from10_45_1g | 45 | 45 | 0 |
| cubeopt30_48T_distance20_random100_from55_45_1g | 45 | 45 | 0 |
| cubeopt30_48T_random_state_200_1g | 200 | 200 | 0 |
| cubeopt30_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| cubeopt30_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| cubeopt30_48T_random_state_200_thp | 200 | 200 | 0 |
| cubeopt30_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| cubeopt30_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| cubeopt30_48T_random_state_200_thp_r7 | 200 | 200 | 0 |
| h48h10_1T_distance20_random10_10_1g | 10 | 10 | 0 |
| h48h10_1T_random_move_15f_100_1g | 100 | 100 | 0 |
| h48h10_1T_random_move_16f_100_1g | 100 | 100 | 0 |
| h48h10_1T_random_move_17f_100_1g | 100 | 100 | 0 |
| h48h10_1T_random_state_50_1g | 50 | 50 | 0 |
| h48h10_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| h48h10_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| h48h10_48T_distance20_random100_from10_90_1g | 90 | 90 | 0 |
| h48h10_48T_random_state_200_1g | 200 | 200 | 0 |
| h48h10_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| h48h10_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| h48h11_1T_distance20_random10_10_1g | 10 | 10 | 0 |
| h48h11_1T_random_move_15f_100_1g | 100 | 100 | 0 |
| h48h11_1T_random_move_16f_100_1g | 100 | 100 | 0 |
| h48h11_1T_random_move_17f_100_1g | 100 | 100 | 0 |
| h48h11_1T_random_state_50_1g | 50 | 50 | 0 |
| h48h11_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| h48h11_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| h48h11_48T_distance20_random100_from10_90_1g | 90 | 90 | 0 |
| h48h11_48T_random_state_200_1g | 200 | 200 | 0 |
| h48h11_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| h48h11_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| h48h7_1T_distance20_random10_2_1g | 2 | 2 | 0 |
| h48h7_1T_distance20_random10_from2_2_1g | 2 | 2 | 0 |
| h48h7_1T_distance20_random10_from4_2_1g | 2 | 2 | 0 |
| h48h7_1T_distance20_random10_from6_2_1g | 2 | 2 | 0 |
| h48h7_1T_distance20_random10_from8_2_1g | 2 | 2 | 0 |
| h48h7_1T_random_move_15f_100_thp | 100 | 100 | 0 |
| h48h7_1T_random_move_16f_100_thp | 100 | 100 | 0 |
| h48h7_1T_random_move_17f_100_thp | 100 | 100 | 0 |
| h48h7_1T_random_state_25_1g | 25 | 25 | 0 |
| h48h7_1T_random_state_25_1g_native | 25 | 25 | 0 |
| h48h7_1T_random_state_25_thp_pgo | 25 | 25 | 0 |
| h48h7_1T_random_state_25_thp_r2 | 25 | 25 | 0 |
| h48h7_1T_random_state_50_thp | 50 | 50 | 0 |
| h48h7_1T_random_state_from25_25_1g | 25 | 25 | 0 |
| h48h7_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| h48h7_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| h48h7_48T_distance20_random100_from10_90_1g | 90 | 90 | 0 |
| h48h7_48T_random_state_200_1g | 200 | 200 | 0 |
| h48h7_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| h48h7_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| h48h7_48T_random_state_200_thp | 200 | 200 | 0 |
| h48h7_48T_random_state_200_thp_pgo | 200 | 200 | 0 |
| h48h7_48T_random_state_200_thp_pgo_r2 | 200 | 200 | 0 |
| h48h7_48T_random_state_200_thp_pgo_r3 | 200 | 200 | 0 |
| h48h7_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| h48h7_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| h48h7_48T_random_state_200_thp_r7 | 200 | 200 | 0 |
| h48h8_1T_distance20_random10_3_1g | 3 | 3 | 0 |
| h48h8_1T_distance20_random10_from3_3_1g | 3 | 3 | 0 |
| h48h8_1T_distance20_random10_from6_3_1g | 3 | 3 | 0 |
| h48h8_1T_distance20_random10_from9_1_1g | 1 | 1 | 0 |
| h48h8_1T_random_move_15f_100_thp | 100 | 100 | 0 |
| h48h8_1T_random_move_16f_100_thp | 100 | 100 | 0 |
| h48h8_1T_random_move_17f_100_thp | 100 | 100 | 0 |
| h48h8_1T_random_state_50_1g | 50 | 50 | 0 |
| h48h8_1T_random_state_50_thp | 50 | 50 | 0 |
| h48h8_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| h48h8_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| h48h8_48T_distance20_random100_from10_90_1g | 90 | 90 | 0 |
| h48h8_48T_random_state_200_1g | 200 | 200 | 0 |
| h48h8_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| h48h8_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| h48h8_48T_random_state_200_thp | 200 | 200 | 0 |
| h48h8_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| h48h8_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| h48h9_1T_distance20_random10_7_1g | 7 | 7 | 0 |
| h48h9_1T_distance20_random10_from7_3_1g | 3 | 3 | 0 |
| h48h9_1T_random_move_15f_100_1g | 100 | 100 | 0 |
| h48h9_1T_random_move_16f_100_1g | 100 | 100 | 0 |
| h48h9_1T_random_move_17f_100_1g | 100 | 100 | 0 |
| h48h9_1T_random_state_50_1g | 50 | 50 | 0 |
| h48h9_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| h48h9_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| h48h9_48T_distance20_random100_from10_90_1g | 90 | 90 | 0 |
| h48h9_48T_random_state_200_1g | 200 | 200 | 0 |
| h48h9_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| h48h9_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| nxopt22_1T_distance20_random10_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from1_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from2_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from3_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from4_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from5_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from6_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from7_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from8_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_distance20_random10_from9_1_1g_pgo | 1 | 1 | 0 |
| nxopt22_1T_random_move_15f_100_thp_pgo | 100 | 100 | 0 |
| nxopt22_1T_random_move_16f_100_thp_pgo | 100 | 100 | 0 |
| nxopt22_1T_random_move_17f_100_thp_pgo | 100 | 100 | 0 |
| nxopt22_1T_random_state_25_thp_pgo | 25 | 25 | 0 |
| nxopt22_1T_random_state_25_thp_r2 | 25 | 25 | 0 |
| nxopt22_1T_random_state_50_1g | 50 | 50 | 0 |
| nxopt22_1T_random_state_50_1g_pgo | 50 | 50 | 0 |
| nxopt22_1T_random_state_50_thp | 50 | 50 | 0 |
| nxopt22_1T_random_state_from25_25_thp_pgo | 25 | 25 | 0 |
| nxopt22_1T_random_state_from50_50_1g_pgo | 50 | 50 | 0 |
| nxopt22_48T_distance20_random100_10_1g_pgo | 10 | 10 | 0 |
| nxopt22_48T_distance20_random100_from10_45_1g_pgo | 45 | 45 | 0 |
| nxopt22_48T_distance20_random100_from55_45_1g_pgo | 45 | 45 | 0 |
| nxopt22_48T_random_state_200_1g | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_1g_pgo | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_1g_pgo_r2 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_1g_pgo_r3 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_thp | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_thp_pgo | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_thp_pgo_r2 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_thp_pgo_r3 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| nxopt22_48T_random_state_200_thp_r7 | 200 | 200 | 0 |
| nxopt31_1T_distance20_random10_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from1_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from2_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from3_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from4_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from5_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from6_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from7_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from8_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_distance20_random10_from9_1_1g_pgo | 1 | 1 | 0 |
| nxopt31_1T_random_move_15f_100_thp_pgo | 100 | 100 | 0 |
| nxopt31_1T_random_move_16f_100_thp_pgo | 100 | 100 | 0 |
| nxopt31_1T_random_move_17f_100_thp_pgo | 100 | 100 | 0 |
| nxopt31_1T_random_state_25_1g | 25 | 25 | 0 |
| nxopt31_1T_random_state_25_1g_pgo | 25 | 25 | 0 |
| nxopt31_1T_random_state_25_thp | 25 | 25 | 0 |
| nxopt31_1T_random_state_25_thp_pgo | 25 | 25 | 0 |
| nxopt31_1T_random_state_25_thp_r2 | 25 | 25 | 0 |
| nxopt31_1T_random_state_from25_25_1g | 25 | 25 | 0 |
| nxopt31_1T_random_state_from25_25_1g_pgo | 25 | 25 | 0 |
| nxopt31_1T_random_state_from25_25_thp | 25 | 25 | 0 |
| nxopt31_1T_random_state_from25_25_thp_pgo | 25 | 25 | 0 |
| nxopt31_1T_random_state_from50_50_1g_pgo | 50 | 50 | 0 |
| nxopt31_48T_distance20_random100_10_1g_pgo | 10 | 10 | 0 |
| nxopt31_48T_distance20_random100_from10_20_1g_pgo | 20 | 20 | 0 |
| nxopt31_48T_distance20_random100_from30_20_1g_pgo | 20 | 20 | 0 |
| nxopt31_48T_distance20_random100_from50_20_1g_pgo | 20 | 20 | 0 |
| nxopt31_48T_distance20_random100_from70_20_1g_pgo | 20 | 20 | 0 |
| nxopt31_48T_distance20_random100_from90_10_1g_pgo | 10 | 10 | 0 |
| nxopt31_48T_random_state_200_1g | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_1g_pgo | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_1g_pgo_r2 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_1g_pgo_r3 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_thp | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_thp_pgo | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_thp_pgo_r2 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_thp_pgo_r3 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| nxopt31_48T_random_state_200_thp_r7 | 200 | 200 | 0 |
| vcube112_1T_distance20_random10_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from1_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from2_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from3_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from4_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from5_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from6_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from7_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from8_1_1g | 1 | 1 | 0 |
| vcube112_1T_distance20_random10_from9_1_1g | 1 | 1 | 0 |
| vcube112_1T_random_move_15f_100_thp | 100 | 100 | 0 |
| vcube112_1T_random_move_16f_100_thp | 100 | 100 | 0 |
| vcube112_1T_random_move_17f_100_thp | 100 | 100 | 0 |
| vcube112_1T_random_state_25_1g | 25 | 25 | 0 |
| vcube112_1T_random_state_25_1g_native | 25 | 25 | 0 |
| vcube112_1T_random_state_25_thp | 25 | 25 | 0 |
| vcube112_1T_random_state_25_thp_pgo | 25 | 25 | 0 |
| vcube112_1T_random_state_25_thp_r2 | 25 | 25 | 0 |
| vcube112_1T_random_state_from25_25_1g | 25 | 25 | 0 |
| vcube112_1T_random_state_from25_25_thp | 25 | 25 | 0 |
| vcube112_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| vcube112_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| vcube112_48T_distance20_random100_from10_45_1g | 45 | 45 | 0 |
| vcube112_48T_distance20_random100_from55_45_1g | 45 | 45 | 0 |
| vcube112_48T_random_state_200_1g | 200 | 200 | 0 |
| vcube112_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| vcube112_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| vcube112_48T_random_state_200_thp | 200 | 200 | 0 |
| vcube112_48T_random_state_200_thp_pgo | 200 | 200 | 0 |
| vcube112_48T_random_state_200_thp_pgo_r2 | 200 | 200 | 0 |
| vcube112_48T_random_state_200_thp_pgo_r3 | 200 | 200 | 0 |
| vcube112_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| vcube112_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| vcube112_48T_random_state_200_thp_r7 | 200 | 200 | 0 |
| vcube208_1T_distance20_random10_2_1g | 2 | 2 | 0 |
| vcube208_1T_distance20_random10_from2_2_1g | 2 | 2 | 0 |
| vcube208_1T_distance20_random10_from4_2_1g | 2 | 2 | 0 |
| vcube208_1T_distance20_random10_from6_2_1g | 2 | 2 | 0 |
| vcube208_1T_distance20_random10_from8_2_1g | 2 | 2 | 0 |
| vcube208_1T_random_move_15f_100_thp | 100 | 100 | 0 |
| vcube208_1T_random_move_16f_100_thp | 100 | 100 | 0 |
| vcube208_1T_random_move_17f_100_thp | 100 | 100 | 0 |
| vcube208_1T_random_state_25_thp_pgo | 25 | 25 | 0 |
| vcube208_1T_random_state_25_thp_r2 | 25 | 25 | 0 |
| vcube208_1T_random_state_50_1g | 50 | 50 | 0 |
| vcube208_1T_random_state_50_thp | 50 | 50 | 0 |
| vcube208_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| vcube208_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| vcube208_48T_distance20_random100_from10_45_1g | 45 | 45 | 0 |
| vcube208_48T_distance20_random100_from55_45_1g | 45 | 45 | 0 |
| vcube208_48T_random_state_200_1g | 200 | 200 | 0 |
| vcube208_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| vcube208_48T_random_state_200_1g_r3 | 200 | 200 | 0 |
| vcube208_48T_random_state_200_thp | 200 | 200 | 0 |
| vcube208_48T_random_state_200_thp_pgo | 200 | 200 | 0 |
| vcube208_48T_random_state_200_thp_pgo_r2 | 200 | 200 | 0 |
| vcube208_48T_random_state_200_thp_pgo_r3 | 200 | 200 | 0 |
| vcube208_48T_random_state_200_thp_r2 | 200 | 200 | 0 |
| vcube208_48T_random_state_200_thp_r3 | 200 | 200 | 0 |
| vcube308_1T_distance20_random10_6_1g | 6 | 6 | 0 |
| vcube308_1T_distance20_random10_from6_1_1g | 1 | 1 | 0 |
| vcube308_1T_distance20_random10_from7_1_1g | 1 | 1 | 0 |
| vcube308_1T_distance20_random10_from8_1_1g | 1 | 1 | 0 |
| vcube308_1T_distance20_random10_from9_1_1g | 1 | 1 | 0 |
| vcube308_1T_random_move_15f_100_1g | 100 | 100 | 0 |
| vcube308_1T_random_move_16f_100_1g | 100 | 100 | 0 |
| vcube308_1T_random_move_17f_100_1g | 100 | 100 | 0 |
| vcube308_1T_random_state_50_1g | 50 | 50 | 0 |
| vcube308_1T_random_state_from50_50_1g | 50 | 50 | 0 |
| vcube308_48T_distance20_random100_10_1g | 10 | 10 | 0 |
| vcube308_48T_distance20_random100_from10_45_1g | 45 | 45 | 0 |
| vcube308_48T_distance20_random100_from55_45_1g | 45 | 45 | 0 |
| vcube308_48T_random_state_200_1g | 200 | 200 | 0 |
| vcube308_48T_random_state_200_1g_r2 | 200 | 200 | 0 |
| vcube308_48T_random_state_200_1g_r3 | 200 | 200 | 0 |

## Rejected runs (page placement, outside load, or interference validation)

- nxopt22_48T_random_state_200_thp.try1: THP 51%, 1G 0% of the table; rerun
- nxopt22_48T_random_state_200_thp.try2: THP 75%, 1G 0% of the table; rerun
- nxopt22_48T_random_state_200_thp_pgo.try1: THP 68%, 1G 0% of the table; rerun
- nxopt22_48T_random_state_200_thp_pgo.try2: THP 81%, 1G 0% of the table; rerun
- vcube208_1T_random_state_50_thp.try1: THP 87%, 1G 0% of the table; rerun
- vcube208_48T_random_state_200_thp.try1: THP 77%, 1G 0% of the table; rerun
- vcube208_48T_random_state_200_thp.try2: THP 76%, 1G 0% of the table; rerun
- vcube208_48T_random_state_200_thp_pgo.try1: THP 83%, 1G 0% of the table; rerun
- vcube208_48T_random_state_200_thp_pgo_r3.try1: THP 0%, 1G 109% of the table; rerun
- vcube208_48T_random_state_200_thp_r3.try1: THP 0%, 1G 109% of the table; rerun


## Harness correction (2026-11-05)

Every 48T row measured before 06:45 on 2026-11-05 used a shell page sampler that forked pgrep/awk/cat
every 0.1 s (later 2 s) at the solver's own priority. With all 48 cores busy, each fork preempted a
worker thread. It made egg-solver's 1 GiB latency rows 11-27% slower at 48T: those runs map their table
instantly, so the whole solve fell in the dense sampling window, while the THP rows spent that window
copying the table (found by hp1; sys time 17-31 s against 6 s). This produced the "1 GiB slower than
THP" result in the provisional rows; measured cleanly, 1 GiB is 3-6% faster. Those runs are kept in
old-sampler/ and are not reported. All 48T rows here were re-measured with a fork-free sampler
(tools/src/pagesampler.py, SCHED_IDLE), and the per-run watcher shell also drops to SCHED_IDLE while
the run is going. 1T rows were kept (47 idle cores: the forks rarely landed on the solver); spot
check, same cubes, old sampler (repeat 1) vs new (repeat 2): cubeoptrsh8 1G 1T 3 cubes 16.890 -> 15.799 s (-6.5%); h48h7 1G 1T 2 cubes 211.203 -> 217.062 s (+2.8%); nxopt31 THP 1T 25 cubes 13.916 -> 14.087 s (+1.2%); vcube112 THP 1T 25 cubes 9.072 -> 8.961 s (-1.2%).

## Fairness

Equalized:
1. **Page types.** Every solver runs on both 1 GiB hugetlb pages and 2 MiB THP for the small and
   medium tables (2.6-7.8 GB); the large tables (egg-solver h9..h11, vcube308, cube48opt10, h48h9..h11)
   are measured on 1 GiB pages only (user decision: a 58 GiB THP copy does not fit next to the 1 GiB
   pool). The measured page placement of each row is in the tables ("pages measured", % of the table).
   - egg-solver: 1G = its table file on hugetlbfs; THP = `CUBEOPT_LOAD=copy` from an exact copy on /data.
   - vcube: 1G = its own MAP_HUGETLB 1 GiB mapping (free pool pages given); THP = hugeshim madvise on its
     fallback mmap (shipped vcube gets 4K pages when the pool is full).
   - nxopt: 1G = hugeshim arena (its 9,930 row mallocs carved from one 1 GiB-page arena); THP = glibc
     malloc THP + 32 MiB mmap threshold (shipped: 4K).
   - nissy H48: 1G = hugeshim arena (the table bytearray); THP = glibc malloc THP (shipped: 4K).
   - cubeopt30 / cube48opt10: THP = as shipped (they madvise themselves); 1G = hugeshim arena.
2. **Builds.** vcube and nissy also built with -O3 -march=native; the faster build at the screening
   is used (see "Build screening"). Result: -O3 -march=native was not faster for either, so vcube and nissy use their shipped flags (nxopt22 1g 1T (first chunk): 5.7291 shipped vs pgo 5.7604 native, +0.5%; nxopt22 1g 48T: 0.1415 shipped vs pgo 0.1370 native, -3.2%; nxopt22 thp 1T (first chunk): 6.2227 shipped vs pgo 6.0697 native, -2.5%; nxopt22 thp 48T: 0.1435 shipped vs pgo 0.1426 native, -0.6%; nxopt31 1g 1T (first chunk): 14.4882 shipped vs pgo 14.3014 native, -1.3%; nxopt31 1g 48T: 0.3076 shipped vs pgo 0.3030 native, -1.5%; nxopt31 thp 1T (first chunk): 14.0016 shipped vs pgo 14.0657 native, +0.5%; nxopt31 thp 48T: 0.3066 shipped vs pgo 0.2973 native, -3.1%; vcube112 1g 1T (first chunk): 8.9932 shipped vs native 8.9864 native, -0.1%; vcube112 thp 1T (first chunk): 9.0164 shipped vs pgo 9.1078 native, +1.0%; vcube112 thp 48T: 0.1922 shipped vs pgo 0.1947 native, +1.3%; vcube208 thp 1T (first chunk): 4.2427 shipped vs pgo 4.1976 native, -1.1%; vcube208 thp 48T: 0.0954 shipped vs pgo 0.0969 native, +1.5%; h48h7 1g 1T (first chunk): 4.1646 shipped vs native 4.5074 native, +8.2%; h48h7 thp 1T (first chunk): 4.2181 shipped vs pgo 4.2409 native, +0.5%; h48h7 thp 48T: 0.1631 shipped vs pgo 0.1616 native, -0.9%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk): 3.1800 shipped vs native 3.2048 native, +0.8%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T: 0.2044 shipped vs native 0.2065 native, +1.0%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk): 3.1800 shipped vs pgo 3.2826 native, +3.2%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T: 0.2044 shipped vs pgo 0.2173 native, +6.3%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk): 3.1800 shipped vs pfnative 2.6924 native, -15.3%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T: 0.2044 shipped vs pfnative 0.1726 native, -15.6%; cube48opt-rust Opt7 batch 1g 48T: 0.0688 shipped vs pfpgo 0.0590 native, -14.2%; cube48opt-rust Opt7 batch thp 48T: 0.0690 shipped vs pfpgo 0.0595 native, -13.8%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 1T (first chunk): 3.1800 shipped vs pfpgo 2.6933 native, -15.3%; cube48opt-rust Opt7 (latency: 18 threads, its cap) 1g 48T: 0.2044 shipped vs pfpgo 0.1701 native, -16.8%; cube48opt-rust Opt7 (latency: 18 threads, its cap) thp 1T (first chunk): 3.1822 shipped vs pfpgo 2.7505 native, -13.6%; cube48opt-rust Opt7 (latency: 18 threads, its cap) thp 48T: 0.2069 shipped vs pfpgo 0.1702 native, -17.7%; cube48opt-rust Opt8 batch 1g 48T: 0.0247 shipped vs native 0.0242 native, -2.1%; cube48opt-rust Opt8 batch thp 48T: 0.0250 shipped vs native 0.0243 native, -2.6%; cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 1T (first chunk): 1.0859 shipped vs native 1.0484 native, -3.4%; cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 48T: 0.0781 shipped vs native 0.0774 native, -0.9%; cube48opt-rust Opt8 (latency: 18 threads, its cap) thp 1T (first chunk): 1.0862 shipped vs native 1.0543 native, -2.9%; cube48opt-rust Opt8 (latency: 18 threads, its cap) thp 48T: 0.0788 shipped vs native 0.0773 native, -1.8%; cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 1T (first chunk): 1.0859 shipped vs pgo 1.0520 native, -3.1%; cube48opt-rust Opt8 (latency: 18 threads, its cap) 1g 48T: 0.0781 shipped vs pgo 0.0783 native, +0.2%; cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 1T (first chunk): 0.3896 shipped vs native 0.3494 native, -10.3%; cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 48T: 0.0258 shipped vs native 0.0249 native, -3.2%; cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 1T (first chunk): 0.3896 shipped vs pf 0.2992 native, -23.2%; cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 48T: 0.0258 shipped vs pf 0.0217 native, -15.9%; cube48opt-rust Opt10 batch 1g 48T: 0.0083 shipped vs pfnative 0.0067 native, -19.3%; cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 1T (first chunk): 0.3896 shipped vs pfnative 0.2925 native, -24.9%; cube48opt-rust Opt10 (latency: 18 threads, its cap) 1g 48T: 0.0258 shipped vs pfnative 0.0209 native, -18.8%). nxopt already uses -march=native. egg-solver: -C target-cpu=native.
   **PGO:** since egg-solver may ship a profile-guided build, nxopt, vcube and nissy got one too, with
   their shipped flags otherwise (pgo/pgo-build.sh): gcc `-fprofile-generate/-fprofile-use
   -fprofile-correction` for nxopt and nissy's C core, clang `-fprofile-instr-generate` +
   `llvm-profdata merge` + `-fprofile-instr-use` for vcube, all with `-fprofile-update=atomic`.
   Training (pgo/pgo-train.sh, pgo-train2.sh), disjoint from the evaluated cubes (random_state.txt
   lines 301-400), the same tables as the evaluation, multithreaded: `nxopt31|22-pgogen -t 8` on 24
   cubes each, `vc-optimal-pgogen -c 112|208 -w 2` on 6 cubes each (profiles merged), nissy
   `cmd.py h48h7 8` on 10 cubes and `cmd.py h48h7 1` on 2. (Fewer threads than the evaluation: the
   instrumented binaries share their counters between threads, and at 48 threads with atomic
   counters nxopt31 did not finish one cube in 10 min; vcube was ~15x slower even at 2 threads.)
   At profile-generation time h48h8's table was not available, so nissy's profile uses h48h7 only. The PGO build is used where it
   is faster (Build screening). **cubeopt30 and cube48opt10 are prebuilt closed-source binaries: they cannot
   be profile-tuned or rebuilt with other flags**; their rows are marked so in the headline. If
   egg-solver's rows use a PGO build, they are shown with and without it.
   nissy THREADS=48 (its build.sh default would follow the build's CPU affinity).
3. **Timing.** Every sec/solv excludes table load and generation (each solver times the solve itself;
   egg-solver's latency rows use the mean of its per-cube times, as h48 and cubeopt do).
4. **Semantics.** Latency rows (all threads on one cube) and throughput rows (many cubes in flight) are
   in separate tables, and speedups are only computed within one.
5. **Noise.** The same cubes for every solver (random_state: the first 200 at 48T, the first 50 at 1T,
   1T runs over 15 min split into two chunks of 25); solvers interleaved in the run order; >= 3 repeats
   per 48T cell (median and range); one exclusive machine hold per run behind a quiet gate; the outside
   load during each run is in "Runs".
6. **Table size.** Size groups; each row shows its size relative to the egg-solver table it is compared
   with, and GB·s.

Remaining asymmetries:
- cubeopt30 and cube48opt10 are prebuilt binaries (GCC 9.4, "use sse, bmi2"); they cannot be rebuilt.
- nissy H48 runs through its Python module (as the harness does); the time is taken around each
  `nissy.solve` call, so the Python call overhead (microseconds) is included. nissy's C shell
  (`./run`) loads the table per invocation, so it is not usable for a batch.
- Different table families: egg-solver and nissy use H48 tables (egg-solver's hN keep different EO bits than
  nissy's), cube48opt its own H48 variant, nxopt and vcube their coset tables; same-size groups are
  approximate (see "size").
- Thread semantics: nxopt and vcube cannot put several threads on one cube, so they have no latency
  rows at 48T; their 48T throughput uses the harness formula (sum of per-cube times / 48), which
  leaves out the idle tail, while egg-solver's batch rows are wall / cube including its tail.
- nissy's node counts vary run to run (uninitialized task rank, competitor-bugs.md #7); its h11 table is
  slightly inadmissible (#1).
- cube48opt-rust (github.com/2017YANR02/cuberoot.me solver/experiments/cube48opt-rust at 1effb34, a Rust
  port of cube48opt1-9 plus its own Opt10; audited and set up by xcr, /data/agents/xcr): its latency mode
  is capped at 18 search threads (`MAX_SEARCH_THREADS`), so its 48T latency rows run on 18 threads (its
  cap); its throughput rows use 48 (one cube per thread). It allocates its table with malloc (4 KiB pages
  as shipped): THP = hugeshim thp mode, 1G = hugeshim arena, as nxopt and nissy. Opt7/Opt8 tables were
  made by its own `init` (SHA-256 = upstream's published hashes). **Opt10 reads cube48opt10's raw file
  through a loader-only shim** (xcr): with cube48opt10's unread block minima (15) written as 10 the bytes
  equal its official Opt10 payload (SHA-256 2ce3d57d..., re-verified on every load); 959 of 962 functions
  (every search function) are byte-identical to the official build. Builds screened: shipped, native,
  PGO, and its optional x86 table prefetch (pf / pfnative, off in its official sets).


## cube48opt10's table vs egg-solver h10

cube48opt10's h48prun31h1.dat (made by its first run in the empty folder /data2/competitors/cube48opt10) against egg-solver's /data2/tables/h10 (data part; ours has a 4 KiB trailer after it). n1's static analysis (/data/agents/n1/cube48opt10-table.md) found the same size, base (10) and 2-bit code meaning, with one expected benign difference: the unread minimum nibble of groups whose entries are all above the base (15 in cube48opt, 11/12 in ours). `gen compare` reads both as egg-solver h10 tables; the byte compare counts benign vs real differences and the histograms of both files (equal histograms with many real differences = the same values in a different index order).

**Result: different tables.** Apart from the benign unread nibbles there are also real differences, so cube48opt10's file is neither our h10 nor a reordering of it.
- **Bytes:** 29,791,179,308 of 31,130,910,720 bytes (95.7%) differ for real. 172,108,390 differences (8.8% of the 1,945,681,920 groups) are the benign kind: an unread minimum nibble, 15 in theirs and 11 (157,973,301) or 12 (14,135,089) in ours.
- **First real difference:** offset 0, the minimum byte of group 0. Ours is 06; theirs is 00, the solved state at index 0. Group 0 then holds mostly codes 1/2 in ours and codes 0/1 in theirs.
- **Entries** (`gen compare`, both read as h10 base 10): 69,126,743,843 of 120,389,068,800 entry codes differ (57.4%). 913,531,947 group minima ≤ base differ, plus 856,376,982 unread ones.
- **Not a permutation:**
  - The code histograms are close but not equal. Ours: 0: 1,133,240,200, 1: 11,196,287,163, 2: 64,164,706,157, 3: 44,138,045,520. Theirs: 1,134,918,810 / 11,222,950,329 / 64,306,616,699 / 43,967,793,202.
  - Ours has 170.3M more entries at code 3 (≥ 13) and 0.15-0.24% fewer at codes 0-2, so ours is slightly higher (stronger) on average. Theirs matches cube48opt10's own generation printout exactly: 11: 11,222,950,329, 12: 64,306,616,699.
  - This fits a different index mapping (coset numbering or which EO bit is dropped from h11), not a different file format: size, base, the 2-bit codes and the group-minimum slot agree, as n1 found.
- **What it means for the comparison:** node counts and timings are unaffected, since each solver reads its own table. For cube48opt-rust, the file cannot be swapped with ours.

```
size 31130910720 (expect 31130910720)
/data2/tables/h10 (h10 base 10) vs /data2/competitors/cube48opt10/h48prun31h1.dat (h10 base 10): Diff { entries: 120389068800, codes: 69126743843, group_minima: 913531947, unread_group_minima: 856376982 }
DIFFERENT
```

```
real diff at offset 0 (group 0, byte 0): ours 06 theirs 00
real diff at offset 4 (group 0, byte 4): ours 54 theirs 00
real diff at offset 5 (group 0, byte 5): ours aa theirs 50
real diff at offset 6 (group 0, byte 6): ours aa theirs 05
real diff at offset 7 (group 0, byte 7): ours aa theirs 51
real diff at offset 8 (group 0, byte 8): ours 95 theirs 40
real diff at offset 9 (group 0, byte 9): ours 96 theirs 14
real diff at offset 10 (group 0, byte 10): ours 66 theirs 90
real diff at offset 11 (group 0, byte 11): ours 19 theirs 25
real diff at offset 13 (group 0, byte 13): ours 54 theirs 00
real diff at offset 14 (group 0, byte 14): ours a6 theirs 50
real diff at offset 15 (group 0, byte 15): ours 9a theirs 05
real diff at offset 16 (group 1, byte 0): ours aa theirs 08
real diff at offset 17 (group 1, byte 1): ours 5a theirs 05
real diff at offset 18 (group 1, byte 2): ours 69 theirs 04
real diff at offset 19 (group 1, byte 3): ours 69 theirs 41
real diff at offset 20 (group 1, byte 4): ours 55 theirs 45
real diff at offset 21 (group 1, byte 5): ours 16 theirs 01
real diff at offset 22 (group 1, byte 6): ours a5 theirs 55
real diff at offset 23 (group 1, byte 7): ours a6 theirs 50
benign nibble diffs 172108390 (ours 11: 157973301, 12: 14135089, other: 0) of 1945681920 groups (8.8%), real diffs 29791179308
ours: codes 0:1133240200 1:11196287163 2:64164706157 3:44138045520; minimum nibbles 0:1 1:1 2:4 3:34 4:316 5:3452 6:40457 7:485503 8:5885693 9:67286659 10:559724594 11:1214503690 12:97750966 15:550
theirs: codes 0:1134918810 1:11222950329 2:64306616699 3:43967793202; minimum nibbles 0:1 1:1 2:4 3:34 4:329 5:3587 6:42455 7:509720 8:6216718 9:71922069 10:599070554 15:1267916448
code histograms DIFFER
exit 1
```


## Competitor implementation findings

<a id="bugs-and-issues-found-in-other-solvers-for-the-final-report-the-user-will-report-them-upstream"></a>
<a id="competitor-bug-findings"></a>

See [the canonical competitor findings](COMPETITOR_FINDINGS.md) for the source locations, reproductions and corrections.

## phack hardware-counter analysis

The separate [hardware-counter study](../performance/HARDWARE_COUNTERS.md) retains the complete Xeon measurements, interpretation and raw-data links. Its results are not a direct speed comparison with this VM.

## Runs

| run | status | wall s | outside load (cores) | gate wait s | pages measured |
|---|---|---:|---:|---:|---|
| c48rsbatchopt10_1T_distance20_random10_10_1g_pfnative | 0 | 373 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_1T_random_state_50_1g_pfnative | 0 | 257 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_distance20_random100_10_1g_pfnative | 0 | 258 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_distance20_random100_from10_90_1g_pfnative | 0 | 274 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_random_state_200_1g | 0 | 245 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_random_state_200_1g_pfnative | 0 | 244 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_random_state_200_1g_pfnative_r2 | 0 | 244 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_random_state_200_1g_pfnative_r3 | 0 | 244 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_random_state_200_1g_r2 | 0 | 245 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt10_48T_random_state_200_1g_r3 | 0 | 245 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_1T_distance20_random10_2_1g_pfpgo | 0 | 323 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_1T_distance20_random10_from2_2_1g_pfpgo | 0 | 332 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_1T_distance20_random10_from4_2_1g_pfpgo | 0 | 323 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_1T_distance20_random10_from6_2_1g_pfpgo | 0 | 247 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_1T_distance20_random10_from8_2_1g_pfpgo | 0 | 302 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_1T_random_state_50_1g_pfpgo | 0 | 161 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_1T_random_state_50_thp_pfpgo | 0 | 162 | 0.0 | 0 | THP 100% |
| c48rsbatchopt7_48T_distance20_random100_10_1g_pfpgo | 0 | 200 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_48T_distance20_random100_from10_90_1g_pfpgo | 0 | 377 | 0.4 | 0 | 1G 100% |
| c48rsbatchopt7_48T_random_state_200_1g | 0 | 53 | 0.1 | 0 | 1G 100% |
| c48rsbatchopt7_48T_random_state_200_1g_pfpgo | 0 | 49 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_48T_random_state_200_1g_pfpgo_r2 | 0 | 49 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_48T_random_state_200_1g_pfpgo_r3 | 0 | 49 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_48T_random_state_200_1g_r2 | 0 | 53 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_48T_random_state_200_1g_r3 | 0 | 53 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt7_48T_random_state_200_thp | 0 | 53 | 0.0 | 0 | THP 100% |
| c48rsbatchopt7_48T_random_state_200_thp_pfpgo | 0 | 50 | 0.0 | 0 | THP 100% |
| c48rsbatchopt7_48T_random_state_200_thp_pfpgo_r2 | 0 | 50 | 0.4 | 0 | THP 98% |
| c48rsbatchopt7_48T_random_state_200_thp_pfpgo_r3 | 0 | 49 | 0.1 | 16 | THP 100% |
| c48rsbatchopt7_48T_random_state_200_thp_r2 | 0 | 53 | 0.0 | 0 | THP 100% |
| c48rsbatchopt7_48T_random_state_200_thp_r3 | 0 | 53 | 0.0 | 0 | THP 100% |
| c48rsbatchopt8_1T_distance20_random10_5_1g_native | 0 | 342 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_1T_distance20_random10_from5_5_1g_native | 0 | 297 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_1T_random_state_50_1g_native | 0 | 103 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_1T_random_state_50_thp_native | 0 | 105 | 0.0 | 0 | THP 100% |
| c48rsbatchopt8_48T_distance20_random100_10_1g_native | 0 | 120 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_distance20_random100_from10_90_1g_native | 0 | 187 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_random_state_200_1g | 0 | 63 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_random_state_200_1g_native | 0 | 62 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_random_state_200_1g_native_r2 | 0 | 62 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_random_state_200_1g_native_r3 | 0 | 62 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_random_state_200_1g_r2 | 0 | 63 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_random_state_200_1g_r3 | 0 | 63 | 0.0 | 0 | 1G 100% |
| c48rsbatchopt8_48T_random_state_200_thp | 0 | 63 | 0.0 | 0 | THP 100% |
| c48rsbatchopt8_48T_random_state_200_thp_native | 0 | 64 | 0.0 | 0 | THP 100% |
| c48rsbatchopt8_48T_random_state_200_thp_native_r2 | 0 | 63 | 0.0 | 0 | THP 100% |
| c48rsbatchopt8_48T_random_state_200_thp_native_r3 | 0 | 64 | 0.0 | 0 | THP 100% |
| c48rsbatchopt8_48T_random_state_200_thp_r2 | 0 | 63 | 0.0 | 0 | THP 100% |
| c48rsbatchopt8_48T_random_state_200_thp_r3 | 0 | 63 | 0.0 | 0 | THP 100% |
| c48rsopt10_1T_distance20_random10_10_1g_pfnative | 0 | 373 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_move_15f_100_1g_pfnative | 0 | 241 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_move_16f_100_1g_pfnative | 0 | 242 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_move_17f_100_1g_pfnative | 0 | 245 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_state_50_1g | 0 | 261 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_state_50_1g_native | 0 | 259 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_state_50_1g_pf | 0 | 256 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_state_50_1g_pfnative | 0 | 256 | 0.0 | 0 | 1G 100% |
| c48rsopt10_1T_random_state_from50_50_1g_pfnative | 0 | 257 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_distance20_random100_10_1g_pfnative | 0 | 249 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_distance20_random100_from10_90_1g_pfnative | 0 | 314 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g | 0 | 247 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g_native | 0 | 246 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g_pf | 0 | 246 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g_pfnative | 0 | 245 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g_pfnative_r2 | 0 | 245 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g_pfnative_r3 | 0 | 246 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g_r2 | 0 | 246 | 0.0 | 0 | 1G 100% |
| c48rsopt10_48T_random_state_200_1g_r3 | 0 | 246 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_distance20_random10_2_1g_pfpgo | 0 | 321 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_distance20_random10_from2_2_1g_pfpgo | 0 | 332 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_distance20_random10_from4_2_1g_pfpgo | 0 | 325 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_distance20_random10_from6_2_1g_pfpgo | 0 | 246 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_distance20_random10_from8_2_1g_pfpgo | 0 | 303 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_random_move_15f_100_1g_pfpgo | 0 | 26 | 0.0 | 12 | 1G 100% |
| c48rsopt7_1T_random_move_16f_100_1g_pfpgo | 0 | 28 | 0.5 | 0 | 1G 100% |
| c48rsopt7_1T_random_move_17f_100_1g_pfpgo | 0 | 59 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_random_state_50_1g | 0 | 185 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_random_state_50_1g_native | 0 | 186 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_random_state_50_1g_pfnative | 0 | 160 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_random_state_50_1g_pfpgo | 0 | 160 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_random_state_50_1g_pgo | 0 | 190 | 0.0 | 0 | 1G 100% |
| c48rsopt7_1T_random_state_50_thp | 0 | 185 | 0.0 | 0 | THP 100% |
| c48rsopt7_1T_random_state_50_thp_pfpgo | 0 | 164 | 0.0 | 0 | THP 100% |
| c48rsopt7_1T_random_state_from50_50_1g_pfpgo | 0 | 167 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_distance20_random100_10_1g_pfpgo | 0 | 117 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_distance20_random100_from10_30_1g_pfpgo | 0 | 296 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_distance20_random100_from40_30_1g_pfpgo | 0 | 295 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_distance20_random100_from70_30_1g_pfpgo | 0 | 299 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g | 0 | 68 | 0.1 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_native | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_pfnative | 0 | 60 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_pfpgo | 0 | 60 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_pfpgo_r2 | 0 | 60 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_pfpgo_r3 | 0 | 60 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_pgo | 0 | 69 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_r2 | 0 | 66 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_1g_r3 | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt7_48T_random_state_200_thp | 0 | 67 | 0.1 | 0 | THP 100% |
| c48rsopt7_48T_random_state_200_thp_pfpgo | 0 | 60 | 0.0 | 0 | THP 100% |
| c48rsopt7_48T_random_state_200_thp_pfpgo_r2 | 0 | 60 | 0.0 | 0 | THP 100% |
| c48rsopt7_48T_random_state_200_thp_pfpgo_r3 | 0 | 60 | 0.0 | 0 | THP 100% |
| c48rsopt7_48T_random_state_200_thp_r2 | 0 | 68 | 0.0 | 0 | THP 100% |
| c48rsopt7_48T_random_state_200_thp_r3 | 0 | 67 | 0.0 | 0 | THP 100% |
| c48rsopt8_1T_distance20_random10_5_1g_native | 0 | 343 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_distance20_random10_from5_5_1g_native | 0 | 298 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_random_move_15f_100_1g_native | 0 | 51 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_random_move_16f_100_1g_native | 0 | 53 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_random_move_17f_100_1g_native | 0 | 66 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_random_state_50_1g | 0 | 106 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_random_state_50_1g_native | 0 | 104 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_random_state_50_1g_pgo | 0 | 104 | 0.0 | 0 | 1G 100% |
| c48rsopt8_1T_random_state_50_thp | 0 | 106 | 0.0 | 0 | THP 100% |
| c48rsopt8_1T_random_state_50_thp_native | 0 | 105 | 0.0 | 0 | THP 100% |
| c48rsopt8_1T_random_state_from50_50_1g_native | 0 | 109 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_distance20_random100_10_1g_native | 0 | 87 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_distance20_random100_from10_90_1g_native | 0 | 370 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_1g | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_1g_native | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_1g_native_r2 | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_1g_native_r3 | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_1g_pgo | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_1g_r2 | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_1g_r3 | 0 | 67 | 0.0 | 0 | 1G 100% |
| c48rsopt8_48T_random_state_200_thp | 0 | 68 | 0.0 | 0 | THP 100% |
| c48rsopt8_48T_random_state_200_thp_native | 0 | 70 | 0.1 | 0 | THP 85% |
| c48rsopt8_48T_random_state_200_thp_native_r2 | 0 | 68 | 0.0 | 0 | THP 100% |
| c48rsopt8_48T_random_state_200_thp_native_r3 | 0 | 68 | 0.0 | 0 | THP 100% |
| c48rsopt8_48T_random_state_200_thp_r2 | 0 | 67 | 0.0 | 0 | THP 100% |
| c48rsopt8_48T_random_state_200_thp_r3 | 0 | 67 | 0.0 | 0 | THP 100% |
| cube48opt10_1T_distance20_random10_7_1g | 0 | 431 | 3.7 | 0 | 1G 100% |
| cube48opt10_1T_distance20_random10_from7_3_1g | 0 | 299 | 4.0 | 0 | 1G 100% |
| cube48opt10_1T_random_move_15f_100_1g | 0 | 253 | 0.0 | 0 | 1G 100% |
| cube48opt10_1T_random_move_16f_100_1g | 0 | 254 | 0.0 | 0 | 1G 100% |
| cube48opt10_1T_random_move_17f_100_1g | 0 | 258 | 0.0 | 0 | 1G 100% |
| cube48opt10_1T_random_state_50_1g | 0 | 272 | 0.1 | 0 | 1G 100% |
| cube48opt10_1T_random_state_from50_50_1g | 0 | 271 | 0.0 | 0 | 1G 100% |
| cube48opt10_48T_distance20_random100_10_1g | 0 | 259 | 0.0 | 0 | 1G 100% |
| cube48opt10_48T_distance20_random100_from10_90_1g | 0 | 311 | 0.0 | 0 | 1G 100% |
| cube48opt10_48T_random_state_200_1g | 0 | 262 | 0.0 | 0 | 1G 100% |
| cube48opt10_48T_random_state_200_1g_r2 | 0 | 266 | 0.0 | 0 | 1G 100% |
| cube48opt10_48T_random_state_200_1g_r3 | 0 | 262 | 0.1 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_1_1g | 0 | 336 | 3.1 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from1_1_1g | 0 | 296 | 3.7 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from2_1_1g | 0 | 335 | 3.8 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from3_1_1g | 0 | 350 | 3.8 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from4_1_1g | 0 | 360 | 3.8 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from5_1_1g | 0 | 343 | 3.9 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from6_1_1g | 0 | 183 | 2.9 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from7_1_1g | 0 | 375 | 3.4 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from8_1_1g | 0 | 368 | 3.4 | 0 | 1G 100% |
| cubeopt30_1T_distance20_random10_from9_1_1g | 0 | 304 | 3.4 | 0 | 1G 100% |
| cubeopt30_1T_random_move_15f_100_thp | 0 | 19 | 0.0 | 0 | THP 100% |
| cubeopt30_1T_random_move_16f_100_thp | 0 | 25 | 0.0 | 0 | THP 100% |
| cubeopt30_1T_random_move_17f_100_thp | 0 | 88 | 0.0 | 0 | THP 100% |
| cubeopt30_1T_random_state_25_1g | 0 | 173 | 0.2 | 0 | 1G 100% |
| cubeopt30_1T_random_state_25_thp | 0 | 183 | 0.1 | 0 | THP 100% |
| cubeopt30_1T_random_state_from25_25_1g | 0 | 169 | 0.2 | 0 | 1G 100% |
| cubeopt30_1T_random_state_from25_25_thp | 0 | 174 | 0.1 | 0 | THP 100% |
| cubeopt30_1T_random_state_from50_50_1g | 0 | 378 | 0.0 | 0 | 1G 100% |
| cubeopt30_48T_distance20_random100_10_1g | 0 | 116 | 0.0 | 0 | 1G 100% |
| cubeopt30_48T_distance20_random100_from10_45_1g | 0 | 451 | 0.0 | 0 | 1G 100% |
| cubeopt30_48T_distance20_random100_from55_45_1g | 0 | 455 | 0.0 | 0 | 1G 100% |
| cubeopt30_48T_random_state_200_1g | 0 | 67 | -0.0 | 0 | 1G 100% |
| cubeopt30_48T_random_state_200_1g_r2 | 0 | 66 | -0.0 | 0 | 1G 100% |
| cubeopt30_48T_random_state_200_1g_r3 | 0 | 68 | -0.1 | 0 | 1G 100% |
| cubeopt30_48T_random_state_200_thp | 0 | 68 | -0.0 | 0 | THP 100% |
| cubeopt30_48T_random_state_200_thp_r2 | 0 | 67 | -0.0 | 0 | THP 100% |
| cubeopt30_48T_random_state_200_thp_r3 | 0 | 67 | -0.0 | 0 | THP 100% |
| cubeopt30_48T_random_state_200_thp_r7 | 0 | 68 | -0.0 | 0 | THP 100% |
| cubeoptrsbatchh10_1T_distance20_random100_20_1g | 0 | 75 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh10_1T_distance20_random100_from20_20_1g | 0 | 72 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh10_1T_distance20_random100_from40_20_1g | 0 | 74 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh10_1T_distance20_random100_from60_20_1g | 0 | 73 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh10_1T_distance20_random100_from80_20_1g | 0 | 75 | 3.9 | 0 | 1G 100% |
| cubeoptrsbatchh10_1T_distance20_random10_10_1g | 0 | 36 | 4.1 | 0 | 1G 100% |
| cubeoptrsbatchh10_1T_random_state_50_1g | 0 | 3 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh10_48T_distance20_random100_100_1g | 0 | 10 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh10_48T_random_state_200_1g | 0 | 0 | 0.2 | 0 | 1G 100% |
| cubeoptrsbatchh10_48T_random_state_200_1g_r2 | 0 | 0 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh10_48T_random_state_200_1g_r3 | 0 | 0 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh11_1T_distance20_random100_20_1g | 0 | 41 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh11_1T_distance20_random100_from20_20_1g | 0 | 40 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh11_1T_distance20_random100_from40_20_1g | 0 | 42 | 3.9 | 0 | 1G 100% |
| cubeoptrsbatchh11_1T_distance20_random100_from60_20_1g | 0 | 40 | 3.9 | 0 | 1G 100% |
| cubeoptrsbatchh11_1T_distance20_random100_from80_20_1g | 0 | 41 | 3.8 | 0 | 1G 100% |
| cubeoptrsbatchh11_1T_distance20_random10_10_1g | 0 | 19 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh11_1T_random_state_50_1g | 0 | 2 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh11_48T_distance20_random100_100_1g | 0 | 5 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh11_48T_random_state_200_1g | 0 | 0 | 0.2 | 0 | 1G 100% |
| cubeoptrsbatchh11_48T_random_state_200_1g_r2 | 0 | 0 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh11_48T_random_state_200_1g_r3 | 0 | 0 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh7_1T_distance20_random10_4_1g | 0 | 140 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh7_1T_distance20_random10_from4_4_1g | 0 | 120 | 3.9 | 0 | 1G 100% |
| cubeoptrsbatchh7_1T_distance20_random10_from8_2_1g | 0 | 63 | 3.9 | 0 | 1G 100% |
| cubeoptrsbatchh7_1T_random_state_50_1g | 0 | 30 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh7_48T_distance20_random100_10_1g | 0 | 9 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh7_48T_distance20_random100_from10_90_1g | 0 | 79 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh7_48T_random_state_200_1g | 0 | 3 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh7_48T_random_state_200_1g_r2 | 0 | 3 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh7_48T_random_state_200_1g_r3 | 0 | 3 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh8_1T_distance20_random10_10_1g | 0 | 146 | 0.8 | 0 | 1G 100% |
| cubeoptrsbatchh8_1T_random_state_50_1g | 0 | 14 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh8_48T_distance20_random100_10_1g | 0 | 4 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh8_48T_distance20_random100_from10_90_1g | 0 | 36 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh8_48T_random_state_200_1g | 0 | 2 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh8_48T_random_state_200_1g_r2 | 0 | 2 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh8_48T_random_state_200_1g_r3 | 0 | 2 | 0.2 | 0 | 1G 100% |
| cubeoptrsbatchh9_1T_distance20_random100_20_1g | 0 | 150 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh9_1T_distance20_random100_from20_20_1g | 0 | 145 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh9_1T_distance20_random100_from40_20_1g | 0 | 149 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh9_1T_distance20_random100_from60_20_1g | 0 | 153 | 3.9 | 0 | 1G 100% |
| cubeoptrsbatchh9_1T_distance20_random100_from80_20_1g | 0 | 149 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh9_1T_distance20_random10_10_1g | 0 | 75 | 4.0 | 0 | 1G 100% |
| cubeoptrsbatchh9_1T_random_state_50_1g | 0 | 7 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh9_48T_distance20_random100_100_1g | 0 | 21 | 0.0 | 0 | 1G 100% |
| cubeoptrsbatchh9_48T_random_state_200_1g | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh9_48T_random_state_200_1g_r2 | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchh9_48T_random_state_200_1g_r3 | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsbatchthph7_1T_random_state_50_thp | 0 | 58 | 0.0 | 0 | THP 100% |
| cubeoptrsbatchthph7_48T_random_state_200_thp | 0 | 30 | 0.1 | 0 | THP 100% |
| cubeoptrsbatchthph7_48T_random_state_200_thp_r2 | 0 | 31 | 0.0 | 0 | THP 100% |
| cubeoptrsbatchthph7_48T_random_state_200_thp_r3 | 0 | 30 | 0.0 | 0 | THP 100% |
| cubeoptrsbatchthph8_1T_random_state_50_thp | 0 | 72 | 0.0 | 0 | THP 100% |
| cubeoptrsbatchthph8_48T_random_state_200_thp | 0 | 59 | 0.0 | 0 | THP 100% |
| cubeoptrsbatchthph8_48T_random_state_200_thp_r2 | 0 | 59 | 0.0 | 0 | THP 100% |
| cubeoptrsbatchthph8_48T_random_state_200_thp_r3 | 0 | 59 | 0.0 | 0 | THP 100% |
| cubeoptrsh10_1T_distance20_random100_50_1g | 0 | 184 | 4.0 | 0 | 1G 100% |
| cubeoptrsh10_1T_distance20_random100_from50_50_1g | 0 | 184 | 4.0 | 0 | 1G 100% |
| cubeoptrsh10_1T_distance20_random10_10_1g | 0 | 36 | 4.1 | 0 | 1G 100% |
| cubeoptrsh10_1T_random_move_15f_100_1g | 0 | 0 | 0.2 | 0 | THP 0% |
| cubeoptrsh10_1T_random_move_16f_100_1g | 0 | 0 | 0.2 | 0 | 1G 100% |
| cubeoptrsh10_1T_random_move_17f_100_1g | 0 | 1 | 0.0 | 0 | 1G 100% |
| cubeoptrsh10_1T_random_state_50_1g | 0 | 3 | 0.0 | 0 | 1G 100% |
| cubeoptrsh10_1T_random_state_from50_50_1g | 0 | 4 | 0.0 | 0 | 1G 100% |
| cubeoptrsh10_48T_distance20_random100_100_1g | 0 | 10 | 0.0 | 0 | 1G 100% |
| cubeoptrsh10_48T_random_state_200_1g | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsh10_48T_random_state_200_1g_r2 | 0 | 1 | 0.0 | 0 | 1G 100% |
| cubeoptrsh10_48T_random_state_200_1g_r3 | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsh11_1T_distance20_random100_50_1g | 0 | 101 | 3.9 | 0 | 1G 100% |
| cubeoptrsh11_1T_distance20_random100_from50_50_1g | 0 | 102 | 3.9 | 0 | 1G 100% |
| cubeoptrsh11_1T_distance20_random10_10_1g | 0 | 19 | 4.0 | 0 | 1G 100% |
| cubeoptrsh11_1T_random_move_15f_100_1g | 0 | 0 | 0.4 | 0 | THP 0% |
| cubeoptrsh11_1T_random_move_16f_100_1g | 0 | 0 | 0.2 | 0 | 1G 100% |
| cubeoptrsh11_1T_random_move_17f_100_1g | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsh11_1T_random_state_50_1g | 0 | 2 | 0.0 | 0 | 1G 100% |
| cubeoptrsh11_1T_random_state_from50_50_1g | 0 | 2 | 0.0 | 0 | 1G 100% |
| cubeoptrsh11_48T_distance20_random100_100_1g | 0 | 5 | 0.1 | 0 | 1G 100% |
| cubeoptrsh11_48T_random_state_200_1g | 0 | 0 | 0.2 | 0 | 1G 100% |
| cubeoptrsh11_48T_random_state_200_1g_r2 | 0 | 0 | 0.1 | 0 | 1G 100% |
| cubeoptrsh11_48T_random_state_200_1g_r3 | 0 | 0 | 0.1 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_15_1g | 0 | 530 | 3.8 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from15_15_1g | 0 | 503 | 3.8 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from30_15_1g | 0 | 531 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from45_10_1g | 0 | 328 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from55_10_1g | 0 | 345 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from65_10_1g | 0 | 347 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from75_10_1g | 0 | 340 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from85_5_1g | 0 | 167 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random100_from90_10_1g | 0 | 343 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random10_2_1g | 0 | 70 | 0.1 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random10_4_1g | 0 | 143 | 3.9 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random10_from4_4_1g | 0 | 119 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_distance20_random10_from8_2_1g | 0 | 64 | 4.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_random_move_15f_100_1g | 0 | 0 | 0.2 | 0 | 1G 100% |
| cubeoptrsh7_1T_random_move_16f_100_1g | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsh7_1T_random_move_17f_100_1g | 0 | 8 | 0.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_random_state_50_1g | 0 | 30 | 0.0 | 0 | 1G 100% |
| cubeoptrsh7_1T_random_state_from50_50_1g | 0 | 29 | 0.0 | 0 | 1G 100% |
| cubeoptrsh7_48T_distance20_random100_10_1g | 0 | 10 | 0.0 | 0 | 1G 100% |
| cubeoptrsh7_48T_distance20_random100_from10_90_1g | 0 | 80 | 0.0 | 0 | 1G 100% |
| cubeoptrsh7_48T_distance20_random10_10_1g | 0 | 9 | 0.1 | 0 | 1G 100% |
| cubeoptrsh7_48T_random_state_200_1g | 0 | 3 | 0.1 | 0 | 1G 100% |
| cubeoptrsh7_48T_random_state_200_1g_r2 | 0 | 4 | 0.1 | 0 | 1G 100% |
| cubeoptrsh7_48T_random_state_200_1g_r3 | 0 | 4 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_random100_30_1g | 0 | 463 | 4.0 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_random100_from30_30_1g | 0 | 466 | 3.8 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_random100_from60_30_1g | 0 | 460 | 3.8 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_random100_from90_10_1g | 0 | 161 | 3.7 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_random10_10_1g | 0 | 156 | 2.7 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_random10_2_1g | 0 | 32 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_val3_3_1g | 0 | 51 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_1T_distance20_val3_3_1g_r2 | 0 | 47 | 2.5 | 0 | 1G 100% |
| cubeoptrsh8_1T_random_move_15f_100_1g | 0 | 0 | 0.2 | 0 | THP 0% |
| cubeoptrsh8_1T_random_move_16f_100_1g | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsh8_1T_random_move_17f_100_1g | 0 | 4 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_1T_random_state_50_1g | 0 | 13 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_1T_random_state_from50_50_1g | 0 | 14 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_48T_distance20_random100_10_1g | 0 | 4 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_48T_distance20_random100_from10_90_1g | 0 | 37 | 0.0 | 0 | 1G 100% |
| cubeoptrsh8_48T_distance20_random10_10_1g | 0 | 4 | 1.3 | 0 | 1G 100% |
| cubeoptrsh8_48T_random_state_200_1g | 0 | 2 | 0.1 | 0 | 1G 100% |
| cubeoptrsh8_48T_random_state_200_1g_r2 | 0 | 2 | 0.1 | 0 | 1G 100% |
| cubeoptrsh8_48T_random_state_200_1g_r3 | 0 | 2 | 0.1 | 0 | 1G 100% |
| cubeoptrsh9_1T_distance20_random100_50_1g | 0 | 387 | 3.9 | 0 | 1G 100% |
| cubeoptrsh9_1T_distance20_random100_from50_50_1g | 0 | 384 | 4.0 | 0 | 1G 100% |
| cubeoptrsh9_1T_distance20_random10_10_1g | 0 | 75 | 4.0 | 0 | 1G 100% |
| cubeoptrsh9_1T_random_move_15f_100_1g | 0 | 0 | 0.3 | 0 | THP 0% |
| cubeoptrsh9_1T_random_move_16f_100_1g | 0 | 0 | 0.1 | 0 | 1G 100% |
| cubeoptrsh9_1T_random_move_17f_100_1g | 0 | 2 | 0.0 | 0 | 1G 100% |
| cubeoptrsh9_1T_random_state_50_1g | 0 | 7 | 0.0 | 0 | 1G 100% |
| cubeoptrsh9_1T_random_state_from50_50_1g | 0 | 7 | 0.0 | 0 | 1G 100% |
| cubeoptrsh9_48T_distance20_random100_100_1g | 0 | 21 | 0.0 | 0 | 1G 100% |
| cubeoptrsh9_48T_random_state_200_1g | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsh9_48T_random_state_200_1g_r2 | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsh9_48T_random_state_200_1g_r3 | 0 | 1 | 0.1 | 0 | 1G 100% |
| cubeoptrsstreamh11_1T_random_state_0_1g | 0 | 50 | 0.0 | 0 | 1G 100% |
| cubeoptrsstreamh11_48T_random_state_0_1g | 0 | 50 | 0.1 | 0 | 1G 100% |
| cubeoptrsstreamh11_48T_random_state_0_1g_r2 | 0 | 50 | 0.1 | 0 | 1G 100% |
| cubeoptrsstreamh11_48T_random_state_0_1g_r3 | 0 | 50 | 0.1 | 0 | 1G 100% |
| cubeoptrsthph7_1T_random_state_50_thp | 0 | 58 | 0.0 | 0 | THP 100% |
| cubeoptrsthph7_48T_random_state_200_thp | 0 | 31 | 0.1 | 0 | THP 100% |
| cubeoptrsthph7_48T_random_state_200_thp_r2 | 0 | 31 | 0.0 | 0 | THP 100% |
| cubeoptrsthph7_48T_random_state_200_thp_r3 | 0 | 28 | 0.0 | 0 | THP 100% |
| cubeoptrsthph8_1T_random_state_50_thp | 0 | 71 | 0.0 | 0 | THP 100% |
| cubeoptrsthph8_48T_random_state_200_thp | 0 | 57 | 0.1 | 0 | THP 100% |
| cubeoptrsthph8_48T_random_state_200_thp_r2 | 0 | 59 | 0.0 | 0 | THP 100% |
| cubeoptrsthph8_48T_random_state_200_thp_r3 | 0 | 59 | 0.0 | 0 | THP 100% |
| h48h10_1T_distance20_random10_10_1g | 0 | 474 | 0.0 | 0 | 1G 100% |
| h48h10_1T_random_move_15f_100_1g | 0 | 237 | 0.0 | 0 | 1G 100% |
| h48h10_1T_random_move_16f_100_1g | 0 | 238 | 0.0 | 0 | 1G 100% |
| h48h10_1T_random_move_17f_100_1g | 0 | 249 | 0.0 | 0 | 1G 100% |
| h48h10_1T_random_state_50_1g | 0 | 270 | 0.0 | 0 | 1G 100% |
| h48h10_1T_random_state_from50_50_1g | 0 | 272 | 0.0 | 0 | 1G 100% |
| h48h10_48T_distance20_random100_10_1g | 0 | 242 | 0.0 | 0 | 1G 100% |
| h48h10_48T_distance20_random100_from10_90_1g | 0 | 304 | 0.0 | 0 | 1G 100% |
| h48h10_48T_random_state_200_1g | 0 | 247 | 0.1 | 0 | 1G 100% |
| h48h10_48T_random_state_200_1g_r2 | 0 | 247 | 0.0 | 0 | 1G 100% |
| h48h10_48T_random_state_200_1g_r3 | 0 | 246 | 0.0 | 0 | 1G 100% |
| h48h11_1T_distance20_random10_10_1g | 0 | 596 | 0.0 | 0 | 1G 100% |
| h48h11_1T_random_move_15f_100_1g | 0 | 475 | 0.0 | 0 | 1G 100% |
| h48h11_1T_random_move_16f_100_1g | 0 | 476 | 0.0 | 0 | 1G 100% |
| h48h11_1T_random_move_17f_100_1g | 0 | 481 | 0.0 | 0 | 1G 100% |
| h48h11_1T_random_state_50_1g | 0 | 494 | 0.0 | 0 | 1G 100% |
| h48h11_1T_random_state_from50_50_1g | 0 | 493 | 0.0 | 0 | 1G 100% |
| h48h11_48T_distance20_random100_10_1g | 0 | 477 | 0.0 | 0 | 1G 100% |
| h48h11_48T_distance20_random100_from10_90_1g | 0 | 511 | 0.0 | 0 | 1G 100% |
| h48h11_48T_random_state_200_1g | 0 | 483 | 0.0 | 0 | 1G 100% |
| h48h11_48T_random_state_200_1g_r2 | 0 | 484 | 0.5 | 0 | 1G 100% |
| h48h11_48T_random_state_200_1g_r3 | 0 | 483 | 0.0 | 0 | 1G 100% |
| h48h7_1T_distance20_random10_2_1g | 0 | 449 | 0.0 | 0 | 1G 100% |
| h48h7_1T_distance20_random10_from2_2_1g | 0 | 486 | 0.0 | 0 | 1G 100% |
| h48h7_1T_distance20_random10_from4_2_1g | 0 | 452 | 0.0 | 0 | 1G 100% |
| h48h7_1T_distance20_random10_from6_2_1g | 0 | 344 | 0.0 | 0 | 1G 100% |
| h48h7_1T_distance20_random10_from8_2_1g | 0 | 418 | 0.0 | 0 | 1G 100% |
| h48h7_1T_distance20_val2_2_1g | 0 | 448 | 0.0 | 0 | 1G 100% |
| h48h7_1T_distance20_val2_2_1g_r2 | 0 | 460 | 3.0 | 0 | 1G 100% |
| h48h7_1T_random_move_15f_100_thp | 0 | 30 | 0.1 | 0 | THP 100% |
| h48h7_1T_random_move_16f_100_thp | 0 | 35 | 0.0 | 0 | THP 100% |
| h48h7_1T_random_move_17f_100_thp | 0 | 88 | 0.0 | 0 | THP 100% |
| h48h7_1T_random_state_25_1g | 0 | 123 | 0.2 | 0 | 1G 100% |
| h48h7_1T_random_state_25_1g_native | 0 | 139 | 0.2 | 0 | 1G 100% |
| h48h7_1T_random_state_25_thp_pgo | 0 | 132 | 0.0 | 0 | THP 100% |
| h48h7_1T_random_state_25_thp_r2 | 0 | 132 | 0.0 | 0 | THP 100% |
| h48h7_1T_random_state_50_thp | 0 | 257 | 0.1 | 0 | THP 100% |
| h48h7_1T_random_state_from25_25_1g | 0 | 131 | 0.2 | 0 | 1G 100% |
| h48h7_1T_random_state_from50_50_1g | 0 | 257 | 0.0 | 0 | 1G 100% |
| h48h7_48T_distance20_random100_10_1g | 0 | 87 | 0.1 | 0 | 1G 100% |
| h48h7_48T_distance20_random100_from10_90_1g | 0 | 570 | 0.0 | 0 | 1G 100% |
| h48h7_48T_random_state_200_1g | 0 | 58 | 0.0 | 0 | 1G 100% |
| h48h7_48T_random_state_200_1g_r2 | 0 | 60 | 0.1 | 0 | 1G 100% |
| h48h7_48T_random_state_200_1g_r3 | 0 | 60 | 0.0 | 0 | 1G 100% |
| h48h7_48T_random_state_200_thp | 0 | 60 | 0.1 | 0 | THP 100% |
| h48h7_48T_random_state_200_thp_pgo | 0 | 60 | 0.1 | 0 | THP 100% |
| h48h7_48T_random_state_200_thp_pgo_r2 | 0 | 59 | 0.0 | 0 | THP 100% |
| h48h7_48T_random_state_200_thp_pgo_r3 | 0 | 59 | 0.0 | 0 | THP 100% |
| h48h7_48T_random_state_200_thp_r2 | 0 | 59 | 0.0 | 0 | THP 100% |
| h48h7_48T_random_state_200_thp_r3 | 0 | 59 | 0.0 | 0 | THP 100% |
| h48h7_48T_random_state_200_thp_r7 | 0 | 59 | 0.0 | 0 | THP 100% |
| h48h8_1T_distance20_random10_3_1g | 0 | 395 | 0.0 | 0 | 1G 100% |
| h48h8_1T_distance20_random10_from3_3_1g | 0 | 397 | 0.0 | 0 | 1G 100% |
| h48h8_1T_distance20_random10_from6_3_1g | 0 | 334 | 0.0 | 0 | 1G 100% |
| h48h8_1T_distance20_random10_from9_1_1g | 0 | 146 | 0.0 | 0 | 1G 100% |
| h48h8_1T_random_move_15f_100_thp | 0 | 60 | 0.1 | 0 | THP 100% |
| h48h8_1T_random_move_16f_100_thp | 0 | 65 | 0.0 | 0 | THP 100% |
| h48h8_1T_random_move_17f_100_thp | 0 | 103 | 0.1 | 0 | THP 100% |
| h48h8_1T_random_state_50_1g | 0 | 202 | 1.2 | 0 | 1G 100% |
| h48h8_1T_random_state_50_thp | 0 | 204 | 0.0 | 0 | THP 100% |
| h48h8_1T_random_state_from50_50_1g | 0 | 199 | 0.0 | 0 | 1G 100% |
| h48h8_48T_distance20_random100_10_1g | 0 | 89 | 0.0 | 0 | 1G 100% |
| h48h8_48T_distance20_random100_from10_90_1g | 0 | 344 | 0.0 | 0 | 1G 100% |
| h48h8_48T_random_state_200_1g | 0 | 81 | 0.0 | 0 | 1G 100% |
| h48h8_48T_random_state_200_1g_r2 | 0 | 80 | 0.0 | 0 | 1G 100% |
| h48h8_48T_random_state_200_1g_r3 | 0 | 83 | 0.0 | 0 | 1G 100% |
| h48h8_48T_random_state_200_thp | 0 | 81 | 0.0 | 0 | THP 100% |
| h48h8_48T_random_state_200_thp_r2 | 0 | 81 | 0.0 | 0 | THP 100% |
| h48h8_48T_random_state_200_thp_r3 | 0 | 81 | 0.0 | 0 | THP 100% |
| h48h9_1T_distance20_random10_7_1g | 0 | 458 | 0.0 | 0 | 1G 100% |
| h48h9_1T_distance20_random10_from7_3_1g | 0 | 263 | 0.0 | 0 | 1G 100% |
| h48h9_1T_random_move_15f_100_1g | 0 | 118 | 0.0 | 0 | 1G 100% |
| h48h9_1T_random_move_16f_100_1g | 0 | 119 | 0.0 | 0 | 1G 100% |
| h48h9_1T_random_move_17f_100_1g | 0 | 138 | 0.0 | 0 | 1G 100% |
| h48h9_1T_random_state_50_1g | 0 | 180 | 0.1 | 0 | 1G 100% |
| h48h9_1T_random_state_from50_50_1g | 0 | 179 | 0.0 | 0 | 1G 100% |
| h48h9_48T_distance20_random100_10_1g | 0 | 131 | 0.0 | 0 | 1G 100% |
| h48h9_48T_distance20_random100_from10_90_1g | 0 | 252 | 0.0 | 0 | 1G 100% |
| h48h9_48T_random_state_200_1g | 0 | 131 | 0.0 | 0 | 1G 100% |
| h48h9_48T_random_state_200_1g_r2 | 0 | 131 | 0.0 | 0 | 1G 100% |
| h48h9_48T_random_state_200_1g_r3 | 0 | 131 | 0.0 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_1_1g_pgo | 0 | 450 | 3.7 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from1_1_1g_pgo | 0 | 419 | 3.8 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from2_1_1g_pgo | 0 | 435 | 3.5 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from3_1_1g_pgo | 0 | 449 | 3.7 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from4_1_1g_pgo | 0 | 433 | 3.5 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from5_1_1g_pgo | 0 | 439 | 3.5 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from6_1_1g_pgo | 0 | 411 | 3.5 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from7_1_1g_pgo | 0 | 412 | 3.5 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from8_1_1g_pgo | 0 | 473 | 3.3 | 0 | 1G 100% |
| nxopt22_1T_distance20_random10_from9_1_1g_pgo | 0 | 428 | 3.2 | 0 | 1G 100% |
| nxopt22_1T_random_move_15f_100_thp_pgo | 0 | 60 | 0.1 | 0 | THP 96% |
| nxopt22_1T_random_move_16f_100_thp_pgo | 0 | 66 | 0.0 | 0 | THP 96% |
| nxopt22_1T_random_move_17f_100_thp_pgo | 0 | 137 | 0.0 | 0 | THP 96% |
| nxopt22_1T_random_state_25_thp_pgo | 0 | 210 | 0.0 | 0 | THP 77% |
| nxopt22_1T_random_state_25_thp_r2 | 0 | 214 | 0.0 | 0 | THP 77% |
| nxopt22_1T_random_state_50_1g | 0 | 344 | 0.2 | 0 | 1G 100% |
| nxopt22_1T_random_state_50_1g_pgo | 0 | 346 | 0.0 | 0 | 1G 100% |
| nxopt22_1T_random_state_50_thp | 0 | 361 | 0.1 | 0 | THP 92% |
| nxopt22_1T_random_state_from25_25_thp_pgo | 0 | 204 | 0.0 | 0 | THP 95% |
| nxopt22_1T_random_state_from50_50_1g_pgo | 0 | 410 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_distance20_random100_10_1g_pgo | 0 | 481 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_distance20_random100_from10_45_1g_pgo | 0 | 562 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_distance20_random100_from55_45_1g_pgo | 0 | 503 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_random_state_200_1g | 0 | 136 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_random_state_200_1g_pgo | 0 | 136 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_random_state_200_1g_pgo_r2 | 0 | 138 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_random_state_200_1g_pgo_r3 | 0 | 139 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_random_state_200_1g_r2 | 0 | 139 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_random_state_200_1g_r3 | 0 | 141 | 0.0 | 0 | 1G 100% |
| nxopt22_48T_random_state_200_thp | 0 | 140 | 0.0 | 0 | THP 81% |
| nxopt22_48T_random_state_200_thp_pgo | 0 | 140 | 0.1 | 0 | THP 82% |
| nxopt22_48T_random_state_200_thp_pgo_r2 | 0 | 141 | 0.0 | 0 | THP 77% |
| nxopt22_48T_random_state_200_thp_pgo_r3 | 0 | 142 | 0.1 | 0 | THP 77% |
| nxopt22_48T_random_state_200_thp_r2 | 0 | 141 | 0.0 | 0 | THP 76% |
| nxopt22_48T_random_state_200_thp_r3 | 0 | 142 | 0.0 | 0 | THP 78% |
| nxopt22_48T_random_state_200_thp_r7 | 0 | 139 | 0.0 | 0 | THP 91% |
| nxopt31_1T_distance20_random10_1_1g_pgo | 0 | 665 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from1_1_1g_pgo | 0 | 620 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from2_1_1g_pgo | 0 | 662 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from3_1_1g_pgo | 0 | 676 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from4_1_1g_pgo | 0 | 683 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from5_1_1g_pgo | 0 | 669 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from6_1_1g_pgo | 0 | 630 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from7_1_1g_pgo | 0 | 708 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from8_1_1g_pgo | 0 | 679 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_distance20_random10_from9_1_1g_pgo | 0 | 611 | 3.9 | 0 | 1G 100% |
| nxopt31_1T_random_move_15f_100_thp_pgo | 0 | 20 | 0.1 | 0 | THP 100% |
| nxopt31_1T_random_move_16f_100_thp_pgo | 0 | 34 | 0.0 | 0 | THP 100% |
| nxopt31_1T_random_move_17f_100_thp_pgo | 0 | 174 | 0.0 | 0 | THP 100% |
| nxopt31_1T_random_state_25_1g | 0 | 363 | 0.2 | 0 | 1G 100% |
| nxopt31_1T_random_state_25_1g_pgo | 0 | 375 | 0.0 | 0 | 1G 100% |
| nxopt31_1T_random_state_25_thp | 0 | 348 | 0.2 | 0 | THP 100% |
| nxopt31_1T_random_state_25_thp_pgo | 0 | 369 | 0.0 | 0 | THP 100% |
| nxopt31_1T_random_state_25_thp_r2 | 0 | 369 | 0.0 | 0 | THP 100% |
| nxopt31_1T_random_state_from25_25_1g | 0 | 339 | 0.2 | 0 | 1G 100% |
| nxopt31_1T_random_state_from25_25_1g_pgo | 0 | 356 | 0.0 | 0 | 1G 100% |
| nxopt31_1T_random_state_from25_25_thp | 0 | 383 | 0.2 | 0 | THP 98% |
| nxopt31_1T_random_state_from25_25_thp_pgo | 0 | 362 | 0.1 | 0 | THP 100% |
| nxopt31_1T_random_state_from50_50_1g_pgo | 0 | 744 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_distance20_random100_10_1g_pgo | 0 | 791 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_distance20_random100_from10_20_1g_pgo | 0 | 941 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_distance20_random100_from30_20_1g_pgo | 0 | 782 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_distance20_random100_from50_20_1g_pgo | 0 | 829 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_distance20_random100_from70_20_1g_pgo | 0 | 856 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_distance20_random100_from90_10_1g_pgo | 0 | 714 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_random_state_200_1g | 0 | 157 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_random_state_200_1g_pgo | 0 | 155 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_random_state_200_1g_pgo_r2 | 0 | 159 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_random_state_200_1g_pgo_r3 | 0 | 159 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_random_state_200_1g_r2 | 0 | 159 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_random_state_200_1g_r3 | 0 | 160 | 0.0 | 0 | 1G 100% |
| nxopt31_48T_random_state_200_thp | 0 | 160 | 0.1 | 0 | THP 100% |
| nxopt31_48T_random_state_200_thp_pgo | 0 | 161 | 0.1 | 0 | THP 100% |
| nxopt31_48T_random_state_200_thp_pgo_r2 | 0 | 154 | 0.0 | 0 | THP 100% |
| nxopt31_48T_random_state_200_thp_pgo_r3 | 0 | 156 | 0.0 | 0 | THP 100% |
| nxopt31_48T_random_state_200_thp_r2 | 0 | 156 | 0.0 | 0 | THP 100% |
| nxopt31_48T_random_state_200_thp_r3 | 0 | 157 | 0.0 | 0 | THP 100% |
| nxopt31_48T_random_state_200_thp_r7 | 0 | 159 | 0.1 | 0 | THP 100% |
| vcube112_1T_distance20_random10_1_1g | 0 | 440 | 3.8 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from1_1_1g | 0 | 408 | 3.9 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from2_1_1g | 0 | 440 | 3.8 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from3_1_1g | 0 | 440 | 3.8 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from4_1_1g | 0 | 453 | 3.8 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from5_1_1g | 0 | 445 | 3.8 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from6_1_1g | 0 | 427 | 3.9 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from7_1_1g | 0 | 489 | 3.6 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from8_1_1g | 0 | 457 | 3.7 | 0 | 1G 100% |
| vcube112_1T_distance20_random10_from9_1_1g | 0 | 419 | 3.9 | 0 | 1G 100% |
| vcube112_1T_random_move_15f_100_thp | 0 | 22 | 0.1 | 0 | THP 100% |
| vcube112_1T_random_move_16f_100_thp | 0 | 29 | 0.0 | 0 | THP 100% |
| vcube112_1T_random_move_17f_100_thp | 0 | 119 | 0.0 | 0 | THP 100% |
| vcube112_1T_random_state_25_1g | 0 | 242 | 0.2 | 4 | 1G 100% |
| vcube112_1T_random_state_25_1g_native | 0 | 242 | 0.2 | 0 | 1G 100% |
| vcube112_1T_random_state_25_thp | 0 | 244 | 0.2 | 0 | THP 100% |
| vcube112_1T_random_state_25_thp_pgo | 0 | 245 | 0.0 | 0 | THP 100% |
| vcube112_1T_random_state_25_thp_r2 | 0 | 241 | 0.0 | 0 | THP 100% |
| vcube112_1T_random_state_from25_25_1g | 0 | 224 | 0.2 | 0 | 1G 100% |
| vcube112_1T_random_state_from25_25_thp | 0 | 239 | 0.1 | 0 | THP 100% |
| vcube112_1T_random_state_from50_50_1g | 0 | 469 | 0.0 | 0 | 1G 100% |
| vcube112_48T_distance20_random100_10_1g | 0 | 494 | 0.0 | 0 | 1G 100% |
| vcube112_48T_distance20_random100_from10_45_1g | 0 | 632 | 0.0 | 0 | 1G 100% |
| vcube112_48T_distance20_random100_from55_45_1g | 0 | 564 | 0.1 | 0 | 1G 100% |
| vcube112_48T_random_state_200_1g | 0 | 106 | 0.0 | 0 | 1G 100% |
| vcube112_48T_random_state_200_1g_r2 | 0 | 107 | 0.1 | 0 | 1G 100% |
| vcube112_48T_random_state_200_1g_r3 | 0 | 106 | 0.0 | 0 | 1G 100% |
| vcube112_48T_random_state_200_thp | 0 | 108 | 0.1 | 0 | THP 100% |
| vcube112_48T_random_state_200_thp_pgo | 0 | 109 | 0.1 | 0 | THP 100% |
| vcube112_48T_random_state_200_thp_pgo_r2 | 0 | 106 | 0.0 | 0 | THP 100% |
| vcube112_48T_random_state_200_thp_pgo_r3 | 0 | 106 | 0.0 | 0 | THP 100% |
| vcube112_48T_random_state_200_thp_r2 | 0 | 105 | 0.0 | 0 | THP 100% |
| vcube112_48T_random_state_200_thp_r3 | 0 | 105 | 0.0 | 0 | THP 100% |
| vcube112_48T_random_state_200_thp_r7 | 0 | 106 | 0.1 | 0 | THP 100% |
| vcube208_1T_distance20_random10_2_1g | 0 | 535 | 3.2 | 0 | 1G 100% |
| vcube208_1T_distance20_random10_from2_2_1g | 0 | 546 | 3.2 | 0 | 1G 100% |
| vcube208_1T_distance20_random10_from4_2_1g | 0 | 561 | 3.2 | 0 | 1G 100% |
| vcube208_1T_distance20_random10_from6_2_1g | 0 | 581 | 3.4 | 0 | 1G 100% |
| vcube208_1T_distance20_random10_from8_2_1g | 0 | 561 | 3.4 | 0 | 1G 100% |
| vcube208_1T_random_move_15f_100_thp | 0 | 60 | 0.0 | 0 | THP 100% |
| vcube208_1T_random_move_16f_100_thp | 0 | 64 | 0.0 | 0 | THP 100% |
| vcube208_1T_random_move_17f_100_thp | 0 | 112 | 0.0 | 0 | THP 100% |
| vcube208_1T_random_state_25_thp_pgo | 0 | 163 | 0.0 | 0 | THP 78% |
| vcube208_1T_random_state_25_thp_r2 | 0 | 164 | 0.0 | 0 | THP 75% |
| vcube208_1T_random_state_50_1g | 0 | 249 | 0.2 | 0 | 1G 100% |
| vcube208_1T_random_state_50_thp | 0 | 259 | 0.1 | 0 | THP 98% |
| vcube208_1T_random_state_from50_50_1g | 0 | 278 | 0.0 | 0 | 1G 100% |
| vcube208_48T_distance20_random100_10_1g | 0 | 341 | 0.0 | 0 | 1G 100% |
| vcube208_48T_distance20_random100_from10_45_1g | 0 | 398 | 0.0 | 0 | 1G 100% |
| vcube208_48T_distance20_random100_from55_45_1g | 0 | 360 | 0.0 | 0 | 1G 100% |
| vcube208_48T_random_state_200_1g | 0 | 112 | 0.0 | 0 | 1G 100% |
| vcube208_48T_random_state_200_1g_r2 | 0 | 112 | 0.0 | 0 | 1G 100% |
| vcube208_48T_random_state_200_1g_r3 | 0 | 113 | 0.0 | 0 | 1G 100% |
| vcube208_48T_random_state_200_thp | 0 | 113 | 0.1 | 0 | THP 82% |
| vcube208_48T_random_state_200_thp_pgo | 0 | 114 | 0.0 | 0 | THP 82% |
| vcube208_48T_random_state_200_thp_pgo_r2 | 0 | 115 | 0.0 | 0 | THP 80% |
| vcube208_48T_random_state_200_thp_pgo_r3 | 0 | 114 | 0.0 | 0 | THP 78% |
| vcube208_48T_random_state_200_thp_r2 | 0 | 116 | 0.1 | 0 | THP 79% |
| vcube208_48T_random_state_200_thp_r3 | 0 | 115 | 0.0 | 0 | THP 77% |
| vcube308_1T_distance20_random10_6_1g | 0 | 596 | 3.5 | 0 | 1G 100% |
| vcube308_1T_distance20_random10_from6_1_1g | 0 | 236 | 3.3 | 0 | 1G 100% |
| vcube308_1T_distance20_random10_from6_4_1g | 124 | 380 | 4.5 | 0 | 1G 100% |
| vcube308_1T_distance20_random10_from7_1_1g | 0 | 246 | 3.3 | 0 | 1G 100% |
| vcube308_1T_distance20_random10_from8_1_1g | 0 | 250 | 3.3 | 0 | 1G 100% |
| vcube308_1T_distance20_random10_from9_1_1g | 0 | 235 | 3.3 | 0 | 1G 100% |
| vcube308_1T_random_move_15f_100_1g | 0 | 175 | 0.0 | 0 | 1G 100% |
| vcube308_1T_random_move_16f_100_1g | 0 | 177 | 0.0 | 0 | 1G 100% |
| vcube308_1T_random_move_17f_100_1g | 0 | 193 | 0.0 | 0 | 1G 100% |
| vcube308_1T_random_state_50_1g | 0 | 245 | 0.1 | 0 | 1G 100% |
| vcube308_1T_random_state_from50_50_1g | 0 | 245 | 0.0 | 0 | 1G 100% |
| vcube308_48T_distance20_random100_10_1g | 0 | 259 | 0.0 | 0 | 1G 100% |
| vcube308_48T_distance20_random100_from10_45_1g | 0 | 261 | 0.0 | 0 | 1G 100% |
| vcube308_48T_distance20_random100_from55_45_1g | 0 | 260 | 0.0 | 0 | 1G 100% |
| vcube308_48T_random_state_200_1g | 0 | 187 | 0.0 | 0 | 1G 100% |
| vcube308_48T_random_state_200_1g_r2 | 0 | 187 | 0.1 | 0 | 1G 100% |
| vcube308_48T_random_state_200_1g_r3 | 0 | 187 | 0.1 | 0 | 1G 100% |

Total measured wall time: 29.87 h.


## Tables

| table | size | status | generation time |
|---|---:|---|---|
| nxopt31 | 2603 MB | made | 55 s (8 cores) |
| vcube112 | 2603 MB | made | ~75 s (8 cores; 71 s search + write) |
| h48h7 | 3794 MB | made | 17 min of compute on 8 cores (8144 s CPU) + 3.5 min of nissy's progress-log sleep |
| nxopt22 | 7809 MB | made | 77 s at 48 threads (24 s search, 53 s writing) |
| vcube208 | 7809 MB | made | 87 s at 48 threads (23 s search + write) |
| cubeopt30 | 2602 MB | made | 243 s at 48 threads, including solving one cube |
| h48h8 | 7586 MB | made (/data2) | 1642 s at 48 threads (72,000 s CPU, 7.5 GB peak): ~10x h48h7's work (base 10 vs 9); h48h5 and h48h7 both take ~17-20 min on 8 cores |
| vcube308 | 22777 MB | made (/data2) | 316 s at 48 threads (7852 s CPU; vcube's own MAP_HUGETLB, 22 pool pages) |
| cube48opt10 | 31131 MB | made (/data2/competitors/cube48opt10) | 1587 s at 48 threads (63,285 s CPU), including solving one cube; hugeshim 1 GiB arena |
| h48h9 | 15169 MB | made (/data2) | 1784 s at 48 threads (75,000 s CPU, 14.9 GB peak) |
| h48h10 | 30336 MB | made (/data2) | 1720 s at 48 threads (68,528 s CPU; h48gen with the hugeshim 1 GiB arena, 29 pool pages) |
| h48h11 | 60671 MB | made (/data2) | 1332 s at 48 threads (39,876 s CPU; h48gen with the hugeshim 1 GiB arena, 57 pool pages); nissy's h11 generator is slightly inadmissible (x-h48: 11,546 entries one level high), so watch the length cross-check |
| egg-solver h9, h10 | 14.5 / 29.0 GiB | made: /data2/tables/h9, h10 (`gen N auto --parts 4`), `gen compare` against /mnt/huge/h11: same | 250 s / 382 s at 48 threads (4.1 / 7.9 GB peak RAM) |

## Memory and hugepage steps

Table files: the competitors' small tables are on /data/competitors, everything large on /data2
(/data2/competitors/{vcube,cube48opt,h48}, egg-solver's h9/h10 in /data2/tables); the harness's table
folders point there. All tables stay on disk, so nothing is deleted or regenerated between runs.
Page columns need: 1G rows = free 1 GiB pool pages for the duration of a run (one run at a time);
THP rows = ordinary RAM (~18 GiB is free; egg-solver's THP copy of h11 needs 58 GiB).

| table | 1G row needs (free pool pages) | THP row needs (ordinary RAM) |
|---|---:|---:|
| nxopt31, vcube112, cubeopt30 | 3 | 2.6 GB |
| h48h7 | 4 | 3.8 GB |
| nxopt22, vcube208, h48h8 | 8 | 7.8 GB |
| egg-solver h9 / h48h9 | 15 / 15 | 15.6 / 15.2 GB |
| vcube308 | 22 (vcube maps them itself) | 22.8 GB |
| egg-solver h10, cube48opt10, h48h10 | 29, 29 (+8 while generating), 29 | 31.1, 31.1 (+7.8), 30.3 GB |
| egg-solver h11, h48h11 | 58 (the hugetlbfs file), 57 | 62.3, 60.7 GB |

Order:
1. Phase 1 (done for the competitors' 1G rows with the 12 pages freed from /mnt/huge/h7+h8): small and
   medium tables, both columns; then /mnt/huge/h7 and h8 restored from /data/tables/h7.mnt, h8.mnt
   (`hugecp`), and egg-solver's h7/h8 rows (1G from /mnt/huge, THP from the .mnt copies).
2. egg-solver h9/h10 into /data2/tables (done: `gen 9|10 auto /data2/tables/hN --parts 4`, both "same" as
   /mnt/huge/h11 under `gen compare`). Also `gen compare /data/tables/h11
   /mnt/huge/h11` (via the sidecar tables-thp/h11.meta), since egg-solver's THP h11 row copies that file.
3. Phase 2: remove /mnt/huge/h11 (58 free pages), `final-compare.sh --prep` for vcube308, cube48opt10,
   h48h8..h11 (written to /data2), then one large table at a time on 1 GiB pages (hugecp egg-solver's
   h9/h10 to /mnt/huge for its rows; competitors via vcube's own MAP_HUGETLB or the hugeshim arena).
4. (No THP rows for the large tables: 1 GiB only, by decision.)
5. Restore h11 from its verified saved copy: `hugecp /data2/tables/h11.mnt /mnt/huge/h11`, then
   `cmp /data2/tables/h11.mnt /mnt/huge/h11`. The pool remains at 70 pages throughout; no pool
   resizing or regeneration is needed. h7 and h8 remain in /mnt/huge.



## Complete coverage ledger

Pending cells retain their measured sample count where available. Distance-20 aggregate timings require complete coverage.

| Required cell | Status | Cubes present / required | Repeats / required |
|---|---|---:|---:|
| headline ours h7 latency shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h7 latency shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h7 latency shipped thp 1T | Complete | 50/50 | 1/1 |
| headline ours h7 latency shipped thp 48T | Complete | 200/200 | 3/3 |
| headline ours h7 batch shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h7 batch shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h7 batch shipped thp 1T | Complete | 50/50 | 1/1 |
| headline ours h7 batch shipped thp 48T | Complete | 200/200 | 3/3 |
| headline ours h8 latency shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h8 latency shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h8 latency shipped thp 1T | Complete | 50/50 | 1/1 |
| headline ours h8 latency shipped thp 48T | Complete | 200/200 | 3/3 |
| headline ours h8 batch shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h8 batch shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h8 batch shipped thp 1T | Complete | 50/50 | 1/1 |
| headline ours h8 batch shipped thp 48T | Complete | 200/200 | 3/3 |
| headline ours h9 latency shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h9 latency shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h9 batch shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h9 batch shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h10 latency shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h10 latency shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h10 batch shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h10 batch shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h11 latency shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h11 latency shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline ours h11 batch shipped 1g 1T | Complete | 50/50 | 1/1 |
| headline ours h11 batch shipped 1g 48T | Complete | 200/200 | 3/3 |
| headline nxopt31 1g 1T | Complete | 50/50 | 1/1 |
| headline nxopt31 1g 48T | Complete | 200/200 | 3/3 |
| headline nxopt31 thp 1T | Complete | 50/50 | 1/1 |
| headline nxopt31 thp 48T | Complete | 200/200 | 3/3 |
| headline nxopt22 1g 1T | Complete | 50/50 | 1/1 |
| headline nxopt22 1g 48T | Complete | 200/200 | 3/3 |
| headline nxopt22 thp 1T | Complete | 50/50 | 1/1 |
| headline nxopt22 thp 48T | Complete | 200/200 | 3/3 |
| headline vcube112 1g 1T | Complete | 50/50 | 1/1 |
| headline vcube112 1g 48T | Complete | 200/200 | 3/3 |
| headline vcube112 thp 1T | Complete | 50/50 | 1/1 |
| headline vcube112 thp 48T | Complete | 200/200 | 4/3 |
| headline vcube208 1g 1T | Complete | 50/50 | 1/1 |
| headline vcube208 1g 48T | Complete | 200/200 | 3/3 |
| headline vcube208 thp 1T | Complete | 50/50 | 1/1 |
| headline vcube208 thp 48T | Complete | 200/200 | 3/3 |
| headline vcube308 1g 1T | Complete | 50/50 | 1/1 |
| headline vcube308 1g 48T | Complete | 200/200 | 3/3 |
| headline cubeopt30 1g 1T | Complete | 50/50 | 1/1 |
| headline cubeopt30 1g 48T | Complete | 200/200 | 3/3 |
| headline cubeopt30 thp 1T | Complete | 50/50 | 1/1 |
| headline cubeopt30 thp 48T | Complete | 200/200 | 4/3 |
| headline cube48opt10 1g 1T | Complete | 50/50 | 1/1 |
| headline cube48opt10 1g 48T | Complete | 200/200 | 3/3 |
| headline h48h7 1g 1T | Complete | 50/50 | 1/1 |
| headline h48h7 1g 48T | Complete | 200/200 | 3/3 |
| headline h48h7 thp 1T | Complete | 50/50 | 1/1 |
| headline h48h7 thp 48T | Complete | 200/200 | 4/3 |
| headline h48h8 1g 1T | Complete | 50/50 | 1/1 |
| headline h48h8 1g 48T | Complete | 200/200 | 3/3 |
| headline h48h8 thp 1T | Complete | 50/50 | 1/1 |
| headline h48h8 thp 48T | Complete | 200/200 | 3/3 |
| headline h48h9 1g 1T | Complete | 50/50 | 1/1 |
| headline h48h9 1g 48T | Complete | 200/200 | 3/3 |
| headline h48h10 1g 1T | Complete | 50/50 | 1/1 |
| headline h48h10 1g 48T | Complete | 200/200 | 3/3 |
| headline h48h11 1g 1T | Complete | 50/50 | 1/1 |
| headline h48h11 1g 48T | Complete | 200/200 | 3/3 |
| headline c48rsopt7 1g 1T | Complete | 50/50 | 1/1 |
| headline c48rsopt7 1g 48T | Complete | 200/200 | 3/3 |
| headline c48rsopt7 thp 1T | Complete | 50/50 | 1/1 |
| headline c48rsopt7 thp 48T | Complete | 200/200 | 3/3 |
| headline c48rsbatchopt7 1g 1T | Complete | 50/50 | 1/1 |
| headline c48rsbatchopt7 1g 48T | Complete | 200/200 | 3/3 |
| headline c48rsbatchopt7 thp 1T | Complete | 50/50 | 1/1 |
| headline c48rsbatchopt7 thp 48T | Complete | 200/200 | 3/3 |
| headline c48rsopt8 1g 1T | Complete | 50/50 | 1/1 |
| headline c48rsopt8 1g 48T | Complete | 200/200 | 3/3 |
| headline c48rsopt8 thp 1T | Complete | 50/50 | 1/1 |
| headline c48rsopt8 thp 48T | Complete | 200/200 | 3/3 |
| headline c48rsbatchopt8 1g 1T | Complete | 50/50 | 1/1 |
| headline c48rsbatchopt8 1g 48T | Complete | 200/200 | 3/3 |
| headline c48rsbatchopt8 thp 1T | Complete | 50/50 | 1/1 |
| headline c48rsbatchopt8 thp 48T | Complete | 200/200 | 3/3 |
| headline c48rsopt10 1g 1T | Complete | 50/50 | 1/1 |
| headline c48rsopt10 1g 48T | Complete | 200/200 | 3/3 |
| headline c48rsbatchopt10 1g 1T | Complete | 50/50 | 1/1 |
| headline c48rsbatchopt10 1g 48T | Complete | 200/200 | 3/3 |
| d20 nxopt31 chosen 1T | Complete | 10/10 | 1/1 |
| d20 nxopt31 chosen 48T | Complete | 100/100 | 1/1 |
| d20 nxopt22 chosen 1T | Complete | 10/10 | 1/1 |
| d20 nxopt22 chosen 48T | Complete | 100/100 | 1/1 |
| d20 vcube112 chosen 1T | Complete | 10/10 | 1/1 |
| d20 vcube112 chosen 48T | Complete | 100/100 | 1/1 |
| d20 vcube208 chosen 1T | Complete | 10/10 | 1/1 |
| d20 vcube208 chosen 48T | Complete | 100/100 | 1/1 |
| d20 vcube308 chosen 1T | Complete | 10/10 | 1/1 |
| d20 vcube308 chosen 48T | Complete | 100/100 | 1/1 |
| d20 cubeopt30 chosen 1T | Complete | 10/10 | 1/1 |
| d20 cubeopt30 chosen 48T | Complete | 100/100 | 1/1 |
| d20 cube48opt10 chosen 1T | Complete | 10/10 | 1/1 |
| d20 cube48opt10 chosen 48T | Complete | 100/100 | 1/1 |
| d20 h48h7 chosen 1T | Complete | 10/10 | 1/1 |
| d20 h48h7 chosen 48T | Complete | 100/100 | 1/1 |
| d20 h48h8 chosen 1T | Complete | 10/10 | 1/1 |
| d20 h48h8 chosen 48T | Complete | 100/100 | 1/1 |
| d20 h48h9 chosen 1T | Complete | 10/10 | 1/1 |
| d20 h48h9 chosen 48T | Complete | 100/100 | 1/1 |
| d20 h48h10 chosen 1T | Complete | 10/10 | 1/1 |
| d20 h48h10 chosen 48T | Complete | 100/100 | 1/1 |
| d20 h48h11 chosen 1T | Complete | 10/10 | 1/1 |
| d20 h48h11 chosen 48T | Complete | 100/100 | 1/1 |
| d20 c48rsopt7 chosen 1T | Complete | 10/10 | 1/1 |
| d20 c48rsopt7 chosen 48T | Complete | 100/100 | 1/1 |
| d20 c48rsbatchopt7 chosen 1T | Complete | 10/10 | 1/1 |
| d20 c48rsbatchopt7 chosen 48T | Complete | 100/100 | 1/1 |
| d20 c48rsopt8 chosen 1T | Complete | 10/10 | 1/1 |
| d20 c48rsopt8 chosen 48T | Complete | 100/100 | 1/1 |
| d20 c48rsbatchopt8 chosen 1T | Complete | 10/10 | 1/1 |
| d20 c48rsbatchopt8 chosen 48T | Complete | 100/100 | 1/1 |
| d20 c48rsopt10 chosen 1T | Complete | 10/10 | 1/1 |
| d20 c48rsopt10 chosen 48T | Complete | 100/100 | 1/1 |
| d20 c48rsbatchopt10 chosen 1T | Complete | 10/10 | 1/1 |
| d20 c48rsbatchopt10 chosen 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h7 shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h7 shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h7 batch shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h7 batch shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h8 shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h8 shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h8 batch shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h8 batch shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h9 shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h9 shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h9 batch shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h9 batch shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h10 shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h10 shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h10 batch shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h10 batch shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h11 shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h11 shipped 48T | Complete | 100/100 | 1/1 |
| d20 egg-solver h11 batch shipped 1T | Complete | 10/10 | 1/1 |
| d20 egg-solver h11 batch shipped 48T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h7 shipped 1T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h8 shipped 1T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h9 shipped 1T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h9 batch shipped 1T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h10 shipped 1T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h10 batch shipped 1T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h11 shipped 1T | Complete | 100/100 | 1/1 |
| d20 extended egg-solver h11 batch shipped 1T | Complete | 100/100 | 1/1 |
| wiki egg-solver h7 shipped random_move_15f | Complete | 100/100 | — |
| wiki egg-solver h7 shipped random_move_16f | Complete | 100/100 | — |
| wiki egg-solver h7 shipped random_move_17f | Complete | 100/100 | — |
| wiki egg-solver h7 shipped random_state | Complete | 100/100 | — |
| wiki egg-solver h8 shipped random_move_15f | Complete | 100/100 | — |
| wiki egg-solver h8 shipped random_move_16f | Complete | 100/100 | — |
| wiki egg-solver h8 shipped random_move_17f | Complete | 100/100 | — |
| wiki egg-solver h8 shipped random_state | Complete | 100/100 | — |
| wiki egg-solver h9 shipped random_move_15f | Complete | 100/100 | — |
| wiki egg-solver h9 shipped random_move_16f | Complete | 100/100 | — |
| wiki egg-solver h9 shipped random_move_17f | Complete | 100/100 | — |
| wiki egg-solver h9 shipped random_state | Complete | 100/100 | — |
| wiki egg-solver h10 shipped random_move_15f | Complete | 100/100 | — |
| wiki egg-solver h10 shipped random_move_16f | Complete | 100/100 | — |
| wiki egg-solver h10 shipped random_move_17f | Complete | 100/100 | — |
| wiki egg-solver h10 shipped random_state | Complete | 100/100 | — |
| wiki egg-solver h11 shipped random_move_15f | Complete | 100/100 | — |
| wiki egg-solver h11 shipped random_move_16f | Complete | 100/100 | — |
| wiki egg-solver h11 shipped random_move_17f | Complete | 100/100 | — |
| wiki egg-solver h11 shipped random_state | Complete | 100/100 | — |
| wiki nxopt31 chosen random_move_15f | Complete | 100/100 | — |
| wiki nxopt31 chosen random_move_16f | Complete | 100/100 | — |
| wiki nxopt31 chosen random_move_17f | Complete | 100/100 | — |
| wiki nxopt31 chosen random_state | Complete | 100/100 | — |
| wiki nxopt22 chosen random_move_15f | Complete | 100/100 | — |
| wiki nxopt22 chosen random_move_16f | Complete | 100/100 | — |
| wiki nxopt22 chosen random_move_17f | Complete | 100/100 | — |
| wiki nxopt22 chosen random_state | Complete | 100/100 | — |
| wiki vcube112 chosen random_move_15f | Complete | 100/100 | — |
| wiki vcube112 chosen random_move_16f | Complete | 100/100 | — |
| wiki vcube112 chosen random_move_17f | Complete | 100/100 | — |
| wiki vcube112 chosen random_state | Complete | 100/100 | — |
| wiki vcube208 chosen random_move_15f | Complete | 100/100 | — |
| wiki vcube208 chosen random_move_16f | Complete | 100/100 | — |
| wiki vcube208 chosen random_move_17f | Complete | 100/100 | — |
| wiki vcube208 chosen random_state | Complete | 100/100 | — |
| wiki vcube308 chosen random_move_15f | Complete | 100/100 | — |
| wiki vcube308 chosen random_move_16f | Complete | 100/100 | — |
| wiki vcube308 chosen random_move_17f | Complete | 100/100 | — |
| wiki vcube308 chosen random_state | Complete | 100/100 | — |
| wiki cubeopt30 chosen random_move_15f | Complete | 100/100 | — |
| wiki cubeopt30 chosen random_move_16f | Complete | 100/100 | — |
| wiki cubeopt30 chosen random_move_17f | Complete | 100/100 | — |
| wiki cubeopt30 chosen random_state | Complete | 100/100 | — |
| wiki cube48opt10 chosen random_move_15f | Complete | 100/100 | — |
| wiki cube48opt10 chosen random_move_16f | Complete | 100/100 | — |
| wiki cube48opt10 chosen random_move_17f | Complete | 100/100 | — |
| wiki cube48opt10 chosen random_state | Complete | 100/100 | — |
| wiki h48h7 chosen random_move_15f | Complete | 100/100 | — |
| wiki h48h7 chosen random_move_16f | Complete | 100/100 | — |
| wiki h48h7 chosen random_move_17f | Complete | 100/100 | — |
| wiki h48h7 chosen random_state | Complete | 100/100 | — |
| wiki h48h8 chosen random_move_15f | Complete | 100/100 | — |
| wiki h48h8 chosen random_move_16f | Complete | 100/100 | — |
| wiki h48h8 chosen random_move_17f | Complete | 100/100 | — |
| wiki h48h8 chosen random_state | Complete | 100/100 | — |
| wiki h48h9 chosen random_move_15f | Complete | 100/100 | — |
| wiki h48h9 chosen random_move_16f | Complete | 100/100 | — |
| wiki h48h9 chosen random_move_17f | Complete | 100/100 | — |
| wiki h48h9 chosen random_state | Complete | 100/100 | — |
| wiki h48h10 chosen random_move_15f | Complete | 100/100 | — |
| wiki h48h10 chosen random_move_16f | Complete | 100/100 | — |
| wiki h48h10 chosen random_move_17f | Complete | 100/100 | — |
| wiki h48h10 chosen random_state | Complete | 100/100 | — |
| wiki h48h11 chosen random_move_15f | Complete | 100/100 | — |
| wiki h48h11 chosen random_move_16f | Complete | 100/100 | — |
| wiki h48h11 chosen random_move_17f | Complete | 100/100 | — |
| wiki h48h11 chosen random_state | Complete | 100/100 | — |
| wiki c48rsopt7 chosen random_move_15f | Complete | 100/100 | — |
| wiki c48rsopt7 chosen random_move_16f | Complete | 100/100 | — |
| wiki c48rsopt7 chosen random_move_17f | Complete | 100/100 | — |
| wiki c48rsopt7 chosen random_state | Complete | 100/100 | — |
| wiki c48rsopt8 chosen random_move_15f | Complete | 100/100 | — |
| wiki c48rsopt8 chosen random_move_16f | Complete | 100/100 | — |
| wiki c48rsopt8 chosen random_move_17f | Complete | 100/100 | — |
| wiki c48rsopt8 chosen random_state | Complete | 100/100 | — |
| wiki c48rsopt10 chosen random_move_15f | Complete | 100/100 | — |
| wiki c48rsopt10 chosen random_move_16f | Complete | 100/100 | — |
| wiki c48rsopt10 chosen random_move_17f | Complete | 100/100 | — |
| wiki c48rsopt10 chosen random_state | Complete | 100/100 | — |

## Safety evidence

<a id="no-panic-source-inventory"></a>
<a id="no-panic-integration-remaining-implicit-check-inventory"></a>

See [the safety report](../safety/NO_UB.md), [explicit-call inventory](../data/comparison/no-panic-inventory.md), and [implicit-check inventory](../data/comparison/implicit-check-inventory.md). The [original compiled snapshot](../data/comparison/reports/compiled-snapshot-20261108.md) is retained for provenance.
