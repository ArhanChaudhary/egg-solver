# The Rust Performance Book: technique audit

Source: https://nnethercote.github.io/perf-book/ (all 18 chapters, read in full from the
book's print.html on 2026-11-04). Code refs are on main d28335f (x86 code identical to 7075875);
the deliverables are rebased onto d690a46 and the final PGO numbers were measured there. "Status" is one of
**done** (with the commit or code), **tried-negative** (with the branch), **N/A** (with the
reason), **not yet tried**, or **newly tried** (by this audit) with the result. "Expected gain" is
for the solver's measured workloads: bench stream (1T/48T), bench share, solve -c 1/-c 0.

Context that decides most rows: 1T stream spends 93% of cpu-clock samples in one function
(`bench::stream_thread`, the lane loop), 2.4% setup_run, 1.6% next_run, 1.3% start_iteration,
1.2% extend_jobs, 0.21% kernel, 0.03% libc (malloc/free/memset/memmove together)
(cpu-clock profile of a 1T stream run on main 03abdfc). At 48T the limit is DRAM latency and
memory-level parallelism, so instruction-count savings show mostly at 1T.

## Summary

120 techniques (one row each, 18 chapters). **done 58** (with commits/code), **N/A 42** (with the
reason), **tried-negative before this audit 1** as the row's main status (G12; another 8 "done" rows carry a
tried-negative variant: C3, C8, C12, I2, I5, T5, G10, plus neg/a7-bolt in C11), **not yet tried 4**
(P8 Coz, H1 FxHash in symstart's dedup, A15 dhat-rs heap tests, S6 parking_lot: left to the crate-replacement work; all
expected <0.5% or 0 on random states), **newly tried or checked by this audit 15** (C5, C7, C10, C11, C16,
L4, P5, A1, A12, A14, T1, T8, IO1, IO2, IO3).

What this audit measured (1T = bench slots, paired and alternating; 48T = exclusive holds):

| Item | Variant | 1T | 48T | Verdict |
|---|---|---|---|---|
| C10 PGO, bench (stream) | pads 0/16/32/48 swept; profile: stream seed 2026 (the evaluation stream: favours PGO, so the negative stands) + share random:300:99 | pad 32 (best): pairs -0.19 -0.57 -0.13 +0.07 -1.29 -0.48 0.00 -0.13% (median 24.667 -> 24.567); pad 0 -2.70 -0.41; pad 16 -4.17 -3.13; pad 48 -1.36 +0.45 -3.86 -1.14 | not run | negative: neg/pb1-pgo-stream |
| C10 PGO, bench (share) | same builds, bench random:1000:2026 share 1T | pad 16 +1.76 +2.80%; pad 32 +3.19 +3.43% solves/s | - | led to the solve-only build below |
| C10 PGO, `solve` only (scripts/pgo-solve.sh, d690a46, trained on 300 script-made 40-move scrambles, pad 0) | solve -t 1 -c 1 -n 200 random_state, ms/cube | 8 pairs -1.97 -3.38 -1.57 -1.85 -2.18 -2.25 -1.29 -2.41%: 44.407 -> 43.474 (-2.1%) | solve -t 48 -c 1 -n 300, 7 holds, median of pairs: random_state +0.2%, random_move_17f -0.7%, random_state_depth17 -0.4% (neutral) | 1T win, 48T neutral: merged as 6ff4350 via `opt/pb1` (`scripts/pgo-solve.sh`, now archived as `scripts/experiments/pgo-solve.sh`; optional; default build unchanged) |
| C10 + page-pinning patch | the page-pinned frames of `opt/s9-pin-v2` (removes the 0x21c0 vs 0x1f40 frame confound), PGO vs non-PGO on it | - | 7 holds, median of pairs: -1.8% / -2.1% / -0.6% (inside 48T noise) | leaning faster, not significant |
| C11 PGO+BOLT, bench | ext-tsp + hfsort + split, on the PGO bench | stream -1.35 -1.15%; share +1.98 +1.75% (PGO alone: +1.8..+3.4) | not run | negative: neg/pb1-pgo-bolt |
| C5/C7, A12 allocator, buffer reuse | counting global allocator + strace + perf stat | 6-26 allocations per cube; 6 madvise / 14 munmap per 35-s 1T run; libc 0.03% of samples | - | nothing to win; not timed (neg/pb1-diag-alloc-count) |
| IO1-IO3 stdout in solve | strace | 1 write(2) per cube, ~0.7 us, after the cube's slot is released | - | <0.1%; not timed (noted for the CLI work) |
| C16 debug = 1 vs 0 | objdump | same instructions everywhere but `main` (2) | - | no effect |
| T8 size asserts | const assert Node/Job/ChildOrder | .text/.rodata byte-identical | - | adopted as a guard: merged as 13ea1af via `opt/pb1` |
| T1 type sizes, L4 nursery lints | -Zprint-type-sizes; clippy nursery | see appendix / L4 | - | nothing new to shrink or fix |

