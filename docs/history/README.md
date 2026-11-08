# Experiment history

[Documentation home](../README.md) · [Performance guide](../performance/README.md)

This archive preserves **478 surviving commit-message records**, including their before/after tables and explanations. Each record names its source commit and date. A commit reachable from the release is history, not necessarily a separate optimization that still applies unchanged.

- [Negative and neutral experiments](NEGATIVE_EXPERIMENTS.md): 151 retained negative/neutral branch-tip records.
- [Archived experiment branches](ARCHIVED_EXPERIMENTS.md): all 59 archived tips.
- [Complete searchable CSV](commit-index.csv) and [JSON](commit-index.json): all surviving records and references recorded at collection.
- [Technique inventories](../reference/inventories/README.md): guided reviews of competing solvers and optimization opportunities.

These are dated experiment notes. Older “pending” statements, test counts and source paths describe the historical state. Use the comparison and safety guides for the measured source scope. Percentages use different baselines and workloads and are not additive. Some records contain only a diagnostic or an untested hypothesis; they are retained as such, not turned into measured wins. The two specifically excluded backend timing comparisons were removed from their historical messages; their remaining evidence is preserved.

## History reachable from the release

| Date | Commit | Topic |
|---|---|---|
| 2026-11-02 | [`ec7e133a181b`](commits/ec7e133a181b.md) | Baseline H48-style optimal solver |
| 2026-11-02 | [`9090310aa36a`](commits/9090310aa36a.md) | Use cube48opt's table layout: E-slice placement innermost, 16-byte groups |
| 2026-11-02 | [`046c33faf7ee`](commits/046c33faf7ee.md) | Merge opt/eslice-groups: cube48opt table layout |
| 2026-11-02 | [`dc2626676ebb`](commits/dc2626676ebb.md) | Fix clippy lint; add check.sh (fmt + clippy -D warnings + tests) |
| 2026-11-02 | [`9f9a4d3b57bf`](commits/9f9a4d3b57bf.md) | Order final-iteration traversal by previous-iteration statistics (jobs) |
| 2026-11-02 | [`7bce67d5847f`](commits/7bce67d5847f.md) | Merge opt/job-order: final-iteration job ordering |
| 2026-11-02 | [`0c48355271f0`](commits/0c48355271f0.md) | Sibling-face pruning and back-jumps from learned child bounds |
| 2026-11-02 | [`22713550ddaf`](commits/22713550ddaf.md) | Merge opt/sibling-prune: sibling-face pruning and back-jumps |
| 2026-11-02 | [`c52cb63096e2`](commits/c52cb63096e2.md) | Compute job-root bounds once per job, with batched prefetch |
| 2026-11-02 | [`31c28787884f`](commits/31c28787884f.md) | Merge opt/job-bounds: cached prefetched job bounds |
| 2026-11-02 | [`2bb148d5026e`](commits/2bb148d5026e.md) | Lane-interleaved explicit-stack IDA* for throughput; streaming benchmark |
| 2026-11-02 | [`a0681ad582cc`](commits/a0681ad582cc.md) | Merge opt/lanes: lane-interleaved search and streaming benchmark |
| 2026-11-02 | [`0b65adffa572`](commits/0b65adffa572.md) | Share the other-side lookup between the X and X' children of a face |
| 2026-11-02 | [`d2674d587346`](commits/d2674d587346.md) | Merge opt/twin-lookup: share X/X' other-side lookups |
| 2026-11-02 | [`aa43192f11bc`](commits/aa43192f11bc.md) | Steeper job statistic: sum of 8^(16-h) instead of 2^(12-h) |
| 2026-11-02 | [`d63535bf026b`](commits/d63535bf026b.md) | Merge opt/stat-weight: steeper job statistic |
| 2026-11-02 | [`5cea944d0141`](commits/5cea944d0141.md) | Single-pass round-robin lane scheduler; 16 lanes by default |
| 2026-11-02 | [`59ce1454e979`](commits/59ce1454e979.md) | Merge opt/single-pass: single-pass lane scheduler |
| 2026-11-03 | [`42a7c98a66a2`](commits/42a7c98a66a2.md) | Edge-only symmetry conjugation for the table index |
| 2026-11-03 | [`ae4fc49e9479`](commits/ae4fc49e9479.md) | Merge opt/edge-conj: edge-only symmetry conjugation (+5.5%) |
| 2026-11-03 | [`f99ebc4a84ba`](commits/f99ebc4a84ba.md) | Packed 3-byte cocsep entries on a huge page; shorter index chain |
| 2026-11-03 | [`3957bbc75f44`](commits/3957bbc75f44.md) | Merge opt/packed-cocsep: packed cocsep on a huge page (+2.8%) |
| 2026-11-03 | [`ebd6c20eee78`](commits/ebd6c20eee78.md) | Prefetch table lines with the NTA hint |
| 2026-11-03 | [`5b0f6ab45269`](commits/5b0f6ab45269.md) | Merge opt/prefetch-nta: NTA prefetch hint (+2.6%) |
| 2026-11-03 | [`e9d751ece525`](commits/e9d751ece525.md) | Index-path micro-optimisations: cocsep LUT, vpmovmskb, lighter compose, cached table pointer |
| 2026-11-03 | [`6d265ddf8ddf`](commits/6d265ddf8ddf.md) | Merge opt/m1-micro: index-path micro-optimisations (+2.2%) |
| 2026-11-03 | [`d708f1c713c7`](commits/d708f1c713c7.md) | Job handling: radix sort, shared inverse lookups, pipelined extension, job prefetch |
| 2026-11-03 | [`814ae96a6977`](commits/814ae96a6977.md) | Merge opt/m2-jobs: faster job handling (+4.3%) |
| 2026-11-03 | [`042f13c8e83d`](commits/042f13c8e83d.md) | Remove bounds checks from the lane hot loop |
| 2026-11-03 | [`fe587403e381`](commits/fe587403e381.md) | Merge opt/m2-nobc: no bounds checks in the lane loop (+1.1%) |
| 2026-11-03 | [`7a173a125caa`](commits/7a173a125caa.md) | Eager sibling-face pruning, X' skip mask, branch-free child setup |
| 2026-11-03 | [`b90b706ac23a`](commits/b90b706ac23a.md) | Merge opt/skip-sibling: branch-free sibling pruning (+13.6%) |
| 2026-11-03 | [`8b2cf71011e3`](commits/8b2cf71011e3.md) | Load the pending value before the phase dispatch; select po; one static for hot tables |
| 2026-11-03 | [`70567ff0c992`](commits/70567ff0c992.md) | Merge opt/m3-hht: pending value before dispatch, hot tables in one static (+3.5%) |
| 2026-11-03 | [`45a5a0bb2d3e`](commits/45a5a0bb2d3e.md) | cargo fmt (the m3-hht merge skipped a failing fmt check) |
| 2026-11-03 | [`18ef7f037d9a`](commits/18ef7f037d9a.md) | Default to 20 lanes per thread |
| 2026-11-03 | [`ff2b7a455786`](commits/ff2b7a455786.md) | Merge opt/lanes20: 20 lanes per thread (+2.9%) |
| 2026-11-03 | [`c914d66a706f`](commits/c914d66a706f.md) | m4: lane-loop |
| 2026-11-03 | [`496b09620f52`](commits/496b09620f52.md) | m4: node-roles |
| 2026-11-03 | [`1326046beeaa`](commits/1326046beeaa.md) | m4: rare-check |
| 2026-11-03 | [`60153a79f756`](commits/60153a79f756.md) | m4: heur-pair |
| 2026-11-03 | [`eee343250739`](commits/eee343250739.md) | m4: index-pext |
| 2026-11-03 | [`a741d66f5c82`](commits/a741d66f5c82.md) | m4: resolve-face |
| 2026-11-03 | [`c509dd1dd342`](commits/c509dd1dd342.md) | m4: phase-encoding |
| 2026-11-03 | [`35c608c3d887`](commits/35c608c3d887.md) | Merge opt/m4-micro: micro round 4, 7 patches (+7.7%) |
| 2026-11-03 | [`431e46a687d6`](commits/431e46a687d6.md) | lanes: keep the depth and the top node's bounds in registers in advance |
| 2026-11-03 | [`236bf33e883d`](commits/236bf33e883d.md) | lanes: generate a pushed child's first child from registers |
| 2026-11-03 | [`ee186533eabb`](commits/ee186533eabb.md) | lanes: no Idle phase |
| 2026-11-03 | [`68dfaf653ca6`](commits/68dfaf653ca6.md) | lanes: count advance's nodes in a local |
| 2026-11-03 | [`9caf0e2b186a`](commits/9caf0e2b186a.md) | Merge opt/p5-sched: hot-loop latency, 4 patches (+3.2% 48T, +1.6% 1T) |
| 2026-11-03 | [`82ff5c811694`](commits/82ff5c811694.md) | lanes: work-sharing batch and latency mode (lanes::share) |
| 2026-11-03 | [`4911370d388a`](commits/4911370d388a.md) | bench: share[:C] option for the work-sharing mode |
| 2026-11-03 | [`5c3efa92b940`](commits/5c3efa92b940.md) | solve: optimal solver CLI for cube_solver_test-style input |
| 2026-11-03 | [`78ce33e418f4`](commits/78ce33e418f4.md) | Merge opt/l5-share: work-sharing batch/latency mode and solve CLI |
| 2026-11-03 | [`ecd311e753f1`](commits/ecd311e753f1.md) | lanes: drop Node::fmax |
| 2026-11-03 | [`26b4f4c546d1`](commits/26b4f4c546d1.md) | lanes: small tables in TABS |
| 2026-11-03 | [`85e9c794ca12`](commits/85e9c794ca12.md) | index path: fewer µops per lookup index |
| 2026-11-03 | [`621b70a2eabb`](commits/621b70a2eabb.md) | lanes: skip count with bzhi; test the live moves directly |
| 2026-11-03 | [`d2a3c2ea1c1f`](commits/d2a3c2ea1c1f.md) | lanes: child node in moved/other terms |
| 2026-11-03 | [`82b291e92cc1`](commits/82b291e92cc1.md) | lanes: fewer µops in the resolve and advance paths |
| 2026-11-03 | [`cb8cb70f3172`](commits/cb8cb70f3172.md) | Merge opt/m6-micro: micro round 6, 6 patches (+8.0% 48T, +6.1% 1T) |
| 2026-11-03 | [`6f033a4c7b2a`](commits/6f033a4c7b2a.md) | share: count down a lane's ended jobs when it takes a piece |
| 2026-11-03 | [`4088803efe28`](commits/4088803efe28.md) | share: builds in stages use all threads, and no fresh pages |
| 2026-11-03 | [`f947454a3ff0`](commits/f947454a3ff0.md) | share: join a fresh cube before publishing it |
| 2026-11-03 | [`7853462a5751`](commits/7853462a5751.md) | miri: run the unit tests under Miri; miri.sh |
| 2026-11-03 | [`b2f4b136117d`](commits/b2f4b136117d.md) | tests: hot index path, conjugates in every mode; cross-check tools |
| 2026-11-03 | [`8953929ecbb0`](commits/8953929ecbb0.md) | tables: check the table meta and the file size when opening a table |
| 2026-11-03 | [`b84fb7c3482c`](commits/b84fb7c3482c.md) | tables: map table files opened for solving read-only |
| 2026-11-03 | [`fcc8d7ba99e7`](commits/fcc8d7ba99e7.md) | Merge opt/l6-share-a: share-mode hang fix, parallel staged builds (-39% latency) |
| 2026-11-03 | [`47b5da2fc3df`](commits/47b5da2fc3df.md) | share: few cubes in progress; data structures |
| 2026-11-03 | [`1235cdd1c725`](commits/1235cdd1c725.md) | share: the solver as a parameter of the visit loop |
| 2026-11-03 | [`c5d7705b542d`](commits/c5d7705b542d.md) | share: no early 4-move prefixes |
| 2026-11-03 | [`f0efcde8d471`](commits/f0efcde8d471.md) | Merge opt/r7-series: share race fix, table hardening, Miri, stress tools |
| 2026-11-03 | [`21e3ca9a6cf9`](commits/21e3ca9a6cf9.md) | lanes: order root children across jobs in the iterations on unchanged jobs (c7, v2) |
| 2026-11-04 | [`36668cca0f38`](commits/36668cca0f38.md) | Merge opt/l6-share-2: share mode with few cubes in progress (-5.6% latency, +4.0% share batch) |
| 2026-11-04 | [`26af596fab44`](commits/26af596fab44.md) | Merge opt/c7-childsplit: root children ordered across jobs (-6..-8% nodes, +4.4% 1T) |
| 2026-11-04 | [`0b3569038929`](commits/0b3569038929.md) | lanes: 24 lanes per thread by default |
| 2026-11-04 | [`2203a07f0326`](commits/2203a07f0326.md) | check.sh: a private test log per run |
| 2026-11-04 | [`f2cb1f407d93`](commits/f2cb1f407d93.md) | tables: binary layout independent of the build folder |
| 2026-11-04 | [`595e42439cde`](commits/595e42439cde.md) | bench: each mode's thread code compiled on its own |
| 2026-11-04 | [`abce87ca68e0`](commits/abce87ca68e0.md) | lanes: code for root children without statistics, rest phase by the job's best code (c8) |
| 2026-11-04 | [`d13944f66f91`](commits/d13944f66f91.md) | share: typed pointers for the build tasks' buffers |
| 2026-11-04 | [`71d69aef4e3c`](commits/71d69aef4e3c.md) | share: names, docs and pedantic lints |
| 2026-11-04 | [`55e048ead00e`](commits/55e048ead00e.md) | share: the measured gain of each optimization in its comment |
| 2026-11-04 | [`b2f2c9a3dd1f`](commits/b2f2c9a3dd1f.md) | lints: clippy pedantic, std idioms instead of C idioms |
| 2026-11-04 | [`301115f53529`](commits/301115f53529.md) | cubie, coord: descriptive names and docs |
| 2026-11-04 | [`44530928bf5d`](commits/44530928bf5d.md) | simd, tables, gen, lib: descriptive names and docs |
| 2026-11-04 | [`ece816ad95c0`](commits/ece816ad95c0.md) | solver: descriptive names, docs, search split into steps |
| 2026-11-04 | [`854dfe834565`](commits/854dfe834565.md) | bins, tests, examples: descriptive names and docs |
| 2026-11-04 | [`3cc6ee3f7ea4`](commits/3cc6ee3f7ea4.md) | lanes: descriptive field and item names |
| 2026-11-04 | [`3f7054d21503`](commits/3f7054d21503.md) | lanes: descriptive local names |
| 2026-11-04 | [`ab5d21a67f5b`](commits/ab5d21a67f5b.md) | lanes: docs |
| 2026-11-04 | [`b60889076b2f`](commits/b60889076b2f.md) | lanes: get_unchecked at the index instead of assert_unchecked hints |
| 2026-11-04 | [`dda14e259d97`](commits/dda14e259d97.md) | annotations: the measured gain of each optimization, inline |
| 2026-11-04 | [`c9a1c4ddfcc0`](commits/c9a1c4ddfcc0.md) | Merge work/s8: readability sweep (names, docs, clippy pedantic, std idioms, get_unchecked, gain comments) |
| 2026-11-04 | [`735dd254c583`](commits/735dd254c583.md) | share: the two gain comments missing from l6-share-3 |
| 2026-11-04 | [`33ca4494589e`](commits/33ca4494589e.md) | share: trace lines buffered, costly counters behind CUBE_TRACE bits |
| 2026-11-04 | [`085a9fd342e7`](commits/085a9fd342e7.md) | share: one barrier-free first pass for the iterations up to 16 |
| 2026-11-04 | [`1f4c85a0983f`](commits/1f4c85a0983f.md) | share: in iteration tails, a thread splits its own lanes only while it has at most 8 active |
| 2026-11-04 | [`9e692430e34f`](commits/9e692430e34f.md) | share: after a solved cube, drop its lanes first, then refill them together |
| 2026-11-04 | [`5cce44d59af6`](commits/5cce44d59af6.md) | share: a first pass claims its jobs in extension order (no sort) |
| 2026-11-04 | [`1c31eca1d56e`](commits/1c31eca1d56e.md) | hot: the stream and share loops start at fixed offsets of a 64-byte line |
| 2026-11-04 | [`f8a4919d0675`](commits/f8a4919d0675.md) | lanes: fewer µops per generated child (lazy child cube, blsmsk count, move tables, early stores, index tweaks) |
| 2026-11-04 | [`673d49e5d6f8`](commits/673d49e5d6f8.md) | tables, gen: any table size h0..h11 generated directly; self-describing table files |
| 2026-11-04 | [`84a4aa1e2d37`](commits/84a4aa1e2d37.md) | backends: portable cube (NEON, scalar; wasm ground) next to the unchanged x86 one |
| 2026-11-04 | [`fe926ed60a20`](commits/fe926ed60a20.md) | laptop: lanes per table size, README "Running on a laptop" |
| 2026-11-04 | [`a5464296eea9`](commits/a5464296eea9.md) | simd: hot tables aligned to a 4 KiB page |
| 2026-11-04 | [`857a0c650407`](commits/857a0c650407.md) | symstart: one job per class of equivalent prefixes for symmetric inputs |
| 2026-11-04 | [`8926c95e79a0`](commits/8926c95e79a0.md) | share: job and build buffers touched when allocated (no page faults while a pool warms up) |
| 2026-11-04 | [`def2cd0a16a5`](commits/def2cd0a16a5.md) | share: the best root-child groups of 10% of the jobs run first in the likely last iterations (top-1 split) |
| 2026-11-04 | [`7eaede8caef6`](commits/7eaede8caef6.md) | wasm: SIMD128 backend; the library runs in WebAssembly (WASI, with and without threads) |
| 2026-11-04 | [`876a97edb90a`](commits/876a97edb90a.md) | wasm: the WebAssembly module (wasm/) for browsers and Node; README "WebAssembly" |
| 2026-11-04 | [`89771347be28`](commits/89771347be28.md) | Merge port/wasm: the solver compiles to WebAssembly (SIMD128 backend, wasm/ module for Node and browsers) |
| 2026-11-04 | [`33a80428ab43`](commits/33a80428ab43.md) | lanes, share: no bounds checks, unwraps or division checks on the search, run and job paths (unsafe pass) |
| 2026-11-04 | [`95cbb11bf2a7`](commits/95cbb11bf2a7.md) | share: the round loop visits the lanes by pointer; share pad 16 |
| 2026-11-04 | [`7608a28f10c4`](commits/7608a28f10c4.md) | Merge opt/m8-unsafe: unchecked search/run/job paths, share rounds by pointer (share pad 16) |
| 2026-11-05 | [`2a0574ffd0fe`](commits/2a0574ffd0fe.md) | wasm: threads build hang fixed (wasi-libc init, threads that never exit, memory that never grows); wasm steps of check.sh time-limited |
| 2026-11-05 | [`3a9a50a1c658`](commits/3a9a50a1c658.md) | Merge port/wasm-hangfix: wasm threads build no longer hangs (wasi-libc init, persistent thread pool, fixed memory) |
| 2026-11-05 | [`9be649d70d6a`](commits/9be649d70d6a.md) | share: a first pass reports no longer solution while it reports its shortest one |
| 2026-11-05 | [`1571c4758aca`](commits/1571c4758aca.md) | share: cancel_done counts down the jobs it leaves with nothing pending (release flake) |
| 2026-11-05 | [`c620d6985f72`](commits/c620d6985f72.md) | check.sh: under qemu, the /proc/self/maps test runs alone; failed greps stop the script |
| 2026-11-05 | [`a7ce28f4a477`](commits/a7ce28f4a477.md) | Merge fix/r8-share: first pass never reports a longer solution; release flake fixed; qemu-safe NEON check |
| 2026-11-05 | [`ca31e63a31f9`](commits/ca31e63a31f9.md) | wasm: Bun instead of Node for the JavaScript tooling (Bun 1.4.4) |
| 2026-11-05 | [`585b3b651afe`](commits/585b3b651afe.md) | Merge port/bun: the wasm/ JavaScript tooling runs on Bun instead of Node (user request) |
| 2026-11-05 | [`89dca7b49811`](commits/89dca7b49811.md) | fast.sh: the fastest optimal solve of one random scramble (largest table, all threads on one cube) |
| 2026-11-05 | [`66c4f038d70a`](commits/66c4f038d70a.md) | scripts/pgo-solve.sh: profile-guided build of `solve` (1T -2.1% ms/cube; 48T -c 1 neutral) |
| 2026-11-05 | [`aee16458e05a`](commits/aee16458e05a.md) | lanes: compile-time size checks of Node, Job and ChildOrder (x86-64) |
| 2026-11-05 | [`0d1a0e8c4307`](commits/0d1a0e8c4307.md) | Merge opt/pb1: Rust Performance Book audit results (optional PGO build of solve, size checks) |
| 2026-11-05 | [`2f4fb7a0b16a`](commits/2f4fb7a0b16a.md) | share: Pad is crossbeam's CachePadded |
| 2026-11-05 | [`bb0759857a64`](commits/bb0759857a64.md) | share: parking_lot's Mutex for the share mode's mutexes |
| 2026-11-05 | [`31525f16bedd`](commits/31525f16bedd.md) | gen: hashbrown's HashSet for the dfs prefix set |
| 2026-11-05 | [`4c74267c73d1`](commits/4c74267c73d1.md) | Merge opt/d1-crates-v2: crates.io swaps where measured >= neutral (user request) |
| 2026-11-05 | [`508cbe503004`](commits/508cbe503004.md) | solve: share workers' state and frames and the round loop at fixed page offsets |
| 2026-11-05 | [`ca820b6c2a32`](commits/ca820b6c2a32.md) | Merge opt/s9-pin-v2: solve's share mode at fixed page offsets (4K-aliasing-proof layout) |
| 2026-11-05 | [`6990603a918d`](commits/6990603a918d.md) | tables, cubie: Box<[T]> for the fixed-size cocsep tables and the symmetry list |
| 2026-11-05 | [`55bca493c5ae`](commits/55bca493c5ae.md) | tables::file, cubie: with_capacity at known sizes |
| 2026-11-05 | [`0d6a899f7a71`](commits/0d6a899f7a71.md) | symstart, cubie: fixed-size arrays instead of temporary Vecs |
| 2026-11-05 | [`a5c7a6ffc17a`](commits/a5c7a6ffc17a.md) | lanes: one allocation per solution in Lane::reconstruct |
| 2026-11-05 | [`43fc26bf8714`](commits/43fc26bf8714.md) | cubie, build.rs: format_moves writes one String |
| 2026-11-05 | [`0c4bfe2e1f64`](commits/0c4bfe2e1f64.md) | Merge opt/g1-idioms: common Rust idioms on cold paths (user request) |
| 2026-11-05 | [`b9fc895f8604`](commits/b9fc895f8604.md) | tables, cubie: thiserror error types for table files and facelets |
| 2026-11-05 | [`b6d7821e3c79`](commits/b6d7821e3c79.md) | binaries: clap, anyhow, log/env_logger, --json, progress bars, completions |
| 2026-11-05 | [`760ad75fb82c`](commits/760ad75fb82c.md) | benches: criterion micro-benchmarks of the kernels (feature `bench`) |
| 2026-11-05 | [`229b99fa9491`](commits/229b99fa9491.md) | Merge opt/u1-cli: clap, anyhow, thiserror, log/env_logger, --json, indicatif, clap_complete, criterion (user request) |
| 2026-11-05 | [`80620c47fba3`](commits/80620c47fba3.md) | solve --fast: fast.sh as a flag (largest table, all threads, warm-up, solve time) |
| 2026-11-05 | [`18c4a66142bc`](commits/18c4a66142bc.md) | Merge opt/u1-fast: solve --fast replaces fast.sh (user request) |
| 2026-11-05 | [`7f4950de2832`](commits/7f4950de2832.md) | comments: measured gains at every notable optimization, checked against git history |
| 2026-11-05 | [`0b5b5a6f23a3`](commits/0b5b5a6f23a3.md) | Merge docs/an1-annotations: inline measured gains at every notable optimization (user request) |
| 2026-11-05 | [`2403aa31344f`](commits/2403aa31344f.md) | gen, solve, bench: check disk and memory before the run, not deep into it |
| 2026-11-05 | [`10977c2742cc`](commits/10977c2742cc.md) | Merge opt/u1-preflight: gen/solve/bench check disk and memory before the run (user request) |
| 2026-11-05 | [`ffd23945c9a0`](commits/ffd23945c9a0.md) | tables: shared memory segments (--shm), as vcube's (user request) |
| 2026-11-05 | [`4bf1b5ed1914`](commits/4bf1b5ed1914.md) | Merge opt/shm1: tables in System V shared memory (--shm), like vcube (user request) |
| 2026-11-05 | [`ae25e0a20fed`](commits/ae25e0a20fed.md) | cubie: the cube model's constants from build.rs, a Move type; no panics in the model |
| 2026-11-05 | [`ae4912557e86`](commits/ae4912557e86.md) | hot: the pad checks are compile-time assertions in the pad constants |
| 2026-11-05 | [`3eff15740b19`](commits/3eff15740b19.md) | tables, gen, parallel: errors instead of panics |
| 2026-11-05 | [`db87483080a3`](commits/db87483080a3.md) | lanes, share, solver, bins, wasm: give up past 20 moves instead of asserting; no panics |
| 2026-11-05 | [`2627ad3906f2`](commits/2627ad3906f2.md) | enforce no panics: crate lints, a scan for asserts in check.sh |
| 2026-11-06 | [`4b1707a7e96a`](commits/4b1707a7e96a.md) | Complete no-panic integration with shared tables and fallible workers |
| 2026-11-06 | [`e12730760288`](commits/e12730760288.md) | Merge opt/no-panics: remove intentional runtime panic paths |
| 2026-11-06 | [`a1fce13a2ee5`](commits/a1fce13a2ee5.md) | Keep stream completion reporting cold and restore its measured page offset |
| 2026-11-06 | [`935cb7fa3692`](commits/935cb7fa3692.md) | Merge opt/stream-completion: keep stream completion cold and pin placement |
| 2026-11-06 | [`0ed8002e3339`](commits/0ed8002e3339.md) | Rename project, CLI, library and WebAssembly assets to egg-solver |
| 2026-11-06 | [`cd963dc26b08`](commits/cd963dc26b08.md) | Merge chore/rename-egg-solver: complete requested naming |
| 2026-11-06 | [`af9a7e73a599`](commits/af9a7e73a599.md) | Remove scalar performance comparisons from README |
| 2026-11-06 | [`56ace9c10792`](commits/56ace9c10792.md) | Merge docs/omit-scalar-comparisons: apply report presentation preference |
| 2026-11-06 | [`447e43277f92`](commits/447e43277f92.md) | Require an unsafe contract for atomic table-memory views; make Miri failures fatal |
| 2026-11-06 | [`12bb580a8e14`](commits/12bb580a8e14.md) | Exercise staged-sort tasks concurrently in the Miri test catalogue |
| 2026-11-06 | [`93867a4597ab`](commits/93867a4597ab.md) | Backport explicit futex pointer arguments and isolate SysV test attachments |
| 2026-11-07 | [`988f289fed03`](commits/988f289fed03.md) | Parallelize independent heavy Miri test families |
| 2026-11-07 | [`316597f64a9b`](commits/316597f64a9b.md) | Close unsafe API invariants and search publication races |
| 2026-11-07 | [`842a5043abfd`](commits/842a5043abfd.md) | Merge audited memory, coordinate and search-publication safety fixes |
