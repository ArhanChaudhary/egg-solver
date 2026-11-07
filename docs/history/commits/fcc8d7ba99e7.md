# Merge opt/l6-share-a: share-mode hang fix, parallel staged builds (-39% latency)

Commit `fcc8d7ba99e7b62f19f2d2b1f35d21fcbedd6774` · 2026-11-03T23:23:38+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

Merge opt/l6-share-a: share-mode hang fix, parallel staged builds (-39% latency)

Two share.rs commits:

1. count down a lane's ended jobs when it takes a piece. This fixes a
   latent hang: a lane that took a donated piece never counted down its
   finished claim, so the iteration never ended. New test share_stress
   (16 threads x 1 lane) hangs on 78ce33e.
2. builds in stages use all threads, and no fresh pages. A cube was
   dropped from the open list between stages, so the staged sort ran on
   one thread. Build buffers are pooled, so there are no page faults.
   Infeasible jobs are sorted last and not claimed.

Search unchanged:
- stream: 3295521 / 2785769, lengths 15:1 16:35 17:270 18:662 19:32;
- depth18: 3607979 / 3053364, lengths 18:100.
check.sh passes.

Share mode, 48 threads x 20 lanes, 3 interleaved runs (exclusive machine
per run), random_state from cube_solver_test:

| test                         | main cb8cb70          | l6-share-a            | change |
|------------------------------|-----------------------|-----------------------|--------|
| solve -c 1 -n 50, ms/cube    | 3.95 4.31 3.85 (3.95) | 2.36 2.40 2.59 (2.40) | -39%   |
| solve -c 0 (500), ms/cube    | 1.282 1.259 1.231     | 1.448 1.292 1.254     | +2.6%  |
| bench random:1000 share, /s  | 838.7 840.5           | 842.0 849.9           | +0.7%  |

The batch is slightly slower in every pair (median 1.259 -> 1.292). The
agent's own paired runs also show it for these two commits, and its
following commits (next stack) recover it.

Stream, 1 thread. Run with bench /mnt/huge/h11 stream 10 60 1 20, 3
paired runs, solves/s:

| build        | runs                 | median |
|--------------|----------------------|--------|
| main cb8cb70 | 21.733 21.750 21.733 | 21.733 |
| l6-share-a   | 21.750 21.783 21.733 | 21.750 |

Change: 0.0%.

Stream, 48 threads. The stream code is untouched (share.rs only). The
4-commit stack that contains these two commits measured 920.3 -> 982.8
median (3 runs, noise).

A second share bug, the fresh() join race, is fixed in the following
merge (opt/r7-series).
