# c4: carry the job root's child in each Node (negative, no measurable change)

Commit `9e7e1fdbe92aca8cd8af2c153e5f94c5be5d8be4` · 2026-11-03T17:25:34+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/c4-root-child-field`

## Original experiment record

c4: carry the job root's child in each Node (negative, no measurable change)

Each Node gets rc, the job root's child it descends from (set in
push_child), so resolve_moved reads the statistic slot from the current
node instead of stack[1].m.

Why: probe whether the per-lookup slot bookkeeping explains part of c4's
~4% higher per-node cost in the search loop.

Stream on 35c608c (48 threads, stream 20 30 48 20), paired with c4; same
nodes:

| pair | c4 solves/s | rc solves/s | c4 nodes/s | rc nodes/s |
|------|-------------|-------------|------------|------------|
| 1    | 893.3       | 898.7       | 2748M      | 2767M      |
| 2    | 904.2       | 893.7       | 2784M      | 2753M      |

A constant-slot probe (breaks the ordering) ran 2840 / 2794M nodes/s
against c4's 2764 / 2743M, so the slot bookkeeping is worth at most about
2%. 1-thread solves/s: not recorded.

Conclusion: no measurable difference (+0.6% and -1.2%). The stack[1].m
load is not the overhead.

Source: scratchpad c4/neg-root-child-field.patch, made on top of the c4 patch (c4 worktree commit 70c221a, the same tree as 11e1de0 on neg-c4-childorder); applies cleanly.
Numbers are the agent's own (not re-measured for this record). Not merged; recorded per the rule that every candidate gets a branch.
