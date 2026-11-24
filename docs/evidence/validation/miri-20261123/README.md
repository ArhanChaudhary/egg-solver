# Miri rerun, November 23, 2026

[Safety overview](../../../safety/README.md) · [Undefined-behavior evidence](../../../safety/NO_UB.md)

The November 7 Miri records describe the code of that day. The quarter-turn metric (November 16) and the lists of all solutions (November 20 to 23) changed the search code after it. This folder records the Miri runs made on that later code. They are finite runs of unit tests in an interpreter: they are not a proof that every execution is free of undefined behavior.

## What ran

`scripts/miri.sh` runs `cargo miri test --lib --bins`: the unit tests of the library and the binaries, with the shared-search tests (`lanes::share::tests::share_*`) under 16 scheduler seeds. Toolchain: miri 0.1.0 (1c3a650696 2026-11-02), rustc 1.101.0-nightly (1c3a65069 2026-11-02). Flags: `-Zmiri-symbolic-alignment-check -Zmiri-num-cpus=4`, default provenance (the synchronization dependency uses exposed provenance, as on November 7).

| Run | Source | Result |
|---|---|---|
| Full catalogue, x86 backend (`-C target-cpu=x86-64-v3`) | `660c14e` | 55 of 61 catalogued tests passed, 6 explicitly ignored; 16 seeds of 5 shared-search tests passed |
| Full catalogue, portable backend (`-C target-cpu=x86-64`) | `660c14e` | 53 of 59 catalogued tests passed, 6 explicitly ignored; 16 seeds of 5 shared-search tests passed |
| Tests of lists that end early (time limit, cancel), both backends | `0231f19` | 3 tests under 16 seeds passed on each backend |
| Input parsing (`cubie::parse_cube`, facelets), both backends | `9d567df` | 2 tests passed on each backend |

The source fingerprint of `660c14e` is `52c7c3bc374b541cada2e5bc9aa145653bcaaead95d01abbb3e1d46e2d10a9f2` (sha256 over the sorted tracked .rs files, Cargo.toml files, Cargo.lock, .cargo/config.toml, scripts/miri.sh and scripts/check.sh (name, NUL, sha256 of the content), read from the git commit).

## What the later commits changed

- `0231f19` adds unit tests only (inside `#[cfg(test)]`); the library code is that of `660c14e`. Its three tests exist because the catalogue of `660c14e` had no unit test that stopped a list: those tests were integration tests, which Miri does not run.
- `9d567df` changes how input lines are read (`cubie::parse_cube`, `cli::input_cube` and their callers). The full catalogues were not rerun on it; its new unit test was.

## Explicitly ignored tests

The same six on both backends, for the reasons in the source:

| Test | Reason |
|---|---|
| `solver::tests::hot_index_matches_reference_index` | Cocsep::generate is too slow under Miri |
| `tables::file::tests::trailer_write_keeps_data` | files |
| `tables::tests::cocsep_classes_and_distribution` | Cocsep::generate is too slow under Miri |
| `tables::tests::opened_table_file_is_read_only` | files |
| `tables::tests::table_from_file_memory_is_checked` | a 29 MiB checksum is too slow under Miri |
| `tables::tests::table_meta_and_size_are_checked` | files |

## Not covered by these runs

- Miri runs unit tests only: integration tests and doctests were not interpreted.
- Native inline assembly, native AVX-512, file mappings, System V shared memory, and the NEON and WebAssembly backends are outside the interpreter, as before.
- AddressSanitizer, ThreadSanitizer and the focused strict-provenance Miri runs of November 7 were not repeated on the later code.
- Sixteen scheduler seeds are repeated schedules, not all interleavings.

## Files

[validation.json](validation.json) has the counts, flags, source fingerprints and the SHA-256 of every log. The logs of the two catalogues are in `x86/` and `portable/` (`run.out` ends with the script's verdict; `catalogue.log` lists the tests; `expected.txt` and `accounted.txt` are the coverage comparison). The targeted runs are `stop-tests-*.log` and `parse-cube-*.log`. Paths in the logs are those of the machine that ran them.
