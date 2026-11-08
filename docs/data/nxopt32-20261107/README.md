# Large nxopt PS=16 table experiment

See [the report](../../comparisons/NXOPT32.html). These records retain the first-50/first-200-state datasets, three repeated input groups, native and profile-trained nxopt results, egg h11 batch results, page samples, build commands and source fingerprints. Large tables and compiled binaries are excluded.

The source version is nxopt `f430921a48306874864b9f2ac8a059b43492fd3a`, compiled with `-DPS=16`. The generated table is 62,474,158,084 bytes. Repeated groups share one process. Nxopt uses summed per-cube durations divided by cubes and workers; egg uses batch wall time divided by cubes. These are not interchangeable end-to-end rates when worker completion is uneven.

Paths in scripts and metadata refer to the measurement VM and need adapting on another host. This is a retained experiment, not a universal solver ranking.
