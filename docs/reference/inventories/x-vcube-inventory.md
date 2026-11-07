# vcube technique inventory (x-vc)

Source read in full: `~/ref/vcube/src/*` (avx2_cube.h, sse_cube.h, types.h, cube.h/.cpp,
cube6.h, nxprune.h/.cpp, nxprune_generator.h, nxsolve.h/.cpp, vc-optimal.cpp, alloc.h/.cpp,
util.h/.cpp, const-cubes.h, gen-const-cubes.cpp), README.md, doc/speffz.md, CMakeLists.txt, tests/.
Ours = main 55e048e (`~/cube-opt`; the file:line references and names are from 55e048e, before s8's renames). vcube paths are relative to `~/ref/vcube/src/`.

Verdicts: **done** (we have it, or a stronger form), **n/a** (does not apply to an H48 h11
48-symmetric single-coset table, or not on a throughput path), **gap** (applies, we lack it).

STATUS: complete (main f8a4919). No patch delivered: the cocsep compaction is neutral (gain within code/data placement effects). Negatives and diagnostics with numbers: $P/x-vc/neg-*.patch.

**Summary.** About 50 vcube techniques. Nearly all are done in our solver (often in a stronger form)
or do not apply to a 48-symmetric single-coset H48 table (the cube6 axis lookups and their
"+1 if all equal" / axis-tight rules have single-coset analogs we already use: NO_HALF, X2/X' twins).
Per-op costs are equal (move, premove) or lower (child cubes 12.5 vs 53.8 Ir, index 45 vs 69 Ir).
Gaps: C12 (15 duplicate depth-4 jobs, 0.035%) and C13 (prefix-cube table, <= 0.4% Ir) are too small
to pay. B6 (vcube's compact corner sym table) does not carry over as such, but its point does: the
cocsep table's L2 footprint costs ~1.6% per +33% (s4); 18-bit packed entries (p18) gained +1.6% at 1T
layout-averaged on 55e048e but nothing in its dense form on later code; the row-padded form read 4 bits
up (p18d) gained +1.2% at 1T on m7's WIP placement but -0.7% (stream) and -1.5% (share) at 1T on main
f8a4919 with the same stream-loop code at the same pinned offset, and is neutral at 48T: not merged
(section F). Mechanism: ~-30% cocsep L2 misses (cache model) = ~+1%, paid back in 1-2 µops and ~1 cycle.
Calibrations: ~0.15-0.2% per cycle of index-chain latency; ~12 cycles (1% per 140K
per cube) per extra cocsep L2 miss. Working-set audit (section G): the table lines insert 78% of the L2
lines and the cocsep entries are their victim; all other structures together insert 5.6%.

## A. Cube representation and SIMD operations

| # | technique | where in vcube | what it does | ours | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| A1 | Whole cube in one 256-bit register | cube.h:353-376 | Edges in bytes 0..11 (`---OEEEE`), corners in bytes 16..23 (`--OO-CCC`); one register per cube | simd.rs:1-6,114-116 (corners bytes 0..7 `piece \| co<<5`, edges 16..27 `piece \| eo<<4`) | yes | — | done |
| A2 | Unused bytes hold identity values | avx2_cube.h:39-49 | Bytes 12..15 / 24..31 = identity, so composes keep them valid and full-width compares need no mask | simd.rs:186-190, 256-262 (garbage bytes, `USED` mask in `vptest`) | alternative | ours is 2 ops (xor+vptest) vs vcube 3 (cmpeq+movemask+cmp); no gain | done (better) |
| A3 | Compose = 1 `vpshufb` + orientation add + mod-3 via `min(x, x - carry)` | avx2_cube.h:78-99 | Permutation by shuffle, twist add, carry fix-up with unsigned min | simd.rs:180-192 | yes | — | done |
| A4 | Move = compose with a constant move cube | cube.h:443-445 | `c.move(m) = compose(moves[m])`: shuffle, and, add, sub, min | simd.rs:194-203 (`apply_move`: orientation term precomputed in `MOVES_ORI`, no AND) | yes | Ir per move: ours 4.56, vcube 4.44 (vcube's AND constant-folded in the unrolled bench loop; with a variable move it is 1 more); ns/move 0.35 both (microbench below) | done (ours ≤) |
| A5 | Premove = compose with the move on the left | cube.h:447-449 | `moves[m].compose(c)` | simd.rs:205-209 (`premove_inv`, inverse moves table `INV_MOVES`) | yes | Ir 5.78 both; 0.35 ns both | done |
| A6 | Mirror compose (subtract twists) for reflections | avx2_cube.h:87-96, cube.h:51-60 | `%` operator: corner twist subtracted for S_LR2-mirrored left operand | simd.rs:269-280 (`transform` + `invert_co` for mirrors) | only in table/cocsep generation | none (not on the search path) | n/a |
| A7 | Inverse by the addition chain c^27719 (19 `vpshufb`) | avx2_cube.h:118-174 | LCM of cycle lengths is 27720; measured 9-10% faster than a 12-step brute force | simd.rs:223-254 (same chain) | yes, but only at cube start | — | done |
| A8 | Bit-plane extraction `movemask(slli(v, 7-b))` | avx2_cube.h:51-53, sse_cube.h:295-297 | Bit b of every byte as a mask in 2 ops (E-slice/D-layer/eo bits are bits of the piece number) | coord.rs:222-230, 274-288 (same; `vpmovmskb` via asm to avoid `vptestmb+kmovd`) | yes | — | done |
| A9 | Piece numbering so that class membership is a bit of the piece index | cube.h:353-375, nxprune.h:94-97 | E-slice edges have bit 3 set, D-layer bit 2: masks by one shift+movemask | coord.rs:221-230, build.rs (SYM_EDGE classifies pieces into bits 7/6) | yes | — | done |
| A10 | `edgecube`: 128-bit edges-only cube for coordinate work | cube.h:455-509, sse_cube.h:317-321 | Edge-only compose (shuffle, and, xor: no mod-3) for symmetry conjugation | coord.rs:245-297 (`edge_parts_sym`: classify pieces, then one permuting shuffle; no full conjugation) | the idea (edges-only work) yes; 128 vs 256 bits no (Zen 5 runs 256-bit ops at full rate) | ours does less work than an edge conjugation | done (better) |
| A11 | Corner-orientation rank via `pmaddwd` dot product | sse_cube.h:333-347 | co coordinate from the 0x30 bits by madd with powers of 3 and a horizontal sum | coord.rs:156-219 (2 `pext` + 3 LUT loads, pre-multiplied by 70 and by 3) | yes | ours shorter (no horizontal sum) | done (better) |
| A12 | Ordering `less_than` from two movemasks | avx2_cube.h:59-76 | `gt < lt` of the two compare masks = lexicographic order | — | only for std::set (table gen, depth-4 dedup) | none | n/a |
| A13 | SIMD parity, corner-orientation unrank, `xor_edge_orient` | avx2_cube.h:101-116, 188-254 | Input validation and table generation helpers | cubie.rs (scalar) | not on a hot path | none | n/a |
| A14 | Nibble-list factorial rank/unrank with `bextr` (`table -= 0x1111.. << shift`) | cube.h:241-321 | Lehmer code without a loop over remaining elements | — | we never rank full permutations | none | n/a |
| A15 | `pdep` construction of representative cubes from combination ranks | cube.cpp:244-300 | Builds a cube for a 8C4 / 12C4 rank with two `pdep` | coord.rs:345-372 (`cube_from_edge_key`, loop) | table generation only | none | n/a |
| A16 | Constants generated as code (`gen-const-cubes` -> const-cubes.h) | gen-const-cubes.cpp | Moves, symmetries, inverse map, move_sym6 as constexpr | build.rs (MOVES, MOVES_ORI, INV_MOVES, SYM_T/TINV, SYM_EDGE) | yes | — | done |

## B. Coordinates, table index and table entry

| # | technique | where in vcube | what it does | ours | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| B1 | 2-bit entries with a base, 4-bit minimum per 16-byte group of 64 entries ("stripe-min"), 2 entries per group reserved | nxprune.h:105-109, 354-363; nxprune_generator.h:256-283 | `val ? BASE+val : stripe[0] & 0xf` | solver.rs:122-137 (same layout; code 0 -> `max(group min, corner depth)`) | yes | — | done (ours adds the corner depth) |
| B2 | Reserved slots made by arithmetic: `ecomb += (ecomb+63)*33/2048*2` | nxprune.h:105-109 | Gaps for the group minimum computed per lookup (33/2048 ~ 1/62) | coord.rs:58-79 (gaps baked into the `ESLOT` table) | yes | ours saves ~5 ops per index | done (better) |
| B3 | nCk ranks by LUT (`rank_12C4` 4096 x u16, `rank_8C4` 256 x u8) | util.h:29-105 | Combination rank from a bit mask in one load | coord.rs:58-131 (`ESLOT`/`eslot_c`, `RANK8`) | yes | — | done |
| B4 | `pext` to compress a mask relative to another (D-layer among non-E positions) | nxprune.h:99, cube.h:130-135 | 8C4 rank of the U/D split | coord.rs:238-239, 293 (msplit = `RANK8[pext(S, ~E)]`) | yes | — | done |
| B5 | Odd-even bit order 4! rank from movemask bits | util.h:67-88, cube.h:121-128 | E-slice permutation rank without unpacking | — | h11 has no E-slice permutation | none | n/a |
| B6 | Corner sym-coordinate by two small tables: `index[co] = {base, os*, prune*}`, `os[c4comb] = {offset, sym}`, 139 deduplicated `os` rows | nxprune.h:288-348, nxprune.cpp:29-81 | 9930 sym classes of (co, 8C4) under 16 symmetries from ~72 KB of tables instead of a 153090-entry table | tables.rs:20-24, 165-178 + solver.rs:113-120 (one packed 3-byte entry per cocsep coordinate, 448 KB of lines, on a THP) | vcube's factoring needs a product action of the symmetries on (co, sep); under H48's 48 symmetries URF3 changes co depending on sep (measured: every sep sees all 48 ttreps; the 70 seps form 8 orbits), so the 2-level form would put a corner conjugation on the index chain. **But the footprint matters:** 4-byte stride of the same data (612 KB) = -1.6% median over 8 layout pairs (all 8 negative); the access histogram is flat (top 45% of coordinates = 50% of accesses), so only true compaction helps. | see section F (p18: 18-bit entries + per-class depth, 336 KB; p18b: rows padded to 72, 354 KB) | gap -> prototyped; neutral (placement-dependent: +1.2% / -0.7% at 1T, 48T 0, share -1.5%; F) |
| B7 | Per-corner-orient precomputed table row pointer (`idx.prune = table + base*stride`) | nxprune.cpp:77-81, nxprune.h:369 | No class multiply at lookup | coord.rs:303-309 (`row * row_mul`, off the critical path: class is known before msplit) | yes | none: the multiply overlaps the edge chain | done (equivalent) |
| B8 | Edge coordinate after symmetry conjugation: 2 edge composes, then 3-5 bit planes | nxprune.h:89-141 | `ec.symConjugate(sym)` then movemasks | coord.rs:245-309 (one classify shuffle + one permute shuffle with per-symmetry 64-byte line, no orientation fix-up) | yes | index Ir: ours 45, vcube 69 (microbench) | done (better) |
| B9 | Prefetch hint T0 | nxprune.h:370 | `_mm_prefetch(stripe, _MM_HINT_T0)` | solver.rs:139-147 (NTA) | yes | T0 measured -1.0% 1T (neg/m6-prefetch-t0-t1) | done (NTA better) |
| B10 | All of a node's lookups prefetched before the first is read (MLP 5-6 per node) | nxprune.h:382-390 | 6 index computations + prefetches, then reads in order | lanes.rs:1-30, 1408-1500 (24 lanes, one miss each, round robin) | yes | ours has 24 misses in flight per thread | done (better) |
| B11 | Lookups of the 3 axis conjugates (cube6) on each side | cube6.h:327-390, nxprune.h:382-464 | A 16-symmetric (U/D axis) table looked up after conjugating by URF3 and URF3^2 | — | H48's coset is 48-symmetric: the conjugates have the same value | none | n/a |
| B12 | "+1 when all three axis values are equal" (`prune_cmp |= blsi << 1`, then highest bit) | nxprune.h:420-425, 448-453 | The last move lies in one axis' subgroup, so length >= min over axes + 1 | solver.rs:256-262, lanes.rs:1243-1248 (single-coset analog: half turns are in H, so a tight bound on one side removes the other side's half turns) | the multi-axis form needs >= 2 different cosets; H48 has one | none beyond the NO_HALF masks we have | n/a (analog done) |
| B13 | Inherited value of the unchanged axis (`skip`/`val`) | nxprune.h:382-394, nxsolve.h:142-144, 168-170 | A move on axis a leaves the other side's axis-a coset unchanged: reuse its value | lanes.rs:1040-1053, 1116-1131 (X2 keeps the other side's value; X' reuses its X sibling's `tw`) | yes | — | done (ours covers X2 and X') |
| B14 | Tight axis -> drop all that axis' moves on the other side (`axis_mask_expand`) | nxsolve.h:60-63, 119-120, nxprune.h:455-461 | The nxopt subgroup contains the axis' quarter turns too | lanes.rs:1243-1248, solver.rs:256-262 (`NO_HALF` when the other side's bound is tight) | H contains only half turns for H48 | — | done (H48 form) |
| B15 | Early exit per value, ordered: forward axes first | nxprune.h:396-444 | Stop reading values once one prunes | lanes.rs:1040-1053 (other side first: inverse-first; prunes 87% at tight bounds) | yes | — | done (better order) |

## C. Search loop

| # | technique | where in vcube | what it does | ours | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| C1 | IDA* from the root's lookup bound | nxsolve.h:75-96, nxprune.h:466-470 | `initial_depth` then increasing depth | lanes.rs:451-475 (`max(ln, li, 1)`) | yes | — | done |
| C2 | Bidirectional search: each node branches on the side (moves vs premoves) with fewer allowed moves | nxsolve.h:119-133 | popcount of the two move masks | lanes.rs:1250-1260 (`child_node`), 1322-1350 (`make_node`) | yes | — | done |
| C3 | Side tie-break: larger sum of pruning values | nxsolve.h:124-131 | Ties of the branching factor go to the side with the larger sum | lanes.rs:1255, 1335 (ties to the normal side) | yes | tested: neg/a2-side-tiebreak (by bound) -0.07% nodes, no throughput effect | done (negative) |
| C4 | Canonical sequences: no same face twice, opposite faces in one order | nxsolve.h:65-68 | `last_face_mask` | solver.rs:12-27 (`ALLOWED_AFTER`) | yes | — | done |
| C5 | Sibling-face pruning: a child value 2 over the limit drops its face | nxsolve.h:147-153, 172-179 | `mask &= ~7 << 3*face` | lanes.rs:983, 1081-1095 (eager, branch-free `face_keep`) | yes | — | done |
| C6 | Bidirectional pathmax (BPMX): return `child - 1` when it exceeds the limit | nxsolve.h:23-27, 146-149 | Back-jump from a child's learned bound | lanes.rs:955-990, 1315-1320 (`lb`, `max_dec`, pops when `lb > r`) | yes | — | done |
| C7 | Leaf: solved test without a lookup at depth 0 | nxsolve.h:105-107 | `c6 != cube()` | lanes.rs:1006-1017 (r1 == 0: move and test, no lookup; r == 1 nodes use the moves allowed on both sides) | yes | — | done (better) |
| C8 | Lookups at every depth | nxsolve.h:109-114 | Also when the limit exceeds BASE+3 | lanes.rs:1006-1031 (no lookup when r1 > base+3: no value can prune) | — | — | done (better) |
| C9 | Depth-4 job queue for deep iterations | nxsolve.h:190-257, nxsolve.cpp:23-46 | Above BASE+4, search each depth-4 position as a job | solver.rs:155-163, lanes.rs:478-551 (jobs extended to 4 moves above base+3) | yes | — | done |
| C10 | Jobs ordered by the previous iteration's cost, face-weighted by sqrt(3):sqrt(2) (58206:47525) | nxsolve.h:229-239 | Densest subtrees first in the next iteration | lanes.rs:93-106 (same weights) + per-child statistic (c7/c8) | yes | ours: steeper lookup statistic (aa43192) and child-level order (-6..-8% nodes) | done (better) |
| C11 | 2-pass LSD radix sort on a 16-bit float key | nxsolve.h:211-253 | Sort 43K jobs cheaply | lanes.rs:108-145 (3-pass 32-bit radix, skips constant digits); runs counting sort | yes | one 43K sort per cube (~0.3% of cycles); a 16-bit key would change ties (search) | done |
| C12 | Depth-4 jobs deduplicated by position (43,239 unique) | nxsolve.cpp:301-320 | std::set of cubes: the 15 duplicate canonical 4-move sequences (all half turns, e.g. R2 F2 D2 U2 = U2 F2 D2 R2) are searched once | lanes.rs:478-551 (43,254 canonical sequences) | yes | measured: 15 of 43,254 jobs (0.035%); half-turn-only prefixes keep the root's H48 coset, so their subtrees are mostly pruned early: < 0.035% nodes | gap (negligible; not worth code) |
| C13 | Job roots by composing precomputed depth-4 cube6 with the input (one compose per cube) | nxsolve.h:190-195, cube6.h:348-357 | `c6 * q.c6` instead of replaying moves | lanes.rs:438-449 (`job_cubes`: 4 moves + 4 premoves per job setup) | yes | measured: 50,140 `job_cubes` calls per cube (random:1000:2026) x ~53 Ir = 2.7M Ir vs ~450M Ir per cube (0.6%); a shared 2.8 MB prefix table would save at most ~0.4% Ir and adds L3 traffic. A per-lane cache variant was negative (neg/c7-jobcache). | n/a (bounded below noise) |
| C14 | Face from move index by `popcnt(011111 << m >> 15)` | nxsolve.h:139, 165 | Division-free m/3 | lanes.rs:998-1000 (`m / 3` as imul+shift; `kind = m - 3*face`) | yes | asm (stream loop): `imul ecx, r14d, 0x56; shr ecx, 8` = 2 µops, 4 cycles; vcube's `shlx; shr; popcnt` = 3 µops, 3 cycles: one µop more for one cycle less on a chain off the value path | n/a (no net gain; kernel micro, m7's area) |
| C15 | Moves of both directions in one array with a 0x80 flag | nxsolve.h:155-157, 181-183, 259-271 | Rebuild the solution at the end | lanes.rs:1291-1307 (`m_normal` per node) | yes | — | done |
| C16 | Recursive DFS, cube6 by const reference, 6 composes per child | nxsolve.h:104-188 | | lanes.rs (explicit stack, 2 composes per child, lanes) | — | ours: 12.5 Ir per child's cubes vs 53.8 | done (better) |

## D. Memory, threads, system

| # | technique | where in vcube | what it does | ours | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| D1 | Huge pages: 1 GB, else 2 MB, else 4 KB (`MAP_HUGETLB` with size bits) | alloc.cpp:31-58 | | tables.rs:234-290 (hugetlbfs file: 1 GB pages) + THP for the cocsep table (tables.rs:209-232) | yes | — | done |
| D2 | Table kept in SysV shared memory with `SHM_HUGETLB` across runs | alloc.cpp:60-90, nxprune.cpp:130-162 | Fast startup | tables.rs:503-548 (table file on /mnt/huge hugetlbfs, mapped read-only) | yes | — | done (equivalent) |
| D3 | One solver per thread, whole cubes per thread, mutex-protected input | vc-optimal.cpp:364-422 | Cube-level parallelism | lanes.rs:1408-1500 (stream: atomic counter) + lanes/share.rs (work sharing) | yes | — | done |
| D4 | clang instead of GCC; `-mavx2 -mbmi -mbmi2 -mlzcnt` | CMakeLists.txt:22-28 | | .cargo/config.toml (target-cpu=native), Cargo.toml (fat LTO, cgu 1, panic=abort) | yes | a7: all toolchain variants neutral/negative | done |
| D5 | `nopti` (no Meltdown page-table isolation): README says +35% on i7-7700K | README.md | kernel parameter | — | this Zen 5 is not affected by Meltdown (`/sys/.../vulnerabilities/meltdown`: Not affected), so there is no PTI to remove; the hot loop makes no syscalls | none | n/a |
| D6 | Faster memory (XMP, timings) | README.md | | — | hardware (Azure VM) | — | n/a |
| D7 | Atomic table save (write `.tmp`, rename) | nxprune.cpp:83-100 | | tables.rs:557-562 | robustness, not speed | — | n/a |
| D8 | Table generation: corner-class neighbour pairs, mod-3 frontier codes, stripe-min, `zero_visited`, `pext` frontier scan, dirty filter | nxprune_generator.h | | gen.rs (2-bit BFS on h11, frontier relabel pass, group minima) | generation is offline (table exists) | — | n/a |

## E. Per-operation cost: vcube vs ours (microbenchmark)

`/data/agents/x-vc/vcube/opbench.cpp` (vcube's own headers, clang 18 -O3, coord 308 = EP3/EO8 index)
vs `/data/agents/x-vc/cube-opt/examples/opbench.rs` (branch x-vc-diag). 4096 random cubes, 1 thread on a
bench slot. Ir from callgrind on x86-64-v3 builds (`/data/agents/x-vc/bin/fnsum.py`), ns native.

| operation | vcube Ir | ours Ir | vcube ns (native / Haswell flags) | ours ns (native) |
|---|---|---|---|---|
| move (forward side) | 4.44 | 4.56 | 0.36 / 0.28 | 0.35 |
| premove (other side) | 5.78 | 5.78 | 0.36 / 0.34 | 0.35 |
| child cubes (vcube: cube6, 6 composes; ours: 2) | 53.8 | 12.5 | 1.87-1.93 / 2.21 | 0.65 |
| table index of one cube (no prefetch) | 69.0 | 45.0 | 4.8-5.0 | 2.75-2.81 |
| move, dependent chain (latency) | — | — | 1.86-1.91 | 1.83-1.86 |
| index, dependent chain incl. a `% n` (latency) | 76.0 | 52.0 | 21.5-22.5 | 15.2-15.5 |

Per expanded node vcube computes 6 cubes per child and up to 6 indices; we compute 2 cubes and 1-2 indices.

## F. Index chain study (B6 footprint, idea 7.4 ranking, chain latency)

Layout-averaged 1T A/B (`abl1t.sh 2 A B 24`: 4 lld-shuffled layouts x 2 paired runs = 8 pairs; B/A).

| experiment | what changes | result |
|---|---|---|
| s4 (diagnostic) | packed cocsep entries at a 4-byte stride (612 KB), same data, same code | **-1.6% median, 8/8 pairs negative** (-1.95 -2.14 -0.82 -0.20 -2.96 -2.11 -1.48 -1.34) |
| dlat vs dnolat (diagnostic) | 3 dependent `imul x,x,1` (+9 cycles) on the packed entry (delays ttrep, class, corner depth) vs the same imuls off the chain | -1.70% median over 4 pairs (-2.41 +0.69 -1.41 -1.70): about **0.15-0.2% per cycle** of index-chain latency (m7's value chain: 0.35%/cycle). $P/x-vc/neg-diag-index-chain-latency.patch |
| access histogram (diag) | per-coordinate counts of indexed cocsep coordinates, random:400:2026 | flat: top 45% of coordinates = 50% of accesses, 88% = 90%; frequency-sorted layout needs 393 KB of lines for 90% vs 398 KB now -> reordering is useless, only compaction helps |
| ck2 (idea 7.4: half-split tables) | `3*cocsep` = `lo[key & 0xFFF] + hi[key >> 12]` with key = one `pext` of the corners (tetrad bit + 2 co bits per corner): colex rank is additive over the halves because the low half's popcount is 4 minus the high half's | index 45 -> 43 Ir (v3), microbench throughput 2.75 -> 2.6 ns; +10.5 KB of L1 tables. **-0.27% median, -1.21% mean** over 8 pairs (min -6.78%): the L1 tables cost more than the 2 µops. NEGATIVE ($P/x-vc/neg-ck2-halfsplit.patch) |
| p18 | 18-bit entries `class << 6 \| ttrep` at bit 18i (336 KB), corner depth from a 3.4 KB per-class table (depth is a class invariant: checked for all 153090 coordinates) | asm: +4 µops (`mov; shr; and; shrx mem`, `and; shl` for ttrep*64, depth load), ~+3 cycles. **+1.59% median, +1.76% mean** over 8 pairs on 55e048e, 7/8 positive (min -0.13%, max +4.00%; stream_thread start offset equal to main's in every layout). Ported to 735dd25 (s8 sweep): **-1.51% median, -1.46% mean** (min -6.31%, max +2.74%), offsets matched in 2 of 4 layouts; on m7's stack with pinned pads: best pad ~+0.2%. NEGATIVE in this form ($P/x-vc/neg-p18-dense.patch); see p18d |
| p18b | p18 with rows of 70 separations padded to 72 entries (162 bytes, 354 KB): the entry's byte in the row and its bit shift come from the tetrad mask alone (parallel loads), no `shr`/`and` on the chain | ~+3 µops, ~+2 cycles; but register allocation in the stream loop lost the TABS base register (4 `lea rip` rematerializations in the index path). **+0.47% median, +0.58% mean** over 8 pairs (min -0.79%) |
| p18c | p18b, entry read from the byte before it and shifted 3 bits less: `v & 0x1F8` = ttrep * 8, a scaled index of the 64-byte `sym_edge` rows (no `shl`) | asm: index path +1 µop vs main (shift-table load, `shrx` with memory source, depth load; no `and`/`shl`), ~+1 cycle; register allocation elsewhere in the stream loop got worse (25 TABS `lea` rematerializations vs 17 in p18). **-0.07% median, -0.20% mean** over 8 pairs: neutral |
| p18 on m7's WIP stack (48d9d27, pinned pads) | p18 at stream pads 0/16/32/48 vs that stack at its pad 16, 2 paired runs each | pad 0 +0.73 -0.26, pad 16 -0.59 -1.39, pad 32 -1.13 -2.40, pad 48 -1.39 -0.82: best pad ~+0.2%, i.e. no gain |
| **p18d** (on m7's stack 48d9d27, then main f8a4919; pinned pads) | 18-bit entries in rows of 72 (354 KB), read from the byte before and 4 bits up: class and symmetry where the 3-byte entries had them (`bextr 0xc0a`, `and 0x3f0` scale 8 unchanged); shift from a tetrad-mask table; depth by class | index path ~1-2 µops above main's. 1T pad screen vs base pad 16: pad 0 +0.54 +0.78, **16 +0.56 +1.66**, 32 -0.07 -0.27, 48 -0.61 +0.07; pad 16 vs 16, 6 pairs: **+1.20%** (medians 25.067 -> 25.367, 6/6 positive). 48T pad 16 vs 16, 5 runs: medians 1070.97 -> 1070.57 (-0.04%; pairs +0.20 +0.44 +1.34 -2.17 -0.04): neutral. Rebased on main f8a4919 (stream loop byte-identical to the WIP's, same pad; other code and HOT_TABLES placed differently): stream 1T pad 16 vs 16, 4 pairs: 0.00 +0.07 -1.12 -1.06 (medians -0.73%); share 1T 3 pairs: -1.29 -1.82 -1.46. NEUTRAL, not merged: $P/x-vc/neg-cocsep-18bit-p18d.patch |
| move with both mod-3 candidates in parallel (derived from vcube's compose) | `apply_move`: `min(shuffle + ori, shuffle + (ori - carry))` with a second precomputed table (`MOVES_ORI - CARRY`) instead of `min(t, t - carry)`: same µops, one step less latency | move-chain microbench 1.50 vs 1.96 ns per move (3 pairs, consistent); only on the X2/X' moved-side index chain. Forwarded to m7 (owner of that code); branch x-vc-lat |

Placement note: with the stream loop pinned (m7), p18d's stream-loop code was byte-identical (addresses
aside) on the WIP stack and on f8a4919, at the same pad, for both sides; yet 1T went from +1.2% (6/6 pairs)
to -0.7%. What differs is the placement of the other code and of the static tables (HOT_TABLES at 3584/3584
mod 4 KiB on the WIP builds, 896 vs 768 on f8a4919: different L1 set mapping). Pinning HOT_TABLES to a
4 KiB boundary (and the cold functions' order) might remove another +-1% confounder from kernel A/Bs.

Idea 7.4 mapped onto our index: the h11 index has no permutation ranks (only base-3 co digits, C(8,4)
and C(12,4) combination ranks and eo bits), so popcount Lehmer and Myrvold-Ruskey have nothing to
apply to; digit-parallel ranking is what the pext + LUT code already does; the half-split applies to
cocsep (ck2). The eslot C(12,4) rank cannot be half-split cheaply: the fixed slot numbering has gaps
(slot = r + 2 + 2*floor(r/62)), which would cost an imul + shift instead of one 8 KB LUT load.

## G. Hot working-set audit (L1/L2 by structure)

Method: cachegrind with a cache model of one Zen 5 core (`--D1=49152,12,64 --LL=1048576,16,64`: the
"LL" is the 1 MB L2), main 735dd25 built for x86-64-v3 with scalar stand-ins for the AVX-512 helpers
(branch x-vc-cg), `bench random:40:2026 40 1 24` (1 thread, 24 lanes). Misses are attributed to source
lines (`/data/agents/x-vc/bin/cgsim.py`). Cachegrind ignores prefetches, so a prefetched line is
counted at its first load; the lines brought into L2 (= L2 misses) are the same. No L3 is modelled.

L2 misses per cube (each one also inserts a line, i.e. may evict another):

| structure | where | L2 misses / cube | share | L1 misses / cube | L1 miss rate |
|---|---|---|---|---|---|
| h11 table lines | `Solver::value` | 2,354K | 77.6% | 2,431K | 97% |
| packed cocsep entries (448 KB) | `Solver::index` | 509K | 16.8% | 2,106K | 84% |
| run lists: `start_iteration` (build_first/build_rest/sort_runs writes) | lanes.rs | 40K | 1.3% | 60K | |
| `extend_jobs` (job vectors, X' fix-up pass) | lanes.rs | 27K | 0.9% | | |
| ChildOrder (JobX, 32 B x 43K per lane): write at run end / read at run start / scans (build_first `codes_above`, `choose_theta`) | lanes.rs:806 / 866 / 257, 640 | 19K / 9K / 24K | 1.7% | 35K / 10K / 24K | 8% / 35% / 3% |
| job list reads (`setup_job`, `moves_after`) | lanes.rs:926, 1574 | 15K | 0.5% | | |
| memset / memmove (Vec resizes, copies) | libc | 17K | 0.5% | | |
| node stacks (24 lanes x 26 x 128 B) | lanes.rs | ~3K | 0.1% | ~250K | 1-8% |
| hot tables (`HOT_TABLES`: moves, sym_edges, eslot, rank8, stats) | coord.rs, simd.rs | ~2K | 0.1% | 420K (eslot 319K, sym_edges 94K) | eslot 12.8% |
| all | | 3,034K | 100% | 5,860K | |

Ranking and what shrinking each could give:
1. **The h11 table lines** are the dominant inserter (one dead line per lookup; inherent). They are what
   evicts the cocsep entries. Keeping them out of L2 would help most, but the only tool, CLFLUSHOPT after
   use, costs -30% (neg-clflushopt-table-lines). NTA is already the hint (T0/T1 measured worse, m6).
2. **The cocsep entries** are the victim: 20% of their reads miss the modelled L2 (84% miss L1).
   Measured: +33% footprint (s4) = -1.6% at 1T; 18-bit entries in rows of 72 (p18d, -21%): +1.2% / -0.7%
   at 1T depending on placement, share -1.5%, 48T neutral: the ~+1% footprint gain is paid back in µops.
3. **Everything else together inserts 5.6%** of the L2 lines. Even removing all of it would cut the cocsep
   misses by at most ~6% (~30K of 509K per cube), i.e. ~0.1-0.2% at the measured cost of a cocsep miss.
   A smaller ChildOrder does not help per run (each run touches one 64-byte line whatever the size);
   it would only halve the per-iteration scans (24K misses per cube). Not worth a patch.
4. L1: after the table and cocsep, the 8 KB `eslot_complement` table is the largest L1 miss source
   (12.8% of its reads; 495 valid entries over 99 of its 128 lines). Shrinking it needs a 2-level rank
   (two 64-entry half tables + a 495-entry rank->slot table: +5 µops and a dependent load), which costs
   more than its L2-hit latency (mostly off the critical msplit path). Not pursued.

Calibration (cache model, same instruction count): s4 adds 226K cocsep L2 misses per cube (hit rate 80% ->
71%) and measured -1.6% at 1T: about 1% per 140K misses per cube (~12 cycles per extra miss). p18 removes
183K per cube (hit rate 87%) for +0.46% Ir (v3 build): the model predicts about +1%.

48 threads: L2 is per core, so the per-core picture is the same. The cocsep table is one shared copy in
the CCD's L3 (a cocsep L2 miss is an L3 hit at 1T and 48T), but at 48T eight cores churn the L3 with table
lines (~8 x 55M lines/s per CCD), so an L3 hit is slower under load and a cocsep L2 miss costs more: the
footprint gain of p18 should be at least as large at 48T. The per-lane job state (24 lanes x ~2 MB per
core, ~380 MB per CCD) cannot stay in L3 at 48T; each run start then reads its job and ChildOrder lines
from DRAM (prefetched one run ahead in `setup_run`), ~50K runs per cube: small next to 2.4M table misses.
