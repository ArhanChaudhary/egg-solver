# x-h48: nissy-core (H48) technique inventory vs cube-opt

Reference: `~/ref/nissy-core` at 1f7e8e9. Ours: `~/cube-opt` main 55e048e.

Paths in nissy are relative to `src/`; `h48/` means `src/solvers/h48/`. "lanes" means `src/lanes.rs` and "solver" means `src/solver.rs` (ours).

Verdict codes:
- **HAVE**: we do the same.
- **BETTER**: we do more, with a measured gain.
- **GAP**: missing and applicable.
- **N/A**: missing but does not apply.
- **MEASURED-NO**: missing, measured, and doesn't pay.

## A. Tables, coordinates, generation

| # | technique | nissy (file:line) | what it does | ours (file:line) | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| A1 | Fully symmetric target group H48 | doc/h48.md "The target subgroup"; h48/coordinate.h:10-43 | H = corners oriented and in their own tetrad, edges in their own slice (+ oriented for h11). Invariant under all 48 symmetries, so one lookup per side and 48-fold reduction. | coord.rs:1-13; tables.rs:54-125 | same | - | HAVE |
| A2 | cocsep: symmetry-reduced corner part (3393 classes) + ttrep + corner depth in one entry | h48/gendata_cocsep.h:84-136 (32-bit `class<<16 \| ttrep<<8 \| depth`, index `co<<7 \| 7-bit csep`, 1.1 MB) | Gives the corner class, the conjugation to its representative, and a corner lower bound in one load. | tables.rs:20-24, 54-125 (3-byte packed, rank-70 csep, 459 KB on a THP); coord.rs:156-218 (pext index) | same idea, smaller and faster | - | BETTER (f99ebc4 packed cocsep +2.8%, e9d751e cocsep LUT) |
| A3 | hN = h11 with 11-N EO bits dropped | h48/coordinate.h:42 (`>> (11-h)`: drops the low EO bits; EO innermost) | Intermediate tables as min-collapses of h11. | coord.rs:327-331 (`reduce_key`: drops EO bits, E-slice innermost) | Only h11 is used in production. Different EO subsets than nissy, so the h7/h8 distributions are not comparable with nissy's (definitional). | - | HAVE |
| A4 | 2-bit entries with a per-table base (h8..h11 base 10; h4..h7 9; h0..h3 8) | h48/gendata_h48.h:154-198 | Codes 1..3 mean base+1..base+3; code 0 means ≤ base (needs a fallback). | tables.rs:333-336 (`default_base`: h11 10; h0 7) | same base for h11 | base 11 for h11: see measurement M2 | HAVE (M2 tests the alternative) |
| A5 | Fallback minimum interleaved with the entries | h48/gendata_types_macros.h:114-127; h48/gendata_h48.h:506-552; doc/h48.md "Fallback tables" | Each 64-byte line holds 254 entries plus a 4-bit min of their true depths, so a code-0 read costs no extra miss. Lines of 256/128 bits were "not significantly" better. | tables.rs:315-347, solver.rs:123-137 (cube48opt layout: 16-byte groups, 62 E-slice placements + 4-bit min) | same idea, finer group | - | BETTER (9090310: neutral at h11, much stronger for small h) |
| A6 | eoesep fallback table | h48/solve.h:123-141 (`p == 0` gives `MAX(pmin, eoesep)`); h48/gendata_eoesep.h (782 esep classes × 2^11 EO, 4-bit, ~0.9 MB) | On a code-0 read, also look up the edges-only distance (edges in slice + oriented). | **no**. solver.rs:132-134 uses `max(group min, corner depth)`. | Valid: H_e contains H, half turns and all symmetries, so it is also safe for X2 inherit, X′ twin and NO_HALF. Code-0 reads that pass happen only at r1 ≤ 9 (diag-code0: ~12K lookups/cube). | small (diag-code0 bound <1% of lookups) | see M1 |
| A7 | Corner-depth (cocsep) precheck before the edge index/lookup | h48/solve.h:208-212, 235-239 | Prunes a child on the corner bound without touching the big table. | lanes:1054, 1129, 1205; solver:362, 390; also folded into the code-0 value (solver:133) | same, plus the fold | - | HAVE (+ fold) |
| A8 | Table alignment | doc/solvers.md (64-byte aligned) | One line per lookup. | 1 GiB hugetlbfs pages (tables.rs Mem::file) | ours is TLB-free too | - | BETTER |
| A9 | Expected-distribution check of a table | h48/checkdata.h:7-132, 142-199 | Detects corrupt or misgenerated tables. | Generator prints the distribution. tables.rs:616 tests cocsep. r7 added meta/size checks (8953929). No h11 code-distribution check on open. | integrity only, no speed | 0 | optional (not needed for speed) |
| A10 | Table generation: depth-≤8 BFS in a hash map, then a parallel DFS 4 levels deep from every depth-8 cube with mutex-chunked marks; brute force for h1..h10 | h48/gendata_h48.h:48-80, 154-466; h48/map.h | Builds 2-bit tables without a full-coordinate BFS. | gen.rs (full BFS on h11 with forward/backward switching, atomics, h7/h8 derived as min-collapses in the same pass: h11+h8+h7 in ~2 min) | ours is exact and much faster | - | BETTER. **nissy bug, see section G** |
| A11 | Self-symmetry closure (FOREACH_H48SIM) | h48/gendata_types_macros.h:136-148 | Marks every coordinate equivalent under the corner representative's self-symmetries. | gen.rs:84-99 (`for_each_index`); backward step stays closed because neighbours are found by inverse moves | same | - | HAVE |
| A12 | Backward BFS step ("torchlight") when the frontier is large | h48/gendata_eoesep.h:131-198 (from depth 9); doc/h48.md | Scan unvisited entries for a neighbour in the frontier. | gen.rs:160-169, 219-261 | same | - | HAVE |

