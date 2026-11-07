# Twin-bound prune saving with the original branch structure (rejected: neutral)

Commit `f190cb97bdec0bb7d33083c951dcf98d22e1f815` · 2026-11-03T02:41:08+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/twinlate`

## Original experiment record

Twin-bound prune saving with the original branch structure (rejected: neutral)

Not merged. Numbers measured by the m2 micro-optimisation subagent (interleaved A/B, bench stream, solves/sec; main = 59ce145 in the same session).

| build     | runs          |
|-----------|---------------|
| main      | 563.6 / 565.8 |
| twinlate  | 558.1 / 571.0 |