Search identity: verify.sh on a PGO bench (d28335f, pad 32) prints main's numbers exactly
(3058198/2588380, lengths 15:1 16:35 17:270 18:662 19:32; 3644301/3084160, 18:100); every one of
the 28 `solve` runs per set in the 48T holds gives the same (cube, length) list, 0 BAD.

## Benchmarking

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| B1 | Realistic workloads, several of them | `bench stream` (random states), `bench ... share`, `solve` on cube_solver_test sets (random_state, random_move_*f, random_state_depth*), testdata/ | done (bench.rs, solve.rs, a separate verification script) | - |
| B2 | A harness (built-in bench, Criterion, Divan, Hyperfine, Gungraun, custom) | custom: `bench` binary + measurement scripts (bench1t, ab1t, ab48, abl1t, abl48, conf48) | done. Criterion/Divan N/A: each sample is a multi-second run on a 58 GiB table, not a microbenchmark | - |
| B3 | Lower-variance metrics (instructions, cycles) next to wall time | cachegrind Ir/node on x86-64-v3 builds, node/lookup counts (count.sh), TAGE branch simulator | done. Cycles N/A: no PMU on this VM | - |
| B4 | Beware layout-induced fluctuations | hot-loop placement pinned (src/hot.rs, 1c2749a); build-folder fix 97e1abd; layout-averaged A/B (abl1t/abl48) | done | - |
| B5 | Continuous benchmarking (Bencher on CI) | no CI; every merge carries 48T + 1T tables | N/A | - |

