# Source identities and evidence scope

[Documentation](README.md) · [Benchmarks](comparisons/README.md) · [Verification](reproduce/README.md)

Usage guides describe the commands in this source checkout. Benchmark and safety reports describe the specific source bytes they tested. A dated source identifier is not a claim that it is still the repository's current `main`.

| Evidence | Measured source / scope | Record |
|---|---|---|
| November 8 nissy throughput | Published source `947c20d`; retained CLI build inputs match except generator help text not linked into `solve` | [Report](comparisons/NISSY_THROUGHPUT.md), [manifest](data/nissy-throughput-20261108/manifest.json) |
| Earlier multi-solver matrix | Egg `229b99f`; later `842a504` checked separately | [Historical matrix](comparisons/RESULTS.md) |
| Miri and sanitizer audit | Candidate `316597f`, integrated as `842a504`; source fingerprint recorded | [Safety assessment](safety/NO_UB.md) |
| Physical Xeon counters | `935cb7f` plus the recorded profiling patch; Westmere build | [Counter study](performance/HARDWARE_COUNTERS.md) |
| Other experiments | Their own pinned sources, compilers, inputs and timing formulas | [Catalog](BENCHMARK_CATALOG.md), [history](history/README.md) |

The retained evidence includes raw text/CSV/JSON, datasets, methods, licenses, source snapshots and dated experiment notes. Executables, build caches, pruning-table data and binary PMU recordings are excluded. Machine paths in raw records identify the measurement environment; they are not installation requirements.

`MANIFEST.sha256` checks the current documentation file bytes. `SOURCES.json` records origins and hashes from the original evidence collection; it is a provenance record, not the checksum list for later edited guides. Individual studies also retain their own raw-output hashes. These checks detect changed files, not incorrect scientific claims.

Historical evidence is preserved when a summary is rewritten. Old status messages, pending work, test counts and benchmark formulas are interpreted at their recorded date. Backend correctness identities remain where relevant; the excluded backend performance comparisons are not republished.
