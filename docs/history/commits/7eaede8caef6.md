# wasm: SIMD128 backend; the library runs in WebAssembly (WASI, with and without threads)

Commit `7eaede8caef6da394909bd67a680cab8c627483a` · 2026-11-04T19:46:05+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

wasm: SIMD128 backend; the library runs in WebAssembly (WASI, with and without threads)

User request: "towards the end, make it also compile to wasm". The crate now
builds and runs for wasm32 with the same searches, tables and node counts as
natively. The next commit adds the module for browsers and Node.

- src/portable/v16_wasm.rs: `V16` on SIMD128 (core::arch::wasm32):
  - lookups: `u8x16_swizzle` (indices from 16 give 0, as NEON `tbl`);
  - the byte shift left: an add;
  - the movemasks: `bitmask` of a 16-bit-lane shift (bit 7 of each byte
    still comes from its own byte).
  build.rs picks cfg(backend = "wasm") for wasm32 with simd128.
  .cargo/config.toml turns simd128 on for wasm32 targets, and sets the
  maximum memory of wasm32-wasip1-threads to 4 GiB (the linker's default
  there is 1 GiB).
  New test `v16_matches_scalar` (NEON and wasm): every operation against the
  scalar reference on 20,000 random vectors, lookup indices over 0..255.
- Targets:
  - wasm32-wasip1: one thread.
  - wasm32-wasip1-threads: std threads through wasi-threads, so share mode
    and multi-thread stream mode run unchanged (stable Rust, no build-std).
  - wasm32-unknown-unknown: keeps compiling. Its std has no clock
    (`Instant` panics there).
- prefetch::read_once is a one-byte load on wasm32, which has no prefetch
  instruction. The CPU runs on past a pending load, so the miss overlaps the
  next lanes' work just as with a prefetch. That makes it an `unsafe fn`
  (`p` must point into an allocation); the two callers pass table entries.
  On x86 and aarch64 the instructions are the same as before.
  Measured on h6, 100 depth-17 cubes, 1 thread, 40 lanes, batch:

  | engine                    | without (s)  | with the load (s) |
  |---------------------------|--------------|-------------------|
  | wasmtime                  | 187.4, 190.7 | 122.4, 125.1      |
  | V8, optimizing compiler   | 188.8        | 130.6             |
  | V8 default (see next commit) | 203.1, 202.9 | 181.3, 182.7   |

  Clamping `p` into linear memory instead, to keep the fn safe, cost 2-4%:
  wasmtime 126.7/125.9 vs 121.6/122.2 s, V8 126.4/127.2 vs 123.6/124.3 s.
- src/parallel.rs: `on_threads(n, f)` runs share 0 on the calling thread
  and spawns n - 1 threads, so one thread spawns nothing (wasm without
  threads). It replaces the spawn loops of the generators (gen.rs,
  gen/dfs.rs, gen/collapse.rs), of `distribution` and of `file::checksum`.
  The work split is the same as before.
  - Regenerated h7 base 9 (8 threads) vs p10's /data/agents/p10/h7b9:
    `gen compare` finds 0 differences in codes and group minima, and the
    data is byte-identical (cmp). Only the trailers differ: p10's file
    predates the `layout` line.
  - `gen check` passes on /mnt/huge/h7, h8 and h11 (read-only).
  - Generation speed is unchanged. h4 base 9 on 8 threads of one L3 domain,
    alternating, main def2cd0 vs this commit:
    - main: 26.7 s and 26.6 s (depth-11 pass 20.1 s, 20.0 s);
    - this commit: 26.6 s and 26.4 s (19.9 s, 19.8 s);
    - outputs byte-identical.
- Memory on wasm32: whole pages from `memory.grow`, not from the allocator,
  which cannot hand out more than isize::MAX = 2 GiB at once (h7 is 3.6
  GiB). Linear memory never shrinks, so the next table reuses the pages of a
  dropped one. `Memory::try_anonymous` (non-unix) reports out-of-memory
  instead of aborting. On 32-bit, `bytes` and `words` assert the 2 GiB
  slice limit.
- `H48Table::from_file_memory(memory, verify, threads)`: a table file whose
  bytes are already in memory (copied there by JavaScript). It is checked
  like `open`: trailer, k, base and sizes, and with `verify` the checksum
  (over pieces of at most 1 GiB).
- `LoadMode::Auto` is Copy where there are no mappings (WASI, Windows).
  `map` failed there.
