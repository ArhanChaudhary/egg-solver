# share: the best root-child groups of 10% of the jobs run first in the likely last iterations (top-1 split)

Commit `def2cd0a16a5f58db9bcc039268eb43eafd4d17e` · 2026-11-04T17:59:21+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

share: the best root-child groups of 10% of the jobs run first in the likely last iterations (top-1 split)

In solve -c 1, 68% of random states end at T = 18. Their solution came
0.75-0.80 ms into T = 18 on average, and mostly from the order in which jobs
start: jobs ran whole, by the sum of their per-child statistics. Stream
mode orders (job, root child) pairs across jobs (c7). A share-mode port of
that (c8, neg/c8-share-child-order) cut nodes by 13%, but its serial run
list and per-child records cost more than they saved.

Replay (c11 diag: per-job child statistics and the final iteration's per-child
nodes and first solutions of 400 random states, replayed through share's
claims at 48 x 24 lanes and 3.2 Gnodes/s; the current order replays to 0.803 ms
vs 0.75 measured): T = 18 found time mean / median, runs per cube:

| order | found mean | found median | runs |
|---|---|---|---|
| whole jobs by statistic (main) | 0.803 | 0.498 | 43254 |
| best children first within each job | 0.752 | 0.415 | 43254 |
| c7 (FIRST 0.1, MERGE 4, X/X' together) | 0.685 | 0.275 | 68679 |
| top-1 group of 10% of the jobs (this) | 0.703 | 0.279 | 46938 |
| top-1 group of 30% / 50% / 70% of the jobs | 0.732 / 0.723 / 0.754 | 0.283 | 54-68K |

The cheapest form with most of the headroom is one run per job for its best
group. The mean stays high because about 60 of 273 cubes find their solution
in low-statistic children, after more than 2 ms, whatever the order.

What it does:
- Leads: at the end of a whole run of a full-length job (normal-side root),
  `lead_of` stores the job's lead in `Inner::leads` (one u32 per job:
  iteration tag, code of the best child, group). The group is the children
  with at least half the best statistic, plus the X or X' twin of each, so a
  separate run never needs X's other-side value. It is computed with a few
  AVX-512 µops on the child statistics, which lanes.rs now leaves in place
  at `JobDone`.
