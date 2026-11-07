# Inspect and reproduce the report

[Documentation home](../README.md) · [Benchmark catalog](../BENCHMARK_CATALOG.md)

## Verify the bundle

From the `docs/` directory, run:

```sh
python3 reproduce/verify_bundle.py
```

The manifest checks file contents, not the truth of a benchmark claim. The original run metadata, source commits and report methods supply the context for those claims.

## Rebuild the comparison tables from the recorded logs

This needs Python3 and the files in this bundle; it does not run solvers or require large tables:

```sh
python3 reproduce/rebuild_comparison.py
```

The parser and aggregation logic come from the comparison report generator. Only its wiki and bug-report input paths were made relative. The packaging check confirmed that **every Markdown table line** reproduced identically to the accepted report before presentation/link adaptation. The script writes a new `reproduced-comparison.md`; it preserves the packaged report and data.

The benchmark run index marks eligibility and selected builds. Preserve the differences between sample intervals, repeat ranges and paired ratios when doing your own analysis. Do not combine chunked distance-20 probes into a headline timing unless the requested sample is complete.

## Collect new measurements

Build the source on the target machine and record the source commit, compiler, CPU flags and binary hash. Generate or copy a verified table that fits available RAM. Use the same cube inputs, table-size group, page type, worker count and solve semantics for both solvers. Separate startup/table loading from search time. Record actual page coverage and outside CPU load. Alternate A/B order and repeat measurements before interpreting small effects.

The full report gives the sampling and acceptance rules. `harness/runtest.sh` is the extended historical harness with its license; it refers to separately installed competitor projects and the original machine layout. It is provided for method inspection, not as a one-command installer. The source project's own `bench`, `solve`, `gen` and shell tools are documented in the user guide.

No new timing measurements were performed to assemble this documentation archive.

## Physical-host analysis code

`historical/analyze-phack.py` preserves the counter audit and normalization formulas used for the physical-host report. Its original full audit also verifies executable hashes and refers to the original VM paths; running it requires the original returned archive, which contains binaries outside this documentation bundle. `historical/pmu_reporting.py` and `historical/replay-phack-exports.py` preserve the annotation/export corrections. The CSV/JSON input and checked counter-ratio output are included under `data/phack/` for independent analysis.
