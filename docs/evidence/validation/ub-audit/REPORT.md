# Additional UB audit — 2026-11-07

The baseline is candidate `988f289fed03f2490359fe660297d9f502363845`, frozen in
`/data/agents/ub-audit/source`. Fixes are being validated in
`/data/agents/ub-audit/fix`. They are not yet merged or certified by complete
Miri runs. The superseded automatic integration and Miri supervisors are held;
the authorized benchmark matrix completed at 06:53 UTC. Main integration
still waits for complete validation of the new candidate.

## Reproduced safe-public-API failures

Every reproduction below calls the baseline library without an unsafe block.
Both x86 AVX2/BMI2 and the portable backend failed. Debug checks sometimes
abort at the unsafe precondition; release-profile Miri reports the underlying
UB. Miri ignores optimization levels, so these are interpreter results, not
execution of optimized native instructions. No Miri UB checks were disabled.

| Case | Trigger | Evidence | Candidate correction |
|---|---|---|---|
| Table index | `H48Table::get(16384)` with a publicly constructible 4096-byte backing allocation | [x86](x86-table_index.log), [portable](portable-table_index.log) | Checked getter returning `Option`, private table fields, checked constructor |
| Symmetry index | `edge_parts_sym(Cube::solved(), 48)` | [x86](release-x86-symmetry_index.log), [portable](release-portable-symmetry_index.log) | Checked public symmetry functions; internal unchecked helper is crate-private |
| Malformed cube | `edge_parts_sym(Cube::from_bytes([4;32]), 0)` | [x86](release-x86-malformed_cube_rank.log), [portable](release-portable-malformed_cube_rank.log) | Validate permutations and orientations, hide raw fields and mask selection; reject invalid coordinate constructors |
| Reserved coordinate | Constructing a cube from edge key `0`, then taking its edge coordinate | [x86](release-x86-malformed_coordinate.log), [portable](release-portable-malformed_coordinate.log) | The public constructor returns `None` for reserved slots and out-of-range ranks |

The original `[255;32]` malformed-cube probe completed without a diagnostic.
It was not evidence of general safety: the `[4;32]` case reaches an out-of-range
rank-table lookup. Inputs and complete diagnostics remain in
[the probe directory](probes/Cargo.toml).

## Reproduced search-publication failure

A new tracing regression test reproduced another UB diagnostic under Miri
seed 15. `finish_build` published jobs, then passed `&Inner` to `trace_build`.
Another worker finished those jobs and entered `build`, obtaining `&mut Inner`
while the tracing argument remained protected. Miri rejected that overlap.
The test enables real search tracing and adds a test-only scheduling delay
immediately after publication; the delay changes scheduling, not the ownership
protocol. This reproduction used the candidate before the tracing fix, rather
than the untouched baseline used by the four public-input probes.

The correction completes tracing before the Release publication. `start_sort`
also no longer receives `&mut Inner` as an argument: that function-entry borrow
could otherwise stay protected across publication of its tasks. Its local
setup borrow now ends before publication. The latter is a manual finding,
not a separately reproduced diagnostic.

Evidence: [diagnostic](trace-before-fix.log),
[saved pre-fix source](trace-before-fix.rs.html),
[reproduction metadata](trace-reproduction.json).

## Manual findings and invariant hardening

| Area | Finding and correction | Evidence limit |
|---|---|---|
| Cocsep | Public mutable entries could invalidate unchecked coordinate/class reads. Fields now have read-only accessors; packed-index access is internal. | Public-field corruption is blocked by compile-fail tests; not a full exhaustive proof of generated entries. |
| Solver | Public table references could be replaced independently of cached pointers/metadata. Read-only accessors now preserve their relationship. | Defensive invariant closure; a dangling-pointer exploit was not demonstrated. |
| Shared-memory trailer | The reader copied payload bytes even when an Acquire load saw the unpublished zero marker. It now copies only after seeing publication. | Manual race finding; a Miri-testable helper exercises the production publication read, but Miri does not execute SysV attachment. |
| File-backed memory | Safe mapping APIs exposed memory that another file handle could mutate or truncate. Mapping-capable APIs now require an unsafe storage-lifetime contract; `open_copy` provides private RAM safely. | OS-level mapping behavior remains outside Miri. CLI map/shm modes retain the immutability requirement. |
| Shared-memory inspection | `loaded` and `remove` also inspect a mapped trailer. Their public and CLI entry points now carry the publication/immutability contract; attachment size is checked before any trailer pointer arithmetic. | No claim that arbitrary external writers can be made safe by an internal Acquire load. |
| Trailer size | `data_bytes + TRAILER_BYTES` could overflow before bounds validation. Checked arithmetic rejects this before mapping. | Regression added to the native file test. |
| wasm32 size | A 3.6-GiB h7 block exceeded `isize::MAX`, while table reads use in-bounds pointer arithmetic. Allocation and attachment paths now reject oversized blocks. | wasm32 now supports h0–h6; 64-bit native h7–h11 are unaffected. |
| Endianness | Packed little-endian corner bytes were decoded as native integers feeding unchecked indices. Big-endian targets are now explicitly rejected. | No big-endian runtime validation is claimed. |
| ISA selection | 256-bit AVX-512 byte blending requires both BW and VL. Its feature guard now checks both. | Native code and backend checks are separate from Miri's AVX2 path. |
| Metadata and pointer construction | Validate generation metadata before allocation; reject null mappings; bound worker-count arithmetic. | Boundary hardening; no practical null-mapping or huge-worker exploit was reproduced. |

