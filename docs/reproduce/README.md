# Check and reproduce recorded results

[Documentation](../README.md) · [Benchmark catalog](../BENCHMARK_CATALOG.md)

Run the commands below from the project root. They read recorded evidence; they do not require pruning tables or rerun solver benchmarks.

## Check file integrity

```sh
python3 docs/reproduce/verify_bundle.py
```

This compares documentation files with `docs/MANIFEST.sha256`. A matching hash establishes file integrity, not the validity of a performance or safety claim. [Provenance](../PROVENANCE.md) explains source versions and original-origin hashes.

## Recompute the newest throughput table

```sh
python3 docs/data/nissy-throughput-20261108/analyze.py --require-complete
```

The analyzer checks accepted raw-output hashes and recomputes the final medians, rates and single-cube scaling. It refreshes the derived `analysis.json` and `run-index.csv` in that study directory; the raw logs are unchanged. The [report](../comparisons/NISSY_THROUGHPUT.md) links every final raw batch.

## Check the historical matrix

```sh
python3 docs/reproduce/rebuild_comparison.py
# Optionally save the reconstructed report:
python3 docs/reproduce/rebuild_comparison.py --output /tmp/egg-historical-comparison.md
```

This runs the recorded parser/aggregator and compares every numerical table row against the immutable reference under `data/comparison/`. The reader-facing historical overview can have different presentation. The reconstructed report retains the old formulas and is not a current-source benchmark.

## Collect new measurements

Record the source commit, compiler, CPU, binary hash, input bytes, table sizes, page coverage, worker allocation and timing boundaries. Distinguish one-cube latency, finite-batch wall time and fixed-window streaming. Repeat fresh processes, alternate solver order, and check results before interpreting small differences. [Methods](../comparisons/METHODS.md) defines the metrics used here.

The archived competitor harness and measurement scripts contain paths and dependencies from the original machines. They are method records, not portable installers. The project commands are documented in the [user guide](../reference/USER_GUIDE.md).

## Physical-host analysis

The [Xeon study](../performance/HARDWARE_COUNTERS.md) has its own host, source and counter definitions. `historical/analyze-phack.py`, `pmu_reporting.py` and `replay-phack-exports.py` retain the analysis and export corrections. Full replay needs the separately retained original counter archive; available CSV/JSON and checked aggregates are under `data/phack/`. This VM has no hardware PMU counters.
