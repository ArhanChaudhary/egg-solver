# lanes: code for root children without statistics, rest phase by the job's best code (c8)

Commit `abce87ca68e0b89ce160aa030d9952fb6de881ac` · 2026-11-04T07:04:26+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

lanes: code for root children without statistics, rest phase by the job's best code (c8)

Two changes to c7's child ordering in the split iterations, both outside the
search loop (end of a run, and the build of an iteration's rest):

- A run's root child whose statistic is 0 gets code `UNSEEN_CODE` = 48
  (plus the face step) instead of 0. Statistic 0 means no lookup was
  resolved in its subtree in the previous iteration: X pruned by its other
  side (13 at T = 17), its X' with it, or a half turn left out by a tight
  root. They are ~48% of the last iteration's nodes, and per node they lead
  to a solution about as often as children with codes near 48 (bounds 12/12
  or 13/12 with nothing below; measured on 1000 cubes with c8's dataset of
  the last iteration). With code 0 they never were candidates and their
  jobs' rest runs came last.
- The rest phase ranks a job by its best code over all its children,
  including those already run in this iteration's first part (whose codes
  then are of this iteration), instead of only the children left: a job
  whose best child ran without a solution is still more promising than one
  whose best child is weaker.

Each alone is about -0.5% nodes; together -1.0% on the large sets. The
neighbours are flat on random:20000:99 (UNSEEN_CODE 44 / 56: +0.03 /
+0.07%; FIRST 0.15 / 0.25: -0.01 / +0.09%; MERGE 3 / 6: -0.02 / +0.07%). Tuning
note: the per-cube cost is heavy-tailed, so over 1000 cubes the standard
error of a change in nodes/cube is 0.6-1.4% of the total for changes of
this kind (+-1.1% here); the 20000- and 33468-cube sets decide (c8's
simulator of this machinery replays the counts exactly; negative study and
tools: c8/neg-predictor-study.patch).

Nodes/cube (count.sh; lengths identical to the base on every set):

| set                        | base    | c8      | change | lookups base -> c8          |
|----------------------------|---------|---------|--------|-----------------------------|
| random:20000:99            | 3111636 | 3080725 | -0.99% | 2634932 -> 2607693 (-1.03%) |
| depth18, all 33468 cubes   | 3382838 | 3346941 | -1.06% | 2869277 -> 2837185 (-1.12%) |
| random:1000:2026           | 3089312 | 3058198 | -1.01% | 2616086 -> 2588380 (-1.06%) |
| random:3000:777            | 3015162 | 3027858 | +0.42% | 2554521 -> 2563907 (+0.37%) |
| random:2000:31337          | 3063050 | 3027943 | -1.15% | 2593492 -> 2562758 (-1.19%) |

(depth18 = ~/ref/cube_solver_test/dataset/random_state_depth18.txt.)

Throughput, 1 thread, layout-averaged (abl1t.sh 2 over 4 code layouts,
bench stream 10 60 1 24, paired, solves/s; base = f2cb1f4, whose search
code is main's):

| layout | base run 1 | base run 2 | c8 run 1 | c8 run 2 | c8/base per run |
|--------|------------|------------|----------|----------|-----------------|
| none   | 24.383     | 25.100     | 24.467   | 25.183   | +0.3, +0.3%     |
| 1      | 25.000     | 24.883     | 25.267   | 25.233   | +1.1, +1.4%     |
| 2      | 25.550     | 25.183     | 25.017   | 25.250   | -2.1, +0.3%     |
| 3      | 25.250     | 25.583     | 24.867   | 24.900   | -1.5, -2.7%     |

Layout-averaged: median +0.3%, mean -0.4% over 8 pairs: within the noise
(layout alone moves 1T results by ~1.6%; and the 60 s window's ~1500 cubes
differ between the two builds: 3274865 vs 3229244 nodes/cube, -1.4%, in
one pair). An earlier single-layout series (ab1t.sh 5): medians 24.900 ->
25.250 (+1.4%; per run +0.9, +1.8, -0.1, +0.6, +0.6%).

Throughput, 48 threads (ab48.sh 5, bench stream 20 30 48 24, interleaved;
one layout):

| build | run 1  | run 2  | run 3  | run 4  | run 5  | median |
|-------|--------|--------|--------|--------|--------|--------|
| base  | 1076.1 | 1078.5 | 1088.3 | 1087.1 | 1087.8 | 1087.1 |
| c8    | 1093.7 | 1090.7 | 1096.1 | 1088.5 | 1096.4 | 1093.7 |

Median +0.6% (per run +1.6, +1.1, +0.7, +0.1, +0.8%; mean +0.9%; outside
load <= 0.3 cores).

Verified: check.sh; unit test quant18_codes_unseen_children (codes of
unseen children, face step); lengths above.