- Split iteration (the next iteration on the same jobs, built in stages; with
  h11 T = 18, and T = 19 for 19-move cubes): the jobs stay in place and the
  claims walk `Inner::runs` (job index, root children). A first part holds
  one run per job whose lead code is above the 90th percentile (from the
  sort's sample), counting-sorted by code in 4-code buckets with ties in job
  order. Then comes one rest run per job (its other children), in the order
  of the existing sort by statistic. All of it is inside the existing stages:
  ST_KEYS reads the leads and counts per chunk and bucket, ST_SCATTER writes
  the first part, ST_BUCKETS writes the rest. `sort_stage_done` adds
  O(chunks x buckets) work. There is no serial pass over the jobs, and every
  buffer is pooled and touched.
- Pieces copy the path and its roots' `todo`, so donated pieces and local
  splits need nothing more. A job's runs add their statistic atomically.

Soundness: each (job, root child) is in exactly one of the job's runs, since
rest = all & !group. A root restricted to some children is a valid node for
them (bounds, sibling-face pruning and back-jumps hold for any subset). X'
runs only with X. Targets still rise one at a time, so the first solution
is optimal. A lead from another cube or iteration (6-bit tag, 1 in 64)
only misorders. The unit test `split_runs_cover_each_child_once` drives the
stages and checks coverage and order. `lead_of_matches_scalar_lead` checks
the AVX-512 lead against the per-child definition; the portable `lead_of`
covers the NEON and scalar backends.

Checks: lengths are identical cube for cube on the check sets (short 1000,
random_move_15f 200, random_state_depth15 200, random_state 500, depth18
3000; -c 1 and -c 0) and on 120 symmetric positions (symgen) at -c 1 and
-c 0. Nodes/cube at 8T -c 1: random_state 3.394M -> 3.274M, depth18
3.706M -> 3.561M. check.sh passes (fmt, clippy -D warnings, tests, other
backends), except for two debug-build flakes that main 857a0c6 has too (r8
is on them). The share tests sometimes hit the `debug_assert` in
`Worker::release` (on 857a0c6, 3 of 8 runs of the scalar-feature share
tests), and qemu sometimes hits an internal SIGSEGV. verify.sh (stream
mode) gives main's numbers:
  random:1000:2026 nodes/cube 3058198 lookups/cube 2588380,
  lengths 15:1 16:35 17:270 18:662 19:32; depth18 nodes/cube 3644301
  lookups/cube 3084160, lengths 18:100 (as 857a0c6).

The lanes.rs change (`child_stats` stays at a shared lane's `JobDone`)
leaves stream mode's search unchanged. Stream throughput:
- 1 thread (ab1t.sh, 6 paired runs, solves/s):
  touch 23.7 23.4 23.7 23.8 23.7 22.5 (median 23.7);
  this 23.8 23.9 23.9 23.9 24.1 22.8 (median 23.9);
  +1.2% (median of pairs, 6 of 6 faster)
- 48 threads (ab48.sh, 5 interleaved runs, solves/s):
  touch 1065.0 1064.5 1062.5 1063.2 1064.0 (median 1064.0);
  this 1079.3 1065.7 1067.1 1075.8 1066.2 (median 1067.1);
  +0.4% (median of pairs, 5 of 5 faster)

Where the time goes (CUBE_TRACE=1, solve -c 1 -n 400 random_state, 48T):

| per cube | touch | this |
|---|---|---|
| T = 18 found, ms into T = 18 (273 cubes): mean / median | 0.753 / 0.450 | 0.656 / 0.267 |
| T = 18 build (291 builds), ms | 0.093 | 0.106 |
| T = 17 claims phase (leads recorded), ms per cube | 0.346 | 0.353 |
| T = 19 found (18 cubes; split from T = 18's leads), ms per cube of 400 | 0.049 | 0.038 |
| wall / cube, ms | 1.685 | 1.630 |

(Replay prediction for T = 18: mean 0.803 -> 0.705, median 0.498 -> 0.284.)

48 threads (c10's conf48: 7 exclusive holds, order rotating; solve -c 1 and
-c 0 -n 500 on random_state, bench share random:1000:2026, and in the first
3 holds solve -c 1 -n 200 on the short sets; ms/cube, bench share
solves/s; touch = the previous commit):

| test | touch runs | split runs | touch median | split median | change (median of pairs) [min, max] | nodes/cube touch -> split |
|---|---|---|---|---|---|---|
| c1-rs | 1.654 1.663 1.612 1.653 1.623 1.662 1.646 | 1.613 1.584 1.607 1.577 1.623 1.608 1.599 | 1.653 | 1.607 | -2.9% [-4.8, +0.0] | 3.711M -> 3.458M |
| c0-rs | 1.047 1.039 1.054 1.055 1.053 1.053 1.045 | 1.041 1.061 1.051 1.055 1.058 1.041 1.043 | 1.053 | 1.051 | -0.2% [-1.1, +2.1] | 3.315M -> 3.316M |
| bshare/s | 973.2 967.8 975.9 968.0 977.0 972.9 974.1 | 969.4 971.9 978.0 960.9 978.9 973.3 974.9 | 973.2 | 973.3 | +0.1% [-0.7, +0.4] | 3.301M -> 3.304M |
| c1-random_move_15f | 0.143 0.156 0.152 | 0.149 0.151 0.144 | 0.152 | 0.149 | -3.2% [-5.3, +4.2] | 0.101M -> 0.101M |
| c1-random_move_16f | 0.214 0.225 0.211 | 0.220 0.217 0.212 | 0.214 | 0.217 | +0.5% [-3.6, +2.8] | 0.147M -> 0.147M |
| c1-random_move_17f | 0.554 0.555 0.555 | 0.552 0.554 0.556 | 0.555 | 0.554 | -0.2% [-0.4, +0.2] | 0.649M -> 0.650M |
| c1-random_state_depth15 | 0.131 0.126 0.126 | 0.124 0.132 0.127 | 0.126 | 0.127 | +0.8% [-5.3, +4.8] | 0.093M -> 0.093M |
| c1-random_state_depth16 | 0.187 0.192 0.193 | 0.195 0.190 0.197 | 0.192 | 0.195 | +2.1% [-1.0, +4.3] | 0.128M -> 0.128M |
| c1-random_state_depth17 | 0.622 0.631 0.630 | 0.619 0.630 0.628 | 0.630 | 0.628 | -0.3% [-0.5, -0.2] | 0.778M -> 0.779M |

1 thread (6 paired runs, solve -t 1 on random_state; Worker::rounds is
pinned), ms/cube:

| test (1T, ms/cube) | touch runs | split runs | touch median | split median | change (median of pairs) [min, max] |
|---|---|---|---|---|---|
| solve -c 0 -n 100 | 43.60 43.87 44.01 43.87 43.77 43.96 | 43.89 43.82 43.85 43.92 43.97 43.74 | 43.87 | 43.87 | -0.00% [-0.50, +0.66] |
| solve -c 1 -n 10 | 33.70 34.06 33.86 33.92 34.04 33.92 | 34.12 34.12 33.88 33.88 33.82 33.79 | 33.92 | 33.88 | -0.04% [-0.65, +1.24] |

Rebased onto 857a0c6 (symmetric start): 3 more holds (base, touch, this):
| test | touch runs | split runs | touch median | split median | change (median of pairs) [min, max] | nodes/cube touch -> split |
|---|---|---|---|---|---|---|
| c1-rs | 1.648 1.682 1.643 | 1.583 1.574 1.649 | 1.648 | 1.583 | -3.9% [-6.4, +0.4] | 3.708M -> 3.466M |
| c0-rs | 1.039 1.050 1.083 | 1.048 1.065 1.045 | 1.050 | 1.048 | +0.9% [-3.5, +1.4] | 3.330M -> 3.327M |
| bshare/s | 967.7 966.8 968.2 | 964.9 964.8 963.5 | 967.7 | 964.8 | -0.3% [-0.5, -0.2] | 3.303M -> 3.305M |
| c1-random_move_15f | 0.147 | 0.149 | 0.147 | 0.149 | +1.4% [+1.4, +1.4] | 0.097M -> 0.101M |
| c1-random_move_16f | 0.226 | 0.208 | 0.226 | 0.208 | -8.0% [-8.0, -8.0] | 0.148M -> 0.145M |
| c1-random_move_17f | 0.567 | 0.589 | 0.567 | 0.589 | +3.9% [+3.9, +3.9] | 0.654M -> 0.642M |
| c1-random_state_depth15 | 0.125 | 0.129 | 0.125 | 0.129 | +3.2% [+3.2, +3.2] | 0.091M -> 0.095M |
| c1-random_state_depth16 | 0.195 | 0.192 | 0.195 | 0.192 | -1.5% [-1.5, -1.5] | 0.130M -> 0.128M |
| c1-random_state_depth17 | 0.631 | 0.625 | 0.631 | 0.625 | -1.0% [-1.0, -1.0] | 0.779M -> 0.775M |
