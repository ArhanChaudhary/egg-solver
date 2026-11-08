# Development scripts

Run from the project root. Build output follows `CARGO_TARGET_DIR` when set.

| Command | Purpose |
|---|---|
| `scripts/check.sh` | Formatting, Clippy, native tests and available backend checks |
| `scripts/miri.sh` | Miri unit-test catalogue and shared-search scheduling seeds |
| `scripts/stress.sh TABLE [set] [threads] [lanes]` | Compare solution lengths across search modes |
| `scripts/bench.sh TABLE [runs] [lanes] [threads]` | Repeated stream throughput measurements |
| `wasm/build.sh` | Build browser/Bun WebAssembly modules |
| `bun scripts/render-docs.mjs` | Regenerate HTML from Markdown (`--check` verifies it is fresh) |
| `python3 scripts/check-docs.py` | Check local links, document/table structure and headline benchmark values |
| `scripts/build-pages.sh OUTPUT_DIRECTORY` | Build the browser solver and stage documentation for Pages |

Miri requires a nightly toolchain with the Miri component. See
[scripts/miri.sh](miri.sh) for toolchain, log-directory and provenance options. Backend checks
report missing tools rather than claiming those backends passed.

`scripts/experiments/pgo-solve.sh` retains the optional historical PGO experiment.
It is not part of the release build or deployment workflow. Its measurements
and limitations remain in the [benchmark history](../docs/history/README.md).