## Build Configuration

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| C1 | Release build | all benches and the CLI use `--release` | done | - |
| C2 | `codegen-units = 1` | Cargo.toml:16 | done | - |
| C3 | Fat LTO (`lto = "fat"`) | Cargo.toml:15 | done; thin LTO tried-negative neg/a7-lto-thin (-1.0% 1T) | - |
| C4 | `lto = false/"off"` | - | N/A (only compile time) | - |
| C5 | jemalloc (tikv-jemallocator) | global allocator of bench/solve | newly checked by this audit, not timed: nothing for an allocator to win (see "Allocations" below: ~13 allocator calls per cube in stream mode, ~9 in share mode, ~25-29 in solve; no mmap/madvise churn) | ~0 (libc incl. malloc/free is 0.03% of 1T stream samples) |
| C6 | jemalloc with THP (`MALLOC_CONF=thp:always`) | the heap | N/A: the only large memory, the table, is on 1 GiB hugetlbfs pages (tables.rs mmap); the heap is ~1.4 MB per lane | 0 |
| C7 | mimalloc | global allocator of bench/solve | as C5: newly checked by this audit, not timed | ~0 |
| C8 | `-C target-cpu=native` (+ check `rustc --print cfg`) | .cargo/config.toml:6 (x86_64 only) | done; znver4 / x86-64-v4 / -avx512f tried-negative neg/a7-target-cpu; native = znver5 features (a7) | - |
| C9 | `-C target-feature` | build.rs selects the x86/AVX-512 kernels from the enabled features | done | - |
| C10 | PGO (rustc PGO / cargo-pgo) | whole binary; rewrites the hot loops | newly re-tried with pinned pads by this audit. **bench/stream: tried-negative** neg/pb1-pgo-stream (best pad 32: -0.2% median of 8 pairs at 1T; pads 0/16/48: -1.6/-3.7/-1.5%; PGO turns cmovs into branches: 114 vs 92 jcc in stream_thread). **solve/share: win at 1T only**: scripts/pgo-solve.sh (merged as 6ff4350 via `opt/pb1`; now archived as scripts/experiments/pgo-solve.sh) (solve 1T -2.1% ms/cube, 8 of 8 pairs; 48T -c 1 neutral, 7 holds) | solve 1T -2%; 48T 0; stream <0 |
| C11 | BOLT (cargo-pgo) | post-link layout | BOLT alone tried-negative neg/a7-bolt (+0.3% in noise). PGO+BOLT newly re-tried by this audit: **tried-negative** neg/pb1-pgo-bolt (stream 1T -1.35/-1.15%; share 1T +2.0/+1.75%, no better than PGO alone; taken branches -34.9%) | <0 |
| C12 | `opt-level = "z"/"s"` | - | N/A (binary size); opt-level 2 tried-negative neg/a7-opt-level-2 (neutral) | - |
| C13 | `panic = "abort"` | Cargo.toml:17 | done | - |
| C14 | `strip = "symbols"` | profile.wasm-release (Cargo.toml) | done for wasm; N/A for speed (size only) | 0 |
| C15 | Faster linker (lld, mold, wild) | link step | N/A (link time only); lld is already the default on Linux since Rust 1.90 (rustc 1.99 here). Layout effects of linking are neutralised by src/hot.rs | 0 |
| C16 | `debug = false` / `"line-tables-only"` | Cargo.toml:18 `debug = 1` in release (for perf annotate) | newly checked by this audit: a `debug = 0` build of bench has the same instructions in every function except `main` (2 instructions), and the hot loops sit at the same offsets mod 64; keep `debug = 1` | 0 (no codegen change) |
| C17 | Parallel front-end (`-Zthreads`) | - | N/A (compile time) | 0 |
| C18 | Cranelift backend | - | N/A (compile time; slower code) | 0 |
| C19 | Custom profiles | profile.wasm-release, profile.test opt-level 3 | done | - |
| C20 | cargo-wizard | - | N/A (the profile was chosen by measurement, C2-C13) | - |

## Linting

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| L1 | Clippy, incl. the `perf` group | check.sh: clippy `-D warnings` (perf is in the default set); `pedantic` in Cargo.toml:23-24 | done (555a6ea, 9e4bc24) | - |
| L2 | `ptr_arg` (`&mut Vec<T>` -> `&mut [T]`) | default style lint, enforced; the remaining `&mut Vec<Job>` (sort_jobs) swaps Vecs | done | - |
| L3 | `disallowed_types` to keep out std types once replaced | one std HashMap (symstart.rs:157); std Mutex in share.rs | N/A now; applies if the crate-replacement work standardizes on crate types | 0 |
| L4 | (added by this audit) perf-adjacent lints outside the default set (`nursery`: needless_collect, redundant_clone, or_fun_call, large_stack_frames ...) | whole crate | newly tried by this audit: `clippy --all-targets -W clippy::nursery -W clippy::perf -W clippy::large_stack_frames -W clippy::large_stack_arrays` on d28335f. Two hits, both off the solve path: `needless_collect` at parallel.rs:15 (a false positive: the collect starts all threads before thread 0 works) and `or_fun_call` at tables/file.rs:443 (startup, trivial call). No `redundant_clone` or large-frame hits. The other 266 are style lints (`use_self`, `missing_const_for_fn`, ...) | 0 |

