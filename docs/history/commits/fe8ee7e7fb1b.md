# NEGATIVE m8: setup_run prefetches the job and child order of the run 4 entries ahead (neutral)

Commit `fe8ee7e7fb1ba4c2c468a7b07903c0344527f149` · 2026-11-05T02:56:50+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/m8-run-prefetch-4`

## Original experiment record

NEGATIVE m8: setup_run prefetches the job and child order of the run 4 entries ahead (neutral)

setup_run fetches the job and the ChildOrder of the next run entry while
it starts the current run (c7): the job vectors are too big for L2 and
the lanes interleave their streams. One entry ahead is too late when
entries are skipped (an entry whose child already ran with an earlier
entry's group `continue`s at once): in a 1T stream profile setup_run was
~2.5% of the time, with 21% of its samples on the ChildOrder load and
7.5% on the job's. Now it prefetches the entry RUN_PREFETCH = 4 ahead.
(Each entry is still prefetched once; the first 3 entries of a part are
not.)

asm: setup_run 833 -> 831 instructions; the hot loops are unchanged
(instruction-identical, same addresses). Search identical (the order of
runs is unchanged).

1T stream (ab1.sh: paired, start order alternating; stream 10 60 1 24):
| run | 7608a28 | this |
|-----|---------|------|
| 1 | 26.067 | 26.150 |
| 2 | 25.967 | 25.967 |
| 3 | 25.833 | 26.033 |
| 4 | 26.183 | 26.283 |
| 5 | 26.233 | 26.183 |
| 6 | 26.033 | 26.283 |
| 7 | 26.100 | 26.117 |
| 8 | 26.217 | 26.183 |
| median | 26.067 | 26.150 (+0.3%; pairs +0.32 0.00 +0.77 +0.38 -0.19 +0.96 +0.07 -0.13) |
48T stream (ab48.sh, stream 20 30 48 24; f1's exclusive holds ran in
between, so the two runs of a pair were up to ~20 min apart):
| run | 7608a28 | this |
|-----|---------|------|
| 1 | 1098.367 | 1098.533 |
| 2 | 1089.000 | 1034.800 |
| 3 | 1092.267 | 1086.233 |
| 4 | 1080.967 | 1081.700 |
| 5 | 1073.733 | 1092.967 |
| median | 1089.000 | 1086.233 (-0.3%; pairs +0.02 -4.98 -0.55 +0.07 +1.79) |
Neutral at both thread counts: +0.3% at 1T is within the A/A noise
(+-0.5%), and 48T shows nothing. Share mode does not run setup_run
(its lanes use setup_job), so the share measurements were skipped. Not
delivered. (On 7608a28; rebased onto 3a9a50a without conflicts.)
