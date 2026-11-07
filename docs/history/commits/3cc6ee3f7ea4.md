# lanes: descriptive field and item names

Commit `3cc6ee3f7ea4f4d254f1fdcb8f51812b1dbf94d9` · 2026-11-04T10:31:18+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

lanes: descriptive field and item names

The first of three lanes.rs commits (fields and items, then locals, then
docs), each instruction-identical in the hot loops, so that a regression is
easy to bisect.
- `Node {a, b, lo, mm_o, r, lb, m, m_normal, tw}` -> `{moved, other,
  other_side_bound, other_moves, remaining_depth, lower_bound, mv,
  mv_normal, twin_values}`; lanes' `Job {ln, li}` -> `{normal_bound,
  inverse_bound}`.
- `Lane`: `root`, `root_inverse`, `run_state`, `depth`, `job_stat`,
  `child_stats`, and the pending child's `pending_moved`, `pending_other`,
  `pending_move`, `pending_face`, `pending_index`, `pending_corner_bound`,
  `pending_other_bound` (were root_c, root_ci, rs, sp, heur, cheur, k_*).
- `JobX` -> `ChildOrder {codes, twin_values, done}`; `RunState`'s
  `child_orders`, `current_job`, `current_children`.
- items: `quantize`, `quantize18`, `codes_above`, `max_code`, `stat_terms`,
  `STAT_TERMS`, `STAT_PAIRS` (and `HotTables::stat_pairs`),
  `max_child_bound`, `unknown_twins`, `FIRST_FRACTION`, `MERGE_CODES`;
  main's newer `UNSEEN_CODE` keeps its name, and its test becomes
  `quantize18_codes_unseen_children`.
- share.rs: only the uses of these (mechanical; the map's field rules
  also cover l6's receivers `scratch`, `lane`, `sl.lane`, `to.lane`).

Verification (this commit built fresh from its own tree):
- check.sh passes (fmt, clippy with pedantic, tests).
- verify.sh output identical to main 55e048e: nodes/cube 3058198
  lookups/cube 2588380, lengths 15:1 16:35 17:270 18:662 19:32;
  depth18 nodes/cube 3644301 lookups/cube 3084160, lengths 18:100.
- cargo asm: the stream loop (bench::stream_thread) and the share
  loop (Worker::rounds) are instruction-identical to main 55e048e
  (normalised for labels; symbols differ only by renames).
- the .text section is byte-identical to the previous commit's
  (lanes.rs renames and docs change no machine code).

Perf, solves/s (stream, 24 lanes; a pure refactor, so neutral is
expected). Measured before the rebases onto 595e424, abce87c and
55e048e (bench's thread functions outlined; c8's child ordering;
l6's share.rs pass): main f2cb1f4 and this commit on f2cb1f4,
built in the same folder. The series as a whole is measured on
55e048e (layout-averaged) in the last commits.

| threads, build | runs | median |
|---|---|---|
| 1T main | 25.317 25.600 25.500 25.267 25.600 | 25.500 |
| 1T this commit | 25.533 25.583 25.450 25.383 25.533 | 25.533 (+0.1%) |
| 48T main | 1083.233 1078.067 1093.300 | 1083.233 |
| 48T c5 (same .text) | 1081.600 1089.267 1089.433 | 1089.267 (+0.6%) |

(48T: this commit's .text is byte-identical to c5's, at the same
addresses, so its 48T run is c5's.)

1T: 5 rounds, the 5 builds of a round at once on separate L3 domains
(start order rotated per round). 48T: 3 interleaved rounds (ab48.sh).
The hot loops are instruction-identical to main, so a difference here is
code placement: layout alone moves 1T results by ~1.6% on this machine.