## B. Lookup pipeline

| # | technique | nissy (file:line) | what it does | ours (file:line) | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| B1 | Pruning pipeline over a node's children + prefetch | h48/solve.h:157-255 (stage 1: other-side cdata + prefetch; stage 2: other-side value + moved-side prefetch; stage 3: moved-side value). Doc: +25% pipeline, +20% prefetch. | Overlaps the misses of one node's children. | lanes:1-28, 1408-1491: 24 interleaved explicit-stack DFS lanes per thread, one NTA miss each, across nodes and cubes. nissy's TODO.md calls this "microthreading", its only remaining idea. | superset | - | BETTER |
| B2 | Prefetch hint | utils/prefetch.h (T0) | | NTA (solver:141-147) | NTA measured better (neg/m6-prefetch-t0-t1, ebd6c20 +2.6%) | - | BETTER |
| B3 | No lookups (or cdata) while remaining > base+3 | h48/solve.h:199-201 | A table value can't prune there. | lanes:1006-1031; solver:356-357 | same | - | HAVE |
| B4 | X2 child inherits the other side's bound | h48/solve.h:173-176, 186-189 | A half turn doesn't change the other side's H-coset. | lanes:1051; solver:339 | same | - | HAVE |
| B5 | Other (inverse) side looked up first | h48/solve.h:203-244 | The other-side value prunes before the moved-side lookup is issued. | lanes:1048-1074, 1116-1139 | same | - | HAVE |
| B6 | X′ shares X's other-side value | **nissy: no** (looks up X′'s other side again) | | lanes:82-87, 1096-1103, 1127 | ours only | - | BETTER (0b65adf: −28% lookups) |
| B7 | Carry normal + inverse cubes; premoves instead of inverting | h48/solve.h:179-180, 192-193; doc "Avoiding inverse cube computation" | | lanes Node a/b; simd.rs:207 `premove_inv` | same | - | HAVE |

## C. Search rules

