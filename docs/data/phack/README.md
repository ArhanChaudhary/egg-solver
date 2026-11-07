# Physical-machine counter evidence

[Analysis and interpretation](../../performance/HARDWARE_COUNTERS.md) · [Benchmark catalog](../../BENCHMARK_CATALOG.md)

This collection was returned from the Xeon X5650 host. It is separate from the EPYC VM comparison. `SOURCE-COMMIT`, `BUILD-MANIFEST.json` and `configuration.json` identify its source, compatible backend and settings. Keep those qualifiers with any reused result.

| File or directory | Purpose |
|---|---|
| `runs.json` | Individual run records |
| `counters.csv`, `counter-medians.csv` | Raw counter rows and recorded aggregate export |
| `analysis/audit.json`, `analysis/counter-ratios.csv` | Checked interpretation and corrected normalized ratios |
| `stat/` | Per-run perf-stat output, command records and solver output |
| `record/` | Text reports, headers, annotations, branch and memory exports for six sampled profiles |
| `analysis/*.annotate*.txt` | Additional detailed symbol annotations |
| `placement/`, `system/` | Measured page placement, topology and machine conditions |
| `preflight/`, `event-support.json` | Which events were actually supported |
| `references/` | Event definitions, perf reference text and applicable upstream license |

Read the analysis before using the original SUMMARY or median exports: the review found reporting/normalization issues and gives the corrected interpretation. Binary perf.data recordings, executables and the symbol filesystem are outside this documentation bundle; their text exports are included. No hardware counters were available on the VM.
