# Provenance and scope

[Documentation home](README.md) · [Benchmark catalog](BENCHMARK_CATALOG.md)

The original evidence bundle accompanied source `842a504`. The documentation now lives in the project's `docs/` directory; current usage guides reflect the subsequent CLI and packaging cleanup. Historical measurements and safety reports retain their original source identities. The principal competitor comparison measured `229b99f`; final-source A/B records disclose the later differences. The physical-host PMU collection used `935cb7f` with its recorded profiling build. Individual historical commits identify their own baselines.

Coverage: 230 matrix cells; 552 comparison run groups; 242 PMU runs, 621 counter rows and 6 sampled profiles; 478 surviving commit messages, 151 negative/neutral branch tips and 59 archived tips. These counts describe records and requested coverage, not percentages of all possible workloads or program behaviors.

`MANIFEST.sha256` covers distributed file bytes. `SOURCES.json` records original source paths and hashes for copied evidence, and presentation edits where applicable. Plain machine paths inside raw logs are historical provenance. Reader-facing links were made relative or mapped to included source views. Original live-status files and internal task conversations are not reader documentation.

The documentation includes raw text/CSV/JSON results, datasets, methods, licenses, selected source views and historical commit messages. It excludes executables, build caches, pruning-table data, binary perf.data recordings, and the source Git database. The project archive includes this `docs/` directory. The physical-host runner remains a separate deliverable.

The requested excluded backend performance comparisons are absent from the comparison data and historical prose. Backend implementation identities and safety-test evidence remain where necessary for interpreting the validation and physical-machine profile. Historical experimental results are preserved as history rather than presented as current-source guarantees.