| # | technique | nissy (file:line) | what it does | ours (file:line) | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| C1 | Iterative deepening over fixed depth-4 tasks, from depth 5 | h48/solve.h:4, 675-680, 455-527 | Tasks are always 4 moves long. Every depth from 5 is searched. | Start at max(ln, li, 1) (lanes:472). Jobs are min(d−(base+3), 4) long (solver:156-163). | ours skips useless shallow iterations | - | BETTER |
| C2 | Task-root bound: normal side only; dfs root starts with lb_normal = lb_inverse = 0 and inverse mask ALL | h48/solve.h:419-427, 487-488 | | Both job-root bounds cached with batched prefetch; tight masks at job roots (lanes:478-540, 886-897) | ours prunes/tightens at the job root | - | BETTER (c52cb63, 9f9a4d3) |
| C3 | Side choice: expand the side with fewer allowed moves (NISS switch) | h48/solve.h:350 | | lanes:1249-1257, 1333-1339; solver:315 | same (ties go to normal) | - | HAVE |
| C4 | Tight bound on one side means no half turn next on the other side | h48/solve.h:275-278, 299-302 | A bound equal to the remaining depth rules out a final half turn from that side's view. | lanes:1243-1248, 892-897; solver:456-461 | same | - | HAVE |
| C5 | Canonical sequences: no repeated face; opposite faces only as U<D, R<L, F<B | core/constants.h:437-444; core/moves.h:331-334 | | solver:13-27 `ALLOWED_AFTER` | identical masks | - | HAVE |
| C6 | Last move: moves allowed on both sides, solved test | h48/solve.h:324-344 | | lanes:1252-1253, 1007-1015; solver:300-313 | same | - | HAVE |
| C7 | Solved before the target depth: abandon | h48/solve.h:314 | | lanes:1019, 1161; solver:433 | same | - | HAVE |
| C8 | Sibling-face pruning and back-jumps | **nissy: no** | | lanes:983-988, 1088-1094; solver:421-479 | ours only | - | BETTER (0c48355 −18% nodes; 7a173a1 +13.6%) |
| C9 | Symmetric-start filtering | h48/solve.h:504, 518-521 (`symmetry_mask` at each of the first 4 levels drops symmetric-equivalent moves); core/transform.h:411-423; solvers/solutions.h (reports the variants) | For an input within 3 moves of a symmetric position, explores one move per symmetry orbit: superflip 43254 → 1038 tasks (41.7×, doc). | **no** | Random states essentially never have a nontrivial symmetry (or are within 3 moves of one). Zero on the benchmark; helps only special inputs (superflip, patterns). nissy has no antisymmetric (inverse-conjugate) filter either. | 0 on random states | N/A for speed (optional robustness patch, see M4) |
| C10 | Stop when enough solutions | h48/solve.h:315 | | FOUND propagation | same (one solution) | - | HAVE |

## D. Ordering and parallelism

| # | technique | nissy (file:line) | what it does | ours (file:line) | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| D1 | Task ranking by nodes visited × face factor (47525 / 58206 ≈ √(2/3)), sorted from depth 16 | h48/solve.h:432-447, 545-553, 687-689 | Finds the solution earlier in the last iteration (doc: ~40% for single solutions). | Statistic Σ 8^(16−h) × the same face factors (lanes:99-109, 269-306), radix sort (lanes:116-147); c7 cross-job per-child ordering (lanes:553-855); c8 unseen code | Measured: node-count rank 1.91M vs ours 1.661M final-iteration nodes/cube (aa43192), then c7 −6..−8% and c8 −1% | - | BETTER |
| D2 | Per-cube multithreading: depth-4 tasks split statically round-robin (`i += threads`) | h48/solve.h:401 | One cube at a time, all threads. | Stream mode (one cube per lane) for throughput. share.rs for batch/latency: dynamic job claims, staged builds, node splitting (cube48opt `split_ida_node`). | ours is dynamic | - | BETTER |
| D3 | Pause / resume / stop polling | h48/solve.h:402-405, 698-721 | API feature | no | not a speed technique | - | N/A |

## E. Low level

