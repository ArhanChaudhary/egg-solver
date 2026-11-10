# Deleted experiments

[Documentation home](../README.md)

When the git history was rebuilt for publication on 2026-11-10, the 213 experiment branches and tags that were never merged were deleted: 151 rejected or neutral experiments, 59 archived experiments and 3 other unmerged branches. Their code is not in the published history. Each row gives the experiment's name, date and one-line summary, which usually states its measured result. Results use different baselines and workloads, so percentages are not additive.

## Rejected and neutral experiments

| Experiment | Date | Summary |
|---|---|---|
| <a id="ref-neg-a2-jobstat-max"></a>`neg/a2-jobstat-max` | 2026-11-03 | a2: job statistic = best root child instead of the sum (negative, +0.3% nodes) |
| <a id="ref-neg-a2-opp-tight"></a>`neg/a2-opp-tight` | 2026-11-03 | a2: exclude the opposite face's half turn on tight bounds (negative, +-0.07% nodes) |
| <a id="ref-neg-a2-side-tiebreak"></a>`neg/a2-side-tiebreak` | 2026-11-03 | a2: side choice tie-break by the other side's bound (negative, -0.07% nodes) |
| <a id="ref-neg-a7-bolt"></a>`neg/a7-bolt` | 2026-11-03 | NEGATIVE: BOLT post-link layout (build-bolt.sh): +0.3% at 1T on 00d998c, within noise |
| <a id="ref-neg-a7-llvm-args"></a>`neg/a7-llvm-args` | 2026-11-03 | NEGATIVE: LLVM codegen flags (-C llvm-args=...): none faster at 1T |
| <a id="ref-neg-a7-lto-thin"></a>`neg/a7-lto-thin` | 2026-11-03 | NEGATIVE: lto = "thin" instead of "fat" (-1.0% at 1T) |
| <a id="ref-neg-a7-nightly"></a>`neg/a7-nightly` | 2026-11-03 | NEGATIVE: nightly toolchain (1.101, LLVM 23.1.5) and nightly -Z flags (0.0% / -0.5% at 1T) |
| <a id="ref-neg-a7-opt-level-2"></a>`neg/a7-opt-level-2` | 2026-11-03 | NEGATIVE: opt-level = 2 for the release profile (neutral at 1T on ea4da12 and 00d998c) |
| <a id="ref-neg-a7-pgo"></a>`neg/a7-pgo` | 2026-11-03 | NEGATIVE: PGO trained on the exact stream workload, with and without BOLT (-1.3..-1.5% at 1T) |
| <a id="ref-neg-a7-relocation-static"></a>`neg/a7-relocation-static` | 2026-11-03 | NEGATIVE: -C relocation-model=static (non-PIE executable) (-0.4% at 1T on ea4da12, +0.1% on 00d998c) |
| <a id="ref-neg-a7-target-cpu"></a>`neg/a7-target-cpu` | 2026-11-03 | NEGATIVE: target-cpu alternatives to native (znver5, znver4, x86-64-v4, -avx512f) |
| <a id="ref-neg-advance-v4"></a>`neg/advance-v4` | 2026-11-03 | Eager sibling pruning + branchless advance (rejected: -2%) |
| <a id="ref-neg-b2-moved-push-branchless-incr"></a>`neg/b2-moved-push-branchless-incr` | 2026-11-03 | b2: branchless moved-side resolution (negative, -2.2%) |
| <a id="ref-neg-b2-readahead-bookkeeping-dispatch"></a>`neg/b2-readahead-bookkeeping-dispatch` | 2026-11-03 | b2: read-ahead with the resolve bookkeeping and a unified dispatch (negative, -8.0%) |
| <a id="ref-neg-b2-readahead-eager-sibling"></a>`neg/b2-readahead-eager-sibling` | 2026-11-03 | b2: read-ahead with eager sibling clearing (negative, -11.0%) |
| <a id="ref-neg-b2-readahead-value"></a>`neg/b2-readahead-value` | 2026-11-03 | b2: read ahead the next lane's lookup value (negative, -4.4%) |
| <a id="ref-neg-b2-skip-cmov-guard-cumulative"></a>`neg/b2-skip-cmov-guard-cumulative` | 2026-11-03 | b2: branch-free skip guard and a single pop branch (negative, -8.1%) |
| <a id="ref-neg-b2-skip-node-ab-layout"></a>`neg/b2-skip-node-ab-layout` | 2026-11-03 | b2: node stores the chosen-side cubes, 96-byte node (negative, -2.9%) |
| <a id="ref-neg-b2-skipk-unified-resolve"></a>`neg/b2-skipk-unified-resolve` | 2026-11-03 | b2: unified resolve on top of skip-sibling (negative, -4.5%) |
| <a id="ref-neg-b2-unified-resolve"></a>`neg/b2-unified-resolve` | 2026-11-03 | b2: unified resolve without the phase dispatch (negative, -3.3%) |
| <a id="ref-neg-b2-unified-resolve-only"></a>`neg/b2-unified-resolve-only` | 2026-11-03 | b2: one resolve for both lookup kinds, advance unchanged (negative, -3.8%) |
| <a id="ref-neg-c10-dord"></a>`neg/c10-dord` | 2026-11-04 | NEGATIVE c10: dord (not merged) |
| <a id="ref-neg-c10-first-pass-order"></a>`neg/c10-first-pass-order` | 2026-11-04 | NEGATIVE c10: first-pass-order (not merged) |
| <a id="ref-neg-c10-pass-big-first"></a>`neg/c10-pass-big-first` | 2026-11-04 | NEGATIVE c10: pass-big-first (not merged) |
| <a id="ref-neg-c11-sort-chunk"></a>`neg/c11-sort-chunk` | 2026-11-04 | NEGATIVE c11: sort-chunk (not merged), plus the order-replay diagnostics |
| <a id="ref-neg-c4-canonical-whole-order"></a>`neg/c4-canonical-whole-order` | 2026-11-03 | c4: whole-job iterations in canonical job order (negative, +2.6% nodes) |
| <a id="ref-neg-c4-childorder"></a>`neg/c4-childorder` | 2026-11-03 | Per-child job ordering, paid only in iterations on unchanged jobs (c4) |
| <a id="ref-neg-c4-free-side"></a>`neg/c4-free-side` | 2026-11-03 | c4: free side choice at job roots (negative, -4.23% vs -5.74% nodes) |
| <a id="ref-neg-c4-group-caps"></a>`neg/c4-group-caps` | 2026-11-03 | c4: cap the first-part runs per job (negative, +1.88% / -4.26% vs -5.74% nodes) |
| <a id="ref-neg-c4-merge-wide"></a>`neg/c4-merge-wide` | 2026-11-03 | c4: MERGE = 8, wider first-part runs (negative, -3.85% vs -4.01% nodes) |
| <a id="ref-neg-c4-next-run-not-cold"></a>`neg/c4-next-run-not-cold` | 2026-11-03 | c4: next_run without #[cold] (negative, -2.7% on c897107, neutral on 55120b7) |
| <a id="ref-neg-c4-no-face-bias"></a>`neg/c4-no-face-bias` | 2026-11-03 | c4: no face bias in the child statistic (negative, -5.48% vs -5.76% nodes) |
| <a id="ref-neg-c4-per-job-2"></a>`neg/c4-per-job-2` | 2026-11-03 | c4: FIRST_PER_JOB = 2 (negative, -5.40% vs -5.76% nodes) |
| <a id="ref-neg-c4-per-job-4"></a>`neg/c4-per-job-4` | 2026-11-03 | c4: FIRST_PER_JOB = 4 (negative, -3.91% vs -4.23% nodes) |
| <a id="ref-neg-c4-root-child-field"></a>`neg/c4-root-child-field` | 2026-11-03 | c4: carry the job root's child in each Node (negative, no measurable change) |
| <a id="ref-neg-c4-sticky-inverse"></a>`neg/c4-sticky-inverse` | 2026-11-03 | c4: sticky inverse side for INV jobs (negative, -3.14% vs -3.87% nodes) |
| <a id="ref-neg-c7-dataset-dseval"></a>`neg/c7-dataset-dseval` | 2026-11-03 | EXPERIMENT (c7): final-iteration dataset (C7DUMP) and offline policy evaluator (dseval) |
| <a id="ref-neg-c7-free-side-inv-whole"></a>`neg/c7-free-side-inv-whole` | 2026-11-03 | NEGATIVE (c7): keep the solver's free side choice at full-length job roots; inverse-side jobs run whole |
| <a id="ref-neg-c7-jc"></a>`neg/c7-jc` | 2026-11-03 | NEGATIVE: lanes: cache the job prefix cubes in setup_job (c7) |
| <a id="ref-neg-c7-jobcache"></a>`neg/c7-jobcache` | 2026-11-03 | NEGATIVE: lanes: cache the job prefix cubes in setup_job (c7) |
| <a id="ref-neg-c7-knob-experiments"></a>`neg/c7-knob-experiments` | 2026-11-03 | NEGATIVE/EXPERIMENT (c7): the knob build behind c7-childsplit's tuning (C7K env), with all variants measured |
| <a id="ref-neg-c7-ntstore"></a>`neg/c7-ntstore` | 2026-11-03 | NEUTRAL (c7): non-temporal JobX stores in whole runs; NTA prefetch in th (patch file) |
| <a id="ref-neg-c7-overhead-probes"></a>`neg/c7-overhead-probes` | 2026-11-03 | NEGATIVE (c7): per-node overhead probes of the per-child bookkeeping (1T (patch file) |
| <a id="ref-neg-c8-predictor-study"></a>`neg/c8-predictor-study` | 2026-11-04 | NEGATIVE c8: predictor study for final-iteration ordering (not merged) |
| <a id="ref-neg-c8-share-child-order"></a>`neg/c8-share-child-order` | 2026-11-04 | NEGATIVE c8: c7's child ordering ported to share mode (not merged) |
| <a id="ref-neg-c9-claims-idle-lanes"></a>`neg/c9-claims-idle-lanes` | 2026-11-04 | NEGATIVE c9: claims sized to idle lanes (not merged) |
| <a id="ref-neg-cold-paths"></a>`neg/cold-paths` | 2026-11-03 | #[cold]/#[inline(never)] on setup_job, start_iteration, reconstruct (rejected: ~-1.5%) |
| <a id="ref-neg-d1-atomic-float"></a>`neg/d1-atomic-float` | 2026-11-05 | NEG share: job statistics as atomic_float::AtomicF32 (fetch_add for add_f32) |
| <a id="ref-neg-d1-backoff-idle"></a>`neg/d1-backoff-idle` | 2026-11-05 | NEG share: idle wait with crossbeam's Backoff |
| <a id="ref-neg-d1-backoff-spinrw"></a>`neg/d1-backoff-spinrw` | 2026-11-05 | NEG share: SpinRw waits with crossbeam's Backoff |
| <a id="ref-neg-d1-fx-dfs"></a>`neg/d1-fx-dfs` | 2026-11-05 | NEG gen: FxHashSet for the dfs prefix set (hashbrown's set adopted instead) |
| <a id="ref-neg-d1-fx-symstart"></a>`neg/d1-fx-symstart` | 2026-11-05 | NEG symstart: FxHashMap for the dedup map |
| <a id="ref-neg-d1-inspection"></a>`neg/d1-inspection` | 2026-11-05 | NEG (no code): in-house items kept after inspection (d1 crate sweep) |
| <a id="ref-neg-d1-pl-rwlock"></a>`neg/d1-pl-rwlock` | 2026-11-05 | NEG share: open list under parking_lot::RwLock instead of SpinRw |
| <a id="ref-neg-d1-radsort"></a>`neg/d1-radsort` | 2026-11-05 | NEG lanes: sort_jobs and sort_runs by radsort |
| <a id="ref-neg-d1-spin-rwlock"></a>`neg/d1-spin-rwlock` | 2026-11-05 | NEG share: open list under spin::RwLock instead of SpinRw |
| <a id="ref-neg-eager-prune"></a>`neg/eager-prune` | 2026-11-03 | Eager pruning on top of jobs+nobc (rejected: -1.3%) |
| <a id="ref-neg-g1-counters-write"></a>`neg/g1-counters-write` | 2026-11-05 | NEG share: write! instead of format! + extend in Pool::counters (+1.1 KB bench::run) |
| <a id="ref-neg-g1-generator-derived-box"></a>`neg/g1-generator-derived-box` | 2026-11-05 | NEG gen: Box<[T]> for Generator::derived (changes the BFS relabel pass) |
| <a id="ref-neg-g1-sort-unstable-trace"></a>`neg/g1-sort-unstable-trace` | 2026-11-05 | NEG share: sort_unstable_by for the trace's finish times (+7.6 KB bench .text) |
| <a id="ref-neg-grouped-scheduler"></a>`neg/grouped-scheduler` | 2026-11-03 | Grouped lane scheduler: step W lanes, then batch their index computations (rejected: -9%/-12%) |
| <a id="ref-neg-jobstat-max"></a>`neg/jobstat-max` | 2026-11-03 | Job statistic = best child instead of sum (rejected: +0.3% nodes) |
| <a id="ref-neg-k11-specialize"></a>`neg/k11-specialize` | 2026-11-03 | Compile-time k = 11 copy of the lane engine (rejected: -2%) |
| <a id="ref-neg-l6-align-all-functions"></a>`neg/l6-align-all-functions` | 2026-11-04 | NEGATIVE l6: align-all-functions (not merged) |
| <a id="ref-neg-l6-early-4move-prefixes"></a>`neg/l6-early-4move-prefixes` | 2026-11-04 | NEGATIVE l6: early-4move-prefixes (not merged) |
| <a id="ref-neg-l6-early-builds"></a>`neg/l6-early-builds` | 2026-11-04 | NEGATIVE l6: early-builds (not merged) |
| <a id="ref-neg-l6-poolrun-inline-never"></a>`neg/l6-poolrun-inline-never` | 2026-11-04 | NEGATIVE l6: poolrun-inline-never (not merged) |
| <a id="ref-neg-l6-yield-at-once"></a>`neg/l6-yield-at-once` | 2026-11-04 | NEGATIVE l6: yield-at-once (not merged) |
| <a id="ref-neg-lazy-cube"></a>`neg/lazy-cube` | 2026-11-03 | Compute only the cube needed for the next lookup (rejected: ~-1%) |
| <a id="ref-neg-m3-k11-specialize"></a>`neg/m3-k11-specialize` | 2026-11-03 | m3: compile-time k = 11 copy of the lane engine (negative, -2.1%) |
| <a id="ref-neg-m3-merged-node-count"></a>`neg/m3-merged-node-count` | 2026-11-03 | m3: merged node counting in advance (negative, -1.1%) |
| <a id="ref-neg-m3-push-select"></a>`neg/m3-push-select` | 2026-11-03 | m3: branch-free side selection in push_child (negative, -4.0%) |
| <a id="ref-neg-m4-diag-bre-incl-c6"></a>`neg/m4-diag-brE-incl-c6` | 2026-11-03 | m4: diagnostic, 50% random early-resolving branch per visit (negative, -13.2%) |
| <a id="ref-neg-m4-diag-brl-incl-c6"></a>`neg/m4-diag-brL-incl-c6` | 2026-11-03 | m4: diagnostic, 50% random branch resolving after the lookup (negative, -15.1%) |
| <a id="ref-neg-m4-diag-extra-index"></a>`neg/m4-diag-extra-index` | 2026-11-03 | m4: diagnostic, one extra index computation per generated node (negative, -13%) |
| <a id="ref-neg-m4-diag-extra-index-prefetch"></a>`neg/m4-diag-extra-index-prefetch` | 2026-11-03 | m4: diagnostic, extra index computation and prefetch per child (negative, -16.8%) |
| <a id="ref-neg-m4-fmask-branchy-incl-c6"></a>`neg/m4-fmask-branchy-incl-c6` | 2026-11-03 | m4: sibling-face masks from a table, written as if/else (negative, -4.3%) |
| <a id="ref-neg-m4-fmask-on-stack"></a>`neg/m4-fmask-on-stack` | 2026-11-03 | m4: sibling-face masks from a table with select_unpredictable (negative, neutral -0.3%) |
| <a id="ref-neg-m4-fmx"></a>`neg/m4-fmx` | 2026-11-03 | m4: Node::lb replaced by a max-fmax byte and a dead mask (negative, -1.1%) |
| <a id="ref-neg-m4-live-on-stack"></a>`neg/m4-live-on-stack` | 2026-11-03 | m4: exhaustion test on live instead of the remaining todo (negative, neutral) |
| <a id="ref-neg-m4-nextjob-on-stack"></a>`neg/m4-nextjob-on-stack` | 2026-11-03 | m4: job-finished path out of line (negative, -1.7%) |
| <a id="ref-neg-m4-nolb"></a>`neg/m4-nolb` | 2026-11-03 | m4: drop Node::lb, compute max(fmax) at pop (negative, -3.3%) |
| <a id="ref-neg-m4-pair-lookups"></a>`neg/m4-pair-lookups` | 2026-11-03 | m4: paired lookups for X children (negative, -40%) |
| <a id="ref-neg-m6-lane-hot-fields-first"></a>`neg/m6-lane-hot-fields-first` | 2026-11-03 | lanes: Lane hot fields first |
| <a id="ref-neg-m6-lazy-child-cube"></a>`neg/m6-lazy-child-cube` | 2026-11-03 | lanes: compute only the looked-up side's child cube |
| <a id="ref-neg-m6-node-96-bytes"></a>`neg/m6-node-96-bytes` | 2026-11-03 | lanes: Node without align(64): 96 bytes |
| <a id="ref-neg-m6-packed-counter"></a>`neg/m6-packed-counter` | 2026-11-03 | lanes: count lookups at issue, merged with the node count |
| <a id="ref-neg-m6-prefetch-t0-t1"></a>`neg/m6-prefetch-t0-t1` | 2026-11-03 | solver: table prefetch hint T0 instead of NTA (negative, -1.0% 1T) |
| <a id="ref-neg-m6-resolve-prologue"></a>`neg/m6-resolve-prologue` | 2026-11-03 | lanes: resolve prologue before the phase dispatch |
| <a id="ref-neg-m6-ring-next-early"></a>`neg/m6-ring-next-early` | 2026-11-03 | lanes: compute the next lane pointer early (neutral/negative) |
| <a id="ref-neg-m6-ring-roundrobin"></a>`neg/m6-ring-roundrobin` | 2026-11-03 | lanes: round-robin without a per-round exit branch |
| <a id="ref-neg-m6-value-branchless"></a>`neg/m6-value-branchless` | 2026-11-03 | lanes: value of the pending lookup without a branch on the code |
| <a id="ref-neg-m7-diag-value-latency"></a>`neg/m7-diag-value-latency` | 2026-11-04 | NEGATIVE m7: diag-value-latency (not merged) |
| <a id="ref-neg-m7-memcnt"></a>`neg/m7-memcnt` | 2026-11-03 | lanes: count nodes into Lane::stats where they are reached |
| <a id="ref-neg-m7-memcnt-595"></a>`neg/m7-memcnt-595` | 2026-11-04 | lanes: count nodes into Lane::stats where they are reached |
| <a id="ref-neg-m7-memory-node-count"></a>`neg/m7-memory-node-count` | 2026-11-04 | NEGATIVE m7: memory-node-count (not merged) |
| <a id="ref-neg-m7-neutral-lazy"></a>`neg/m7-neutral-lazy` | 2026-11-04 | lanes: fewer µops per child (lazy child cube, early stores, blsmsk count, move tables, index tweaks) |
| <a id="ref-neg-m7-ra"></a>`neg/m7-ra` | 2026-11-03 | lanes: read the next lane's lookup ahead, and the prune bound |
| <a id="ref-neg-m7-read-ahead"></a>`neg/m7-read-ahead` | 2026-11-04 | NEGATIVE m7: read-ahead (not merged) |
| <a id="ref-neg-m7-spec-prefetch"></a>`neg/m7-spec-prefetch` | 2026-11-04 | NEGATIVE m7: spec-prefetch (not merged) |
| <a id="ref-neg-m8-index-rows-applymove-runprefetch"></a>`neg/m8-index-rows-applymove-runprefetch` | 2026-11-04 | NEGATIVE m8: index without the multiply after the msplit lookup, both fix-up candidates in apply_move, runs prefetched 4 ahead (neutral as a stack) |
| <a id="ref-neg-m8-rounds-live-set"></a>`neg/m8-rounds-live-set` | 2026-11-05 | NEGATIVE m8: Worker::rounds keeps only the lane pointers live (neutral to negative) |
| <a id="ref-neg-m8-run-prefetch-4"></a>`neg/m8-run-prefetch-4` | 2026-11-05 | NEGATIVE m8: setup_run prefetches the job and child order of the run 4 entries ahead (neutral) |
| <a id="ref-neg-merged-node-count"></a>`neg/merged-node-count` | 2026-11-03 | Merged node counting in advance (rejected: -1%) |
| <a id="ref-neg-merged-tails"></a>`neg/merged-tails` | 2026-11-03 | Merge the X2/X'/X index/prefetch sites (rejected: -1.6%) |
| <a id="ref-neg-moved-push-branchless"></a>`neg/moved-push-branchless` | 2026-11-03 | Branchless moved-side resolution (rejected: -2.2%) |
| <a id="ref-neg-neutral-cptr-mergedcount"></a>`neg/neutral-cptr-mergedcount` | 2026-11-03 | Cached cocsep pointer plus merged node counting (rejected: neutral) |
| <a id="ref-neg-neutral-early-pop"></a>`neg/neutral-early-pop` | 2026-11-03 | Early pop via a k_last flag (rejected: neutral) |
| <a id="ref-neg-next-lane-prefetch"></a>`neg/next-lane-prefetch` | 2026-11-03 | Prefetch the next lane's header and top node (rejected: no gain) |
| <a id="ref-neg-node-ab-layout"></a>`neg/node-ab-layout` | 2026-11-03 | Node stores the chosen-side cubes, 96-byte node (rejected: -2.9%) |
| <a id="ref-neg-opp-tight"></a>`neg/opp-tight` | 2026-11-03 | Opposite-face half-turn rule on tight bounds (rejected: no effect) |
| <a id="ref-neg-p10-h11-lanes40"></a>`neg/p10-h11-lanes40` | 2026-11-04 | NEGATIVE p10: h11-lanes40 (not merged) |
| <a id="ref-neg-p10-h7-base10"></a>`neg/p10-h7-base10` | 2026-11-04 | NEGATIVE p10: h7-base10 (not merged) |
| <a id="ref-neg-p10-map-4k-pages"></a>`neg/p10-map-4k-pages` | 2026-11-04 | NEGATIVE p10: map-4k-pages (not merged) |
| <a id="ref-neg-p5-bucket-2groups"></a>`neg/p5-bucket-2groups` | 2026-11-03 | p5: phase buckets within two fixed lane groups (negative, -8.4%) |
| <a id="ref-neg-p5-bucket-bitmask"></a>`neg/p5-bucket-bitmask` | 2026-11-03 | p5: phase buckets as lane bit masks (negative, +6.9% instructions/node, not timed) |
| <a id="ref-neg-p5-bucket-fifo"></a>`neg/p5-bucket-fifo` | 2026-11-03 | p5: phase-bucketed lane scheduling with FIFO pointer arrays (negative, -6.1%) |
| <a id="ref-neg-p5-bucket-fifo-lanepf"></a>`neg/p5-bucket-fifo-lanepf` | 2026-11-03 | p5: bucket-fifo with prefetch of the next lanes' state (negative, -2.5% 1T vs bucket-fifo) |
| <a id="ref-neg-p5-bucket-fifo-t0"></a>`neg/p5-bucket-fifo-t0` | 2026-11-03 | p5: bucket-fifo with prefetcht0 instead of NTA (negative, -8.2%) |
| <a id="ref-neg-p5-diag-perm-identity"></a>`neg/p5-diag-perm-identity` | 2026-11-03 | p5: diagnostic, lanes visited through FIFO pointer arrays in identity order (negative, -5.1% 1T) |
| <a id="ref-neg-p5-diag-perm-random"></a>`neg/p5-diag-perm-random` | 2026-11-03 | p5: diagnostic, bucket-fifo's visit order with main's dispatch (negative, -4.5% 1T vs identity) |
| <a id="ref-neg-p5-fresh-child"></a>`neg/p5-fresh-child` | 2026-11-03 | p5: fresh-child fast path on top of regs (negative, neutral -0.0%) |
| <a id="ref-neg-pb1-diag-alloc-count"></a>`neg/pb1-diag-alloc-count` | 2026-11-05 | DIAGNOSTIC (not for merging): counting global allocator in bench and solve |
| <a id="ref-neg-pb1-pgo-bolt"></a>`neg/pb1-pgo-bolt` | 2026-11-05 | NEGATIVE: BOLT on top of PGO (bench): stream -1.3% at 1T, share no better than PGO alone |
| <a id="ref-neg-pb1-pgo-stream"></a>`neg/pb1-pgo-stream` | 2026-11-05 | NEGATIVE: PGO of bench, stream mode, re-measured with pinned hot-loop pads (best pad 32: -0.2% at 1T) |
| <a id="ref-neg-pgo"></a>`neg/pgo` | 2026-11-03 | PGO measurement (rejected: -1 to -1.5%) |
| <a id="ref-neg-pin-threads"></a>`neg/pin-threads` | 2026-11-03 | bench: pin worker thread i to CPU i (negative, -0.4%) |
| <a id="ref-neg-pmu-byte-packed-load"></a>`neg/pmu-byte-packed-load` | 2026-11-07 | Archive rejected byte-sized packed table decoder |
| <a id="ref-neg-pmu-outlined-lane-advance"></a>`neg/pmu-outlined-lane-advance` | 2026-11-07 | Archive rejected outlined lane advancement |
| <a id="ref-neg-push-select"></a>`neg/push-select` | 2026-11-03 | Branch-free side selection in push_child (rejected: -4%) |
| <a id="ref-neg-r8-worker-new-frame"></a>`neg/r8-worker-new-frame` | 2026-11-05 | NEGATIVE (r8): Pool::run building its Worker through a `Worker::new` constructor (applies on top of work/r8 f99a27b, i.e. 3a9a50a + r8-share-fix). |
| <a id="ref-neg-readahead"></a>`neg/readahead` | 2026-11-03 | Read-ahead: resolve the next lane's value one visit early (rejected: -4.4%) |
| <a id="ref-neg-s9-full-pin"></a>`neg/s9-full-pin` | 2026-11-05 | neg: page-pin the hot code and stack of both binaries (stream loop, its helpers, share) |
| <a id="ref-neg-s9-placement-sweep"></a>`neg/s9-placement-sweep` | 2026-11-05 | neg: stack and heap placement sweeps of the share and stream threads (tools, no gain) |
| <a id="ref-neg-s9-stack-pin-only"></a>`neg/s9-stack-pin-only` | 2026-11-05 | neg: stack placement pinned without pinning the hot code's page offset |
| <a id="ref-neg-side-tiebreak"></a>`neg/side-tiebreak` | 2026-11-03 | Side choice tie-break by bound (rejected: no effect) |
| <a id="ref-neg-skip-cmov-guard"></a>`neg/skip-cmov-guard` | 2026-11-03 | Branch-free skip guard plus a single pop branch (rejected: -8%) |
| <a id="ref-neg-twinearly"></a>`neg/twinearly` | 2026-11-03 | Hoist the X' twin-bound prune before cube computation (rejected: -2.5%) |
| <a id="ref-neg-twinlate"></a>`neg/twinlate` | 2026-11-03 | Twin-bound prune saving with the original branch structure (rejected: neutral) |
| <a id="ref-neg-unified-resolve"></a>`neg/unified-resolve` | 2026-11-03 | Unified resolve without the phase dispatch (rejected: -3.3%) |
| <a id="ref-neg-xh48-base11-h11"></a>`neg/xh48-base11-h11` | 2026-11-04 | NEGATIVE x-h48: base11-h11 (not merged) |
| <a id="ref-neg-xh48-eoesep-fallback"></a>`neg/xh48-eoesep-fallback` | 2026-11-04 | NEGATIVE x-h48: eoesep-fallback (not merged) |
| <a id="ref-neg-xh48-parity-bit"></a>`neg/xh48-parity-bit` | 2026-11-04 | NEGATIVE x-h48: permutation parity bit instead of an EO bit (not merged) |
| <a id="ref-neg-xnx-axis-inheritance"></a>`neg/xnx-axis-inheritance` | 2026-11-04 | NEGATIVE x-nx: axis-inheritance (not merged) |
| <a id="ref-neg-xnx-diagnostics"></a>`neg/xnx-diagnostics` | 2026-11-04 | NEGATIVE x-nx: diagnostics (not merged) |
| <a id="ref-neg-xvc-ck2-halfsplit"></a>`neg/xvc-ck2-halfsplit` | 2026-11-04 | NEGATIVE x-vc: ck2-halfsplit (not merged) |
| <a id="ref-neg-xvc-clflushopt-table-lines"></a>`neg/xvc-clflushopt-table-lines` | 2026-11-04 | NEGATIVE x-vc: clflushopt-table-lines (not merged) |
| <a id="ref-neg-xvc-cocsep-18bit-p18d"></a>`neg/xvc-cocsep-18bit-p18d` | 2026-11-04 | NEGATIVE x-vc: cocsep-18bit-p18d (not merged) |
| <a id="ref-neg-xvc-diag-index-chain-latency"></a>`neg/xvc-diag-index-chain-latency` | 2026-11-04 | NEGATIVE x-vc: diag-index-chain-latency (not merged) |
| <a id="ref-neg-xvc-p18-dense"></a>`neg/xvc-p18-dense` | 2026-11-04 | NEGATIVE x-vc: p18-dense (not merged) |
| <a id="ref-neg-xvc-p18bc-variants"></a>`neg/xvc-p18bc-variants` | 2026-11-04 | NEGATIVE x-vc: p18bc-variants (not merged) |

## Archived experiments

| Experiment | Date | Summary |
|---|---|---|
| <a id="ref-archive-c7-childsplit"></a>`archive/c7-childsplit` | 2026-11-03 | lanes: order root children across jobs in the iterations on unchanged jobs (c7) |
| <a id="ref-archive-c7-clean"></a>`archive/c7-clean` | 2026-11-03 | c7-glob WIP: split iterations with knobs |
| <a id="ref-archive-c7-exp"></a>`archive/c7-exp` | 2026-11-03 | c7: diag knobs (force normal), dseval policies |
| <a id="ref-archive-c7-glob"></a>`archive/c7-glob` | 2026-11-03 | c7-glob: fix knob 11 enable |
| <a id="ref-archive-m7"></a>`archive/m7` | 2026-11-03 | wip memcnt: count nodes in stats directly |
| <a id="ref-archive-m7-align-abce"></a>`archive/m7-align-abce` | 2026-11-04 | wip align: hot sections at fixed 64-byte offsets |
| <a id="ref-archive-m7-c1"></a>`archive/m7-c1` | 2026-11-04 | wip c1 align |
| <a id="ref-archive-m7-c1n"></a>`archive/m7-c1n` | 2026-11-04 | wip c1 align |
| <a id="ref-archive-m7-c2"></a>`archive/m7-c2` | 2026-11-04 | wip c2 stack |
| <a id="ref-archive-m7-c2n"></a>`archive/m7-c2n` | 2026-11-04 | wip c2 stack |
| <a id="ref-archive-m7-cnt"></a>`archive/m7-cnt` | 2026-11-03 | wip cnt: count the reached moves with blsmsk |
| <a id="ref-archive-m7-diag"></a>`archive/m7-diag` | 2026-11-04 | diag: prune outcome of each lookup by the node's bound on that side |
| <a id="ref-archive-m7-dlat"></a>`archive/m7-dlat` | 2026-11-04 | diag: 9 cycles of extra latency on the pending value |
| <a id="ref-archive-m7-dnolat"></a>`archive/m7-dnolat` | 2026-11-03 | diag: 3 imuls off the value chain |
| <a id="ref-archive-m7-early"></a>`archive/m7-early` | 2026-11-03 | wip early2: corner-depth test against r |
| <a id="ref-archive-m7-final"></a>`archive/m7-final` | 2026-11-04 | wip align: hot sections at fixed 64-byte offsets |
| <a id="ref-archive-m7-idx"></a>`archive/m7-idx` | 2026-11-03 | wip idx3: 64-bit vpmovmskb output |
| <a id="ref-archive-m7-kind"></a>`archive/m7-kind` | 2026-11-03 | wip kind2: X-move blend mask from a table |
| <a id="ref-archive-m7-lanes"></a>`archive/m7-lanes` | 2026-11-04 | lanes: 24 lanes per thread by default |
| <a id="ref-archive-m7-lazy"></a>`archive/m7-lazy` | 2026-11-03 | wip lazy: only the looked-up child cube, from selected operands |
| <a id="ref-archive-m7-lz"></a>`archive/m7-lz` | 2026-11-04 | lz: names, docs, pedantic |
| <a id="ref-archive-m7-lz-595"></a>`archive/m7-lz-595` | 2026-11-04 | lanes: compute only the looked-up child cube (wip message) |
| <a id="ref-archive-m7-lz-f2"></a>`archive/m7-lz-f2` | 2026-11-04 | lanes: compute only the looked-up child cube (wip message) |
| <a id="ref-archive-m7-lz-next"></a>`archive/m7-lz-next` | 2026-11-04 | lanes: compute only the looked-up child cube (wip message) |
| <a id="ref-archive-m7-lz-rb"></a>`archive/m7-lz-rb` | 2026-11-04 | lz: names, docs, pedantic |
| <a id="ref-archive-m7-memcnt-595"></a>`archive/m7-memcnt-595` | 2026-11-04 | lanes: count nodes into Lane::stats where they are reached |
| <a id="ref-archive-m7-memcnt-f2"></a>`archive/m7-memcnt-f2` | 2026-11-04 | lanes: count nodes into Lane::stats where they are reached |
| <a id="ref-archive-m7-port"></a>`archive/m7-port` | 2026-11-04 | port: stack + align on b3f0bfb (wip) |
| <a id="ref-archive-m7-port-base"></a>`archive/m7-port-base` | 2026-11-04 | mine renamed |
| <a id="ref-archive-m7-ra"></a>`archive/m7-ra` | 2026-11-03 | wip ra6: read-ahead word through the next lane's k_word |
| <a id="ref-archive-m7-ra-next"></a>`archive/m7-ra-next` | 2026-11-04 | lanes: read the next lane's lookup ahead, and the prune bound |
| <a id="ref-archive-m7-ra2"></a>`archive/m7-ra2` | 2026-11-03 | wip ra2: pending child's remaining moves kept in the lane |
| <a id="ref-archive-m7-ra5"></a>`archive/m7-ra5` | 2026-11-03 | wip ra2: pending child's remaining moves kept in the lane |
| <a id="ref-archive-m7-ra7"></a>`archive/m7-ra7` | 2026-11-03 | wip ra2: pending child's remaining moves kept in the lane |
| <a id="ref-archive-m7-s1"></a>`archive/m7-s1` | 2026-11-03 | wip idx3: 64-bit vpmovmskb output |
| <a id="ref-archive-m7-smc-abce"></a>`archive/m7-smc-abce` | 2026-11-04 | wip smc: stack + node count into stats |
| <a id="ref-archive-m7-spec"></a>`archive/m7-spec` | 2026-11-03 | lanes: speculative prefetch of the next face's first child |
| <a id="ref-archive-m7-stack-595"></a>`archive/m7-stack-595` | 2026-11-04 | stack: gain comments |
| <a id="ref-archive-m7-stack-abce"></a>`archive/m7-stack-abce` | 2026-11-04 | stack: gain comments |
| <a id="ref-archive-m7-stack-next"></a>`archive/m7-stack-next` | 2026-11-04 | stack: gain comments |
| <a id="ref-archive-m8-e1"></a>`archive/m8-e1` | 2026-11-04 | WIP e1: prefetch runs 4 ahead |
| <a id="ref-archive-m8-ix"></a>`archive/m8-ix` | 2026-11-04 | WIP ix: msplit rows pre-shifted (no imul after the msplit lookup) |
| <a id="ref-archive-m8-ix3"></a>`archive/m8-ix3` | 2026-11-04 | WIP ix: msplit rows pre-shifted (no imul after the msplit lookup) |
| <a id="ref-archive-m8-pm"></a>`archive/m8-pm` | 2026-11-04 | WIP pm: apply_move both fix-up candidates in parallel |
| <a id="ref-archive-m8-pre-def2"></a>`archive/m8-pre-def2` | 2026-11-04 | WIP rp: share lanes by pointer |
| <a id="ref-archive-m8-rp"></a>`archive/m8-rp` | 2026-11-04 | WIP rp: share lanes by pointer |
| <a id="ref-archive-m8-rp2"></a>`archive/m8-rp2` | 2026-11-04 | WIP rp2: only next/end live across the visits |
| <a id="ref-archive-m8-stack"></a>`archive/m8-stack` | 2026-11-04 | WIP rp: share lanes by pointer |
| <a id="ref-archive-m8-stack2"></a>`archive/m8-stack2` | 2026-11-04 | NEGATIVE m8: index without the multiply after the msplit lookup, both fix-up candidates in apply_move, runs prefetched 4 ahead (neutral as a stack) |
| <a id="ref-archive-m8-ul"></a>`archive/m8-uL` | 2026-11-04 | WIP uL: unsafe pass without share.rs |
| <a id="ref-archive-opt-batch-other"></a>`archive/opt/batch-other` | 2026-11-02 | Batch the other-side lookups of a node (REGRESSION, not merged) |
| <a id="ref-archive-opt-childorder"></a>`archive/opt/childorder` | 2026-11-03 | Per-child final-iteration ordering (neutral: -4..6% nodes, +0.3% throughput) |
| <a id="ref-archive-opt-cocsep-additive"></a>`archive/opt/cocsep-additive` | 2026-11-03 | Additive cocsep coordinate (rejected: neutral) |
| <a id="ref-archive-opt-cocsep-prefetch-lanes"></a>`archive/opt/cocsep-prefetch-lanes` | 2026-11-03 | Prefetch the moved-side cocsep entry early (rejected: -3%) |
| <a id="ref-archive-opt-value-branchless"></a>`archive/opt/value-branchless` | 2026-11-03 | Branchless value() (rejected: -4%) |
| <a id="ref-archive-r6c"></a>`archive/r6c` | 2026-11-03 | lanes: value of the pending lookup without a branch on the code (wip) |
| <a id="ref-archive-r6e"></a>`archive/r6e` | 2026-11-03 | lanes: Node without align(64): 96 bytes |
| <a id="ref-archive-s2-prologue"></a>`archive/s2-prologue` | 2026-11-03 | lanes: resolve prologue before the phase dispatch (wip) |
| <a id="ref-archive-stash-82da6e7"></a>`archive/stash-82da6e7` | 2026-11-03 | WIP on (no branch): 82da6e7 Merge opt/skip-sibling: branch-free sibling pruning (+13.6%) |

## Other unmerged branches

| Experiment | Date | Summary |
|---|---|---|
| <a id="ref-work-m8-r2"></a>`work/m8-r2` | 2026-11-05 | WIP e1: prefetch runs 4 ahead |
| <a id="ref-work-p11"></a>`work/p11` | 2026-11-05 | wasm: Bun instead of Node for the JavaScript tooling (Bun 1.4.4) |
| <a id="ref-work-r8"></a>`work/r8` | 2026-11-05 | check.sh: under qemu, the /proc/self/maps test runs alone; failed greps stop the script |
