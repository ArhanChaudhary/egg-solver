# m4: WIP stack c6, baseline for the following diagnostics

Commit `a5bbce53c2bc9e2a1b7484284cc5ea838c3c7ca5` · 2026-11-03T17:25:34+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

m4: WIP stack c6, baseline for the following diagnostics

c6: the m4 micro stack as it stood after its sixth step (lane-loop,
node-roles, rare-check, heur-pair, index-pext, resolve-face). It differs
from the merged stack's resolve-face step by a few lines in lanes.rs.
Committed only as the baseline that the next commit was measured against.

48 threads (bench /mnt/huge/h11 stream 20 30 48 20, capped, interleaved,
3 runs, solves/s):

| build          | run 1 | run 2 | run 3 | median |
|----------------|-------|-------|-------|--------|
| main ff2b7a4   | 842.8 | 832.5 | 829.3 | 832.5  |
| c6             | 887.7 | 887.3 | 886.9 | 887.3  |

Source: scratchpad m4/wip-c6.patch, against ff2b7a4.
