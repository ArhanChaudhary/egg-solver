# Mordant audit of egg-solver

**Safety candidate audit recorded — 2026-11-07 09:30 UTC**

Mordant was rerun on candidate `316597f64a9bd749e8a01b67f96c108c03611e68`, source SHA-256 `93478bc5c45ba267255574f8cb466f02f006f1c7bf558fb8f9d088ec4307a4c9`, after the additional safety changes. Both supported-target commands completed successfully with no compiler errors. This is a completed lint audit, not a claim of zero warnings or a memory-safety certificate.

| Target | Distinct lint diagnostics | Compiler errors | Command result |
|---|---:|---:|---|
| Native workspace, excluding the WASM wrapper | 14 | 0 | PASS |
| WASI wrapper and its library dependencies | 70 | 0 | PASS |

The native findings cover the shared-search `Sync` implementation, internal constructors, enum catch-all branches, tuple return types and conditional fields. The WASI count includes 51 target-scoped unused-public-API findings and seven error-to-string conversions at the JavaScript-facing boundary. Findings overlap across targets and should not be added as distinct defects.

Review of the candidate constructor warnings found the checks at their producers: generation validates k/base before allocating; the DFS generator validates its settings; collapse derives k from a validated source with a checked shape; shared-memory attachment validates its size before constructing `Memory`. The table constructors are now internal. The preflight metadata failure still reaches the validating loader rather than accepting a bad table. The `CubeCtx` warning remains an appropriate reminder that its `UnsafeCell` ownership protocol depends on the publication and completion rules described in `NO_UB.md`; the new tracing/publication correction addresses the reproduced overlap. Tuple and enum suggestions alone do not establish an incorrect access. No source edits were made merely to silence these diagnostics.

The first WASI invocation compiled successfully but Mordant itself returned 101 because a cached build lacked its `over-baseline.txt` count. The cache was preserved under `target-safety-native/mordant-before-final-wasm-retry`, and a fresh-cache WASI retry completed with exit 0. The failed tool attempt remains recorded and is not counted as a passing run.

Evidence: [source-bound validation](../evidence/validation/mordant/final-candidate-validation.json), [recorded structured findings](../evidence/validation/mordant/final-candidate-findings.json), [preserved first attempt](../evidence/validation/mordant/final-candidate-first-attempt.json), [UB assessment](NO_UB.md).

## Earlier source snapshots and detailed triage

Completed 2026-11-07 03:52 UTC.

All six supported-target runs completed with exit status **0**. They emitted warnings; this is not a warning-free or memory-safety certification. No automatic fixes or production-source changes were made.

