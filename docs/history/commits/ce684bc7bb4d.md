# m4: drop Node::lb, compute max(fmax) at pop (negative, -3.3%)

Commit `ce684bc7bb4dbc06b19a1322b5de2a672601de41` · 2026-11-03T17:25:34+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/m4-nolb`

## Original experiment record

m4: drop Node::lb, compute max(fmax) at pop (negative, -3.3%)

Node::lb is dropped: it always equals max(fmax) - 1 (saturating). fmax
grows to 8 bytes (entries 6 and 7 stay 0); a node whose bound exceeds r
gets todo = 0 at once (no separate lb > r test per advance step); the
bound itself is computed only at pop, with a u64 SIMD load of fmax.

Why: one fewer field update per noted value and one fewer test per
advance step.

48 threads (bench /mnt/huge/h11 stream 20 30 48 20, capped, interleaved,
3 runs, solves/s; measured on top of loop2, the lane-loop step later
merged as c914d66):

| build              | run 1 | run 2 | run 3 | median |
|--------------------|-------|-------|-------|--------|
| main ff2b7a4       | 832.9 | 817.3 | 828.4 | 828.4  |
| loop2              | 842.8 | 842.0 | 840.3 | 842.0  |
| loop2 + nolb       | 805.7 | 818.9 | 813.9 | 813.9  |

Change: -3.3% vs loop2 (median); per run -4.4, -2.7, -3.1%.
1-thread solves/s: not recorded.

Conclusion: slower. Likely a store-to-load forwarding stall: the 8-byte
fmax load at pop comes right after byte stores into fmax (the values just
noted), and a wide load over narrow in-flight stores cannot be forwarded.

Source: scratchpad m4/neg-nolb.patch (mk-nolb.py applied to ff2b7a4),
made against ff2b7a4; applies cleanly. (It was measured on top of
loop2.)
Numbers are the agent's own (not re-measured for this record). Not merged; recorded per the rule that every candidate gets a branch.
