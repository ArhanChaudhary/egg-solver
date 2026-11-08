# Remaining implicit-check inventory

Before: 0b5b5a6. After: 935cb7f (no-panic integration plus the stream follow-up). Rust Clippy diagnostics from release library and binaries,
union of native and scalar configurations, deduplicated by file, line, column and lint.
These are source-level diagnostics, not a count of reachable panic branches: the optimizer
can prove bounds or nonzero divisors, and arithmetic warnings also include non-panicking
operations. No unchecked access was added merely to silence these diagnostics.

| file | indexing/slicing before | after | arithmetic before | after |
|---|---:|---:|---:|---:|
| src/bin/bench.rs | 2 | 2 | 4 | 4 |
| src/bin/gen.rs | 3 | 4 | 13 | 14 |
| src/bin/solve.rs | 4 | 2 | 2 | 1 |
| src/cli.rs | 0 | 0 | 2 | 2 |
| src/cli/preflight.rs | 0 | 0 | 0 | 2 |
| src/coord.rs | 15 | 15 | 34 | 34 |
| src/cubie.rs | 83 | 44 | 45 | 31 |
| src/gen.rs | 17 | 16 | 15 | 14 |
| src/gen/collapse.rs | 6 | 6 | 13 | 12 |
| src/gen/dfs.rs | 11 | 6 | 24 | 21 |
| src/hot.rs | 2 | 0 | 2 | 1 |
| src/lanes.rs | 53 | 52 | 72 | 72 |
| src/lanes/share.rs | 67 | 63 | 150 | 150 |
| src/lib.rs | 0 | 0 | 2 | 1 |
| src/parallel.rs | 1 | 0 | 0 | 0 |
| src/portable/coord.rs | 0 | 0 | 1 | 1 |
| src/portable/mod.rs | 40 | 40 | 11 | 11 |
| src/portable/v16_scalar.rs | 3 | 3 | 0 | 0 |
| src/simd.rs | 27 | 27 | 5 | 5 |
| src/solver.rs | 42 | 39 | 40 | 40 |
| src/symstart.rs | 11 | 10 | 4 | 4 |
| src/tables.rs | 31 | 22 | 32 | 31 |
| src/tables/file.rs | 3 | 1 | 13 | 8 |
| src/tables/shm.rs | 0 | 5 | 0 | 13 |

Explicit runtime panic macros and unwrap/expect calls are enforced separately by crate
restriction lints and tests/no_runtime_panics.rs, across all supported backends in check.sh.
Tests and compile-time assertions are allowed; existing debug_assert checks remain per the
standing policy. See np1-panic-inventory.md for the site-by-site explicit-call remediation.

### Rename validation

Final renamed source `cd963dc26b08` also passed the enforced explicit-call scan and full native/scalar/ARM/WASM checks on2026-11-06. The benchmark and compiler-diagnostic inventories above keep their recorded935cb7f provenance. The rename preserves the checked hot instructions, required page offsets and stack frames.