Tool: mordant 0.1.0 (564c8da5cb0d4fc9b1717b4b42e6e60dbeef237b, rustc 1.100.0-nightly (0dfb098f3 2026-08-31)). [Pinned upstream source](https://github.com/scarletindustries/mordant/tree/564c8da5cb0d4fc9b1717b4b42e6e60dbeef237b). Default Mordant configuration, no baseline or added suppressions. The tool lists 49 registered lints; configuration-dependent/opt-in rules were not separately enabled.

| Source snapshot | Native findings | WASI findings | Compiler errors |
|---|---:|---:|---:|
| Earlier main `56ace9c` | 14 | 58 | 0 |
| Pending safety fixes `988f289` | 14 | 58 | 0 |
| SSSE3 candidate on `93867a4` plus staged changes | 12 | 58 | 0 |

Findings are deduplicated within each run by lint, message and source location. Rows and targets overlap: do not add these counts as distinct defects. Native SSSE3 uses `-C target-cpu=westmere`; the other native runs use the workspace native CPU configuration. This table is a lint comparison, with no performance measurements.

Native command:

```bash
cargo mordant --workspace --exclude egg-solver-wasm --all-targets --features bench --locked --keep-going --message-format=json
```

WASI command (with `RUSTFLAGS="-C target-feature=+simd128"`):

```bash
cargo mordant -p egg-solver-wasm --lib --target wasm32-wasip1 --locked --keep-going --message-format=json
```

The initial whole-workspace Linux commands returned101 because the WASM wrapper references `Memory::try_anonymous`, which exists on its supported WebAssembly target but not the Unix memory implementation. Those logs remain preserved. The corrected commands above completed successfully without changing source.

These are compiler/lint runs, not runtime tests. The WASI pass covers the non-threaded WASI library. ARM/NEON and the WASI threads target were not separately scanned by Mordant in this audit. Existing backend tests and the incomplete full Miri work are separate evidence.

**Review of the current native findings**

| Lint | Count | Assessment |
|---|---:|---|
| `defaulted_failure` | 1 | Deferred validation; no accepted-invalid-input bug demonstrated |
| `field_valid_only_when` | 1 | State representation improvement |
| `tuple_wants_struct` | 4 | Type clarity and prevention of swapped values |
| `unchecked_construction` | 5 | Review construction boundaries; generated-memory and file-input paths differ |
| `unsafe_send_covers_more_than_pointers` | 1 | Manual concurrency proof required |
| `wildcard_over_own_enum` | 2 | Future exhaustiveness protection |

The highest-priority manual review is the `unsafe impl Sync for CubeCtx`: its `UnsafeCell<Inner>` relies on the shared-search synchronization protocol. Mordant did not demonstrate a race. All16 shared-search seed harnesses previously passed, but the full x86 Miri run hit its90-minute time limit in the checksum test; the complete Miri validation remains unfinished.

**`defaulted_failure`:** LoadArgs::open_reported calls preflight, then open_logged. The preflight metadata failure is deliberately deferred; H48Table::open_reporting calls read_meta(path)? before using the table. Propagating the first error directly could make this clearer.

**`unsafe_send_covers_more_than_pointers`:** CubeCtx contains UnsafeCell<Inner>, so its unsafe Sync implementation depends on the builder/worker synchronization protocol, not just ownership of raw pointers. The lint does not demonstrate a race. This remains a high-priority safety-review area; completed scheduling-seed tests are evidence, and the complete Miri run remains incomplete.

**`unchecked_construction`:** Generated and derived H48 tables construct fresh memory sized from k; from_file_memory additionally validates a serialized file trailer, which those fresh buffers do not have. The shared-memory path wraps an existing kernel attachment instead of allocating anonymous memory. These are distinct construction paths, so replacing every literal with the suggested constructor would be incorrect. Encapsulating their separate invariants in named constructors would improve auditability.

**`field_valid_only_when`:** Inner.write_tag is meaningful while record_leads is active. Encoding this relationship in a type could prevent future invalid combinations. Any representation change needs code-layout and performance validation because Inner participates in the shared search.

**`wildcard_over_own_enum`:** The shared-search Step match and LoadMode::mode_for use catch-all cases. Explicit current variants would make future additions require review. No current incorrect variant handling was demonstrated.

**`tuple_wants_struct`:** Named result structs would distinguish same-typed coordinate fields, collapse dimensions, and split-search results. This is a maintainability finding, not evidence of a current swapped value. Hot-coordinate changes require codegen/performance checks.

**`stringified_error`:** The wrapper uses string errors for its exported last-error interface. Converting typed errors here loses programmatic classification, but the current ABI already communicates failure through a status and error text. Keep typed errors longer if structured JavaScript error handling is added; these warnings do not demonstrate a failed solve.

**`unused_pub`:** These41 findings came from the WASI library-only build, with native CLI targets, tests, and external Rust clients absent. Several named items are used by native binaries or tests. Do not remove or restrict public APIs solely from these results.

### Earlier main: native locations

| Lint | Location | Diagnostic |
|---|---|---|
| `defaulted_failure` | [src/cli.rs:198](../evidence/validation/mordant/sources/main/src/cli.rs.html#L198) | `tables::read_meta` can reject its input, and the `else` here returns success when that happens. Rejected input is reported as handled |
| `unsafe_send_covers_more_than_pointers` | [src/lanes/share.rs:554](../evidence/validation/mordant/sources/main/src/lanes/share.rs.html#L554) | `unsafe impl Sync for CubeCtx` covers fields that are not `Sync` for a reason other than a raw pointer |
| `wildcard_over_own_enum` | [src/lanes/share.rs:2451](../evidence/validation/mordant/sources/main/src/lanes/share.rs.html#L2451) | this catch-all will also match any variant added to `lanes::Step` later (4 today). A new variant lands here silently instead of failing to compile |
| `wildcard_over_own_enum` | [src/tables/file.rs:598](../evidence/validation/mordant/sources/main/src/tables/file.rs.html#L598) | this catch-all will also match any variant added to `tables::file::LoadMode` later (4 today). A new variant lands here silently instead of failing to compile |
| `unchecked_construction` | [src/gen/collapse.rs:87](../evidence/validation/mordant/sources/main/src/gen/collapse.rs.html#L87) | `tables::H48Table` is built by hand here, skipping the check on `memory` that `H48Table::from_file_memory` makes |
| `unchecked_construction` | [src/tables/shm.rs:398](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L398) | `tables::Memory` is built by hand here, skipping the check on `len` that `Memory::anonymous` makes |
| `unchecked_construction` | [src/lib.rs:130](../evidence/validation/mordant/sources/main/src/lib.rs.html#L130) | `tables::H48Table` is built by hand here, skipping the check on `memory` that `H48Table::from_file_memory` makes |
| `unchecked_construction` | [src/lib.rs:141](../evidence/validation/mordant/sources/main/src/lib.rs.html#L141) | `tables::H48Table` is built by hand here, skipping the check on `memory` that `H48Table::from_file_memory` makes |
| `unchecked_construction` | [src/lib.rs:190](../evidence/validation/mordant/sources/main/src/lib.rs.html#L190) | `tables::H48Table` is built by hand here, skipping the check on `memory` that `H48Table::from_file_memory` makes |
| `field_valid_only_when` | [src/lanes/share.rs:163](../evidence/validation/mordant/sources/main/src/lanes/share.rs.html#L163) | `write_tag` is only read after checking `record_leads` (1 read), and every other `Inner` fills it with a placeholder (1 place). The field only means something in that one case |
| `tuple_wants_struct` | [src/coord.rs:213](../evidence/validation/mordant/sources/main/src/coord.rs.html#L213) | both of the 2 calls to `corner_fields` unpack the result as `(co_fields, tetrads)`. The names exist everywhere except in the type, and since both members are `u64` a swapped pair compiles |
| `tuple_wants_struct` | [src/coord.rs:348](../evidence/validation/mordant/sources/main/src/coord.rs.html#L348) | both of the 2 calls to `edge_parts_sym_raw` unpack the result as `(msplit, eo_bits, eslot)`. The names exist everywhere except in the type, and since `.0` and `.2` are both `u64` a swapped pair compiles |
| `tuple_wants_struct` | [src/gen/collapse.rs:54](../evidence/validation/mordant/sources/main/src/gen/collapse.rs.html#L54) | both of the 2 calls to `collapse_shape` unpack the result as `(rows, factor)`. The names exist everywhere except in the type, and since both members are `usize` a swapped pair compiles |
| `tuple_wants_struct` | [src/lanes/share.rs:3350](../evidence/validation/mordant/sources/main/src/lanes/share.rs.html#L3350) | both of the 2 calls to `find_split` unpack the result as `(busy, depth, give)`. The names exist everywhere except in the type, and since `.0` and `.1` are both `usize` a swapped pair compiles |

### Earlier main: additional WASI diagnostics

The WASI build has10 overlapping core findings, seven typed-error-to-string findings in the wrapper, and41 public-API usage warnings. The usage warnings apply to this library-only target selection; they are not a basis for deleting APIs used by native binaries, tests or external callers.

| Lint | Location | Diagnostic |
|---|---|---|
| `stringified_error` | [wasm/src/lib.rs:101](../evidence/validation/mordant/sources/main/wasm/src/lib.rs.html#L101) | this `map_err` turns a `egg_solver::tables::TableError` into a `String`. From here on, callers cannot match on which failure it was |
| `stringified_error` | [wasm/src/lib.rs:142](../evidence/validation/mordant/sources/main/wasm/src/lib.rs.html#L142) | this `map_err` turns a `egg_solver::cubie::FaceletError` into a `String`. From here on, callers cannot match on which failure it was |
| `stringified_error` | [wasm/src/lib.rs:244](../evidence/validation/mordant/sources/main/wasm/src/lib.rs.html#L244) | this `map_err` turns a `egg_solver::tables::TableError` into a `String`. From here on, callers cannot match on which failure it was |
| `stringified_error` | [wasm/src/lib.rs:286](../evidence/validation/mordant/sources/main/wasm/src/lib.rs.html#L286) | this `map_err` turns a `std::num::TryFromIntError` into a `String`. From here on, callers cannot match on which failure it was |
| `stringified_error` | [wasm/src/lib.rs:289](../evidence/validation/mordant/sources/main/wasm/src/lib.rs.html#L289) | this `map_err` turns a `egg_solver::tables::TableError` into a `String`. From here on, callers cannot match on which failure it was |
| `stringified_error` | [wasm/src/lib.rs:331](../evidence/validation/mordant/sources/main/wasm/src/lib.rs.html#L331) | this `map_err` turns a `egg_solver::parallel::WorkerPanicked` into a `String`. From here on, callers cannot match on which failure it was |
| `stringified_error` | [wasm/src/lib.rs:463](../evidence/validation/mordant/sources/main/wasm/src/lib.rs.html#L463) | this `map_err` turns a `egg_solver::parallel::WorkerPanicked` into a `String`. From here on, callers cannot match on which failure it was |
| `unused_pub` | [src/coord.rs:51](../evidence/validation/mordant/sources/main/src/coord.rs.html#L51) | constant `egg_solver::coord::N_EO` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/coord.rs:327](../evidence/validation/mordant/sources/main/src/coord.rs.html#L327) | function `egg_solver::coord::edge_key_sym` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/coord.rs:419](../evidence/validation/mordant/sources/main/src/coord.rs.html#L419) | function `egg_solver::coord::index_from_parts` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:29](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L29) | constant `egg_solver::cubie::CORNER_POSITIONS` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:41](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L41) | constant `egg_solver::cubie::EDGE_POSITIONS` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:60](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L60) | constant `egg_solver::cubie::FACE_NORMALS` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:118](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L118) | method `egg_solver::cubie::CubieCube::inverse` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:528](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L528) | function `egg_solver::cubie::move_cubes` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:535](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L535) | function `egg_solver::cubie::move_name` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:619](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L619) | function `egg_solver::cubie::symmetries` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:627](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L627) | function `egg_solver::cubie::conjugate` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/cubie.rs:639](../evidence/validation/mordant/sources/main/src/cubie.rs.html#L639) | function `egg_solver::cubie::symmetry_inverses` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/env.rs:20](../evidence/validation/mordant/sources/main/src/env.rs.html#L20) | function `egg_solver::env::name` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/env.rs:51](../evidence/validation/mordant/sources/main/src/env.rs.html#L51) | function `egg_solver::env::var_os` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/gen/collapse.rs:75](../evidence/validation/mordant/sources/main/src/gen/collapse.rs.html#L75) | function `egg_solver::gen::collapse::derive` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/gen/collapse.rs:116](../evidence/validation/mordant/sources/main/src/gen/collapse.rs.html#L116) | function `egg_solver::gen::collapse::compare` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/gen/progress.rs:23](../evidence/validation/mordant/sources/main/src/gen/progress.rs.html#L23) | function `egg_solver::gen::progress::install` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/hot.rs:25](../evidence/validation/mordant/sources/main/src/hot.rs.html#L25) | constant `egg_solver::hot::STREAM_PAD` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/hot.rs:44](../evidence/validation/mordant/sources/main/src/hot.rs.html#L44) | constant `egg_solver::hot::SHARE_PAD` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/lanes/share.rs:893](../evidence/validation/mordant/sources/main/src/lanes/share.rs.html#L893) | method `egg_solver::lanes::share::Pool::<'a>::counters` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/lanes/share.rs:1027](../evidence/validation/mordant/sources/main/src/lanes/share.rs.html#L1027) | function `egg_solver::lanes::share::solve_all` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/lanes/share.rs:3727](../evidence/validation/mordant/sources/main/src/lanes/share.rs.html#L3727) | method `egg_solver::lanes::share::Pool::<'_>::run_pinned` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/parallel.rs:18](../evidence/validation/mordant/sources/main/src/parallel.rs.html#L18) | function `egg_solver::parallel::set_runner` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/portable/mod.rs:43](../evidence/validation/mordant/sources/main/src/portable/mod.rs.html#L43) | struct `egg_solver::simd::Aligned64` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/solver.rs:306](../evidence/validation/mordant/sources/main/src/solver.rs.html#L306) | method `egg_solver::solver::Solver::<'a>::solve` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables.rs:86](../evidence/validation/mordant/sources/main/src/tables.rs.html#L86) | function `egg_solver::tables::entry_depth` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables.rs:560](../evidence/validation/mordant/sources/main/src/tables.rs.html#L560) | method `egg_solver::tables::Memory::is_empty` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables.rs:666](../evidence/validation/mordant/sources/main/src/tables.rs.html#L666) | function `egg_solver::tables::h48_slots` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables.rs:735](../evidence/validation/mordant/sources/main/src/tables.rs.html#L735) | method `egg_solver::tables::H48Table::get` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables.rs:747](../evidence/validation/mordant/sources/main/src/tables.rs.html#L747) | method `egg_solver::tables::H48Table::prefetch` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables.rs:901](../evidence/validation/mordant/sources/main/src/tables.rs.html#L901) | associated function `egg_solver::tables::H48Table::open` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables.rs:1090](../evidence/validation/mordant/sources/main/src/tables.rs.html#L1090) | method `egg_solver::tables::H48Table::save` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/file.rs:612](../evidence/validation/mordant/sources/main/src/tables/file.rs.html#L612) | function `egg_solver::tables::file::save_tuned_lanes` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/file.rs:643](../evidence/validation/mordant/sources/main/src/tables/file.rs.html#L643) | function `egg_solver::tables::file::default_lanes` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/shm.rs:42](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L42) | function `egg_solver::tables::shm::key` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/shm.rs:72](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L72) | method `egg_solver::tables::shm::ShmPages::sizes` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/shm.rs:105](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L105) | function `egg_solver::tables::shm::page_name` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/shm.rs:117](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L117) | function `egg_solver::tables::shm::segment_bytes` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/shm.rs:179](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L179) | function `egg_solver::tables::shm::loaded` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/shm.rs:188](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L188) | function `egg_solver::tables::shm::new_segment_page` is public, but nothing in the workspace uses it |
| `unused_pub` | [src/tables/shm.rs:197](../evidence/validation/mordant/sources/main/src/tables/shm.rs.html#L197) | function `egg_solver::tables::shm::remove` is public, but nothing in the workspace uses it |

Raw evidence: [all structured findings](../evidence/validation/mordant/findings.json), [commands and exit statuses](../evidence/validation/mordant/results.json), [source hashes](../evidence/validation/mordant/sources.json), and each run’s `.stdout`, `.stderr`, and `.diagnostics.txt` under `/data/agents/mordant-audit/`.

The recorded main and then-pending safety-candidate diagnostics have the same warning categories and counts. The SSSE3 candidate produces no new warning category; it compiles a different coordinate implementation, which accounts for two fewer tuple warnings. No inference about runtime safety or speed follows from that count.
