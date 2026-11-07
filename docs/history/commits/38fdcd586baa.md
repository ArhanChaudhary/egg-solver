# c4: MERGE = 8, wider first-part runs (negative, -3.85% vs -4.01% nodes)

Commit `38fdcd586baaae26ba8bdb8068d7b5b13210c2d6` · 2026-11-03T17:25:34+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/c4-merge-wide`

## Original experiment record

c4: MERGE = 8, wider first-part runs (negative, -3.85% vs -4.01% nodes)

MERGE (how far below a job's best child another first-part candidate may
be and still share its run) goes from 4 to 8; 16 was measured as well.

Why: wider runs mean fewer run setups.

Nodes/cube on random:1000:2026 vs main 3295521 (pj2, free-side config of
that time):

| MERGE        | nodes vs main |
|--------------|---------------|
| 2            | -4.04%        |
| 4 (c4)       | -4.01%        |
| 8 (this)     | -3.85%        |
| 16           | -3.36%        |

48-thread and 1-thread solves/s: not measured.

Conclusion: fewer nodes saved as the window grows; MERGE 2 to 4 are
equivalent, and 4 stayed. Likely the children merged into a run go ahead
of better candidates of other jobs.

Source: scratchpad c4/neg-merge-wide.patch, made on top of the c4 patch (c4 worktree commit 70c221a, the same tree as 11e1de0 on neg-c4-childorder); applies cleanly.
Numbers are the agent's own (not re-measured for this record). Not merged; recorded per the rule that every candidate gets a branch.