File mappings' external-mutation risk is also documented by
[memmap2](https://docs.rs/memmap2/latest/memmap2/struct.MmapOptions.html#safety).
Rust's pointer-offset requirements include the signed-size bound used here:
[`pointer::add`](https://doc.rust-lang.org/std/primitive.pointer.html#method.add).

## Review coverage and remaining limits

Reviewed the public constructors feeding unchecked indexing; Memory ownership,
alignment, slice lengths and mutation contracts; coordinate ranges; cached
solver pointers; the lane stack's `MAX_TARGET < MAXD` bound; move masks; job
sorting indices; shared-search claims, publication, buffer ownership and
mailboxes; parallel runner callbacks; platform vector/assembly entry points;
and the WASM allocator and pointer-bearing ABI. This is a manual review, not a
formal exhaustive audit of every execution or dependency.

The custom parallel runner accepts safe `Fn + Sync` callbacks. Current internal
callers either read disjoint/immutable slices or allocate work with atomic
counters, so duplicate callback IDs were not established to cause UB. Its
exactly-once behavior is a separate API-contract concern.

WASM threads use a lifetime extension for a borrowed callback and wait for all
workers before returning. Current shipped WASM builds abort on panic; unwinding
or changes to the join protocol require a fresh audit. Foreign callers must
respect pointer lifetime and immutable-input requirements.

Tests remain finite. Native AVX-512/assembly, actual mmap/SysV operations,
WASM host behavior, dependency code and all possible weak-memory executions
are not covered by a universal proof. Full source-bound native/backend and
Miri validation must finish before integration. Existing ignored tests must
remain explicitly listed, not counted as passes.

## Validation record

The pre-tracing-fix source fingerprint
`232607b4d60721c25c5a742fb7d642c87cf6a501d865bb1506f29ecc5a7140ec`
passed the full native/backend check script, both hot-code comparisons, three
new boundary/publication tests under strict-provenance Miri on both backends,
16 publication seeds per backend, and native ThreadSanitizer checks of the
shared-search and boundary tests. Those results are preserved in
`before-trace-fix/` (historical reference: `/data/agents/ub-audit/before-trace-fix`). They do **not**
certify later edits and demonstrably did not cover the tracing bug.

The final candidate `316597f64a9bd749e8a01b67f96c108c03611e68`
(source SHA-256 `93478bc5c45ba267255574f8cb466f02f006f1c7bf558fb8f9d088ec4307a4c9`)
passed the full native/backend script, both hot-code comparisons, focused
strict-provenance Miri checks on both backends and ThreadSanitizer. Evidence
is recorded in
[`FROZEN.json`](FROZEN.json). Current certificates are
accepted only if their source fingerprints match. An intermediate compile
failure from missing unsafe blocks in test cleanup and a stale-source Miri
run are preserved in
`test-callsite-compile-failure/` (historical reference: `/data/agents/ub-audit/test-callsite-compile-failure`);
neither is counted as a pass. Full x86 and portable Miri catalogues began
concurrently at 07:07 UTC and remain a separate integration requirement.
They include all four shared-search tests under 16 seeds per backend.
The additional native AddressSanitizer run passed the library and integration
tests, including native ISA and actual mapping paths omitted by Miri.
Its source-bound certificate is [asan-validation.json](asan-validation.json).
See [current status](current-status.json) and
[NO_UB.md](../../../safety/NO_UB.md) for later source-bound completion.
