# nxopt with the large PS=16 table

Measured on 2026-11-07 on the 48-core AMD EPYC 9V45 VM. This completes the requested experiment with nxopt's nominal 62.3 GB table: the generated file is **62,474,158,084 bytes** (62.474 decimal GB). The program is labeled `nxopt32` in the retained files.

Both programs used resident 1 GiB huge pages. Egg used h11, 62,277,025,792 bytes, with 40 lanes per worker and batch concurrency. The nxopt source was commit `f430921a48306874864b9f2ac8a059b43492fd3a`, built with its native optimization flags and `-DPS=16`. These results concern this host, build, table, dataset prefix and measurement formula; they do not establish a ranking on other machines or against larger tables.

| Workers | Unique random states | Repeated groups | Egg batch wall time/cube | nxopt normalized time/cube | nxopt time / egg time |
|---|---:|---:|---:|---:|---:|
| 1 | 50 | 3 | 41.803 ms | 715.342 ms | 17.11× |
| 48 | 200 | 3 | 1.1584 ms | 15.9014 ms | 13.73× |

**Metric limitation:** nxopt's figure is the sum of per-cube solve durations divided by the number of cubes and workers, as in the original comparison harness. Egg's figure is measured batch wall time divided by cubes. Uneven worker completion can make nxopt's normalized value smaller than its actual batch wall time per cube. Do not present nxopt's reciprocal as a directly measured end-to-end completion rate. Table loading and generation are excluded for both.

The three groups repeat the same prefix inside one process; they are not three independent process starts or 150/600 distinct states. Nxopt's group ranges were 714.942–715.625 ms at one worker and 15.824–15.945 ms at 48 workers. Its mean node counts were 3.317M and 3.292M respectively; node instrumentation is implementation-specific.

A profile-guided build trained on 24 disjoint positions was also tested. It measured 717.845 ms at one worker and 15.9460 ms at 48 workers, so it did not improve this sample. The native build is the main result.

All returned solution lengths matched egg for all 150 one-worker and 600 48-worker solves, for each nxopt build. Egg also verified that its solutions solved the input states. Agreement is a cross-check on these inputs, not a proof covering every possible state.

The 182 GB and 546 GB nxopt configurations were not measured. The earlier nxopt22 results use a different table and must not be substituted for these PS=16 results.

Evidence: `comparison.json`, `raw/`, `meta/`, `datasets/`, `src/`, `build.sh`, `build-pgo.sh`, `run.py`, and `analyze.py` in this directory. The h11 reconstruction certificate preserves the exact previously loaded table, including its trailer. Restoring the h11/h7/h8 RAM copies is recorded separately in `restored-tables.json` when finished.
