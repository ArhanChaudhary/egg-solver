# NEGATIVE c8: predictor study for final-iteration ordering (not merged)

Commit `d64adf1ba3dfdbb15511ef642d285638a443eccf` · 2026-11-04T07:05:29+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/c8-predictor-study`

## Original experiment record

NEGATIVE c8: predictor study for final-iteration ordering (not merged)

Nothing beat c7's per-child statistic out of sample: logistic p and p/c
models (with/without kernel-available features), empirical rates per
node by own bounds / statistic / frontier / job bucket, global orders.
A parameter search fitted on 305 cubes gave -2.97% in-sample and
-0.41% / +0.39% on the full sets. T=17 finals (4.15% of nodes) cannot
be helped: only 2.3% of their items have grandchild statistics at T=16.
Headroom (diag, random:1000:2026): final iteration 46.9% of nodes, all
before the solving run; inside runs / interior ordering ~0.01%.
Contains the dataset tool and the simulator of c7's machinery.