- bin/gen compiles on WASI: positioned writes by seek + write, no
  getrusage.
- cubie: `CubieCube::from_facelets` and `to_facelets`, in Kociemba's
  54-sticker order U R F D L B with any six color symbols. Tested on:
  - known strings (solved, U, R, superflip);
  - round trips and recolorings;
  - rejected non-positions.
- Tests that assumed 64 bits or unix:
  - scratch files go through `scratch_path` (WASI has no temp dir and no
    process id);
  - the hot-index test uses a one-page stand-in table, since it only
    computes indices and a zeroed h11 cannot be allocated on wasm32;
  - the read-only-mapping test runs only on Linux.
- check.sh:
  - clippy for wasm32-unknown-unknown (lib), and for wasm32-wasip1 and
    wasm32-wasip1-threads (all targets);
  - the wasm library tests under wasmtime, when installed (WASMTIME=path,
    else skipped with a note).

Validation:
- Library tests in wasm (wasm32-wasip1-threads under wasmtime 44; v50 no
  longer has wasi-threads): 36 passed, 1 ignored (the Linux mapping test).
- Integration tests in wasm (tests/correctness.rs, with h0 generated in
  wasm): stream, share and recursive solvers, conjugates, symmetric start,
  the DFS vs BFS generator vs brute force. 16 passed in 91 s.
- Node-count identity on h5 (base 9), random:20:2026, 1 thread, 40 lanes.
  These four give nodes/cube 155935448, lookups/cube 133837454, lengths
  16:1 17:1 18:18:
  - native x86;
  - native scalar (--features scalar);
  - bench.wasm under wasmtime;
  - the wasm module in Node (next commit).
  The same holds on h6 depth-17 (14806167 nodes/cube in every engine and
  variant above).
- check.sh passes on the final tree (both commits), with all backends,
  the wasm tests and the module test.
- For this commit alone, two full check.sh runs failed in the debug scalar
  library tests: 1 of 36 tests, and check.sh's filter hides which. The same
  tests passed in 10 separate runs. The library code is the same as in the
  final tree, so this matches the intermittent share debug_assert reported
  in that step (pre-existing; r8).

x86 unchanged. No hot code touched except the unsafe block around the
table prefetch.
- Hot functions instruction-identical to main def2cd0 (objdump, addresses
  normalized):
  - bench::stream_thread: 1411 instructions, at offset 16 of its line, as
    in main;
  - share Worker::rounds: 1213 instructions, at offset 0, as in main;
  - Worker::restart 1337, Lane::advance_cold 335, Lane::next_run 207,
    Lane::setup_run 991, Lane::start_iteration 919, Lane::extend_jobs
    1328, run_lanes's closure 449.
- Stack frames (first `sub rsp` of every function, bench and solve)
  identical to main def2cd0, including the share thread closures:
  bench::share_thread 0xfc0, and the solve_all thread closure of solve
  0xfc0. The one exception is the one-time move table init in cubie
  (0x678 -> 0x668).
- verify.sh, on main def2cd0 and on this stack:
  nodes/cube 3058198 lookups/cube 2588380
  lengths 15:1 16:35 17:270 18:662 19:32
  nodes/cube 3644301 lookups/cube 3084160
  lengths 18:100
- Speed: bench stream on h11 with 24 lanes, main def2cd0 vs this stack,
  both built from the same folder.
  1 thread (ab1t, 6 paired runs), solves/s:
  | build | median | runs                                        |
  |-------|--------|---------------------------------------------|
  | main  | 24.767 | 24.717 25.517 24.867 24.767 25.217 24.150   |
  | stack | 24.733 | 24.983 24.783 24.733 24.717 25.117 24.217   |
  -0.1%: neutral.
  48 threads (ab48, 3 runs, outside load 0.1 cores), solves/s:
  | build | median   | runs                        |
  |-------|----------|-----------------------------|
  | main  | 1027.433 | 1014.367 1027.433 1030.200  |
  | stack | 1036.333 | 1023.700 1078.333 1036.333  |
  +0.9%, within the 48T noise: neutral.
- Earlier, on main 857a0c6 with the stack as it was then (read_once still
  a no-op on wasm, same hot code):
  - 1T: 25.167 [25.200 25.167 25.167 25.250 25.317 25.033] vs 25.300
    [25.417 25.317 25.317 25.300 25.217 25.217];
  - 48T: 1074.467 [1074.467 1070.500 1087.000] vs 1082.733 [1082.733
    1076.800 1088.100].
