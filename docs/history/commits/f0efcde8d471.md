# Merge opt/r7-series: share race fix, table hardening, Miri, stress tools

Commit `f0efcde8d471eb144b420a62e8b05e05a1cf09ef` · 2026-11-03T23:49:42+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

Merge opt/r7-series: share race fix, table hardening, Miri, stress tools

Correctness series from the review/stress agent. None of it is intended
to change performance.

1. share: join a fresh cube before publishing it. Worker::fresh joined
   its new cube only after build() had published it. In between,
   another thread could claim, solve and release the cube, so local()
   returned None and the unwrap panicked (abort in release builds).
   Reproduced on cb8cb70 (30 panics + 14 hangs in 100 mock-search runs)
   and on the l6 build (4/20 heavy stress runs). The deterministic test
   share_fresh_cube_solved_by_others_at_once fails 5/5 without the fix
   and passes 100/100 with it.
2. miri: cfg(miri) intrinsic equivalents for the 3 inline-asm sites
   (normal builds unchanged; disassembly identical up to addresses),
   a Miri-compatible Mem::anon, and miri.sh (cubie, simd, coord, hot
   index path, sorts, share protocol). No UB found.
3. tests and tools: hot_index_matches_reference_index and
   conjugates_have_equal_lengths_in_all_modes in check.sh; the
   examples/xsolve cross-check driver (independent plain IDA* reference
   mode), examples/tabcheck and stress.sh.
4. tables: read_meta rejects k > 11 or base > 12, and open checks the
   file size against k. A bad base broke an assert_unchecked in
   resolve_moved (UB), and a stale meta file gave inadmissible bounds,
   i.e. silently non-optimal solutions.
5. tables: map table files opened for solving read-only.

Stress coverage, all solutions verified, 0 length mismatches:
- 3 seeds x 20000 random states in stream, share -c 0 and -c 1;
- 48T, -c 7, 3 lanes;
- vcube cross-check on 400 cubes;
- cube20.org's 100 distance-20 positions, all 20;
- tabcheck of 4e9 h11 entries;
- TSan on all share tests: clean.

Search unchanged:
- stream: 3295521 / 2785769, lengths 15:1 16:35 17:270 18:662 19:32;
- depth18: 3607979 / 3053364, lengths 18:100.
check.sh passes.

Stream, 48 threads. Run with bench /mnt/huge/h11 stream 20 30 48 20, 3
interleaved runs, solves/s:

| build            | runs                  | median |
|------------------|-----------------------|--------|
| opt/l6-share-a   | 996.1 948.7 991.2     | 991.2  |
| opt/r7-series    | 1000.9 992.4 995.9    | 995.9  |

Change: +0.5% (noise).

Stream, 1 thread. Run with bench /mnt/huge/h11 stream 10 60 1 20, 3
paired runs, solves/s:

| build            | runs                  | median |
|------------------|-----------------------|--------|
| opt/l6-share-a   | 21.583 21.633 21.700  | 21.633 |
| opt/r7-series    | 21.667 21.733 21.750  | 21.733 |

Change: +0.5% (noise).
