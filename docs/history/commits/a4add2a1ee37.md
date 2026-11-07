# Node stores the chosen-side cubes, 96-byte node (rejected: -2.9%)

Commit `a4add2a1ee372f9576be441492541de307030cc0` · 2026-11-03T05:28:05+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/node-ab-layout`

## Original experiment record

Node stores the chosen-side cubes, 96-byte node (rejected: -2.9%)

Cumulative with the skip-sibling change.

| build                    | solves/sec |
|--------------------------|------------|
| skip-sibling             | 771.3      |
| + chosen-side node layout| 748.7 (-2.9%) |

Not merged. Numbers measured by the b2 architecture subagent (interleaved
A/B, bench stream, solves/sec). Variants of this code swing by about +-3%
between builds, so single-run differences under 3% are layout noise.