| # | technique | nissy (file:line) | what it does | ours (file:line) | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| E1 | Cube in one AVX2 register (vcube layout); compose = shuffle + orientation add/min | arch/avx2.h:101-121 | | simd.rs:182-205 | same; moves are specialised AVX-512 in ours | - | HAVE / BETTER |
| E2 | Inverse by a vpshufb addition chain | arch/avx2.h:123-160 | | simd.rs:225-255 (identical chain; used once per cube) | same | - | HAVE |
| E3 | Moves = compose with constant move cubes (switch) | core/moves.h:4-5, `move`/`premove` | | simd.rs:195-210 | ours specialised | - | BETTER |
| E4 | Conjugation by full compose (transform) | core/transform.h | | Edge-only conjugation with two shuffles (coord.rs:245-297) | ours fewer µops | - | BETTER (42a7c98 +5.5%) |
| E5 | Coordinates: co by a scalar 7-step loop; csep by movemask (7-bit binary index); esep by a scalar binomial loop; eo by movemask | arch/avx2.h `coord_co`, `coord_csep`, `coord_esep`, `coord_eo` | | pext + LUTs, vpmovmskb asm (coord.rs:156-297) | ours faster | - | BETTER (m1, m4, m6 rounds) |
| E6 | popcount side choice | arch/avx2.h `popcount_u64` | | `count_ones` | same | - | HAVE |

## F. Solutions

| # | technique | nissy (file:line) | what it does | ours (file:line) | applicable / why | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| F1 | Un-NISS: normal moves + inverted premoves in reverse | solvers/solutions.h `appendsolution` | | lanes:1293-1309; solver:269-272 | same | - | HAVE |
| F2 | Multi-solution dedup (`allowedmoves` after un-NISS), symmetric variants | solvers/solutions.h | | n/a (one solution) | not needed | - | N/A |

## G. Correctness finding: nissy's h11 table is slightly inadmissible (the r7 distribution difference)

- **Bug:** h48/gendata_h48.h:307-316 (`gendata_h48_runthread`) together with h48/map.h:68-87 (`h48map_nextkvpair`, which post-increments the slot index).
  - The loop `kv = nextkvpair(map, next); if (*next == capacity) break;` drops the entry in the last slot (`capacity - 1`).
  - With nissy's fixed capacity 10000019 and hash function, that slot holds the depth-8 h11 entry 189880194105 (class 2675, no self-symmetry).
  - Its 4-level DFS never runs (the loop visits 7,437,854 of 7,437,855 short entries).
  - The same loop shape is in `gendata_h48short` (gendata_h48.h:62-65). It is harmless there: the entry's val is 8, the last BFS level.
