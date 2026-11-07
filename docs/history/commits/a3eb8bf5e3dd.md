# lanes: read the next lane's lookup ahead, and the prune bound

Commit `a3eb8bf5e3ddb7457425c21bd0a474dc00a0f6ad` · 2026-11-04T06:26:43+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `archive/m7-ra-next`

## Original experiment record

lanes: read the next lane's lookup ahead, and the prune bound

NEGATIVE (not for merge). Against f2cb1f4 (ported from cb8cb70; c7's
per-root-child statistic kept).

Idea: the prune branches of a visit (po > r1, pm > r1, and the
exhausted-node test after the face cut) wait for the pending value: lane
state load -> table word load -> shift -> code. A diagnostic that adds 9
cycles to that chain (3 dependent imuls on the value) costs 3.1% more
than the same imuls off the chain (1T at 24 lanes, same build folder, 3
runs: main 24.533, on-chain 23.300, off-chain 24.050), so the chain's
latency counts.

This version (the last of four):
- each visit loads the next lane's table word (`Solver::value_word`,
  the load half of `value`) into that lane's `k_word`. The visit then
  needs only `k_word` & 3 and the base (the code-0 path loads the group
  minimum as before). A spare lane past the active ones absorbs the
  read-ahead of a round's last visit;
- the pending child's r - 1 is kept in the lane (`k_remaining`), so the
  prune tests do not wait for sp -> node loads.

Search unchanged. But every version is slower:
- the read-ahead takes one visit of slack from each lookup, at a lane
  count that is already short of slack (16/20/28 lanes: -9.6% / 0 /
  +10.2% at 1T);
- the extra loads and stores raise register pressure: the TABS base
  register is lost, so each table access needs an lea.

1T on f2cb1f4, both sides built in the same folder (ab1t.sh, 24 lanes,
solves/s):
| build | runs                               | median |
|-------|------------------------------------|--------|
| main  | 25.400 25.583 25.067 25.583 25.100 | 25.400 |
| ra7   | 24.800 24.733 24.533 24.567 24.417 | 24.567 (-3.3%) |
Earlier, on cb8cb70 (builds in different folders, so up to ~2% of
layout noise; superseded by the table above):
| build                                  | lanes | main runs            | variant runs         | change |
|----------------------------------------|-------|----------------------|----------------------|--------|
| ra: value carried in a register        | 20    | 21.467               | 20.350               | -5.2% (1 run; cachegrind 148.79 -> 161.8 Ir/node, spills) |
| ra6: word through the next lane's field | 24    | 23.617 23.783 23.583 | 22.617 22.800 22.517 | -4.2% (medians) |
| ra7: ra6 + k_remaining                 | 24    | 23.467 22.667 22.667 | 22.517 21.500 21.650 | -4.5% (medians) |
(ra4, the word carried in a register: cachegrind 157.19 Ir/node, more
spills; not timed.) Not run at 48T.
