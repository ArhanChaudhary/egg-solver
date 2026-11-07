# an1: inline gain-comment audit of main 18c4a66

Patch: `$P/an1-annotations.patch` (one commit, 53fdbf1, on 18c4a66; comments
only: 9 files, +51/-34 lines).

Source of every number: the commit and merge messages of `git log main`
(first-parent merges and every merge's second-parent commits) and
`git notes`; `neg/*` ignored. No number is new: each one below is quoted
or computed from a before/after pair in the named commit. Basis labels:
**48T** / **1T** = `bench stream` (20 30 48 L / 10 60 1 L), **batch** =
old `bench.sh` 1000-cube wall clock, **thr** = steady-state throughput of
the early benches, **nodes** = nodes/cube (random:1000:2026 unless named),
**-c 1** / **-c 0** = `solve` latency / random_state batch at 48T (ms/cube),
**bench share** = `bench random:1000:2026 ... share` (solves/s).

Status: **ok** = an inline comment with the right number and basis was
already there; **added** = new comment (or a number added to a comment
that had none); **fixed** = number, basis or wording corrected;
**skipped** = not annotated, with the reason.

Counts: see the end.

## Stream kernel, index path, tables (early history to m8)

| # | change | commit (merge) | measured gain, basis | code site now | status |
|---|---|---|---|---|---|
| 1 | cube48opt table layout (E-slice innermost, 16-byte groups, group minimum) | 9090310 (046c33f) | neutral at h11 (48T batch 143.4 -> 144.5); h0 -71% nodes (72.0M -> 21.1M) | tables.rs, H48 header comment | ok |
| 2 | final-iteration job ordering | 9f9a4d3 (7bce67d) | -36% nodes, +46% 48T batch | solver.rs module doc | ok |
| 3 | sibling-face pruning + back-jumps | 0c48355 (2271355) | -17.7% nodes, +7.5% thr | solver.rs `search` doc; lanes.rs `advance` back-jump ("see Solver::search") | ok |
| 4 | job-root bounds once per job, batched prefetch | c52cb63 (31c2878) | +2.4% thr, +1.8% 48T batch | solver.rs `job_bounds` | ok |
| 5 | lane-interleaved explicit-stack search | 2bb148d (a0681ad) | +57% 48T (276.9 -> 434.7) | lanes.rs module doc | ok |
| 6 | X/X' share the other-side lookup (twin values) | 0b65adf (d2674d5) | -29% lookups, +10% 48T | lanes.rs `Node::twin_values` | ok |
| 7 | job statistic 8^(16-h) | aa43192 (d63535b) | -2.1% nodes, +2.8% 48T | lanes.rs `STAT_TERMS` | ok |
| 8 | single-pass round-robin scheduler | 5cea944 (59ce145) | +15.9% 48T (32 lanes 3-phase -> 16 lanes) | lanes.rs `run_lanes` | ok |
| 9 | edge-only symmetry conjugation | 42a7c98 (ae4fc49) | +5.5% 48T | coord.rs `edge_key_sym` | ok |
| 10 | packed 3-byte cocsep on a huge page + direct index | f99ebc4 (3957bbc) | +2.8% 48T (stack of 2) | tables.rs `Cocsep::packed`; coord.rs `index_sym` | ok |
| 11 | NTA prefetch of table lines | ebd6c20 (5b0f6ab) | +2.6% 48T | solver.rs `prefetch`, tables.rs `prefetch`, prefetch.rs `read_once` | ok |
| 12 | m1-micro: CO tables x70, asm vpmovmskb, compose without mask, move orient term, cached table pointer | e9d751e (6d265dd) | +2.2% 48T (stack of 4) | coord.rs `CO_LOW`, `edge_parts_sym_raw`; simd.rs `compose`, `apply_move_unchecked`; solver.rs `table_ptr` | ok |
| 13 | m2-jobs: radix sort, X-only inverse lookups in extension, job prefetch | d708f1c (814ae96) | +4.3% 48T (stack) | lanes.rs `sort_jobs`, `extend_jobs`, `setup_job` | ok |
| 13a | branchless face weight (m2-jobs) + masked index (m8) | d708f1c, 33a8042 | part of m2-jobs' +4.3% 48T | lanes.rs `job_weight` | added |
| 14 | no bounds checks in the lane loop | 042f13c (fe58740) | +1.1% 48T | lanes.rs `advance` SAFETY block | ok |
| 15 | skip-sibling: eager face pruning, X' skip mask, branch-free child setup | 7a173a1 (b90b706) | +13.6% 48T (stack of 3) | lanes.rs module doc, `advance`; simd.rs `select` | ok |
| 16 | m3-hht: pending value before dispatch, select of the twin value, one static for hot tables | 8b2cf71 (70567ff) | +3.5% 48T (stack of 3) | lanes.rs `pending_value`, `advance`; simd.rs `HotTables` | ok |
| 17 | 20 lanes | 18ef7f0 (ff2b7a4) | +2.9% 48T | superseded by 24 lanes (#22) | skipped: superseded |
| 18 | m4-micro: lane-loop, node-roles, rare-check, heur-pair, index-pext, resolve-face, phase-encoding | 35c608c | +7.7% 48T, +6.4% 1T (note); per patch +1.9 / +0.5 / +1.1 / ~0 / +1.7 / +1.2 / +1.7% | lanes.rs `run_lanes`, `Node`, rare-kind test, `STAT_PAIRS`, `pending_face`, `Phase`; coord.rs `corner_fields` | ok |
| 19 | p5: loop state in registers | 431e46a (9caf0e2) | +2.7% 48T, +1.8% 1T | lanes.rs `advance` doc | ok |
| 19a | p5: nodes counted in a local; no Idle phase | 68dfaf6, ee18653 | part of p5's +3.2% 48T, +1.6% 1T | lanes.rs `advance`, `advance_cold` | ok |
| 19b | p5: push_advance from registers | 236bf33 | alone within noise; against the stack without it +0.9% 1T, +0.6% 48T | lanes.rs `push_advance` | fixed: said "within noise when measured: +0.9% ..."; now says the number is a stack ablation |
| 20 | m6 nofmax | ecd311e (cb8cb70) | +2.6% 48T, +1.0% 1T | lanes.rs `note_value` doc | ok |
| 20a | m6 tabs (small tables in HOT_TABLES, face cut as a `face_keep` select) | 26b4f4c | +1.4% 1T, -0.1% 48T (whole patch) | lanes.rs `note_value` doc | fixed: the patch's number was given to the select alone, 48T missing |
| 20b | m6 index (cached packed pointer, x3 tables, row_mul / eo_mask) | 85e9c79 | +0.7% 1T, -0.4% 48T | solver.rs `row_mul`; coord.rs `cocsep_x3`, `index_sym` | ok |
| 20c | m6 skipcount (exhaustion test on `live`) | 621b70a | +1.0..1.3% 1T (48T +0.6% paired check) | lanes.rs `advance`, `live == 0` | added, "since reworked" (m7 replaced its bzhi count) |
| 20d | m6 childnode | d2a3c2e | +0.3% 1T, "neutral" per merge | lanes.rs `child_node` | ok |
| 20e | m6 resolve (twin stored only when X passes, signed max, solved test at bound 0) | 82b291e | +4.6% 48T, +0.9% 1T | lanes.rs `resolve_other`, `max_child_bound`, `resolve_moved` | ok |
| 21 | c7: (job, root child) pairs ordered across jobs | 21e3ca9 (26af596) | -6..-8% nodes, +4.4% 1T, +0.5% 48T (merge) | lanes.rs `start_iteration` | ok |
| 21a | c7: +1 code step on faces 1, 3, 5 | 21e3ca9 | -0.6% nodes | lanes.rs `quantize18` | ok |
| 21b | c7: tree sum of the child statistics | 21e3ca9 | a sequential sum took 2% of the samples (profile; no A/B) | lanes.rs `sum18` | added |
| 21c | c7: whole run's ChildOrder written without reading it | 21e3ca9 | reading it took 2-3% of the samples (profile) | lanes.rs `finish_run` | added |
| 21d | c7: cold `next_run`, job prefetch in `setup_run`, normal-side roots | 21e3ca9 | part of c7, not measured separately | lanes.rs `next_run`, `setup_run`, `setup_job` | ok |
| 21e | c7: run state boxed, off the lane's hot lines | 21e3ca9 | part of c7, not measured separately | lanes.rs `Lane::run_state` | added (attribution) |
| 22 | 24 lanes per thread | 0b35690 | +3.3% 48T, +9.7% 1T vs 20 (on c7: +2.2%, +6.0%) | lanes.rs `DEFAULT_LANES` | ok |
| 23 | c8: unseen-child code + rest by best code | abce87c | -1.0% nodes (each ~-0.5%), +0.6% 48T, 1T within noise | lanes.rs `finish_run`, `build_rest` (doc and inline) | fixed: commit name and basis added; the doc's "measured 0.5% fewer nodes" reworded |
| 24 | s8 get_unchecked at the index; `resolve_other` keeps its hint | b608890 (c9a1c4d) | 1T layout-averaged neutral; get_unchecked in resolve_other -1.6% share 1T | lanes.rs `resolve_other` (the negative is documented) | ok |
| 25 | m7 µop stack: lazy child cube, blsmsk count, move tables, early stores, vpternlog fold | f8a4919 | +1.5% 1T averaged over code offsets (+1.53% median, 12 pairs), 48T even | lanes.rs `advance` (5 sites); coord.rs `edge_parts_sym_raw` | ok |
| 26 | m8 unsafe pass (unchecked move tables, run and job paths) | 33a8042 (7608a28) | alone on 857a0c6: +1.5% 1T, +0.4% 48T; with 95cbb11 on def2cd0: +3.2% 1T, +1.3% 48T, -3.3% instructions/node | lanes.rs `advance` SAFETY block | fixed: the two-commit numbers were given as the unsafe pass's; both now, with their bases |
| 27 | HOT_TABLES aligned to 4 KiB | a546429 | neutral (+0.2% 1T, +0.4% 48T), layout pin | simd.rs `HotTables` ("neutral on its own") | ok |
| 28 | hot-loop placement (pads) | 1c31eca | not a speedup (pins the layout); share pad 0 by 1T share +1.7 +0.1 +0.0% | hot.rs module doc, `SHARE_PAD` | ok |
| 28a | stream pad 16 kept by m8's sweep | 95cbb11 | 1T vs main, 2 pairs per pad: 0 -0.3 +0.6%, 16 +4.0 +0.5%, 32 -0.1 +1.2%, 48 +2.4 +1.3% | hot.rs `STREAM_PAD` | added (SHARE_PAD had its sweep, STREAM_PAD only the 735dd25 one) |
| 29 | symmetric start | 857a0c6 | timing set 22.8x 1T, 7.9x 48T; stream 8 threads 28.9x, -96.6% nodes; recursive -29% nodes | symstart.rs module doc; lanes.rs `extend_jobs`; solver.rs `sym_start` | ok |
| 30 | build folder out of the layout | f2cb1f4 | layout only (+0.9% 1T was placement) | tables.rs `BUILD_DIR` (explains why) | skipped: robustness (measurement) |
| 31 | bench's mode threads compiled apart | 595e424 | neutral (measurement robustness) | bench.rs `share_thread` doc (ok), comment in `run` | fixed: the comment in `run` still said "the hot loop is the spawned closure, with run_lanes and Pool::run inlined" |
| 32 | size asserts | aee1645 | no code | lanes.rs size asserts | skipped: robustness |

## Share mode (batch and latency)

| # | change | commit (merge) | measured gain, basis | code site now | status |
|---|---|---|---|---|---|
| 40 | the work-sharing mode (claims, staged builds, splits, donations) | 82ff5c8 (78ce33e) | 48T batch 4.3x, random_state 7.86 -> 1.26 ms (6.3x), one cube 306 -> 3.6 ms | share.rs module doc, claim word, `build_task`, `split`, `claim` | ok |
| 40a | module doc summary of later latency work | c10, c11, m8 | -c 1 ~2.2 -> ~1.6 ms at 48T (m8 1.609, r8 1.622, g1 1.634 ms on random_state) | share.rs module doc | fixed: stopped at l6 (~2.2 ms) |
| 41 | l6-share-a: staged builds on all threads, no fresh pages, spin RW lock, waiting removal, stats from atomics, infeasible jobs not claimed | 4088803 (fcc8d7b) | -c 1 -38.8% at 48T (stack) | share.rs `JOBS_CAP`, `SpinRw`, `find_open`, `open_stage`, `build`, `copy_stats`, `finish_build`, `build_task` | ok |
| 41a | spin wait yields | 4088803 | a fix, not a gain | share.rs `spin_wait` | ok |
| 42 | l6-share-2: kept cubes, staged builds of few cubes, fixed mailbox slot, claim prefetch, padding | 47b5da2 (36668cc) | series -5.6% -c 1, +4.0% bench share; screening ablations | share.rs `keep`, `build`, `Mailbox`, `claim`, `Pad` | ok |
| 42a | solver as a parameter of the round loop | 1235cdd | -3.0% -c 0, -2.6% -c 1, +3.8% bench share at 48T; -2.0% -c 0 1T | share.rs `Worker::rounds` | ok |
| 42b | no early 4-move prefixes | c5d7705 | -8.0% -c 1 | share.rs `build` | ok |
| 43 | job statistic from per-child sums | 21e3ca9 | part of c7 | share.rs `end_job` | ok |
| 44 | trace buffered, counters behind bits | 33ca449 | trace cost: counters +35% -c 1, printing at once 2.0 -> 2.9 ms | share.rs `TRACE_*` bits, `Pool`'s trace-lines field | ok (+35% is the counters' step from `CUBE_TRACE=1` to 3: 2.053 -> 2.786 ms) |
| 44a | `trace_found` cold, out of line | 33ca449 | inlined: untraced 48T -c 1 +3.7%, -c 0 +4.1%, bench share -4.2% | share.rs `trace_found` | added |
| 45 | first pass for the iterations up to 16 | 085a9fd | -7.4% -c 1, -7.1% -c 0, +2.0% bench share; 15/16/17-move -18/-43/-22% | share.rs `build` | ok |
| 46 | TAIL_LANES | 1f4c85a | -3.6% -c 1 | share.rs `split` | ok |
| 47 | drop solved lanes first, refill together | 9e69243 | -5.1% -c 1 | share.rs `cancel_done` | ok |
| 48 | first pass without sort | 5cce44d | -1.9% -c 1; 15/16-move -16/-11%, depth-15/16 -20/-28% | share.rs `first_pass_order` | ok |
| 49 | touched buffers, WARM_BUFFERS | 8926c95 | -8.1% 17-move, -9.6% depth-17 (3 holds), -0.7% random states (7 holds) | share.rs `touched` | fixed: hold counts (all were given as 7) |
| 50 | split iterations (top-1 lead groups) | def2cd0 | -2.9% -c 1, nodes -6.8%; replay figures for `LEAD_FRACTION` | share.rs `plan_leads`, `LEAD_FRACTION` | ok (the replay's 20% row is from c11's own replay, not in the commit message) |
| 51 | symmetric start's parallel extension with `keep` | 857a0c6 | -9.9% -c 1 vs serial (7 symmetric positions) | share.rs `build` | ok |
| 52 | m8: rounds by pointer, share pad 16 | 95cbb11 (7608a28) | both m8 commits on def2cd0: share +2.5% 1T, -c 1 -2.1%, -c 0 -2.5%, bench share +2.5% | share.rs `rounds`; hot.rs `SHARE_PAD` | ok |
| 52a | m8: unchecked slots, indexes and divisions on the run/job paths | 33a8042 | part of m8 (measured with #52) | share.rs `joined` | added (attribution) |
| 53 | parking_lot Mutex | bb07598 (4c74267) | +0.8% bench share 48T, +1.3% 1T share, -c 1 noise | share.rs Mutex import comment | ok |
| 54 | CachePadded for Pad | 2f4fb7a | same code, neutral | share.rs `Pad` | ok |
| 55 | s9 page pin of solve's share mode | 508cbe5 (ca820b6) | neutral vs 0d1a0e8 (+0.5 / -1.0%); at the merge -1.35 / -1.2 / -0.5% -c 1 vs 4c74267 | share.rs `run_pinned`, solve.rs `SHARE_LINE` | added: the merge's numbers (kept for layout pinning) |
| 56 | r8 fixes (pass race, release flake, Worker::new frame note) | 9be649d, 1571c47 | robustness; Worker::new frame: -c 1 +8% | share.rs `Pool::run` note | skipped: robustness (frame note was ok) |
| 57 | l6 hang fix, r7 join fix | 6f033a4, f947454 | robustness | share.rs `refill`, `fresh` | skipped: robustness |

## Generation, laptop, wasm, scripts, cold paths

| # | change | commit (merge) | measured gain, basis | code site now | status |
|---|---|---|---|---|---|
| 60 | any-k DFS generator; 32-deep prefetched write ring | 673d49e | feature; ring not measured alone | gen/dfs.rs `Ring` | added "(not measured separately)" |
| 61 | hashbrown set in the DFS generator | 31525f1 | gen 2 8 -26% (8 threads), gen 0 7 --dfs -32% | gen/dfs.rs `Prefix` | ok |
| 62 | 40 lanes up to h8, 32 for h9/h10 | fe926ed | h7 +10% 1T, +10.6% 8T; h8 +8.9% 8T; h6 +4.5% 1T; h11 stays 24 (40: -3.4% 8T) | tables/file.rs `untuned_lanes` | fixed: h8 said +8.6% for 26.73 -> 29.10 (+8.9%) |
| 63 | default base per k | fe926ed | h6 base 9: +15-20% vs base 10; h7 tie | tables.rs `default_base` | ok |
| 64 | portable backends, wasm SIMD128 | 84a4aa1, 7eaede8 | features; x86 neutral | - | skipped: not a speedup |
| 65 | wasm: a load in place of a prefetch | 7eaede8 | wasmtime +53%, V8 +45% (h6, 1T) | prefetch.rs `read_once` | ok |
| 66 | wasm module: 16 lanes | 876a97e | 8 lanes +62% vs 1 (200.6 -> 123.8 s), no gain beyond | wasm/src/lib.rs `lanes_for` | added % (had times only) |
| 67 | V8 warm-up | 876a97e, ca31e63 | +45% on V8 (187.7 -> 129.5 s); none on Bun | wasm/cubeopt.mjs `warmUp` | added % (had times only) |
| 68 | wasm threads hang fix, Bun port | 2a0574f, ca31e63 | robustness / tooling | - | skipped |
| 69 | PGO build of solve | 66c4f03 (0d1a0e8) | 1T solve -c 1 -2.1% (8/8 pairs), 48T neutral | scripts/pgo-solve.sh header | added |
| 70 | g1 cold-path idioms | 6990603..43fc26b | allocations only (solve -c 1 25.7 -> 6.5 per cube), timing neutral | cubie.rs `format_moves`, lanes.rs `reconstruct` (allocation counts) | ok |
| 71 | u1 CLI crates, solve --fast | b6d7821, 80620c4 | neutral; feature | - | skipped |
| 72 | early baseline techniques (side choice, quarter turns on tight bounds, inverse by addition chain, hugetlbfs, BFS direction switch) | ec7e133 | not measured separately | solver.rs `search`, simd.rs `inverse`, tables.rs `map_file`, gen.rs module doc | ok ("not measured separately") |

## Numbers not found, and caveats

- No change in main lacks a number where one was measured. The ones
  without an A/B say "not measured separately" (first-version techniques,
  c7's bookkeeping, the DFS write ring).
- c7's tree sum and its ChildOrder write-without-read only have profile
  shares (2% and 2-3% of the samples), and the comments say so: they are
  not speedups.
- m1's four, m3's three, skip-sibling's three, m7's five pieces and l6's
  pieces were measured only as stacks, and the comments say "part of".
- Two existing figures are not literally in the commit messages, but they
  follow from them or come from the agent's own data, so I left them:
  share.rs `TRACE_*` "+35%" (the counters' step, CUBE_TRACE=1 -> 3, is
  +29..36% in 33ca449's table), and the `LEAD_FRACTION` replay's "20%"
  row (c11's replay output; the commit lists 10/30/50/70%).
- m4's per-patch gains have no thread count in their source (agent A/Bs);
  the comments give them as "part of m4-micro's +7.7% at 48T", as before.

## Counts

Rows: 83. ok 54, added 13, fixed 8, skipped 8 (superseded 1, robustness or
measurement 5, features 2).

## Checks (comments only)

- Every changed line is a comment (`git diff -U0`: all `+`/`-` lines start
  with `//`, `///`, `//!`, `#` (shell) or `*` (JS doc)); `cargo fmt --check`
  passes (crate and wasm/).
- bench, solve and gen of 18c4a66 and of the patch, both built in
  /data/agents/an1/repo (same folder, so the same `BUILD_DIR` bytes),
  `objcopy --only-section` + `cmp`:

  | binary | .text | .rodata | other differences |
  |---|---|---|---|
  | bench | identical (1004145 B) | identical (126138 B) | .debug_line; 67 u32 words of .data.rel.ro (panic-location line numbers, +2/+6/+7) |
  | solve | identical (955243 B) | identical (126194 B) | the same |
  | gen | identical (993814 B) | identical (114008 B) | none: the file is byte-identical |

- `check.sh` with /data/tools/bin on PATH, under `timeout -s KILL 1800` and
  locked.sh: exit 0, `CHECK OK` (x86 tests, wasm32 library tests under
  wasmtime, `bun wasm/test.mjs` for both modules, scalar and NEON library
  tests).
- No benchmarks (comments only).
- Coordination: comment-only edits in lanes.rs and share.rs (xcr's area),
  so they rebase trivially.
