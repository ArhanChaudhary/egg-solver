# Nxopt with the 62.474 GB table

[Benchmarks](README.md) · [Timing definitions](METHODS.md) · [Raw records](../data/nxopt32-20261107/README.md)

Measured **November 7, 2026**, on the **48-core EPYC 9V45 VM**, with resident **1 GiB pages**. Nxopt used EO3/EP2 with `PS=16`: a **62,474,158,084-byte** table (62.474 decimal GB). `nxopt32` is this experiment's configuration label, not a release version. Egg used h11 and 40 lanes per worker.

## Results and metric boundary

**Lower reported time is faster**, but the two multi-worker columns use different timing formulas. Egg measures complete batch wall time ÷ cubes. Nxopt reports summed cube durations ÷ workers ÷ cubes. Nxopt's normalization can omit the idle finishing tail. Its reciprocal is not a measured end-to-end throughput rate.

| Workers | Distinct states | Egg batch time/cube (ms) | Nxopt normalized time/cube (ms) |
|---:|---:|---:|---:|
| 1 | 50 | 41.803 | 715.342 |
| 48 | 200 | 1.1584 | 15.9014 |

Each row repeats the same input prefix in **three groups inside one process**. They are not three independent process starts. Nxopt's group ranges were 714.942–715.625 ms at one worker and 15.824–15.945 ms at 48 workers. Loading and generation are excluded.

The ratios of the reported columns are 17.11× and 13.73×. The second is a ratio of these specific timing formulas, not a direct ratio of two measured batch completion rates.

## Builds and correctness

Nxopt source was `f430921a48306874864b9f2ac8a059b43492fd3a`, using its native optimization flags and `-DPS=16`. A PGO build trained on 24 disjoint positions measured 717.845 ms and 15.9460 ms under the same formulas, so it did not improve this sample. The native build is reported above. Egg's source/build fingerprint and table bytes are in the [experiment metadata](../data/nxopt32-20261107/README.md).

For each nxopt build, all 150 one-worker and 600 48-worker returned lengths matched egg. Egg also replayed its solutions. Those are checks on the measured inputs, not universal correctness proofs.

## Scope and records

The 182 GB and 546 GB nxopt configurations remain unmeasured. Earlier nxopt22/31 results use smaller tables. Neither dataset substitutes for this large-table experiment or establishes a ranking against every nxopt configuration.

[The evidence directory](../data/nxopt32-20261107/README.md) contains `comparison.json`, raw output, commands, datasets, source/build records and analysis. Its restoration files describe the machine at that earlier date. The [November 8 page record](../data/nissy-throughput-20261108/restored-tables.json) describes the later h6–h10 RAM working set; neither is a requirement for running the program elsewhere.