## Profiling

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| P1 | perf (+ Hotspot / Firefox Profiler) | cpu-clock sampling (no PMU), perf annotate | done (perf profiles of the stream and share loops) | - |
| P2 | Instruments / VTune / AMD uProf | - | N/A (macOS / Intel; uProf needs the PMU, absent on this VM) | - |
| P3 | samply / flamegraph | - | N/A: one function holds 93% of samples; perf annotate is the useful view | - |
| P4 | Cachegrind / Callgrind | x86-64-v3 builds (valgrind cannot run AVX-512) | done (Ir/node in the m-series micro rounds) | - |
| P5 | DHAT (allocation sites and rates, memcpy) | allocations per cube | newly done by this audit with a counting `#[global_allocator]` instead (valgrind cannot run the AVX-512 build): branch `neg/pb1-diag-alloc-count`; results under "Allocations" | - |
| P6 | heaptrack / bytehound | - | N/A (same question as P5; heap is small) | - |
| P7 | `counts` / ad-hoc eprintln profiling | share trace lines (CUBE_TRACE), diag patches (diag-code0, c9-trace) | done | - |
| P8 | Coz causal profiling | - | not yet tried; low value: the stream loop is one function, share latency was mapped by CUBE_TRACE | <0.5% (finding only) |
| P9 | `debug = "line-tables-only"` in release | Cargo.toml:18 (`debug = 1`) | done | - |
| P10 | std with debuginfo / build-std | - | N/A (std is not in the hot path) | 0 |
| P11 | `-C force-frame-pointers=yes` | - | N/A (sampling one function; stream_thread already keeps rbp for its 64-byte stack realignment) | 0 |
| P12 | v0 symbol mangling | - | N/A (perf demangles the legacy names) | 0 |

## Inlining

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| I1 | `#[inline]` / `#[inline(always)]` on small or single-call hot functions | 101 `inline(always)` sites; the kernel is fully inlined into `bench::stream_thread` and `share::Worker::rounds` | done (m1-m8 rounds; `inline_always` allowed in Cargo.toml) | - |
| I2 | `#[inline(never)]` | bench.rs:108,124 (one function per mode, 8163366); share.rs:2318,2414,3397,3469,1740 | done; Pool::run inline(never) tried-negative neg/l6-poolrun-inline-never | - |
| I3 | Check inlining (cachegrind / asm) | cargo asm in every micro round | done | - |
| I4 | Split a function into inline(always) + inline(never) variants (hot vs cold call sites) | `Lane::advance` inlined in the loop, `advance_cold` (lanes.rs:1581) for refills | done | - |
| I5 | Outlining rare code with `#[cold]` | lanes.rs:1033 next_run; share.rs:1234,1514,2109,2175. The stream loop's found path is already out of line (calls reconstruct, Timespec::now, the record and refill closures, per perf annotate) | done; more tried-negative: opt/neg-cold-paths (setup_job/start_iteration/reconstruct, -1.5%), neg c4 next_run not cold (6efbdec) | ~0: the remaining rare paths in the stream loop are already calls; the loop bodies were being changed by the m8 series |

