# CUBEOPT comparison: all currently known results

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


## Bugs and issues found in other solvers (for the final report; the user will report them upstream)

Source commits read: nissy-core 1f7e8e9 (2026-09-17), vcube 9f5bc2c (2024-04-18), cube20src/nxopt f430921 (2025-07-18), cube_solver_test b9cca71 (2026-02-01); cubeopt30/cube48opt10 = the prebuilt binaries in ~/ (no source).

Each entry: project, file:line at the commit we read, what is wrong, a repro or proof, impact, and a suggested fix.

| # | project | severity | summary |
|---|---|---|---|
| 1 | nissy-core (H48) | correctness (admissibility) | h11 generator skips the last hash-map slot: 11,546 h11 entries are one level too high |
| 2 | cube_solver_test (h48cmd.py) | harness | per-cube lines are mis-paired (stdout is buffered, nissy logs to stderr); the Avg line is unaffected |
| 3 | cube_solver_test (h48cmd.py) | harness / memory | `bytearray(f.read())` holds the table twice (2x RAM); h48h8 does not fit in ~16 GiB free |
| 4 | cube_solver_test (runtest.sh) | harness | passes `-g $ngroup` to cubeopt/cube48opt, but the published binaries accept only `-l -m -n -t -v` (no `-g`); the option is ignored |
| 5 | vcube | performance | when MAP_HUGETLB (1G, then 2M) fails, falls back to a plain mmap with no madvise(MADV_HUGEPAGE): 4K pages, ~10% slower on THP=madvise systems |
| 6 | nxopt | performance | the table is one malloc per corner row with no MADV_HUGEPAGE: 4K pages on THP=madvise systems, ~11% slower |

### 1. nissy-core: h11 table slightly inadmissible (generator skips the last hash slot)

- **Where:** h48/gendata_h48.h:307-316 (`gendata_h48_runthread`) with h48/map.h:68-87 (`h48map_nextkvpair` post-increments the slot index).
  - Commit: nissy-core 1f7e8e9.
- **What:** the loop `kv = nextkvpair(map, next); if (*next == capacity) break;` breaks before processing the entry in the last slot (`capacity - 1`).
  - With the fixed capacity 10000019 and nissy's hash, that slot holds the depth-8 h11 entry 189880194105 (class 2675, no self-symmetry). Its 4-level DFS never runs: the loop visits 7,437,854 of 7,437,855 short entries.
  - The same loop shape in `gendata_h48short` (gendata_h48.h:62-65) is harmless, because that entry's value is 8 (the last BFS level).
- **Proof:**
  - nissy's own depth-≤8 BFS equals ours per depth (1/1/4/34/333/3815/45382/548562/6839723).
  - Modelling nissy's values in the 49,206 entries within 4 moves of the skipped root gives ours − nissy = [+159, +1689, +7691, −9539] per 2-bit code. That exactly matches the difference between our exact h11 distribution [1150763320, 12045847349, 91163441021, 136418085910] and nissy's checkdata.h.
  - Scripts: /data/agents/x-h48/drv/{short,lastslot,skipped}.c.
- **Impact:** 11,546 entries are over-estimated by one level (159 true ≤10 coded 1, 1848 true 11 coded 2, 9539 true 12 coded 3). The table is therefore not admissible: a solve whose optimal path passes through one of these positions near the end could return a non-optimal solution. The probability is tiny on random cubes but nonzero.
- **Fix:** after `h48map_nextkvpair`, test `kv.key == MAP_KEYMASK` while holding the mutex. That sentinel means the iterator is exhausted; every non-sentinel entry, including the last occupied slot, must be processed. Regenerate h11 and update checkdata.h.
- **To check in the final comparison:** whether h9/h10 (other k) are affected; the length cross-check on h48h11 runs.

### 2–4. cube_solver_test harness issues (found by f1 while extending it)

- **2. h48cmd.py output pairing.** Python's stdout is block-buffered, while nissy prints its stats to stderr, so runtest.sh's awk pairs "Nodes visited" and "Time" lines from different cubes. Per-cube lines are wrong; totals and Avg are right. Fix: `flush=True` / `sys.stdout.flush()` after each solve. Patch in /data/agents/f1/cube_solver_test-cubeoptrs.patch.
- **3. h48cmd.py double table copy.** `bytearray(f.read())` keeps the bytes object and the bytearray (2x the table) alive, so h48h8+ needs twice the RAM. Fix: `buf = bytearray(size); f.readinto(buf)`. Patch: same file.
- **4. runtest.sh `-g`.** cubeopt()/cube48opt() pass `-g $ngroup`, but the published cubeopt30/cube48opt10 binaries parse only `-l -m -n -t -v` (from their disassembly), so ngroup has no effect. Either the binaries or the script are out of date.

### 5–6. Huge-page handling (performance, not correctness)

- **5. vcube** (src/alloc.cpp): tries MAP_HUGETLB 1G, then 2M (both fail when the hugetlb pool has no free pages), then falls back to a plain mmap with no madvise. On THP=madvise systems the table ends up on 4K pages. Measured with an LD_PRELOAD madvise shim: 1T −10%, 48T −9% sec/solve.
  - Fix: `madvise(p, len, MADV_HUGEPAGE)` on the fallback mapping.
- **6. nxopt** (cube20src nxopt.w): one malloc per corner row (114 × 17.5 MiB + smaller), no madvise, so 4K pages on THP=madvise. With glibc.malloc.hugetlb=1 + mmap_threshold=32 MiB: 1T −11%, 48T −11%.
  - Fix: one aligned allocation for the whole table + MADV_HUGEPAGE.

### f1: exact locations for #2-#6, and new findings #7-#10 (f1, 2026-11-04)

Exact locations for the entries above (code as read; harness = ~/ref/cube_solver_test b9cca71):
- **#2:** h48cmd.py:31 `print (scramble)` and :34 `print('Time: ...')` go to Python's stdout, which is block-buffered when piped. nissy's stats go to stderr: python/nissy_module.c:447-449 `log_stdout()` (misnamed) does `fprintf(stderr, ...)` and is installed at :455. runtest.sh:85 merges them with `2>&1`.
  - Mechanism, reproduced: `python3 -c 'import sys; [ (sys.stderr.write("Nodes visited: %d\n" % i), print("Time: %d" % i)) for i in range(2)]' 2>&1 | cat` prints both "Nodes" lines before both "Time" lines. runtest.sh's awk (h48(), runtest.sh:86-97) then pairs every Time with the last cube's nodes.
  - Fix in our copy: `sys.stdout.flush()` after each cube.
- **#3:** h48cmd.py:17 `solver_data = bytearray(f.read())`.
- **#4:** runtest.sh:124 `./cube$3$2 -t $nthread -g $ngroup -m $maxl -`.
  - Binaries: the option switch in main does `sub $0x6c,%eax; cmp $0xa,%al` (cube48opt10 at 0x2a79, cubeopt30 at 0x2a99), then a jump table (0x183f4 / 0x193f4). That table has cases only for l, m, n, t and v: `-l` scramble length (default 15), `-m` max depth (20), `-n` count (100), `-t` threads, `-v` debug.
  - `-g` is skipped; its value "1" briefly becomes the mode character, and the final "-" sets the stdin mode again. So `-g` is harmless but has no effect.
- **#5:** vcube src/alloc.cpp:45-57 (`alloc::huge_impl`: `map_huge(n, 30)` -> `map_huge(n, 21)` -> `map_huge(n, 0)`; `map_huge` at :36-43 adds no madvise). The SysV-shm path is at :77-89 (the `-S` option).
  - strace: `mmap(3072 MiB, MAP_HUGETLB) = -1`, `mmap(2484 MiB, MAP_HUGETLB) = -1`, then a plain 2482 MiB mmap.
  - 200-cube numbers: /data/agents/f1/final/results.md (phase A).
- **#6:** cube20src nxopt.w:749 `firstlevs[j].p = (unsigned int *)malloc(MEMSIZE * (size_t)siz);` (nxopt.cpp:837 after ctangle). It allocates one block per distinct corner row.
  - glibc serves each block with its own mmap: strace shows 114 mmaps of 17.5 MiB plus smaller ones, and no madvise.
  - `glibc.malloc.hugetlb=1` alone does not help: blocks under 2 MiB are separate small mmaps, and with 700 KiB blocks the test showed 0% THP.
  - It also needs `glibc.malloc.mmap_threshold=33554432`, so that the rows come from the madvised heap.

| # | project | severity | summary |
|---|---|---|---|
| 7 | nissy-core (H48 solve) | nondeterminism (uninitialized read) | task `rank` is never initialized, so the depth >= 16 task sort reads stack garbage; node counts (and times) vary between identical 1-thread runs |
| 8 | nissy-core (python binding) | memory leak | `gendata()` copies its PyMem_Malloc buffer into a new bytearray and never frees it: 2x the table, and the buffer leaks |
| 9 | nissy-core (build.sh) | build pitfall | THREADS defaults to `nproc`, i.e. the CPUs of the build's affinity mask; a build under taskset/cpuset caps every later solve (and gendata) at that count, with only a log line |
| 10 | nissy-core (H48 gendata) | performance (minor) | the progress logger sleeps a fixed ~10%-of-expected-time interval and joins only afterwards: up to ~3.5 min of idle wait after h48h7's workers finished (8 cores) |

