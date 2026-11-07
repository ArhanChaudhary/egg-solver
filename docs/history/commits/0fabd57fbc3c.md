# c4: free side choice at job roots (negative, -4.23% vs -5.74% nodes)

Commit `0fabd57fbc3c6d0bb29e8f3d0277e5e23030ce90` · 2026-11-03T17:25:34+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/c4-free-side`

## Original experiment record

c4: free side choice at job roots (negative, -4.23% vs -5.74% nodes)

Reverse of c4's WIP commit 27db444 ("normal-side job roots"), i.e. git
diff 27db444 ca6fe95: whole-job runs choose their side freely (make_node's
rule, as on main); an INV flag marks jobs whose whole run was on the
inverse side, which in split iterations run whole, ranked by their best
child; children of INV parents inherit the parent's best child's
statistic. It also restores that time's experiment knobs and build-time
grouping.

Why: keep main's side choice (fewer allowed moves) at the job roots.

Nodes/cube (pj3, face bias; bench random:N:SEED N 48 20; combined = all
4000 cubes):

| config                                     | 1000:2026         | 3000:777          | combined |
|--------------------------------------------|-------------------|-------------------|----------|
| main                                       | 3295521           | 3249811           |          |
| free side (this patch)                     | 3145754 (-4.54%)  | 3115670 (-4.13%)  | -4.23%   |
| normal side at every job root (fn2)        |                   |                   | -5.74%   |
| normal side only before split iterations (fn1) |               |                   | -5.76%   |

Nodes/s unchanged: forced 2553M vs free 2557M nodes/s (v10).
48-thread and 1-thread solves/s: not measured separately.

Conclusion: the free side cuts fewer nodes at the same speed. With the
normal side forced, a job's per-child statistics always refer to the same
18 normal-side children, so the next split iteration can rank them; an
inverse-side run's statistics do not map onto them.

Source: scratchpad c4/neg-free-side.patch, against c4 WIP commit 27db444
(reachable from this branch); applies cleanly.
Numbers are the agent's own (not re-measured for this record). Not merged; recorded per the rule that every candidate gets a branch.
