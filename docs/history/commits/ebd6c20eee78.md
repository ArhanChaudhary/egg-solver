# Prefetch table lines with the NTA hint

Commit `ebd6c20eee78be7c581e01b0bf66f91ed3cfee14` · 2026-11-03T01:56:11+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

Prefetch table lines with the NTA hint

Each pruning-table line is read exactly once per lookup and never reused,
so prefetching it non-temporally keeps it from displacing L2-resident hot
data (the packed cocsep table, SYM_EDGE, lane stacks). An earlier attempt
before the cocsep packing measured worse; with the smaller cocsep
footprint it now wins.

Benchmark (bench stream 20 30 48 16, interleaved A/B, 3 runs each):

| build          | run 1 | run 2 | run 3 | median |
|----------------|-------|-------|-------|--------|
| main 3957bbc   | 614.2 | 619.1 | 613.2 | 614.2  |
| prefetch NTA   | 635.6 | 630.1 | 627.1 | 630.1  |

+2.6% solves/sec.