#### 7. nissy-core: uninitialized task rank, so node counts are nondeterministic
- **Where:** src/solvers/h48/solve.h:578 `solve_h48_task_t tasks[H48_STARTING_CUBES];` is a stack array. `solve_h48_maketasks` (:487-:492) sets cube, pval, moves and tmask, but not `rank`.
  - `rank` is written only after a task is searched (:443-446).
  - From depth 16 (`H48_SORT_TASKS_MIN_DEPTH`) the tasks are sorted by rank (:688). Tasks skipped so far (`pval + 4 > depth`, :419-421) still hold whatever was on the stack.
- **Repro:** the same h48h7 cube at `threads = 1`, in four runs (random_state #0): 38,739,453 / 38,726,886 / 38,725,684 / 38,726,886 nodes. Cube #1: 7,090,571 vs 7,090,825.
  - Solution lengths are unaffected: every depth is still searched completely before the next.
  - What changes is when the first solution is found inside the last depth. That changes nodes and time.
- **Fix:** `tasks[*ntasks].rank = 0;` in maketasks (or zero the array).

#### 8. nissy-core python binding: gendata leaks and doubles memory
- **Where:** python/nissy_module.c:247 `buf = PyMem_Malloc(size);` then :254 `return PyByteArray_FromStringAndSize((char *)buf, size);`. That call copies the buffer, and `buf` is never freed.
- **Impact:** 2x the table during generation, and the original copy stays leaked: about 15 GB for h48h8 and 121 GB for h48h11.
  - The harness's h48cmd.py generates through this path when the table file is missing.
- **Fix:** allocate the bytearray first (`PyByteArray_FromStringAndSize(NULL, size)`) and generate into `PyByteArray_AS_STRING`. Or free `buf` after the copy.
- **Workaround used here:** /data/agents/f1/tools/src/h48gen.c (one malloc, fwrite).

#### 9. nissy-core build.sh: thread maximum taken from the affinity mask
- **Where:** build.sh:44 `THREADS=${THREADS-...}`, :61-65 `detectthreads` -> `portable_nproc` -> `nproc` (:50). `nproc` honours `sched_getaffinity`.
  - The value becomes the compile-time `-DTHREADS`.
  - At solve time, src/nissy.c:349-353 clamps any larger request to it with only a LOG line.
- **Repro:** `taskset -c 0-7 ./build.sh python` gives THREADS=8. Then `nissy.solve(..., threads=48)` logs "[solve] Selected number of threads (48) is above the maximum value (8), using 8 threads instead".
  - With runtest.sh's `2>&1 | awk` that line is never seen. Our 48T smoke runs of h48h7 ran on 8 threads.
  - The final runs were rebuilt with THREADS=48, and init_h48.sh now passes `THREADS=$(nproc --all)`.
- **Fix:** `nproc --all` (or `getconf _NPROCESSORS_ONLN`), or a much larger default maximum: THREADS only sizes arrays.

#### 10. nissy-core gendata: progress logger delays the end
- **Where:** src/solvers/h48/gendata_h48.h:262-281.
  - The main thread sleeps `sleeptime = 100*(nshort - velocity)/velocity` ms between progress lines (about 10% of the expected total, estimated from the first second).
  - It reaches `wrapthread_join` (:287) only after its current sleep.
- **Observed:** h48h7 on 8 cores. The workers finished, and h48gen sat in `hrtimer_nanosleep` with 1 thread and 0% CPU for about 3.5 min before writing. Wall 21.5 min against 17 min of compute (8144 s of CPU).
- **Fix:** wait on the workers with a timed join or a condition variable, instead of a fixed sleep.

#### Watch list (no bug confirmed)
- **cubeopt30:** its table-generating run solved its one cube (random_state #1, 18 moves, 33.3M nodes) in 10.36 s at `-t 48`, which is 311 ns/node. That is far slower than its README class. It may be first-touch or compaction right after generation; the 200-cube phase-A run will tell.
  - Note: its cube numbers start at 1 ("Cube1").
- **nissy h48h8 generation:** at 48 threads it reached only ~15% after 5 min (3.5 CPU-h; ~21-24 CPU-h projected, ~30 min). h48h7 took 2.3 CPU-h. Base 10 (h8..h11) makes the depth-8 DFS ~10x more work.
  - Not a bug, but h48h9/h10/h11 generation will take >= 30 min each at 48 threads.

#### f1 update (2026-11-05): cubeopt30 watch item resolved
- The 10.36 s for one cube came from the run that had just generated the table. In the 200-cube 48T run (table loaded from file), the same cube (Cube1) took 166.7 ms, and the mean was 0.250 s/cube on 1 GiB pages.
- No bug. It looks like a one-off cost right after generation (THP compaction / first touch).

### shm1: vcube shared memory (found while porting `--shm`, 2026-11-05)

| # | project | severity | summary |
|---|---|---|---|
| 11 | vcube (`--shm`) | robustness (silent slow or stale table) | a `--shm` load that fails or is interrupted after creating the segment leaves it zero-filled or partly filled, and every later run attaches it with no check; the key names only the coordinate and base, so a regenerated table is also shadowed silently |

#### 11. vcube: unchecked SysV shared-memory segments
- **Where** (vcube 9f5bc2c, as read; not run here):
  - src/nxprune.cpp:130-162 `prune_base::loadShared(key, filename)`: `alloc::shared(sz, key, true)` creates the segment (IPC_CREAT|IPC_EXCL), then `fopen` (:144), `fread` (:149) and the size check (:154). On any failure it returns `false` and the created segment stays, zero-filled or partly filled. Ctrl-C during the `fread` of a 23 GiB table does the same.
  - src/vc-optimal.cpp:337 `if (!P.loadShared(shm_key))`: every run, with or without `--shm`, first attaches any segment under the key and uses it at once (`setPrune(mem)`, nxprune.cpp:159). There is no size, header or checksum check, and no "load complete" mark.
  - src/vc-optimal.cpp:84-87 (`S()`): the key is `0x76630000 | Base << 8 | (EP + 1) << 4 | (EO + 1) * 4`. It names the coordinate and base, not the table, so a table regenerated after a generator fix (same file name) is shadowed by the old segment until `ipcrm` or reboot.
  - src/alloc.cpp:77-89 `shared_impl`: the creating run keeps its read-write attachment for the search (no `SHM_RDONLY` re-attach).
- **Impact:** in a zero-filled segment every entry reads 0, so `fetch()` (nxprune.h:357-362) returns the stripe minimum 0 for every position. The bounds are still admissible, so solutions stay optimal, but the search is unpruned: in practice a hang for 17-20 move positions. A partly filled segment gives weaker bounds for the rest. Neither is reported.
  - vcube's README ("Shared memory") mentions removing a segment "which is corrupt because it was interrupted while loading", but nothing detects it. The user has to know to run `ipcrm`.
- **Fix:** write a small header after the data, last (the table's size and a checksum or generator version), and check it on every attach; remove the segment (IPC_RMID) when the load fails. egg-solver's `--shm` (shm1) writes the file's trailer last and compares it on every attach, and removes a segment whose loader died.

### Continuation review (2026-11-05)

The source mechanisms for #1–#3 and #5–#11 were checked again against the exact commits named above, using `git show` rather than the harness's modified working files. Source excerpts with full commit IDs are retained in `/data/agents/codex-resume/competitor-source-evidence.txt`. The performance numbers and the large-table numerical proof remain the original recorded measurements; this source review does not claim to have rerun those experiments. #4's harness invocation was rechecked; the binary option-switch evidence remains the original disassembly.

**Correction to #1's suggested fix:** changing `*next == capacity` to `*next > capacity` is insufficient and can loop forever: `h48map_nextkvpair` stops advancing at `capacity`. In this source the exhausted iterator returns `kv.key == MAP_KEYMASK`; check that sentinel under the mutex, and process every non-sentinel entry, including the last occupied slot. Regenerate the table after fixing the loop.

The distance-20 interference validation also requires serial nissy timings: h48h7 took 210.867 s/cube alone and 217.062 s/cube in the parallel block (+2.94%, above the agreed 2% limit). Parallel distance-20 timings are excluded from the final comparison and preserved as rejected evidence.

### Pinned source links

- [nissy-core source snapshot](https://github.com/sebastianotronto/nissy-core/tree/1f7e8e937d0e71e7eb3b5deab38b34d0a0159d88)
- [vcube source snapshot](https://github.com/Voltara/vcube/tree/9f5bc2cce18f29437879ace825f22917f6705378)
- [cube20src source snapshot](https://github.com/rokicki/cube20src/tree/f430921a48306874864b9f2ac8a059b43492fd3a)
- [cube_solver_test source snapshot](https://github.com/cs0x7f/cube_solver_test/tree/b9cca71cdf3f6e57bd3c7bfdc2491e5b806dda31)


## phack hardware-counter analysis

Archive: `egg-solver-pmu-phack-20261106T194245Z.tar.gz`.
SHA256: `e4367ce0595fffacadda8647a22024761a4be542e4aae1040a0514c587fa33a3`. Analysis is reproducible with
`/data/agents/codex-resume/analyze-phack.py`; checked aggregates are in
`/data/agents/codex-resume/phack-analysis/audit.json` and `counter-ratios.csv`.

All **242/242 collection runs succeeded**: **236 counted runs**, **621 counter rows**,
and **six sampled profiles**. Every raw stat value matches `runs.json` and every
counter reports **100% running**. All six profiles report **zero lost samples**.
The returned bench/gen/solve SHA256 hashes match the included build manifest.
These checks validate collection consistency; they do not establish that every
derived comparison has enough completed solves or that every CPU event is free of hardware errata.

### Scope and measurement quality

This is the dual-socket Xeon X5650, family 6/model 44, with 12 physical cores and
24 logical CPUs. The tested source is **935cb7fa369248161244507b7c13aa9c405f9ff1**, plus the included
profiling-control patch, compiled for Westmere. The table is **h6,
1,945,686,016 bytes (1.946 GB / 1.812 GiB)**. This is a separate host/build/table
dataset from the EPYC comparison matrix. It predates the rename and the current
Miri-driven safety fixes; those fixes cannot be credited as measured here.

Each run loads the table, warms for eight seconds, then enables counting through
acknowledged perf control FIFOs for approximately 20 seconds. Observed controlled
windows are 20.0001–20.0037 seconds. Table load/generation are excluded. Hardware
events count user execution. IPC and event/instruction ratios use counters from
the **same run and event group**. Different auxiliary events were measured in
separate windows, usually once; critical events have three repeats. Ratios across
different events are estimates, not simultaneous observations.

Two optional text exports failed: instruction annotations selected the wrong
hot symbol, and sample listings requested a CPU field absent from five profiles.
The raw profiles contain samples. All six annotations and sample listings were
recovered locally using the returned `symfs`; outputs and replay checks are in
`phack-analysis/reporting-replay/`. Focused cycle annotations of the actual
`run_lanes` symbol are also in `phack-analysis/cycles-{one,physical}.annotate.txt`.
No host rerun was needed. The collector's “valid” status describes measurement,
not successful generation of every optional postprocessing file.

### Cycle profile and counter evidence

The search loop accounts for **97.34% of one-thread cycle samples** and
**97.36% at 12 physical workers**. Ordinary cycle sampling can skid, so an individual
annotated instruction's percentage is not its exclusive cost. Both recovered
annotations concentrate on the same hot regions, with integer/vector unpacking,
stack traffic and loop control visible. This locates work; it does not prove an
optimization or its gain.

| Metric | One physical worker |12 physical workers | Interpretation |
|---|---:|---:|---|
| Baseline median IPC |1.762|1.767|Instructions per counted logical-thread cycle|
| Instruction length decoder stall event / cycles |32.08%|30.90%|Front-end signal; overlaps other stalls|
| Reservation station full event / cycles |20.82%|19.76%|Scheduling/dependency pressure signal|
| Any resource stall event / cycles |21.51%|20.53%|Includes overlapping resource conditions|
| Load resource stall event / cycles |0.26%|0.25%|Different event from all memory-related delay|
| DTLB page-walk event / cycles |6.43%|6.54%|Activity while page walks occur, not exclusive stall time|
| Branch misses per 1000 instructions |1.331|1.342|Median of three within-run ratios|
| Generic LLC-load misses per 1000 instructions |1.198|1.201|Request-level observation|
| Retired-load LLC misses per 1000 instructions |0.00102|0.00152|Retired-load event; not interchangeable with generic LLC misses|
| Offcore demand-read occupancy / cycles |0.592|0.650|Mean outstanding reads, averaged over all counted cycles|
| Offcore demand-read busy event / cycles |53.77%|58.61%|At least one such request outstanding|

Intel defines these model-specific events in its [Westmere catalogue](https://github.com/intel/perfmon/blob/main/WSM-EP-DP/events/WestmereEP-DP_core.json).
The stall events **must not be added** into a pie chart or interpreted as a
complete partition of execution time. These are older PMU events, not a modern
Top-Down analysis. The very large difference between generic request misses and
retired-load misses means the latter cannot stand for all memory traffic. This
dataset alone does not distinguish speculative requests, prefetch timing and
other event-definition or hardware effects behind that difference.

Dividing separately measured normalized outstanding-read occupancy by busy cycles
gives roughly 1.1 outstanding reads while busy. This is an approximate demand-read
concurrency metric; it excludes other traffic and is **not** total system MLP or
DRAM bandwidth. The interleaved 12-worker run shows remote DRAM responses near half
of the two captured local/remote response classes, consistent with placement.
No memory-controller bandwidth measurement was collected, so bandwidth saturation
is not established.

**Inference:** the strongest optimization leads on this Westmere build are the
search loop's instruction delivery and scheduling/dependency pressure, with page
translation also material. The measurements do not justify treating DRAM bandwidth
as the sole bottleneck or predicting gains on the EPYC build.

### Worker scaling and SMT

Each row is three 20-second measurements. Solves/s is the completion count divided
by the window; Ginst/s is counted instructions divided by the acknowledged window.
Ginst/s measures instruction execution, not useful search progress.

| Configuration | Median solves/s [range] | Median IPC | Median Ginst/s [range] |
|---|---:|---:|---:|
|1 worker, local memory|0.15 [0.15–0.15]|1.762|5.35 [5.33–5.36]|
|6 physical workers, one socket|1.05 [0.85–1.05]|1.790|22.50 [21.24–22.75]|
|12 physical workers, interleaved memory|1.55 [1.55–1.60]|1.767|52.07 [51.66–52.17]|
|24 SMT workers, interleaved memory|1.40 [1.30–1.60]|1.022|55.95 [55.94–56.25]|

The separate thread sweep observed 1.60 solves/s at 12 workers and 1.20 at 24 workers.
The baseline observed 1.55 and 1.40 respectively. SMT increases counted instruction
execution modestly but does not improve completed-solve throughput in these
windows. Aggregate SMT IPC uses the sum of both logical threads' cycles; it
cannot be read as physical-core IPC.

Use 12 physical workers as the candidate for a longer matched validation, not as
a proven universal setting. One-socket scaling also varies: counted cycles per
worker per wall second are lower at 6 workers than at 1 or 12. The archive does not
include per-window CPU temperature/frequency telemetry or enough controls to
identify the cause. Do not attribute this confidently to NUMA, throttling or
software contention.

### Pages and NUMA placement

Actual large table mapping coverage from the start-of-window `smaps` snapshots:

| Family | Table bytes backed by THP |
|---|---:|
|baseline-one|97.55–99.92%|
|baseline-physical|99.92–99.92%|
|pages-4k-one|0.00–0.00%|
|pages-4k-physical|0.00–0.00%|
|numa-node1|99.92–99.92%|
|numa-node0|89.37–90.55%|

The 4 KiB intervention really disabled THP for the table. Median IPC changed from
1.762 to 1.610 at 1 worker and
from 1.767 to 1.617 at 12 workers.
That is an instruction-execution signal in favor of THP for this workload, but
not a measured percentage improvement in completed solves: one-thread solves/s
remained quantized at 0.15, and 12-worker completion rates overlap.

CPU 1 belongs to node 1 on this host. The local test put the table on node 1; the
remote test put it on node 0. The latter also had only 89–91% THP coverage versus
99.92% locally, so it changes two conditions. Both completed 0.15 solves/s and IPC
was 1.760 local versus 1.743 remote. This cannot establish a clean NUMA latency
penalty. The 12-worker baseline table was split approximately equally between
the nodes, as requested. No 1 GiB hugetlb measurements exist in this archive:
the optional intervention was skipped because suitable existing pages were
unavailable. No kernel huge-page pool was changed.

Linux documents `AnonHugePages` as the amount of anonymous memory backed by THP
in [its THP guide](https://docs.kernel.org/admin-guide/mm/transhuge.html).
These are placement snapshots, not continuous residency measurements.

### Lane sweep: exploratory only

| Lanes | Completed cubes in each 20-second run | Observed median solves/s | Median IPC |
|---:|---|---:|---:|
|0|0, 0, 0|0.00|2.072|
|1|0, 0, 0|0.00|1.352|
|2|1, 1, 1|0.05|1.945|
|4|1, 1, 1|0.05|1.956|
|8|5, 5, 5|0.25|1.945|
|12|5, 5, 5|0.25|1.916|
|16|4, 4, 4|0.20|1.867|
|24|3, 3, 3|0.15|1.763|
|32|2, 2, 2|0.10|1.679|
|40|4, 4, 4|0.20|1.630|
|48|4, 4, 4|0.20|1.609|
|64|4, 4, 4|0.20|1.595|

Eight and 12 lanes show the highest observed completion counts, but there are
only 0–5 completions per window. Zero completions does not mean zero work: those
runs have billions of counted instructions and unfinished searches. Lane count
also changes the instruction mix. Neither IPC nor Ginst/s can replace an exact
completed-work denominator here. Warm-up and end-of-window in-flight work can
change which cubes finish. **Do not claim a 67% speedup or change the default lane
count from this sweep.** A longer, interleaved, same-cube validation is required
before adopting a performance change.

The `share=1` runs use cooperative latency semantics and are not a directly
equivalent throughput comparison with independent stream workers. Their counters
are retained in the raw dataset; no cross-mode speedup is claimed.

### Precise sampling and useful next experiments

All DWARF cycle, LBR, branch PEBS and load PEBS captures succeeded. The two load
profiles are sparse: 55 retired-LLC-miss samples and 41 latency samples above 128 cycles.
The latter have observed weights 129–534 cycles, median 193. This is a threshold-
selected tail with only 41 samples, not the mean or distribution of all loads.
The 55-sample LLC profile is dominated by setup paths and cannot establish their
share of total cache-miss cost accurately.

The existing data supports investigating hot-loop instruction footprint, unpacking
and register/stack pressure first, while retaining THP. It supports testing 8/12
lanes and 12 physical workers with longer, matched solves. Changes still need
correctness/Miri checks and measured 1T and full-machine gains before adoption.
No performance code was changed on the strength of this archive. A complete
repeat of all 242 runs is unnecessary to recover annotations or to retain these
counter findings; any follow-up should target the unresolved throughput ranking
and, if needed, a NUMA test with matched page coverage.

### Full auxiliary counter ratios

The table below contains all 56 auxiliary events. Entries are median within-run
event counts per 1000 instructions; this normalization is only a numerical rate.
Software `task-clock` retains milliseconds as its original unit. Counts of cycles,
bytes and instructions are not interchangeable. The accompanying CSV also contains
event/cycle ratios and repeat counts. Each configuration has the listed number
of independent windows.

| Event | Repeats each |1 worker /1000 insn |12 workers /1000 insn |
|---|---:|---:|---:|
|`branches:u`|1|43.287750|43.414955|
|`branch-misses:u`|3|1.331145|1.341987|
|`cache-references:u`|1|2.905948|2.950581|
|`cache-misses:u`|1|1.198571|1.203458|
|`ref-cycles:u`|1|493.950659|611.331675|
|`L1-dcache-loads:u`|1|250.916012|251.220429|
|`L1-dcache-load-misses:u`|1|14.002615|13.917177|
|`LLC-loads:u`|1|2.851037|2.873600|
|`LLC-load-misses:u`|1|1.197693|1.201234|
|`dTLB-loads:u`|1|250.918788|251.225814|
|`dTLB-load-misses:u`|1|2.169763|2.168326|
|`task-clock`|1|0.000187|0.000233|
|`context-switches`|1|0.000000|0.000000|
|`cpu-migrations`|1|0.000000|0.000000|
|`page-faults`|1|0.000043|0.000055|
|`UOPS_EXECUTED.PORT0`|1|271.465967|271.659104|
|`UOPS_EXECUTED.PORT1`|1|284.407529|284.140108|
|`UOPS_EXECUTED.PORT5`|1|312.457111|311.760828|
|`L1I.MISSES`|1|0.799519|1.188991|
|`ARITH.CYCLES_DIV_BUSY`|1|0.005003|0.072243|
|`SSEX_UOPS_RETIRED.VECTOR_INTEGER`|1|253.328904|253.157948|
|`MEM_INST_RETIRED.LOADS`|1|250.917139|251.233587|
|`MEM_INST_RETIRED.STORES`|1|81.185833|81.347010|
|`MEM_LOAD_RETIRED.L1D_HIT`|1|237.451658|237.818922|
|`MEM_LOAD_RETIRED.L2_HIT`|1|6.155979|6.183236|
|`MEM_LOAD_RETIRED.LLC_UNSHARED_HIT`|1|1.438021|1.464296|
|`MEM_LOAD_RETIRED.LLC_MISS`|3|0.001023|0.001524|
|`MEM_LOAD_RETIRED.HIT_LFB`|1|5.625567|5.666890|
|`MEM_LOAD_RETIRED.DTLB_MISS`|1|2.162382|2.156685|
|`MEM_LOAD_RETIRED.OTHER_CORE_L2_HIT_HITM`|1|0.000000|0.000001|
|`L1D.REPL`|1|13.901934|13.910860|
|`L2_RQSTS.LD_HIT`|1|10.370175|10.395427|
|`L2_RQSTS.LD_MISS`|1|2.834606|2.878541|
|`L2_RQSTS.PREFETCH_HIT`|1|0.184530|0.218566|
|`L2_RQSTS.PREFETCH_MISS`|1|0.060993|0.073986|
|`LOAD_HIT_PRE`|1|0.016838|0.015475|
|`DTLB_LOAD_MISSES.ANY`|1|2.169292|2.170434|
|`DTLB_LOAD_MISSES.STLB_HIT`|1|0.016882|0.138268|
|`DTLB_LOAD_MISSES.WALK_COMPLETED`|1|2.167479|2.163726|
|`DTLB_LOAD_MISSES.WALK_CYCLES`|3|36.587808|37.001671|
|`ITLB_MISSES.ANY`|1|0.000102|0.000530|
|`ITLB_MISSES.WALK_CYCLES`|3|0.006612|0.011572|
|`OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA`|3|335.758720|368.080178|
|`OFFCORE_REQUESTS_OUTSTANDING.DEMAND.READ_DATA_NOT_EMPTY`|3|304.814752|331.532392|
|`RESOURCE_STALLS.ANY`|1|121.979431|116.148142|
|`RESOURCE_STALLS.LOAD`|3|1.474936|1.431617|
|`RESOURCE_STALLS.STORE`|1|0.351922|0.443178|
|`RESOURCE_STALLS.ROB_FULL`|1|2.507491|3.268369|
|`RESOURCE_STALLS.RS_FULL`|1|118.169838|111.995745|
|`UOPS_ISSUED.ANY`|1|1186.148549|1185.964810|
|`UOPS_RETIRED.ANY`|1|1221.435167|1221.344321|
|`UOPS_RETIRED.RETIRE_SLOTS`|1|1120.801986|1120.554166|
|`ILD_STALL.ANY`|1|182.341239|174.828068|
|`ILD_STALL.LCP`|1|0.214666|0.214995|
|`OFFCORE_RESPONSE.DEMAND_DATA_RD.ALL_LOCAL_DRAM_AND_REMOTE_CACHE_HIT`|1|1.197336|0.605355|
|`OFFCORE_RESPONSE.DEMAND_DATA_RD.REMOTE_DRAM`|1|0.000156|0.595690|


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

<a id="competitor-bug-findings"></a>

## Competitor bug findings

## Bugs and issues found in other solvers (for the final report; the user will report them upstream)

Source commits read: nissy-core 1f7e8e9 (2026-09-17), vcube 9f5bc2c (2024-04-18), cube20src/nxopt f430921 (2025-07-18), cube_solver_test b9cca71 (2026-02-01); cubeopt30/cube48opt10 = the prebuilt binaries in ~/ (no source).

Each entry: project, file:line at the commit we read, what is wrong, a repro or proof, impact, and a suggested fix.

| # | project | severity | summary |
|---|---|---|---|
| 1 | nissy-core (H48) | correctness (admissibility) | h11 generator skips the last hash-map slot: 11,546 h11 entries are one level too high |
| 2 | cube_solver_test (h48cmd.py) | harness | per-cube lines are mis-paired (stdout is buffered, nissy logs to stderr); the Avg line is unaffected |
| 3 | cube_solver_test (h48cmd.py) | harness / memory | `bytearray(f.read())` holds the table twice (2x RAM); h48h8 does not fit in ~16 GiB free |
| 4 | cube_solver_test (runtest.sh) | harness | passes `-g $ngroup` to cubeopt/cube48opt, but the published binaries accept only `-l -m -n -t -v` (no `-g`); the option is ignored |
| 5 | vcube | performance | when MAP_HUGETLB (1G, then 2M) fails, falls back to a plain mmap with no madvise(MADV_HUGEPAGE): 4K pages, ~10% slower on THP=madvise systems |
| 6 | nxopt | performance | the table is one malloc per corner row with no MADV_HUGEPAGE: 4K pages on THP=madvise systems, ~11% slower |

### 1. nissy-core: h11 table slightly inadmissible (generator skips the last hash slot)

- **Where:** h48/gendata_h48.h:307-316 (`gendata_h48_runthread`) with h48/map.h:68-87 (`h48map_nextkvpair` post-increments the slot index).
  - Commit: nissy-core 1f7e8e9.
- **What:** the loop `kv = nextkvpair(map, next); if (*next == capacity) break;` breaks before processing the entry in the last slot (`capacity - 1`).
  - With the fixed capacity 10000019 and nissy's hash, that slot holds the depth-8 h11 entry 189880194105 (class 2675, no self-symmetry). Its 4-level DFS never runs: the loop visits 7,437,854 of 7,437,855 short entries.
  - The same loop shape in `gendata_h48short` (gendata_h48.h:62-65) is harmless, because that entry's value is 8 (the last BFS level).
- **Proof:**
  - nissy's own depth-≤8 BFS equals ours per depth (1/1/4/34/333/3815/45382/548562/6839723).
  - Modelling nissy's values in the 49,206 entries within 4 moves of the skipped root gives ours − nissy = [+159, +1689, +7691, −9539] per 2-bit code. That exactly matches the difference between our exact h11 distribution [1150763320, 12045847349, 91163441021, 136418085910] and nissy's checkdata.h.
  - Scripts: /data/agents/x-h48/drv/{short,lastslot,skipped}.c.
- **Impact:** 11,546 entries are over-estimated by one level (159 true ≤10 coded 1, 1848 true 11 coded 2, 9539 true 12 coded 3). The table is therefore not admissible: a solve whose optimal path passes through one of these positions near the end could return a non-optimal solution. The probability is tiny on random cubes but nonzero.
- **Fix:** after `h48map_nextkvpair`, test `kv.key == MAP_KEYMASK` while holding the mutex. That sentinel means the iterator is exhausted; every non-sentinel entry, including the last occupied slot, must be processed. Regenerate h11 and update checkdata.h.
- **To check in the final comparison:** whether h9/h10 (other k) are affected; the length cross-check on h48h11 runs.

### 2–4. cube_solver_test harness issues (found by f1 while extending it)

- **2. h48cmd.py output pairing.** Python's stdout is block-buffered, while nissy prints its stats to stderr, so runtest.sh's awk pairs "Nodes visited" and "Time" lines from different cubes. Per-cube lines are wrong; totals and Avg are right. Fix: `flush=True` / `sys.stdout.flush()` after each solve. Patch in /data/agents/f1/cube_solver_test-cubeoptrs.patch.
- **3. h48cmd.py double table copy.** `bytearray(f.read())` keeps the bytes object and the bytearray (2x the table) alive, so h48h8+ needs twice the RAM. Fix: `buf = bytearray(size); f.readinto(buf)`. Patch: same file.
- **4. runtest.sh `-g`.** cubeopt()/cube48opt() pass `-g $ngroup`, but the published cubeopt30/cube48opt10 binaries parse only `-l -m -n -t -v` (from their disassembly), so ngroup has no effect. Either the binaries or the script are out of date.

### 5–6. Huge-page handling (performance, not correctness)

- **5. vcube** (src/alloc.cpp): tries MAP_HUGETLB 1G, then 2M (both fail when the hugetlb pool has no free pages), then falls back to a plain mmap with no madvise. On THP=madvise systems the table ends up on 4K pages. Measured with an LD_PRELOAD madvise shim: 1T −10%, 48T −9% sec/solve.
  - Fix: `madvise(p, len, MADV_HUGEPAGE)` on the fallback mapping.
- **6. nxopt** (cube20src nxopt.w): one malloc per corner row (114 × 17.5 MiB + smaller), no madvise, so 4K pages on THP=madvise. With glibc.malloc.hugetlb=1 + mmap_threshold=32 MiB: 1T −11%, 48T −11%.
  - Fix: one aligned allocation for the whole table + MADV_HUGEPAGE.

### f1: exact locations for #2-#6, and new findings #7-#10 (f1, 2026-11-04)

Exact locations for the entries above (code as read; harness = ~/ref/cube_solver_test b9cca71):
- **#2:** h48cmd.py:31 `print (scramble)` and :34 `print('Time: ...')` go to Python's stdout, which is block-buffered when piped. nissy's stats go to stderr: python/nissy_module.c:447-449 `log_stdout()` (misnamed) does `fprintf(stderr, ...)` and is installed at :455. runtest.sh:85 merges them with `2>&1`.
  - Mechanism, reproduced: `python3 -c 'import sys; [ (sys.stderr.write("Nodes visited: %d\n" % i), print("Time: %d" % i)) for i in range(2)]' 2>&1 | cat` prints both "Nodes" lines before both "Time" lines. runtest.sh's awk (h48(), runtest.sh:86-97) then pairs every Time with the last cube's nodes.
  - Fix in our copy: `sys.stdout.flush()` after each cube.
- **#3:** h48cmd.py:17 `solver_data = bytearray(f.read())`.
- **#4:** runtest.sh:124 `./cube$3$2 -t $nthread -g $ngroup -m $maxl -`.
  - Binaries: the option switch in main does `sub $0x6c,%eax; cmp $0xa,%al` (cube48opt10 at 0x2a79, cubeopt30 at 0x2a99), then a jump table (0x183f4 / 0x193f4). That table has cases only for l, m, n, t and v: `-l` scramble length (default 15), `-m` max depth (20), `-n` count (100), `-t` threads, `-v` debug.
  - `-g` is skipped; its value "1" briefly becomes the mode character, and the final "-" sets the stdin mode again. So `-g` is harmless but has no effect.
- **#5:** vcube src/alloc.cpp:45-57 (`alloc::huge_impl`: `map_huge(n, 30)` -> `map_huge(n, 21)` -> `map_huge(n, 0)`; `map_huge` at :36-43 adds no madvise). The SysV-shm path is at :77-89 (the `-S` option).
  - strace: `mmap(3072 MiB, MAP_HUGETLB) = -1`, `mmap(2484 MiB, MAP_HUGETLB) = -1`, then a plain 2482 MiB mmap.
  - 200-cube numbers: /data/agents/f1/final/results.md (phase A).
- **#6:** cube20src nxopt.w:749 `firstlevs[j].p = (unsigned int *)malloc(MEMSIZE * (size_t)siz);` (nxopt.cpp:837 after ctangle). It allocates one block per distinct corner row.
  - glibc serves each block with its own mmap: strace shows 114 mmaps of 17.5 MiB plus smaller ones, and no madvise.
  - `glibc.malloc.hugetlb=1` alone does not help: blocks under 2 MiB are separate small mmaps, and with 700 KiB blocks the test showed 0% THP.
  - It also needs `glibc.malloc.mmap_threshold=33554432`, so that the rows come from the madvised heap.

| # | project | severity | summary |
|---|---|---|---|
| 7 | nissy-core (H48 solve) | nondeterminism (uninitialized read) | task `rank` is never initialized, so the depth >= 16 task sort reads stack garbage; node counts (and times) vary between identical 1-thread runs |
| 8 | nissy-core (python binding) | memory leak | `gendata()` copies its PyMem_Malloc buffer into a new bytearray and never frees it: 2x the table, and the buffer leaks |
| 9 | nissy-core (build.sh) | build pitfall | THREADS defaults to `nproc`, i.e. the CPUs of the build's affinity mask; a build under taskset/cpuset caps every later solve (and gendata) at that count, with only a log line |
| 10 | nissy-core (H48 gendata) | performance (minor) | the progress logger sleeps a fixed ~10%-of-expected-time interval and joins only afterwards: up to ~3.5 min of idle wait after h48h7's workers finished (8 cores) |

#### 7. nissy-core: uninitialized task rank, so node counts are nondeterministic
- **Where:** src/solvers/h48/solve.h:578 `solve_h48_task_t tasks[H48_STARTING_CUBES];` is a stack array. `solve_h48_maketasks` (:487-:492) sets cube, pval, moves and tmask, but not `rank`.
  - `rank` is written only after a task is searched (:443-446).
  - From depth 16 (`H48_SORT_TASKS_MIN_DEPTH`) the tasks are sorted by rank (:688). Tasks skipped so far (`pval + 4 > depth`, :419-421) still hold whatever was on the stack.
- **Repro:** the same h48h7 cube at `threads = 1`, in four runs (random_state #0): 38,739,453 / 38,726,886 / 38,725,684 / 38,726,886 nodes. Cube #1: 7,090,571 vs 7,090,825.
  - Solution lengths are unaffected: every depth is still searched completely before the next.
  - What changes is when the first solution is found inside the last depth. That changes nodes and time.
- **Fix:** `tasks[*ntasks].rank = 0;` in maketasks (or zero the array).

#### 8. nissy-core python binding: gendata leaks and doubles memory
- **Where:** python/nissy_module.c:247 `buf = PyMem_Malloc(size);` then :254 `return PyByteArray_FromStringAndSize((char *)buf, size);`. That call copies the buffer, and `buf` is never freed.
- **Impact:** 2x the table during generation, and the original copy stays leaked: about 15 GB for h48h8 and 121 GB for h48h11.
  - The harness's h48cmd.py generates through this path when the table file is missing.
- **Fix:** allocate the bytearray first (`PyByteArray_FromStringAndSize(NULL, size)`) and generate into `PyByteArray_AS_STRING`. Or free `buf` after the copy.
- **Workaround used here:** /data/agents/f1/tools/src/h48gen.c (one malloc, fwrite).

#### 9. nissy-core build.sh: thread maximum taken from the affinity mask
- **Where:** build.sh:44 `THREADS=${THREADS-...}`, :61-65 `detectthreads` -> `portable_nproc` -> `nproc` (:50). `nproc` honours `sched_getaffinity`.
  - The value becomes the compile-time `-DTHREADS`.
  - At solve time, src/nissy.c:349-353 clamps any larger request to it with only a LOG line.
- **Repro:** `taskset -c 0-7 ./build.sh python` gives THREADS=8. Then `nissy.solve(..., threads=48)` logs "[solve] Selected number of threads (48) is above the maximum value (8), using 8 threads instead".
  - With runtest.sh's `2>&1 | awk` that line is never seen. Our 48T smoke runs of h48h7 ran on 8 threads.
  - The final runs were rebuilt with THREADS=48, and init_h48.sh now passes `THREADS=$(nproc --all)`.
- **Fix:** `nproc --all` (or `getconf _NPROCESSORS_ONLN`), or a much larger default maximum: THREADS only sizes arrays.

#### 10. nissy-core gendata: progress logger delays the end
- **Where:** src/solvers/h48/gendata_h48.h:262-281.
  - The main thread sleeps `sleeptime = 100*(nshort - velocity)/velocity` ms between progress lines (about 10% of the expected total, estimated from the first second).
  - It reaches `wrapthread_join` (:287) only after its current sleep.
- **Observed:** h48h7 on 8 cores. The workers finished, and h48gen sat in `hrtimer_nanosleep` with 1 thread and 0% CPU for about 3.5 min before writing. Wall 21.5 min against 17 min of compute (8144 s of CPU).
- **Fix:** wait on the workers with a timed join or a condition variable, instead of a fixed sleep.

#### Watch list (no bug confirmed)
- **cubeopt30:** its table-generating run solved its one cube (random_state #1, 18 moves, 33.3M nodes) in 10.36 s at `-t 48`, which is 311 ns/node. That is far slower than its README class. It may be first-touch or compaction right after generation; the 200-cube phase-A run will tell.
  - Note: its cube numbers start at 1 ("Cube1").
- **nissy h48h8 generation:** at 48 threads it reached only ~15% after 5 min (3.5 CPU-h; ~21-24 CPU-h projected, ~30 min). h48h7 took 2.3 CPU-h. Base 10 (h8..h11) makes the depth-8 DFS ~10x more work.
  - Not a bug, but h48h9/h10/h11 generation will take >= 30 min each at 48 threads.

#### f1 update (2026-11-05): cubeopt30 watch item resolved
- The 10.36 s for one cube came from the run that had just generated the table. In the 200-cube 48T run (table loaded from file), the same cube (Cube1) took 166.7 ms, and the mean was 0.250 s/cube on 1 GiB pages.
- No bug. It looks like a one-off cost right after generation (THP compaction / first touch).

### shm1: vcube shared memory (found while porting `--shm`, 2026-11-05)

| # | project | severity | summary |
|---|---|---|---|
| 11 | vcube (`--shm`) | robustness (silent slow or stale table) | a `--shm` load that fails or is interrupted after creating the segment leaves it zero-filled or partly filled, and every later run attaches it with no check; the key names only the coordinate and base, so a regenerated table is also shadowed silently |

#### 11. vcube: unchecked SysV shared-memory segments
- **Where** (vcube 9f5bc2c, as read; not run here):
  - src/nxprune.cpp:130-162 `prune_base::loadShared(key, filename)`: `alloc::shared(sz, key, true)` creates the segment (IPC_CREAT|IPC_EXCL), then `fopen` (:144), `fread` (:149) and the size check (:154). On any failure it returns `false` and the created segment stays, zero-filled or partly filled. Ctrl-C during the `fread` of a 23 GiB table does the same.
  - src/vc-optimal.cpp:337 `if (!P.loadShared(shm_key))`: every run, with or without `--shm`, first attaches any segment under the key and uses it at once (`setPrune(mem)`, nxprune.cpp:159). There is no size, header or checksum check, and no "load complete" mark.
  - src/vc-optimal.cpp:84-87 (`S()`): the key is `0x76630000 | Base << 8 | (EP + 1) << 4 | (EO + 1) * 4`. It names the coordinate and base, not the table, so a table regenerated after a generator fix (same file name) is shadowed by the old segment until `ipcrm` or reboot.
  - src/alloc.cpp:77-89 `shared_impl`: the creating run keeps its read-write attachment for the search (no `SHM_RDONLY` re-attach).
- **Impact:** in a zero-filled segment every entry reads 0, so `fetch()` (nxprune.h:357-362) returns the stripe minimum 0 for every position. The bounds are still admissible, so solutions stay optimal, but the search is unpruned: in practice a hang for 17-20 move positions. A partly filled segment gives weaker bounds for the rest. Neither is reported.
  - vcube's README ("Shared memory") mentions removing a segment "which is corrupt because it was interrupted while loading", but nothing detects it. The user has to know to run `ipcrm`.
- **Fix:** write a small header after the data, last (the table's size and a checksum or generator version), and check it on every attach; remove the segment (IPC_RMID) when the load fails. egg-solver's `--shm` (shm1) writes the file's trailer last and compares it on every attach, and removes a segment whose loader died.

### Continuation review (2026-11-05)

The source mechanisms for #1–#3 and #5–#11 were checked again against the exact commits named above, using `git show` rather than the harness's modified working files. Source excerpts with full commit IDs are retained in `/data/agents/codex-resume/competitor-source-evidence.txt`. The performance numbers and the large-table numerical proof remain the original recorded measurements; this source review does not claim to have rerun those experiments. #4's harness invocation was rechecked; the binary option-switch evidence remains the original disassembly.

**Correction to #1's suggested fix:** changing `*next == capacity` to `*next > capacity` is insufficient and can loop forever: `h48map_nextkvpair` stops advancing at `capacity`. In this source the exhausted iterator returns `kv.key == MAP_KEYMASK`; check that sentinel under the mutex, and process every non-sentinel entry, including the last occupied slot. Regenerate the table after fixing the loop.

The distance-20 interference validation also requires serial nissy timings: h48h7 took 210.867 s/cube alone and 217.062 s/cube in the parallel block (+2.94%, above the agreed 2% limit). Parallel distance-20 timings are excluded from the final comparison and preserved as rejected evidence.

### Pinned source links

- [nissy-core source snapshot](https://github.com/sebastianotronto/nissy-core/tree/1f7e8e937d0e71e7eb3b5deab38b34d0a0159d88)
- [vcube source snapshot](https://github.com/Voltara/vcube/tree/9f5bc2cce18f29437879ace825f22917f6705378)
- [cube20src source snapshot](https://github.com/rokicki/cube20src/tree/f430921a48306874864b9f2ac8a059b43492fd3a)
- [cube_solver_test source snapshot](https://github.com/cs0x7f/cube_solver_test/tree/b9cca71cdf3f6e57bd3c7bfdc2491e5b806dda31)

<a id="no-panic-source-inventory"></a>

## No-panic source inventory

## No-panic source inventory

Before: `0b5b5a6`. After: `935cb7f`. The same lexer used by the project enforcement test
scanned all shipped Rust files, including alternative backends, binaries, build scripts and WASM.
Comments, strings, tests and compile-time initializers are excluded. Existing `debug_assert!`
is allowed by the standing policy. Raw findings: `panic-source-audit.tsv`.

Explicit calls matching the enforced policy: **127 before → 0 after**.

| file | explicit before | after | `/` before | after | `%` before | after |
|---|---:|---:|---:|---:|---:|---:|
| benches/kernels.rs | 0 | 0 | 0 | 0 | 3 | 3 |
| build.rs | 25 | 0 | 0 | 0 | 0 | 0 |
| src/bin/bench.rs | 4 | 0 | 8 | 8 | 0 | 0 |
| src/bin/gen.rs | 3 | 0 | 3 | 3 | 0 | 0 |
| src/bin/solve.rs | 4 | 0 | 5 | 5 | 2 | 2 |
| src/cli.rs | 0 | 0 | 2 | 2 | 0 | 0 |
| src/cli/preflight.rs | 0 | 0 | 0 | 2 | 0 | 0 |
| src/coord.rs | 0 | 0 | 2 | 2 | 1 | 1 |
| src/cubie.rs | 15 | 0 | 6 | 8 | 19 | 15 |
| src/gen.rs | 1 | 0 | 4 | 4 | 0 | 0 |
| src/gen/collapse.rs | 6 | 0 | 0 | 0 | 0 | 0 |
| src/gen/dfs.rs | 8 | 0 | 7 | 5 | 2 | 2 |
| src/hot.rs | 2 | 0 | 0 | 0 | 0 | 0 |
| src/lanes.rs | 1 | 0 | 7 | 7 | 7 | 7 |
| src/lanes/share.rs | 12 | 0 | 19 | 19 | 5 | 5 |
| src/lib.rs | 7 | 0 | 0 | 0 | 0 | 0 |
| src/parallel.rs | 2 | 0 | 0 | 0 | 0 | 0 |
| src/portable/mod.rs | 2 | 0 | 0 | 0 | 0 | 0 |
| src/simd.rs | 2 | 0 | 1 | 1 | 1 | 1 |
| src/solver.rs | 0 | 0 | 9 | 9 | 2 | 2 |
| src/symstart.rs | 3 | 0 | 4 | 4 | 0 | 0 |
| src/tables.rs | 17 | 0 | 3 | 3 | 1 | 1 |
| src/tables/file.rs | 10 | 0 | 0 | 0 | 0 | 0 |
| src/tables/shm.rs | 0 | 0 | 0 | 1 | 0 | 0 |
| wasm/src/lib.rs | 2 | 0 | 2 | 2 | 0 | 0 |
| wasm/src/threads.rs | 1 | 0 | 0 | 0 | 0 | 0 |

Division and remainder are **source syntax counts**, not reachable panic counts. They include
floating-point division and constant nonzero divisors. Runtime indexing, slicing and arithmetic
diagnostics are recorded separately in `no-panic-implicit-inventory.md`; optimization can remove
proven checks. These inventories do not claim to prove the absence of every implicit or dependency
panic. Explicit intentional calls are enforced by check.sh across all installed backends.

### 1. Explicit panic sites in non-test code on 0b5b5a6, and their fixes

Heat: **hot** = per node / per lookup (the stream loop `bench::stream_thread`, share's `Worker::rounds`),
**warm** = per run / job / iteration / solution (job extension, iteration builds, `settle`, `found`, `fresh`),
**cold** = startup, table load and generation, parsing, the binaries' I/O, build.rs, wasm glue.
Fix kinds: (a) impossible by construction, (b) error returned, (c) handled without panicking,
(d) documented proven-unreachable unchecked path. Line numbers are 0b5b5a6's.

| file:line | kind | heat | fix |
|---|---|---|---|
| src/simd.rs:305, 337; src/portable/mod.rs:486, 521 | `assert!(mv < 18)` in `Cube::apply_move` / `premove_inverse` | warm (job extension, reference solver), cold (tables, gen) | (a) they take `cubie::Move`, an enum of the 18 moves; indexing the 18-entry tables needs no check (`apply_move_unchecked(mv.index())`, SAFETY: a `Move` is below 18). Mask loops use `Move::lowest(mask)` (the same test as `mask != 0`) |
| src/lanes.rs:740 | `assert!(target < MAXD)` in `Lane::start_iteration` | warm (per iteration) | (c) `start_iteration` returns false when the target would pass `solver::MAX_TARGET` (20, God's number): the lane gives the cube up and reports `None` (new `Step::Unsolved`, `Start::Unsolved`); `const _: () = assert!(MAX_TARGET < MAXD)` makes the stack bound a compile-time fact. Fires only for an unsolvable cube or a table whose bounds are too high (it used to panic there) |
| src/lanes/share.rs:1186 | `assert!(target < MAXD)` in `build` | warm (per iteration build) | (c) `settle` gives the cube up (`give_up`, result `None`) instead of building past `MAX_TARGET`; the first pass only jumps to a pass target ≤ `MAX_TARGET`; `build` clamps `inner.target.min(MAX_TARGET)` (never lowers a target except a first bound above 20, whose iteration finds nothing) |
| src/lanes/share.rs:1254 | `assert!(offset <= out.capacity())` | warm | (a) `out.reserve(offset)` after `clear()`: capacity by construction (a no-op: the buffer is pre-sized) |
| src/lanes/share.rs:1741, 1745, 1756 | `assert!(n <= capacity)` (keys, key_buckets, out, rest_children, runs) | warm | (a) `reserve(n)` / `reserve(2 * n)` after `clear()` |
| src/lanes/share.rs:2081 | `assert_eq!(scratch.jobs.len(), children)` before a raw copy | warm | (d) `debug_assert_eq!` + the copy reads `min(len, children)` jobs; proof in a comment: `kept_children` counts exactly what `extend_jobs` keeps (same `keep` filter on the same canonical moves; dedup filters never extend in parallel) |
| src/lanes/share.rs:2102 | `child_offsets.last().unwrap()` | warm | (c) `map_or(0, ..)` (never empty: `build` pushes the total) |
| src/lanes/share.rs:2518, 2576 | `locals[slot].as_ref().unwrap()` in `found`, `report` | warm (per solution) | (c) `let Some(Some(local)) = locals.get(slot) else { return }` (always joined: a lane with a job holds its cube's slot, as `end_job` relies on) |
| src/lanes/share.rs:3027 | `locals[slot].take().unwrap()` in `release` | warm | (c) let-else: nothing to release is a no-op |
| src/lanes/share.rs:3170 | `self.local(&ctx).unwrap()` in `fresh` | warm (per cube) | (a) new `add_local` joins a cube whose part the thread already holds (fresh: the unsolved part), infallibly; `local` uses it after its CAS |
| src/lanes/share.rs:3333 | `unreachable!` after `get_disjoint_mut` in `split` | warm | (c) `else { return false }`: no split (find_split excludes lane `li`) |
| src/lanes/share.rs:1034 | `scope.spawn` (panics if a thread cannot start) in `solve_all` | cold | (b) `Builder::spawn_scoped`; on failure the started threads are stopped and joined, `solve_all` returns the io error; join errors too |
| src/cubie.rs:255, 263, 264 | `try_from(..).expect` in `Rng::below_u8/below_usize` | cold (bench/random cubes) | (a) `as` casts of values below `n` (with `#[expect(cast_possible_truncation, reason)]`), `const _: () = assert!(usize::BITS <= u64::BITS)`; same values, verify unchanged |
| src/cubie.rs:338, 339, 522, 533, 591, 657, 790, 804 | `expect`/`unwrap`/`unreachable!` in the sticker model (sticker search, face letters, facelets, symmetries) | cold (startup derivation, facelet parsing) | (a) build.rs derives moves, symmetries, inverses and facelet tables once (`cubie::model`, every step returns `None`; `build.rs` fails with a message) and writes `$OUT_DIR/cubie_consts.rs`; the library includes it (`cfg(cubie_generated)`) and derives nothing at run time. `from_facelets`/`to_facelets` use the tables (no search, no fallthrough) |
| src/cubie.rs:610, 611, 622, 623 | `u8::try_from(piece/twist).expect` in `from_facelets` | cold | (a) the search ranges are `u8` |
| src/cubie.rs (fields) | `CubieCube` pub fields: invalid cubes made outside could index out of range in `compose`/`inverse`/`to_facelets` | cold | (a) fields `pub(crate)`: every constructor yields a valid cube |
| src/cubie.rs:144 (`apply_moves`) | took `&[u8]`, indexed `move_cubes()[mv]` | cold | (b) returns `Option<CubieCube>` (`None` for a byte that is no move); parse+apply in the bins/wasm is one `and_then` with the existing parse error |
| src/symstart.rs:189 | `index.expect(..)` + `try_from().unwrap()` computing move conjugates at run time | cold (once) | (a) the table is build.rs's `SYM_MOVE_MAP` (the same table; the existing test checks it) |
| src/symstart.rs:218 | `.min().unwrap()` | warm (per dedup) | (a) `fold((MAX, MAX), min)` |
| src/hot.rs:49, 56 | `assert!` in `pub const fn pad` | compile time (but callable at run time) | (a) `pad` is private and returns `usize::MAX` for a bad value; the asserts sit in the `STREAM_PAD`/`SHARE_PAD` constants (compile-time) |
| src/tables.rs:113, 149, 157; 131 | `assert_eq!`/`assert!`/`unwrap` checking the cocsep BFS | cold (startup) | (b) `Cocsep::generate() -> Result`, `TableError::CocsepModel`; packed entries written as `[u8; 3]` chunks |
| src/tables.rs:287, 304, 371 | mmap `assert!`, `expect("out of memory")`, `Layout ..expect` | cold | (b) `Memory::anonymous -> Result<Memory, TableError>` (`TableError::Alloc`), `layout` an `Option` |
| src/tables.rs:531, 541 | `assert!(len <= isize::MAX)` (32-bit) in `words`/`bytes` | cold | (b) `Result`, `TableError::TooLargeForSlice` |
| src/tables.rs:688, 947, 761 | `expect("64-bit usize")`, `file_name().unwrap()` | cold | (b) `TableError::TooLarge`; `meta_path` falls back to the sidecar for a hugetlbfs path without a file name |
| src/tables.rs:893, 894 | `try_from().expect` in `from_file_memory`'s pieces | cold | (c) the piece checks its range (empty out of range; the checksum then fails) |
| src/tables.rs:994, 1001, 1002 | `read_exact_at().expect`, `off_t::try_from().expect` in `read_parallel` | cold (table load) | (b) io errors returned; fadvise skipped if a number does not fit; a thread that cannot start reads its range on the caller; join errors returned |
| src/tables.rs:485 (`prefault`) | `scope.spawn` | cold | (b)/(c) as `read_parallel` |
| src/tables/file.rs:143 | `assert!(text < trailer)` | cold | (a) bounded text (fixed-width numbers, ~250 of 4096 bytes); `write_trailer` reads it back |
| src/tables/file.rs:129, 266, 314, 318, 325, 327, 369 | `try_from(const).unwrap()`, `try_into().unwrap()` | cold | (a) `TRAILER_LEN` const (const-asserted equal), `as_chunks::<8>()`; (b) mmap offsets via `try_from` → io error |
| src/tables/file.rs:455, 462 | `panic!` on bad `CUBEOPT_LOAD` / `CUBEOPT_PREFAULT` | cold | (b) `LoadOptions::from_env -> Result`, `TableError::BadEnv` |
| src/lib.rs:68, 75, 76, 78, 111, 126, 158 | `assert!(k == 0 \|\| 11)`, `expect`s | cold (gen) | (b) `generate`/`generate_table -> Result<_, TableError>` (`Generate`, `TooLarge`, `Io`) |
| src/gen.rs:332 | `expect("64-bit usize")` | cold | (b) `table_words -> Result`; `Generator::run -> Result` checks k, memory sizes; derived tables are word slices |
| src/gen/collapse.rs:53, 55, 56, 95, 99, 102 | `assert!`, `assert_eq!`, `unwrap` | cold | (b) `derive`/`compare -> Result` (`collapse_shape` checks k, sizes, overflow) |
| src/gen/dfs.rs:392, 421, 424-426, 435, 450, 451 | `assert!`/`assert_eq!` on settings and parts, `unwrap`s | cold | (b) `Generator::new`/`generate -> Result` (`TableError::Generate`); `(0u8..).zip(levels)` |
| src/gen/dfs.rs:316 | `s.spawn` progress thread | cold | (c) `Builder`; no progress line if it cannot start; joined explicitly |
| src/parallel.rs:40, 50 | `expect("the runner ran every share")`, `join().unwrap()`; `s.spawn` | cold/warm (gen, checksums, wasm solves) | (c) a share whose thread cannot start or that the runner skipped runs on the caller; (b) `WorkerPanicked` for a panicked worker |
| wasm/src/threads.rs:58, 62 | `assert!(work.is_none())`, `thread::spawn` | cold | (c) nested jobs run inline; shares without a thread run on the caller |
| wasm/src/lib.rs:89, 204 | `expect("created by state")` | cold | (a) the state is a plain `Mutex<State>` (corners made on demand, error returned) |
| src/bin/bench.rs:78, 82, 109, 113; 74 | `cubes(i).unwrap()`, `lock().unwrap()`, `assert_eq!(bad, 0)`, `into_inner().unwrap()`, `s.spawn` | warm (per solution), cold | (b)/(c) a missing cube or solution counts as bad; `run -> anyhow::Result` (`ensure!` bad == 0, thread start/join errors); poison → `into_inner` |
| src/bin/solve.rs:268, 350, 443, 447; 78 | `lock().unwrap()`, `expect("below 6")`, `into_inner().unwrap()`, `scope.spawn` | warm (per solution), cold | (c) poison → `into_inner`; `below_usize`; (b) thread errors, "no solution within 20 moves" error, BAD line for `None` |
| src/bin/gen.rs:479, 502 | `try_from().unwrap()` for sync_file_range/fadvise | cold | (c) skipped if it does not fit (advice only); `write_table` uses `get(..)` |
| build.rs:82-166 | 25 × `writeln!(..).unwrap()`, `env::var("OUT_DIR").unwrap()`, `fs::write().unwrap()` | build | (b) `main() -> Result<(), Box<dyn Error>>`, `?` |
### 4. Enforcement

* **Clippy, at every crate root** (src/lib.rs, src/main.rs, src/bin/{bench,gen,solve}.rs, build.rs,
  wasm/src/lib.rs, wasm/build.rs, benches/kernels.rs):
  `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic,
  clippy::unreachable, clippy::todo, clippy::unimplemented))]`. `cfg(test)` builds (unit tests) and
  tests/, examples/ (separate crates without the attribute) are not linted. check.sh's clippy runs
  (x86, scalar, aarch64, wasm32 targets, and `-p cubeopt-wasm` on both wasi targets) enforce it.
* **No lint flags `assert!`**: `tests/no_runtime_panics.rs` scans the shipped sources (src/,
  build.rs, wasm/src, wasm/build.rs, benches) and fails on `assert!`, `assert_eq!`, `assert_ne!`,
  `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `.unwrap()` or `.expect(` outside
  - test code: any item, statement or field after `#[cfg(test)]` / `#[cfg(all(test, ..))]`, to the
    end of its `{ .. }` block (or its `;`/`,`),
  - compile-time code: `const`/`static` initializers (from `=` to `;`, e.g. `const _: () =
    assert!(..)`, `static BUILD_DIR`) and inline `const { .. }` blocks (not `const fn` bodies).
  Comments, strings (incl. raw) and char literals are blanked first, so text in them never matches.
  A second test checks the scanner itself on a sample (comments, strings, constants, cfg(test)
  modules, fields and functions; finds a runtime `assert_eq!` and `.unwrap()`). check.sh runs it
  as its own step right after `cargo fmt` (and it also runs with the other tests). It is a Rust
  test, so it needs nothing but cargo (no Python on the laptop/CI).
* `debug_assert!` stays allowed (not in release builds).
* **Not enabled:** `clippy::indexing_slicing` (≈300 sites, most in in-range loops and hot kernels;
  per policy, no `get_unchecked` sprinkled to satisfy a lint) and `clippy::arithmetic_side_effects`
  (thousands of sites). Sections 2-3 list where the remaining implicit checks are.

## No-panic integration: remaining implicit-check inventory

Before: 0b5b5a6. After: 935cb7f (no-panic integration plus the stream follow-up). Rust Clippy diagnostics from release library and binaries,
union of native and scalar configurations, deduplicated by file, line, column and lint.
These are source-level diagnostics, not a count of reachable panic branches: the optimizer
can prove bounds or nonzero divisors, and arithmetic warnings also include non-panicking
operations. No unchecked access was added merely to silence these diagnostics.

| file | indexing/slicing before | after | arithmetic before | after |
|---|---:|---:|---:|---:|
| src/bin/bench.rs | 2 | 2 | 4 | 4 |
| src/bin/gen.rs | 3 | 4 | 13 | 14 |
| src/bin/solve.rs | 4 | 2 | 2 | 1 |
| src/cli.rs | 0 | 0 | 2 | 2 |
| src/cli/preflight.rs | 0 | 0 | 0 | 2 |
| src/coord.rs | 15 | 15 | 34 | 34 |
| src/cubie.rs | 83 | 44 | 45 | 31 |
| src/gen.rs | 17 | 16 | 15 | 14 |
| src/gen/collapse.rs | 6 | 6 | 13 | 12 |
| src/gen/dfs.rs | 11 | 6 | 24 | 21 |
| src/hot.rs | 2 | 0 | 2 | 1 |
| src/lanes.rs | 53 | 52 | 72 | 72 |
| src/lanes/share.rs | 67 | 63 | 150 | 150 |
| src/lib.rs | 0 | 0 | 2 | 1 |
| src/parallel.rs | 1 | 0 | 0 | 0 |
| src/portable/coord.rs | 0 | 0 | 1 | 1 |
| src/portable/mod.rs | 40 | 40 | 11 | 11 |
| src/portable/v16_scalar.rs | 3 | 3 | 0 | 0 |
| src/simd.rs | 27 | 27 | 5 | 5 |
| src/solver.rs | 42 | 39 | 40 | 40 |
| src/symstart.rs | 11 | 10 | 4 | 4 |
| src/tables.rs | 31 | 22 | 32 | 31 |
| src/tables/file.rs | 3 | 1 | 13 | 8 |
| src/tables/shm.rs | 0 | 5 | 0 | 13 |

Explicit runtime panic macros and unwrap/expect calls are enforced separately by crate
restriction lints and tests/no_runtime_panics.rs, across all supported backends in check.sh.
Tests and compile-time assertions are allowed; existing debug_assert checks remain per the
standing policy. See np1-panic-inventory.md for the site-by-site explicit-call remediation.

### Rename validation

Final renamed source `cd963dc26b08` also passed the enforced explicit-call scan and full native/scalar/ARM/WASM checks on2026-11-06. The benchmark and compiler-diagnostic inventories above keep their recorded935cb7f provenance. The rename preserves the checked hot instructions, required page offsets and stack frames.
