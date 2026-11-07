# No-panic source inventory

Before: `0b5b5a6`. After: `935cb7f`. The same lexer used by the project enforcement test
scanned all shipped Rust files, including alternative backends, binaries, build scripts and WASM.
Comments, strings, tests and compile-time initializers are excluded. Existing `debug_assert!`
is allowed by the standing policy. Raw findings: `panic-source-audit.tsv`.

Explicit calls matching the enforced policy: **127 before → 0 after**.

| file | explicit before | after | `/` before | after | `%` before | after |
|---|---:|---:|---:|---:|---:|---:|
| benches/kernels.rs | 0 | 0 | 0 | 0 | 3 | 3 |
| build.rs | 25 | 0 | 0 | 0 | 0 | 0 |
| src/bin/bench.rs | 4 | 0 | 8 | 8 | 0 | 0 |
| src/bin/gen.rs | 3 | 0 | 3 | 3 | 0 | 0 |
| src/bin/solve.rs | 4 | 0 | 5 | 5 | 2 | 2 |
| src/cli.rs | 0 | 0 | 2 | 2 | 0 | 0 |
| src/cli/preflight.rs | 0 | 0 | 0 | 2 | 0 | 0 |
| src/coord.rs | 0 | 0 | 2 | 2 | 1 | 1 |
| src/cubie.rs | 15 | 0 | 6 | 8 | 19 | 15 |
| src/gen.rs | 1 | 0 | 4 | 4 | 0 | 0 |
| src/gen/collapse.rs | 6 | 0 | 0 | 0 | 0 | 0 |
| src/gen/dfs.rs | 8 | 0 | 7 | 5 | 2 | 2 |
| src/hot.rs | 2 | 0 | 0 | 0 | 0 | 0 |
| src/lanes.rs | 1 | 0 | 7 | 7 | 7 | 7 |
| src/lanes/share.rs | 12 | 0 | 19 | 19 | 5 | 5 |
| src/lib.rs | 7 | 0 | 0 | 0 | 0 | 0 |
| src/parallel.rs | 2 | 0 | 0 | 0 | 0 | 0 |
| src/portable/mod.rs | 2 | 0 | 0 | 0 | 0 | 0 |
| src/simd.rs | 2 | 0 | 1 | 1 | 1 | 1 |
| src/solver.rs | 0 | 0 | 9 | 9 | 2 | 2 |
| src/symstart.rs | 3 | 0 | 4 | 4 | 0 | 0 |
| src/tables.rs | 17 | 0 | 3 | 3 | 1 | 1 |
| src/tables/file.rs | 10 | 0 | 0 | 0 | 0 | 0 |
| src/tables/shm.rs | 0 | 0 | 0 | 1 | 0 | 0 |
| wasm/src/lib.rs | 2 | 0 | 2 | 2 | 0 | 0 |
| wasm/src/threads.rs | 1 | 0 | 0 | 0 | 0 | 0 |

Division and remainder are **source syntax counts**, not reachable panic counts. They include
floating-point division and constant nonzero divisors. Runtime indexing, slicing and arithmetic
diagnostics are recorded separately in `no-panic-implicit-inventory.md`; optimization can remove
proven checks. These inventories do not claim to prove the absence of every implicit or dependency
panic. Explicit intentional calls are enforced by check.sh across all installed backends.

## 1. Explicit panic sites in non-test code on 0b5b5a6, and their fixes

