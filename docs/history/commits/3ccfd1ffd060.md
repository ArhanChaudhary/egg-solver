# m3: branch-free side selection in push_child (negative, -4.0%)

Commit `3ccfd1ffd06093bd345c924c13c0298083e66f1d` · 2026-11-03T17:25:34+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/m3-push-select`

## Original experiment record

m3: branch-free side selection in push_child (negative, -4.0%)

push_child selects the cubes, bounds and move masks by node.normal with
std::hint::select_unpredictable instead of an if/else on the side.

Why: m3's round removed mispredicting branches; the side looked
data-dependent.

48 threads (bench /mnt/huge/h11 stream 20 30 48 16, interleaved, 3 runs,
solves/s; hht = the m3-hht changes, the same code as 70567ff):

| build              | run 1 | run 2 | run 3 | median |
|--------------------|-------|-------|-------|--------|
| main b90b706       | 778.7 | 773.2 | 766.4 | 773.2  |
| hht (70567ff)      | 809.7 | 816.2 | 807.8 | 809.7  |
| hht + push-select  | 789.1 | 777.3 | 773.3 | 777.3  |

Change: -4.0% vs hht (median). At 20 lanes hht + push-select gave
785.2 / 795.7 / 790.7 (no hht run at 20 lanes).
Hot-loop instructions per node (callgrind, znver5 build without AVX-512,
random:16:2026, 1 thread): 188.04 (hht) -> 186.58 (-0.8%).
1-thread solves/s: not recorded.

Conclusion: fewer instructions but 4% slower. The side branch is
predicted well (m4's later TAGE trace: 0.020 mispredicts per node on
`normal`), so the selects only add data dependencies: the 32-byte cube
blends now wait for node.normal and the looked-up values, which puts
them on the push's critical path.

Source: scratchpad m3/neg-push-select.patch (= wip-pushsel), made against
b90b706 (that version is opt/neg-push-select, 51f5b2a). The numbers are
for the change on top of hht, so it is committed here on 70567ff (the
m3-hht merge, whose tree equals the agent's hht build up to one rustfmt
line wrap); applies cleanly.
Numbers are the agent's own (not re-measured for this record). Not merged; recorded per the rule that every candidate gets a branch.
