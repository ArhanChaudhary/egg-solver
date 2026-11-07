# NEGATIVE x-h48: eoesep-fallback (not merged)

Commit `ecfca0cdf9c91c35ee47f03a9cd8265687edb0d1` · 2026-11-04T10:08:09+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/xh48-eoesep-fallback`

## Original experiment record

NEGATIVE x-h48: eoesep-fallback (not merged)

From d382cf12f61e6531cbf273ebef75dfca83e79d3e Mon Sep 17 00:00:00 2001
From: Ubuntu
 <super@claude-cube.usubflgyqkfexnnpr2q2liw1rb.bx.internal.cloudapp.net>
Date: Wed, 4 Nov 2026 08:51:28 +0000
Subject: [PATCH] neg: edge-only (eoesep) bound as a code-0 fallback or folded
 into the corner bound

nissy-core's H48 looks up a second, edges-only table on every code-0 read
(h48/solve.h:133-137: max(line minimum, eoesep)), where eoesep is the
distance of the edges to "in their slice and oriented" (782 symmetry classes
x 2^11 EO, 4 bits). Count-only experiment (src/exp.rs): an unreduced byte
table indexed by the edge part of our key (msplit, eo11, eslot; 73 MB, BFS
distribution 0:1 1:6 2:75 3:870 4:9810 5:102520 6:986669 7:8137622
8:34145345 9:27231078 10:349198 11:6), in two modes:
- CUBE_EXP=1, nissy's rule: code 0 -> max(group minimum, corner depth,
  eoesep) (h11 only: its index holds every edge bit);
- CUBE_EXP=3, the edge bound folded into the corner bound at index time
  (precheck and code-0 fallback, any k).

8 threads of one CCD (count.sh style), lengths identical:

| table | set              | baseline nodes / lookups | eoesep nodes / lookups      | nodes  |
|-------|------------------|--------------------------|-----------------------------|--------|
| h11   | random:1000:2026 | 3058198 / 2588380        | mode 1: 3063419 / 2592913   | +0.17% |
| h11   | random:1000:2026 | 3058198 / 2588380        | mode 3: 3060822 / 2590050   | +0.09% |
| h8    | random:300:2026  | 23870810 / 20486759      | mode 3: 23399477 / 20054380 | -2.0%  |
| h7    | random:200:2026  | 48536433 / 41297277      | mode 3: 48181355 / 40955915 | -0.7%  |

Negative. At h11, nothing (a stronger bound can only lower an exhaustive
iteration's count; the + is last-iteration ordering noise; consistent with