Heat: **hot** = per node / per lookup (the stream loop `bench::stream_thread`, share's `Worker::rounds`),
**warm** = per run / job / iteration / solution (job extension, iteration builds, `settle`, `found`, `fresh`),
**cold** = startup, table load and generation, parsing, the binaries' I/O, build.rs, wasm glue.
Fix kinds: (a) impossible by construction, (b) error returned, (c) handled without panicking,
(d) documented proven-unreachable unchecked path. Line numbers are 0b5b5a6's.

| file:line | kind | heat | fix |
|---|---|---|---|
| src/simd.rs:305, 337; src/portable/mod.rs:486, 521 | `assert!(mv < 18)` in `Cube::apply_move` / `premove_inverse` | warm (job extension, reference solver), cold (tables, gen) | (a) they take `cubie::Move`, an enum of the 18 moves; indexing the 18-entry tables needs no check (`apply_move_unchecked(mv.index())`, SAFETY: a `Move` is below 18). Mask loops use `Move::lowest(mask)` (the same test as `mask != 0`) |
| src/lanes.rs:740 | `assert!(target < MAXD)` in `Lane::start_iteration` | warm (per iteration) | (c) `start_iteration` returns false when the target would pass `solver::MAX_TARGET` (20, God's number): the lane gives the cube up and reports `None` (new `Step::Unsolved`, `Start::Unsolved`); `const _: () = assert!(MAX_TARGET < MAXD)` makes the stack bound a compile-time fact. Fires only for an unsolvable cube or a table whose bounds are too high (it used to panic there) |
| src/lanes/share.rs:1186 | `assert!(target < MAXD)` in `build` | warm (per iteration build) | (c) `settle` gives the cube up (`give_up`, result `None`) instead of building past `MAX_TARGET`; the first pass only jumps to a pass target ≤ `MAX_TARGET`; `build` clamps `inner.target.min(MAX_TARGET)` (never lowers a target except a first bound above 20, whose iteration finds nothing) |
| src/lanes/share.rs:1254 | `assert!(offset <= out.capacity())` | warm | (a) `out.reserve(offset)` after `clear()`: capacity by construction (a no-op: the buffer is pre-sized) |
| src/lanes/share.rs:1741, 1745, 1756 | `assert!(n <= capacity)` (keys, key_buckets, out, rest_children, runs) | warm | (a) `reserve(n)` / `reserve(2 * n)` after `clear()` |
| src/lanes/share.rs:2081 | `assert_eq!(scratch.jobs.len(), children)` before a raw copy | warm | (d) `debug_assert_eq!` + the copy reads `min(len, children)` jobs; proof in a comment: `kept_children` counts exactly what `extend_jobs` keeps (same `keep` filter on the same canonical moves; dedup filters never extend in parallel) |
| src/lanes/share.rs:2102 | `child_offsets.last().unwrap()` | warm | (c) `map_or(0, ..)` (never empty: `build` pushes the total) |
| src/lanes/share.rs:2518, 2576 | `locals[slot].as_ref().unwrap()` in `found`, `report` | warm (per solution) | (c) `let Some(Some(local)) = locals.get(slot) else { return }` (always joined: a lane with a job holds its cube's slot, as `end_job` relies on) |
| src/lanes/share.rs:3027 | `locals[slot].take().unwrap()` in `release` | warm | (c) let-else: nothing to release is a no-op |
| src/lanes/share.rs:3170 | `self.local(&ctx).unwrap()` in `fresh` | warm (per cube) | (a) new `add_local` joins a cube whose part the thread already holds (fresh: the unsolved part), infallibly; `local` uses it after its CAS |
| src/lanes/share.rs:3333 | `unreachable!` after `get_disjoint_mut` in `split` | warm | (c) `else { return false }`: no split (find_split excludes lane `li`) |
| src/lanes/share.rs:1034 | `scope.spawn` (panics if a thread cannot start) in `solve_all` | cold | (b) `Builder::spawn_scoped`; on failure the started threads are stopped and joined, `solve_all` returns the io error; join errors too |
| src/cubie.rs:255, 263, 264 | `try_from(..).expect` in `Rng::below_u8/below_usize` | cold (bench/random cubes) | (a) `as` casts of values below `n` (with `#[expect(cast_possible_truncation, reason)]`), `const _: () = assert!(usize::BITS <= u64::BITS)`; same values, verify unchanged |
| src/cubie.rs:338, 339, 522, 533, 591, 657, 790, 804 | `expect`/`unwrap`/`unreachable!` in the sticker model (sticker search, face letters, facelets, symmetries) | cold (startup derivation, facelet parsing) | (a) build.rs derives moves, symmetries, inverses and facelet tables once (`cubie::model`, every step returns `None`; `build.rs` fails with a message) and writes `$OUT_DIR/cubie_consts.rs`; the library includes it (`cfg(cubie_generated)`) and derives nothing at run time. `from_facelets`/`to_facelets` use the tables (no search, no fallthrough) |
| src/cubie.rs:610, 611, 622, 623 | `u8::try_from(piece/twist).expect` in `from_facelets` | cold | (a) the search ranges are `u8` |
| src/cubie.rs (fields) | `CubieCube` pub fields: invalid cubes made outside could index out of range in `compose`/`inverse`/`to_facelets` | cold | (a) fields `pub(crate)`: every constructor yields a valid cube |
| src/cubie.rs:144 (`apply_moves`) | took `&[u8]`, indexed `move_cubes()[mv]` | cold | (b) returns `Option<CubieCube>` (`None` for a byte that is no move); parse+apply in the bins/wasm is one `and_then` with the existing parse error |
| src/symstart.rs:189 | `index.expect(..)` + `try_from().unwrap()` computing move conjugates at run time | cold (once) | (a) the table is build.rs's `SYM_MOVE_MAP` (the same table; the existing test checks it) |
| src/symstart.rs:218 | `.min().unwrap()` | warm (per dedup) | (a) `fold((MAX, MAX), min)` |
| src/hot.rs:49, 56 | `assert!` in `pub const fn pad` | compile time (but callable at run time) | (a) `pad` is private and returns `usize::MAX` for a bad value; the asserts sit in the `STREAM_PAD`/`SHARE_PAD` constants (compile-time) |
| src/tables.rs:113, 149, 157; 131 | `assert_eq!`/`assert!`/`unwrap` checking the cocsep BFS | cold (startup) | (b) `Cocsep::generate() -> Result`, `TableError::CocsepModel`; packed entries written as `[u8; 3]` chunks |
| src/tables.rs:287, 304, 371 | mmap `assert!`, `expect("out of memory")`, `Layout ..expect` | cold | (b) `Memory::anonymous -> Result<Memory, TableError>` (`TableError::Alloc`), `layout` an `Option` |
| src/tables.rs:531, 541 | `assert!(len <= isize::MAX)` (32-bit) in `words`/`bytes` | cold | (b) `Result`, `TableError::TooLargeForSlice` |
| src/tables.rs:688, 947, 761 | `expect("64-bit usize")`, `file_name().unwrap()` | cold | (b) `TableError::TooLarge`; `meta_path` falls back to the sidecar for a hugetlbfs path without a file name |
| src/tables.rs:893, 894 | `try_from().expect` in `from_file_memory`'s pieces | cold | (c) the piece checks its range (empty out of range; the checksum then fails) |
| src/tables.rs:994, 1001, 1002 | `read_exact_at().expect`, `off_t::try_from().expect` in `read_parallel` | cold (table load) | (b) io errors returned; fadvise skipped if a number does not fit; a thread that cannot start reads its range on the caller; join errors returned |
| src/tables.rs:485 (`prefault`) | `scope.spawn` | cold | (b)/(c) as `read_parallel` |
| src/tables/file.rs:143 | `assert!(text < trailer)` | cold | (a) bounded text (fixed-width numbers, ~250 of 4096 bytes); `write_trailer` reads it back |
| src/tables/file.rs:129, 266, 314, 318, 325, 327, 369 | `try_from(const).unwrap()`, `try_into().unwrap()` | cold | (a) `TRAILER_LEN` const (const-asserted equal), `as_chunks::<8>()`; (b) mmap offsets via `try_from` → io error |
| src/tables/file.rs:455, 462 | `panic!` on bad `CUBEOPT_LOAD` / `CUBEOPT_PREFAULT` | cold | (b) `LoadOptions::from_env -> Result`, `TableError::BadEnv` |
| src/lib.rs:68, 75, 76, 78, 111, 126, 158 | `assert!(k == 0 \|\| 11)`, `expect`s | cold (gen) | (b) `generate`/`generate_table -> Result<_, TableError>` (`Generate`, `TooLarge`, `Io`) |
| src/gen.rs:332 | `expect("64-bit usize")` | cold | (b) `table_words -> Result`; `Generator::run -> Result` checks k, memory sizes; derived tables are word slices |
| src/gen/collapse.rs:53, 55, 56, 95, 99, 102 | `assert!`, `assert_eq!`, `unwrap` | cold | (b) `derive`/`compare -> Result` (`collapse_shape` checks k, sizes, overflow) |
| src/gen/dfs.rs:392, 421, 424-426, 435, 450, 451 | `assert!`/`assert_eq!` on settings and parts, `unwrap`s | cold | (b) `Generator::new`/`generate -> Result` (`TableError::Generate`); `(0u8..).zip(levels)` |
| src/gen/dfs.rs:316 | `s.spawn` progress thread | cold | (c) `Builder`; no progress line if it cannot start; joined explicitly |
| src/parallel.rs:40, 50 | `expect("the runner ran every share")`, `join().unwrap()`; `s.spawn` | cold/warm (gen, checksums, wasm solves) | (c) a share whose thread cannot start or that the runner skipped runs on the caller; (b) `WorkerPanicked` for a panicked worker |
| wasm/src/threads.rs:58, 62 | `assert!(work.is_none())`, `thread::spawn` | cold | (c) nested jobs run inline; shares without a thread run on the caller |
| wasm/src/lib.rs:89, 204 | `expect("created by state")` | cold | (a) the state is a plain `Mutex<State>` (corners made on demand, error returned) |
| src/bin/bench.rs:78, 82, 109, 113; 74 | `cubes(i).unwrap()`, `lock().unwrap()`, `assert_eq!(bad, 0)`, `into_inner().unwrap()`, `s.spawn` | warm (per solution), cold | (b)/(c) a missing cube or solution counts as bad; `run -> anyhow::Result` (`ensure!` bad == 0, thread start/join errors); poison → `into_inner` |
| src/bin/solve.rs:268, 350, 443, 447; 78 | `lock().unwrap()`, `expect("below 6")`, `into_inner().unwrap()`, `scope.spawn` | warm (per solution), cold | (c) poison → `into_inner`; `below_usize`; (b) thread errors, "no solution within 20 moves" error, BAD line for `None` |
| src/bin/gen.rs:479, 502 | `try_from().unwrap()` for sync_file_range/fadvise | cold | (c) skipped if it does not fit (advice only); `write_table` uses `get(..)` |
| build.rs:82-166 | 25 × `writeln!(..).unwrap()`, `env::var("OUT_DIR").unwrap()`, `fs::write().unwrap()` | build | (b) `main() -> Result<(), Box<dyn Error>>`, `?` |
## 4. Enforcement

* **Clippy, at every crate root** (src/lib.rs, src/main.rs, src/bin/{bench,gen,solve}.rs, build.rs,
  wasm/src/lib.rs, wasm/build.rs, benches/kernels.rs):
  `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic,
  clippy::unreachable, clippy::todo, clippy::unimplemented))]`. `cfg(test)` builds (unit tests) and
  tests/, examples/ (separate crates without the attribute) are not linted. check.sh's clippy runs
  (x86, scalar, aarch64, wasm32 targets, and `-p cubeopt-wasm` on both wasi targets) enforce it.
* **No lint flags `assert!`**: `tests/no_runtime_panics.rs` scans the shipped sources (src/,
  build.rs, wasm/src, wasm/build.rs, benches) and fails on `assert!`, `assert_eq!`, `assert_ne!`,
  `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `.unwrap()` or `.expect(` outside
  - test code: any item, statement or field after `#[cfg(test)]` / `#[cfg(all(test, ..))]`, to the
    end of its `{ .. }` block (or its `;`/`,`),
  - compile-time code: `const`/`static` initializers (from `=` to `;`, e.g. `const _: () =
    assert!(..)`, `static BUILD_DIR`) and inline `const { .. }` blocks (not `const fn` bodies).
  Comments, strings (incl. raw) and char literals are blanked first, so text in them never matches.
  A second test checks the scanner itself on a sample (comments, strings, constants, cfg(test)
  modules, fields and functions; finds a runtime `assert_eq!` and `.unwrap()`). check.sh runs it
  as its own step right after `cargo fmt` (and it also runs with the other tests). It is a Rust
  test, so it needs nothing but cargo (no Python on the laptop/CI).
* `debug_assert!` stays allowed (not in release builds).
* **Not enabled:** `clippy::indexing_slicing` (≈300 sites, most in in-range loops and hot kernels;
  per policy, no `get_unchecked` sprinkled to satisfy a lint) and `clippy::arithmetic_side_effects`
  (thousands of sites). Sections 2-3 list where the remaining implicit checks are.

# No-panic integration: remaining implicit-check inventory

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

## Rename validation

Final renamed source `cd963dc26b08` also passed the enforced explicit-call scan and full native/scalar/ARM/WASM checks on2026-11-06. The benchmark and compiler-diagnostic inventories above keep their recorded935cb7f provenance. The rename preserves the checked hot instructions, required page offsets and stack frames.
