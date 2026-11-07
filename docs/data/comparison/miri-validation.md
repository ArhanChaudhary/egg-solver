## Fresh Miri validation

The x86 AVX2/BMI2 and portable solver paths passed Miri with symbolic-alignment checks and four virtual CPUs. Each path also passed all catalogued shared-search tests under scheduling seeds 0 through 15. The staged-sort tests execute disjoint tasks concurrently with up to three workers and join before every stage barrier. A contention test also exercises the mutex park/wake path.

| Path | Catalogued unit tests | Executed unique tests | Explicitly ignored | Shared-search seeds |
|---|---:|---:|---:|---:|
| x86 | 56 | 50 | 6 | 16 |
| portable | 54 | 48 | 6 | 16 |

Explicit Miri exclusions:

- `solver::tests::hot_index_matches_reference_index`: Cocsep::generate is too slow under Miri.
- `tables::file::tests::trailer_write_keeps_data`: files.
- `tables::tests::cocsep_classes_and_distribution`: Cocsep::generate is too slow under Miri.
- `tables::tests::opened_table_file_is_read_only`: files.
- `tables::tests::table_from_file_memory_is_checked`: a 29 MiB checksum is too slow under Miri.
- `tables::tests::table_meta_and_size_are_checked`: files.

Linux SysV shared-memory calls, native inline assembly, AVX-512, NEON and WASM-specific code were not interpreted by these runs. The native, portable, ARM/QEMU, WASI/wasmtime and Bun/WASM checks validate those supported paths separately; full table-generation and solver integration tests run natively. Miri checks the executions and schedules exercised by the tests; it does not prove safety for every input or interleaving.

The initial optional strict-provenance attempt stopped at pointer tagging in `parking_lot_core 0.9.12` (`word_lock.rs:320`, seed14): an unsupported operation in that mode, not a reported undefined-behavior finding. The accepted runs use default provenance handling, retain the pointer-cast warnings, and keep aliasing, alignment, bounds, validity and race checks enabled. The original diagnostic is preserved separately.

A subsequent run rejected `parking_lot_core 0.9.12` Linux futex c-variadic pointer arguments under seed15. The production dependency contains a minimal backport of upstream explicit raw-pointer conversions; the accepted Miri runs use that same dependency source.

Toolchain: `miri 0.1.0 (1c3a650696 2026-11-02)`. Source SHA-256: `93478bc5c45ba267255574f8cb466f02f006f1c7bf558fb8f9d088ec4307a4c9`. Completed: 2026-11-07 14:47:20 UTC. Raw logs and catalogues are retained in `/data/agents/codex-resume/miri-current/` and `miri-portable/`. Build, individual-worker and catalogue-coverage failures all fail the runner; failure-injection probes verify those guards.