- **Proof:**
  - nissy's own depth-≤8 BFS, run from its source, equals ours per depth (1/1/4/34/333/3815/45382/548562/6839723).
  - Modelling nissy's values in the 49,206 entries within 4 moves of the skipped root (8 + distance to the other depth-8 roots) gives ours − nissy = **[+159, +1689, +7691, −9539]** per code. That is exactly r7's difference (ours [1150763320, 12045847349, 91163441021, 136418085910] vs nissy's checkdata.h).
  - Scripts: `/data/agents/x-h48/drv/{short,lastslot,skipped}.c`.
- **Consequence:**
  - **Our h11 is exact; nothing to fix.**
  - nissy's h11 over-estimates 11,546 entries by one level: 159 true ≤10 coded 1, 1848 true 11 coded 2, 9539 true 12 coded 3. Its table is thus very slightly inadmissible.
  - h0 is unaffected: ours (base 7) equals nissy's h0 on every comparable level.
  - h7/h8 can't be compared: different EO subsets.

## Measurements

All counts are deterministic node counts (count.sh-style: 8 threads of one CCD, h11 unless noted). Baseline main 55e048e on random:1000:2026: **3,058,198 nodes/cube, 2,588,380 lookups/cube**, lengths 15:1 16:35 17:270 18:662 19:32. Every experiment kept the lengths identical.

### M1. eoesep fallback (A6): MEASURED-NO at h11

- **Edge-only table:** unreduced, one byte per (msplit, eo11, eslot) key, 73 MB; BFS in `src/exp.rs`.
  - Distribution: 0:1 1:6 2:75 3:870 4:9810 5:102520 6:986669 7:8137622 8:34145345 9:27231078 10:349198 11:6; 34650 × 2048 valid keys.
  - nissy's 782-class reduction has the same depths (maxvalue 11).
  - It is weak: mostly 8-9, against h11 values of 11-13.
- **Mode 1, nissy's rule:** on a code-0 read, take max(group min, corner depth, eoesep).
  - Result: **3,063,419 nodes/cube (+0.17%), 2,592,913 lookups/cube (+0.17%)**.
  - A stronger bound can only lower node counts in an exhaustive iteration. The +0.17% is therefore ordering noise in the last iteration (the job statistics change).
  - Even before table cost, there is no gain. This matches diag-code0: only ~12K code-0 lookups per cube pass at r1 ≤ 9.
- **Mode 3:** the edge bound folded into the corner bound (precheck + fallback, any k), on h7/h8 for the same-size comparisons. See M1b.

### M2. base 11 for h11 (A4): MEASURED-NO (−10% nodes worse)

- **Emulation:** an exact base-11 h11 table emulated from our base-10 table.
  - True 11 (code 1) becomes a code-0 fallback, max(group min, corner depth).
  - Code 2 stays 12.
  - Code 3 becomes 13 if any of the 18 neighbours has code 2 (exactly 12), else 14 (exact, since dist = 1 + min over neighbours).
- **Result:** **3,368,592 nodes/cube (+10.2%), 2,960,401 lookups/cube (+14.4%)**.
- **Why:** losing the exact "11" costs far more at r1 = 10 (0.33M lookups/cube) than the new "14" saves at r1 = 13 (63K lookups/cube).
- **Other mappings:** base 9, or merging 12/13 or 11/12, would lose an exact level at r1 = 11/12, where 2.3M of the 2.6M lookups happen. So base 10 is the right offset for h11, as nissy and cube48opt also chose.

### M4. Symmetric-start filtering (C9): N/A on random states

- 2000 random states, all 7,004,000 positions at depths 0..3 of their canonical prefix trees (the levels nissy's `symmetry_mask` checks): **0 with a nontrivial symmetry**. The filter never fires on the benchmark distribution.
- For symmetric inputs (superflip: 43254 → 1038 tasks in nissy) it would cut the job tree.
- Cost for us: only a root-level check is affordable (48 conjugations, ~0.2 µs/cube). nissy's full 4-level check (~3.5K prefix nodes × 48 conjugations ≈ 0.5 ms/cube) would cost ~1% at our 40 ms/cube at 1T.
- Correctness for one solution: keeping the first allowed move of each orbit is sound. If a solution starts with a dropped move t(m), conjugating it by t gives one starting with m, or, after canonical reordering of commuting moves, with an earlier face. That potential strictly decreases, so a kept first move always exists.

### M1b / M3: see below

## Feasibility: a bigger effective table than h11 within ~65 GiB (ideas 2.2-2.4, 4.2-4.4). Analysis only, no table generated

**Verdict:** infeasible. The smallest H48-compatible refinement of the coset space is 2×, and at its information content it needs ≥ 70 GiB.

**1. Which refinements keep the H48 design.** A refinement H′ ⊂ H must keep two properties:
- Invariance under all 48 symmetries: one lookup per side, and 48-fold reduction.
- All half turns in H′: X2 inherit (B4), the X/X′ twin (B6) and NO_HALF on tight bounds (C4) all depend on it.

So G2 = ⟨U2, D2, R2, L2, F2, B2⟩ ⊆ H′ ⊆ H.
- |H| = 4!·4!·(4!)³/2 = 3,981,312 and |G2| = 663,552, so [H : G2] = 6.
- In G2 the corner and edge permutations are always even, while H contains odd ones (a corner 2-swap inside a tetrad with an edge 2-swap inside a slice). So K = H ∩ {even permutations} is an index-2 subgroup with G2 ⊆ K.
  - K is symmetry-invariant: conjugation preserves parity.
  - It is the only one: "parity of one tetrad" or "of one slice" is not symmetry-invariant, since symmetries swap tetrads and permute slices.
  - K-coordinate = (H48 coordinate, parity bit). Parity = number of quarter turns mod 2, unchanged by half turns.
- Above K, the next refinement is G2 itself: corners in the half-turn state, ×6.
- Refinements that drop the half-turn property lose B4/B6/C4 and are larger anyway (a full corner permutation is ×576, an edge permutation inside slices ×13824).

| coset space | entries | 2 bit | 1.6 bit (trits) | 1.3 bit | 1.25 bit (h11's entropy) |
|---|---:|---:|---:|---:|---:|
| h11 (now: 2 bit, 58.0 GiB with the group-min layout) | 2.408e11 | 56.1 GiB | 44.8 | 36.4 | 35.0 |
| K11 = h11 × parity | 4.816e11 | 112.1 | 89.7 | **72.9** | 70.1 |
| h11 × G2-corners | 1.445e12 | 336 | 269 | 219 | 210 |

- K11 in 65 GiB allows **≤ 1.16 bits/entry**, with no room for fallback minima.
- The h11 code distribution [0.48%, 5.0%, 37.9%, 56.7%] already has 1.248 bits of entropy. K11's distribution has its mass one level deeper and is at least as spread (13 and ≥14 both large), so lossless coding needs ≥ 1.25 bits.
- To fit, a lossy code would have to merge levels. M2 shows what losing one exact level costs: base 11 is +10.2% nodes; losing 12 or 13 is far worse (most lookups happen at r1 = 11..12).

**2. Gain if it fit.** A 2× coset space adds ~0.27 ply of mean h (13.3× nodes per ply), i.e. at most ~2× fewer nodes. Our own scaling agrees: h8 → h11 (8× entries) gives 23.9M → 3.06M nodes/cube, 7.8×. The lossy coding needed to fit would eat a large, uncertain part of that.

**3. Decode cost per lookup.** Our lookup is ~15 ns at 1T, mostly the DRAM miss; decode today is one shift and mask.
- Trits (5 per byte, a 243-entry LUT): one extra L1 load and a few ALU ops (~1-2 ns). But it gives only 3 levels at 1.6 bits, which doesn't fit K11 anyway.
- tANS: ~170 sequential symbol decodes (~1000 instructions, ~300 ns) per lookup, about 20× our whole per-lookup cost. Infeasible. This matches CS's and TR's own judgement in ideas 2.4.

**4. 96-way merge (inverse folded into the table): impossible for H48.**
- The coordinate is a right coset H·x. A premove by h ∈ H leaves it unchanged; that is exactly why an X2 child keeps the *other* side's bound.
- But (h·x)⁻¹ = x⁻¹·h⁻¹ is in a different coset in general. So the inverse's coset is not a function of the entry, and no single entry can stand for {x, x⁻¹}. That would need H to be normal in G, and it isn't.
- Storing lexmin(pos, pos⁻¹) needs a position-indexed (hashed) table, not a coset table.
- We already do the 48-way part (A1), and we probe the inverse separately (B5).

**5. What does fit.** Equal or smaller spaces with *different* information:
- h10 × parity: exactly h11's 2.408e11 entries and the same 2-bit layout.
- h8 × G2-corners: 1.806e11 entries, 42 GiB.

Whether parity (or the half-turn corner state) is worth more than the EO bits it replaces is unknown. h10 × parity can't be BFS-generated exactly in our RAM: the true coordinate K11 would need 112 GiB at 2 bits. It would need a nissy-style DFS min-collapse instead.

A cheap way to find out is a small-scale analogue: h1 vs h0 × parity (both 2.35e8 entries, ~60 MB). The generator already BFSes true coordinates, and h0 × parity is one. Adding the parity bit to the index would let us compare node counts directly. If parity loses to an EO bit at small scale, the idea is dead. If it wins clearly, h10 × parity at h11's size is the next step.

**6. Using the 23 GiB that entropy coding of h11 itself would free (35 vs 58 GiB).** Nothing H48-compatible fits it (the next coset space is 2×). A second, differently-shaped table (e.g. a 16-symmetry UD-axis one) would add a second DRAM probe per lookup. Ideas 3.1 ("one big table beats several small ones") and our own lookup-bound profile argue against it.