## Hashing

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| H1 | Faster hasher (FxHash, FNV, aHash) | symstart.rs:157 std `HashMap<(u64,u128),usize>` (SipHash): `dedup` of job levels <= 8192 jobs, only for cubes with a symmetry AND an antisymmetry (superflip-like); gen/dfs.rs:70 in-house set | not yet tried (the generator's set was left to the crate-replacement work) | 0 on random states; the map is a minor part of dedup's ~0.2 us/job (96 conjugations per key dominate): <0.5% on superflip-like cubes only |
| H2 | nohash_hasher for random-valued keys | - | N/A (no such map in the solve path) | 0 |
| H3 | Byte-wise hashing (`ByteHash`) | - | N/A (no derived Hash in hot code) | 0 |
| H4 | `disallowed_types` for the std maps | - | N/A (one site) | 0 |

## Heap Allocations

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| A1 | Check whether malloc/free are hot | 1T stream profile: libc 0.03%, kernel 0.21% | done (perf profile); counted by this audit: 0.0004 allocations per 1000 nodes in stream mode (13 per ~3.3M-node cube) vs the book's ~10 per million instructions threshold | 0 |
| A2 | Box a field to shrink a hot type | `Lane::run_state: Box<RunState>` (lanes.rs:481) keeps the cold run state out of the lane's hot lines | done | - |
| A3 | Rc/Arc only for shared values | `Arc<CubeCtx>` per share-mode cube (shared by the threads); clones only at rare events (share.rs:2828,2952,3357) | done | 0 |
| A4 | `Vec::with_capacity` / `reserve` / growth | share: `Inner`/`Bufs` recycled (share.rs:653-654), buffers pre-touched (887c892, 17f -8%); lanes reuse `jobs`/`jobs_tmp` | done | - |
| A5 | SmallVec for many short vectors | - | N/A (no hot short Vecs; job prefixes are `[u8; 4]`) | 0 |
| A6 | ArrayVec when the max length is known | the per-cube solution `Vec<u8>` (<= 20 moves) | N/A (once per cube) | 0 |
| A7 | `shrink_to_fit` | - | N/A (memory is not the constraint; reuse is wanted) | 0 |
| A8 | String: with_capacity, smartstring, avoid `format!` | `format!` only in trace/summary/output paths; solve formats one line per cube | N/A for speed | 0 |
| A9 | `clone_from` | - | N/A (no hot clones of heap data) | 0 |
| A10 | Remove unnecessary `clone` | - | done (s8 sweep; see L4) | 0 |
| A11 | Avoid `to_owned` / use `Cow` | - | N/A (no owned strings in the solve path) | 0 |
| A12 | Reuse a workhorse collection | lanes, share: done for jobs, Inner, Bufs. NOT for `sort_jobs`' keys + scratch (lanes.rs:145-150): two n x 8-byte Vecs per sorted iteration (~43K jobs: 2 x 344 KB) | newly checked by this audit, not timed: the 2.2 allocations of >= 128 KiB per cube come from glibc's arena without page faults after warm-up (strace, 35 s of 1T stream: 6 madvise, 14 munmap, 25 mmap in total; perf stat: 24.8K page faults per run, nearly all start-up). A reused buffer would save one malloc/free pair per ~0.4 ms sort | ~0 (<0.01%) |
| A13 | `read_line` into a workhorse String | solve.rs:59 `stdin().lock().lines()` | N/A (input read once at startup, before the clock starts) | 0 |
| A14 | Alternative allocator | see C5/C7 | see C5/C7 | - |
| A15 | dhat-rs heap-usage tests against regressions | - | not yet tried; the hot loops cannot allocate by construction | 0 |

## Type Sizes

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| T1 | Measure layouts with `-Zprint-type-sizes` | Node, Job, Lane, RunState, ChildOrder, SharedLane, Local, CubeCtx, Inner | newly done by this audit (nightly-2026-11-03, target-cpu=native): see "Type sizes" below. Nothing is bigger than its design: Node 128 B, two lines with 40 B of tail padding (96 B was tried: neg/m6-node-96-bytes, -2.9%); Job 12 B; the Lane's per-visit fields span 6 lines after its 3328-B stack (packing the hot ones first was tried: neg/m6-lane-hot-fields-first). The per-cube CubeCtx is 1792 B (ArcInner 1920 B) | 0: no untried shrink with a mechanism |
| T2 | Types > 128 bytes are copied with memcpy | Node = 128 B (copied inline); Lane (~3.5 KB) is never copied in the loop | done (memmove: 0.01% of 1T samples) | 0 |
| T3 | Field ordering | automatic (no `repr(C)` on hot structs except ChildOrder, deliberately) | done | 0 |
| T4 | Box an outsized enum variant | enums are tiny (`Phase` repr(u8), `Step`) | N/A | 0 |
| T5 | Smaller integers | u8 depths/bounds/moves, u32 move masks; Job = 12 bytes | done; Node 96 bytes tried-negative neg/m6-node-96-bytes (-2.9%), packed counter neg/m6-packed-counter | - |
| T6 | Boxed slices instead of Vec | CubeCtx task_log / thread_done_ns already `Box<[T]>`; Vec fields elsewhere grow | N/A for speed (8 bytes per field) | 0 |
| T7 | ThinVec | - | N/A | 0 |
| T8 | Static size assertions for hot types | Node (128 B), Job (12 B), ChildOrder (32 B) | newly done by this audit: const assert on x86-64 in lanes.rs, merged as 13ea1af via `opt/pb1`; .text and .rodata byte-identical to main d690a46 | 0 (regression guard) |

## Standard Library Types

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| S1 | `vec![0; n]` for zeroed Vecs | sort_jobs scratch (lanes.rs:150) | done | - |
| S2 | `swap_remove` instead of `remove` | - | N/A (no `Vec::remove` in the solve path) | 0 |
| S3 | `retain` | share.rs:846,888 (open list), symstart.rs:169 | done | - |
| S4 | Lazy `ok_or_else` / `unwrap_or_else` / `map_or_else` | `spare.unwrap_or_else(Inner::touched)` (share.rs) | done (see L4 for `or_fun_call`) | 0 |
| S5 | `Rc::make_mut` / `Arc::make_mut` | - | N/A | 0 |
| S6 | parking_lot Mutex / RwLock / Condvar / Once | share.rs std `Mutex` (per-cube or rare: spare, bufs, result, pass) and in-house `SpinRw` (share.rs:279, -38.8% -c 1 vs std RwLock, l6-share-a) | not tried by this audit: left to the crate-replacement work (crates for spin locks, Pad, Mutex) | <0.5% (the std locks are off the per-node and per-job paths) |

## Iterators

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| It1 | Don't `collect` only to iterate again | sort_jobs collects keys once (needed for the radix passes) | N/A | 0 |
| It2 | `extend` instead of collect + append | sort_jobs `out.extend`, share bufs | done | - |
| It3 | `size_hint` / `ExactSizeIterator` on own iterators | - | N/A (no custom iterators) | 0 |
| It4 | Avoid `chain` in hot iterators | parallel.rs:18 (thread join), tests | N/A | 0 |
| It5 | `filter_map` instead of filter + map | - | N/A (none in hot code) | 0 |
| It6 | `chunks_exact` instead of `chunks` | share.rs:1188 (`inner.jobs.chunks(chunk)`, once per build stage, uneven tail) | N/A (not hot) | 0 |
| It7 | `iter().copied()` for small types | hot loops walk lanes by pointer and index tables with `get_unchecked` | N/A in hot code | 0 |
| It8 | (added by this audit) iterator vs index codegen in the hot loops | stream loop iterates `lanes[..n_active]` (lanes.rs:1975); share rounds walk lane pointers (03abdfc, +3.2% 1T as part of `opt/m8-unsafe`) | done | - |

## Bounds Checks

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| BC1 | Iterate instead of indexing | lane loops (lanes.rs:1975; share rounds by pointer 03abdfc) | done | - |
| BC2 | Slice before the loop | done throughout | done | - |
| BC3 | Assert index ranges | replaced by `get_unchecked` under the project's rule for unchecked indexing (f66ec5a) | done | - |
| BC4 | `get_unchecked` | 710716d (unsafe pass of `opt/m8-unsafe`: search, run and job paths), f66ec5a (s8) | done | - |

## I/O

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| IO1 | Lock stdout once for repeated prints | solve.rs:96-114: `Mutex<Stdout>`, `writeln!` + `flush` per cube | newly checked by this audit, not timed; noted for the CLI work, which owned the binaries. One `write(2)` per cube, 0.7 us each under strace (101 writes for 100 cubes). The callback runs in `Worker::release` after `publish_solution` has already freed the cube's `active` slot (share.rs:2507), so idle threads start the next cube without waiting for the write | <0.1% at -c 1 48T (17f: 0.57 ms/cube); not measurable |
| IO2 | BufWriter for many small writes | solve: stdout is line-buffered, one `write(2)` per solved cube; bench prints only a summary; gen writes tables through mmap | as IO1. A BufWriter would also delay the per-cube lines that cube_solver_test's awk reads as they come | <0.1% |
| IO3 | Lock + buffer stdout together | as IO1/IO2 | as IO1 | as IO1 |
| IO4 | BufReader / read_line | solve.rs:59 reads stdin once | N/A (startup) | 0 |
| IO5 | Read raw bytes (`read_until`), skip UTF-8 checks | - | N/A (startup) | 0 |

## Logging and Debugging

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| LD1 | No logging/debug work when disabled | share trace and costly counters behind `CUBE_TRACE` bits (3bf5530; c10 found the always-on trace cost -c 0 +4.7%) | done | - |
| LD2 | `debug_assert!` for hot non-safety asserts | hot paths use `debug_assert!` / unchecked calls; `assert!` only in setup, tests and the checked `apply_move` (simd.rs:305,337) not used by the kernel | done | - |

## Wrapper Types

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| W1 | One wrapper for values accessed together | CubeCtx: rarely written atomics in one `Pad<RarelyWritten>`, result as one `Mutex<(Vec<u8>, f64)>`, read-mostly and written fields on separate 128-byte lines (share.rs:388) | done (l6) | - |

## Machine Code

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| MC1 | Inspect the asm (Compiler Explorer, cargo-show-asm) | cargo asm on the two hot loops, every micro round | done | - |
| MC2 | `core::arch` intrinsics | simd.rs (AVX2, AVX-512 VBMI, BMI2 pext/bzhi), portable/ NEON and wasm SIMD128 | done | - |

## Parallelism

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| Pa1 | Threads (rayon, crossbeam, atomics and locks) | std::thread::scope + in-house work-sharing pool (share.rs: atomics, mailboxes, spin locks) | done in-house; crate replacements were separate work (`opt/d1-crates-v2`) | - |
| Pa2 | Data parallelism (SIMD) | cube ops in SIMD; 24 interleaved lanes per thread for memory-level parallelism | done | - |

## General Tips

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| G1 | Optimize only hot code | profile-driven rounds | done | - |
| G2 | Algorithms and data structures first | h11 table on 1 GiB pages, IDA* jobs, child ordering (c7/c8), work sharing (l5/l6/c10/c11), symmetric start | done | - |
| G3 | Fewer cache misses and mispredictions | NTA prefetch per lookup, lanes for MLP, branchless kernels (TAGE-driven) | done | - |
| G4 | Many small speedups add up | m1-m8 micro rounds | done | - |
| G5 | More than one profiler | perf, cachegrind, TAGE sim, cargo asm, CUBE_TRACE | done | - |
| G6 | Make it faster or call it less | e.g. twin lookups (-29% lookups), sibling-face pruning | done | - |
| G7 | Remove silly slowdowns | trace cost (c10), page faults in share builds (887c892) | done | - |
| G8 | Lazy / on-demand computation | lazy child cube (387034c), other side only when needed | done | - |
| G9 | Special-case the common small cases | first pass for early iterations (0339dd2), tail lanes (7c46566), corner-bound precheck | done | - |
| G10 | Compact representation + fallback table | 2-bit table entries with base offset; eoesep fallback tried-negative neg/xh48-eoesep-fallback | done | - |
| G11 | Most common cases first | resolve order by phase frequency (m4) | done | - |
| G12 | Small cache in front of a structure with locality | cocsep table is the in-cache stage; extra in-cache table tried-negative (idea 3.6 in the [nxopt inventory](x-nxopt-inventory.md), `neg/xnx-diagnostics`) | done / tried-negative | - |
| G13 | Comments that cite measurements | annotation rule (56f5b58, e7b094e) | done | - |

## Compile Times

| # | Technique | Where it applies | Status | Expected gain for us |
|---|---|---|---|---|
| CT1 | `cargo build --timings` | one crate + libc | N/A (runtime is the goal) | 0 |
| CT2 | `-Zmacro-stats`, cargo-expand | no heavy macros | N/A | 0 |
| CT3 | `cargo llvm-lines`; non-generic inner functions | generic `run`/`record` closures only | N/A (runtime unaffected) | 0 |
| CT4 | `match` instead of `Option::map` etc. for compile time | - | N/A | 0 |

## Appendix: type sizes (T1)

`RUSTFLAGS="-C target-cpu=native -Zprint-type-sizes" cargo +nightly build --release --lib`
(nightly-2026-11-03, main d28335f; the full output is not published).

| Type | Size, align | Layout notes |
|---|---|---|
| `lanes::Node` | 128 B, 64 | moved + other cubes (64 B), other_moves/todo/skip (12 B), twin_values (6 B), 6 one-byte fields; 40 B tail padding |
| `lanes::Job` | 12 B, 4 | |
| `lanes::ChildOrder` | 32 B, 32 | repr(C) |
| `lanes::RunState` | 144 B, 8 | boxed in `Lane` (cold) |
| `lanes::Lane` | 3712 B, 64 | `stack` (26 Nodes, 3328 B) first; the per-visit fields (pending_cube, depth, pending_index, pending_move/face/bounds, phase, stats) then span 6 lines (3328..3651) |
| `share::SharedLane` | 3776 B, 64 | Lane + 64 B |
| `share::CubeCtx` | 1792 B, 128 (ArcInner 1920 B) | 7 padded 128-B lines of atomics, then root/inverse, Inner (560 B), result/pass mutexes |
| `share::Inner` | 560 B | |
| `share::Bufs` | 328 B | |
| `share::Local` | 160 B | |
| `share::Worker` | 576 B | |
| `share::Mailbox` | 3456 B, 64 | |

## Appendix: allocations and memory syscalls (A1, A12, C5/C7)

Counting global allocator (branch diag/alloc-count, 10a9237, retained as `neg/pb1-diag-alloc-count`,
[38ab386](../../history/DELETED_EXPERIMENTS.md#ref-neg-pb1-diag-alloc-count); counts every alloc, realloc and alloc_zeroed). Runs in 1T bench slots on main d28335f:

| Run | Cubes | Allocs per cube | Reallocs per cube | Bytes per cube | >= 128 KiB per cube |
|---|---|---|---|---|---|
| bench stream 5+30 s, 1T, 24 lanes | 850 | 9.9 | 3.3 | 888 KB | 2.2 |
| bench share random:500, 1T | 500 | 5.7 | 3.0 | 432 KB | 1.0 |
| solve -t 2 -c 1, random_state 100 | 100 | 25.7 | 3.1 | 106 KB | 0.22 |
| solve -t 2 -c 0, random_state 300 | 300 | 25.2 | 3.0 | 245 KB | 0.57 |
| solve -t 2 -c 1, random_move_15f 300 | 300 | 21.9 | 2.1 | 40 KB | 0.06 |

Memory syscalls under `strace -f -c` (whole run including start-up):

| Run | mmap | munmap | madvise | mprotect | brk | write |
|---|---|---|---|---|---|---|
| bench stream 5+30 s, 1T | 25 | 14 | 6 | 267 | 4 | 12 |
| bench share random:300, 1T | 24 | 12 | 3 | 58 | 4 | 16 |
| solve -t 2 -c 1, 100 cubes | 27 | 14 | 4 | 64 | 4 | 101 |

`perf stat -e page-faults` on a 1T stream run (5+30 s): 24.8K faults in total, nearly all
start-up (24 lanes' buffers, cocsep generation). Had each of the ~940 sorts in such a run
(1,889 allocations of >= 128 KiB, two per sort) faulted its two ~344-KB buffers in afresh,
that alone would be ~160K faults. So glibc keeps them in its arena, and an alternative allocator or a reused sort
buffer has nothing measurable to win.
