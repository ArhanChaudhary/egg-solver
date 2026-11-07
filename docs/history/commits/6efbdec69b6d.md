# c4: next_run without #[cold] (negative, -2.7% on ff2b7a4, neutral on 35c608c)

Commit `6efbdec69b6d5f6b5a3c67be3fbf572cdf662737` · 2026-11-03T17:25:34+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/c4-next-run-not-cold`

## Original experiment record

c4: next_run without #[cold] (negative, -2.7% on ff2b7a4, neutral on 35c608c)

c4's next_run (set up the next run of child jobs) loses #[cold] but stays
#[inline(never)].

Why: next_run runs about 47K times per cube; check what the attribute
costs or buys.

Stream throughput, 48 threads (bench stream 20 30 48 20):

| base    | build                        | solves/s      | nodes/s       | change (nodes/s) |
|---------|------------------------------|---------------|---------------|------------------|
| ff2b7a4 | c4 (#[cold])                 | 841.3         | 2594M         |                  |
| ff2b7a4 | not cold (this patch)        | 818.3         | 2525M         | -2.7%            |
| ff2b7a4 | #[inline(always)]            | 823.6         | 2546M         | -1.9%            |
| 35c608c | c4 (#[cold]), 2 runs         | 897.6 / 890.9 | 2764 / 2743M  |                  |
| 35c608c | not cold, 2 runs             | 902.1 / 888.3 | 2778 / 2735M  | ~0               |

1-thread solves/s: not recorded.

Conclusion: on ff2b7a4, without #[cold] LLVM laid the search loop out
worse (extra spills, a rematerialized TABS base); on 35c608c the
difference is gone. #[cold] stays because it is never worse.

Source: scratchpad c4/neg-next-run-not-cold.patch, made on top of the c4 patch (c4 worktree commit 70c221a, the same tree as 11e1de0 on neg-c4-childorder); applies cleanly.
Numbers are the agent's own (not re-measured for this record). Not merged; recorded per the rule that every candidate gets a branch.
