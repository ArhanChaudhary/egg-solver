# nxopt technique inventory

Sources read in full, from Tom Rokicki's [cube20src](https://github.com/rokicki/cube20src): `nxopt.w` (2062 lines), `nxopt.md`, `makefile-nxopt`, `README.md`,
`TODO.md`, `cubepos.w` (move, inverse, movepc, symmetry, canonical sequences, threading), `kocsymm.w`,
`phase1prune.w`, `phase1prune2.w` (the last three are the two-phase solver's, but they are in the
same source set, so their techniques are listed too).

"Ours" cites egg-solver (then named cube-opt) main **e7b094e**; line numbers are of that commit. Every "have it" was
checked in the code, not assumed. nxopt line numbers are `nxopt.w` unless another file is named.

Mapping used below (proved in the inventory notes at the end): nxopt's subgroup contains all U/D
moves for the UD-axis table; H48's 48-symmetric subgroup H contains exactly the six half turns
among the 18 moves. In our kernel a node branches on one side (the "moved" side `a`); the other
side `b` is premultiplied by `m^-1`, so its value depends on the coset `H m^-1 b`.

Verdicts: **done** (we have it, or something strictly stronger), **gap** (applies, we lack it),
**n/a** (does not apply to H48 / our setting), **neg** (tried, measured not to pay).

## A. Search loop

| # | technique | where in nxopt | what it does | ours | applies to H48? | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| A1 | IDA* from the root heuristic | `solve` 1760-1796 | iterate d from `lookup6(root)` up | `lanes.rs:472` (`target = max(ln, li, 1)`), `start_iteration` 553 | yes | - | done |
| A2 | Lookups on normal and inverse position | `lookup6` 1540-1568 (`invert_into` 1558) | 3 probes on p, 3 on p^-1 | both sides per child: `advance` 1062-1090, `resolve_other` 1116, `resolve_moved` 1143; inverse kept incrementally (`simd.rs:207` `premove_inv`) instead of inverting per node | yes | - | done (stronger: no per-node inversion) |
| A3 | 3 axis probes per side via remaps m=0,32,16 | `lookup3` 1430-1479, `lookup(cp,m)` 1373-1399 | the UD-axis table looked up for the UD, FB, LR axes | - | **no**: H48 is 48-symmetric, all conjugates give the same entry | 0 | n/a |
| A4 | (x,x,x) -> x+1 | 1476-1477, 1536-1537 | all 3 axes tight: no last move possible, so +1 | - | **no direct analog**: H48 excludes only half turns (6 of 18 last moves) per tight side; the 12 quarter turns are never excluded by one probe | 0 | n/a |
| A5 | Tight probe restricts moves (`expandm`, `fmask`/`rfmask`) | 825-828, 1473-1474, 1689-1695 | probe == togo: that axis' moves cannot be the move at that end | `child_node` 1240-1248, `setup_job` 889-897, `solver.rs:457-461` (`NO_HALF` when the other side's bound is tight) | yes (half turns only) | - | done |
| A6 | Restrictions carried to descendants | not done: children get `cs_mask` only (1714, 1740) and re-probe | - | `child_node` 1242 `mm_x = node.mm_o` carries the other side's tight mask down while that end is untouched | yes | - | done (stronger than nxopt) |
| A7 | Bidirectional search: extend the end with fewer allowed moves | 1693-1701 (`dir = popcount(rfmask) - popcount(fmask)`) | per node, branch on the front (movepc) or back (move) end | `child_node` 1255, `make_node` 1335, `solver.rs:315` | yes | - | done |
| A8 | Tie-break of A7 by probe sums | 1697-1700 | equal counts: extend the end whose fresh probes are higher | `neg/a2-side-tiebreak` (count*64+bound = exactly this rule) | yes | a2: -0.07% nodes (1K); re-measured on main, 20K: -0.02% (M3) | neg |
| A9 | Inherit one probe from the parent (`skipat`/`skipval`) | 1710-1711, 1736-1737, `lookup3` switch | the probe of the moved axis on the moved end is the parent's | X2 keeps the other side's value (`advance` 1065 `po = select(half, lo, tw)`); X and X' share it (`Node::tw` 79-87, `skip_xprime` 1099) | yes | - | done |
| A10 | **Inheritance along an axis** (nxopt chains `skipval` through consecutive same-axis moves: U then D keeps the UD probe) | `skipat` passed through recursive same-axis moves | value reused across two same-axis moves | no (`child_node` starts every node with `tw = [0xFF; 6]`) | **yes, partly**: for X on even face F and Y on the opposite face, `H Y^-1 X^-1 b = H Y^-1 X b` (X2 in H, X/Y commute) and `H Y^-1 X2 b = H Y^-1 b`; so the X' child reuses the X child's face-F+1 value, and the X2 child's face-F+1 value is the node's own | measured: -2.4% lookups, nodes identical; but -5.6% to -15.6% at 1T in every kernel form (spills double) | **neg** (`neg/xnx-axis-inheritance`) |
| A11 | Same-face sibling pruning on a child's return value | 1719-1726, 1745-1752 (`tt > togo+1`) | a child >= 2 over prunes its face | `note_value` 1088, pop path 993-1003 | yes | - | done |
| A12 | Return value of a subtree | 1755-1758 (returns the node's own probe, or 1) | - | children's bounds back-propagated (`max_dec` 1315, pop 998): back-jumps over whole nodes | yes | - | done (stronger) |
| A13 | Canonical move sequences (7-state automaton, HTM) | `cubepos.w` 1813-1960; `next_cs`/`cs_mask` 1712, 1738 | no same face twice, opposite faces in fixed order | `solver.rs:12-26` `ALLOWED_AFTER` | yes | - | done |
| A14 | Separate canonical state per end | `recur(fprev, rprev)` 1667-1668 | each end has its own automaton state | `Node::mm_o` (other end) + `allowed_after[k_face]` (moved end) in `child_node` 1240 | yes | - | done |
| A15 | Leaves: check identity at togo == 0 after probing at togo == 1 | 1669-1682 | probes at the last level too | `advance` 1020-1029: r == 1 nodes try `mm_n & mm_i` moves and test `is_solved`, no lookups | yes | - | done (stronger) |
| A16 | No probes where they cannot prune | - (nxopt always probes) | - | `advance` 1020: lookups only for r1 <= base+3 (2-bit codes cannot exceed base+3) | yes | - | done (ours only) |
| A17 | Early exit inside a side's probes | `lookup3` 1436-1441 | stop at the first probe > togo | per child: other side first, moved side only if it passes (`resolve_other` 1116) | yes | - | done |
| A18 | Prefix jobs ("heads", 4 moves) | `topn` 1586-1666, `TOPSIZE` 1588-1591 | above base+4, split the root into 4-move prefixes | `solver.rs:156` `job_len` (1..4 moves above base+3), `lanes.rs` `extend_jobs` 478 | yes | - | done |
| A19 | Order prefixes by last iteration's effort / `order_mult[cs]` | 1597, 1664, `order_mult` 2021-2042 (stationary count of canonical continuations) | most promising prefix first in the final iteration | `job_weight` 100 (47525/58206 = the same 1.2247 ratio of continuations after an even/odd face), `sort_jobs` 116, c7/c8 per-child ordering (`build_first` 616, `build_rest` 671) | yes | - | done (stronger: 8^(16-h) statistic, per-child split) |
| A20 | Symmetric start (`-s`) | `calcsymm` 1301-1331, 1774-1775 | skip first-move faces whose children are 96-equivalent (symmetry or antisymmetry) to an earlier face's | no | yes, only for symmetric/antisymmetric inputs | 0 on random states | gap; handed to the symmetric-start work then in progress (later merged as f07b109) |
| A21 | `-l` leftover (no U/D first move) | 1783-1785 | coset-search special case | - | no (coset solver feature) | - | n/a |
| A22 | `-A` all solutions, `-n/-x` depth limits | 2001-2017 | features | - | not speed | - | n/a |
| A23 | QTM parity bound | 1767-1780, `parity` 1278-1295 | odd/even depth from parity | - | no (HTM) | - | n/a |
| A24 | Solution check | `testit` 1801-1830 | replay the solution | bench verifies each solution (`bench.rs:54`) | - | - | done |

## B. Pruning table design and encoding

| # | technique | where in nxopt | what it does | ours | applies? | expected gain | verdict |
|---|---|---|---|---|---|---|---|
| B1 | Coset tables of subgroups containing all U/D moves (12 sizes) | `nxopt.md` Subgroups/Edges; 144-338 | EO1-3 x EP1-4 subgroups, 20 MB..4 TB | H48 (nissy) h11: 48-sym subgroup containing half turns (`coord.rs` 1-12) | different design | H48 h11 (58 GiB): ~3.06M nodes/cube; nxopt's comparable 62 GB EO3EP2 (avg6 12.72) is weaker per byte (16-fold vs 48-fold reduction) | done (H48 chosen) |
| B2 | 2-bit entries relative to a base | 342-582 (BASE per table), lookup 1354 | codes <=base, +1, +2, >=+3 | `solver.rs:123-138` `value` (base 10) | yes | - | done |
| B3 | Base chosen empirically per table ("yes; 10% faster than 10") | 393 | base tuned by timing | base 10 inherited from nissy/cube48opt | yes | simulated (M2): base 11 +38% nodes (exact emulation in the [nissy-core inventory](x-h48-inventory.md): +10.2%), base 9 +268% | done (10 is best) |
| B4 | 4-bit group minimum interleaved in the same 16 bytes | `nxopt.md` Edges; 697-700, 1359-1360 | fallback bound for code 0, same cache line | `value` 123-138 (`*p.add(w & !1) & 15`), slots 64k/64k+1 reserved (`coord.rs:60-78`) | yes | - | done |
| B5 | 495 -> 512 packing to make room for the minimum | `epadd` 604, test 776 | 2 free slots per 64 | `coord.rs:60-78` `mk_eslot` | yes | - | done |
| B6 | E-slice occupancy (495) as least significant index part | `nxopt.md` | cheap packing | same layout (`coord.rs:11-12`) | yes | - | done |
| B7 | Symmetry reduction (16, UD axis) with approximate canonical form (self-symmetric corner classes stored redundantly) | `initmem` 701-760, `firstlevs`/`mind` 42-49 | 9930 corner rows | 48-fold, 3393 cocsep classes, redundant entries for self-symmetric classes (`tables.rs` `Cocsep::generate`) | yes | - | done (stronger) |
| B8 | Corner-only bound | via the 4-bit min only | - | cocsep depth `cd` precheck before the big lookup (`solver.rs:115-120`, `lanes.rs:1068`) and `max(min, cd)` for code 0 | yes | - | done (ours only) |
| B9 | Branchless code-0 fallback (`NOBRANCH`) | 1355-1358 | mask select | `neg/m6-value-branchless` | yes | measured negative | neg |
| B10 | Next-move masks in the entry (phase1prune) | `phase1prune.w` 252-306, 412-467 | per entry, which moves decrease/keep/increase the distance (4 bits/face), so children need no probe | - | in principle (moved side only: `H a m` is a function of `H a`; the other side's child coset is not), but 24+ bits/entry = 13x the h11 table | memory: impossible at h11; a smaller table with masks loses far more pruning than it saves | n/a (memory) |
| B11 | Forced-move chains (phase1prune2) | `phase1prune2.w` 679-890 | encode unique descent paths in spare bits | - | no spare bits in 2-bit entries | - | n/a |
| B12 | Table file header (magic, metric, base) and size check | `writetab`/`readtab` 993-1009, 1185-1219 | - | `tables.rs` meta files, size and meta validation (r7) | yes | - | done |
| B13 | Table checksum (phase1prune `datahash`) | `phase1prune.w` 129-145 | integrity | size/meta validation only; r7's cross-check tools | marginal | - | done enough |

## C. Coordinates, cube moves, symmetry

| # | technique | where in nxopt | what it does | ours | applies? | gain | verdict |
|---|---|---|---|---|---|---|---|
| C1 | Additive per-cubie tables (`fastcornercoords`, `fastedges`) | 58-102, 129-338, `getedgecoord` 583-606 | sum of 8/12 table entries gives all coordinate fields at once | AVX2 shuffles + movemask + pext (`coord.rs` `cocsep3`, `edge_parts_sym_raw`, `index_sym`) | - | ours fewer µops (m4/m6 rounds) | done (different, faster) |
| C2 | Symmetry composed into the coordinate tables (`fastedges[mm[m][m2]]`) | 681-692, 1379 | one pass for remap + canonicalizing symmetry | `edge_parts_sym_raw`: the conjugate's edge classes built with 2 shuffles from `sym_edge[t]` | yes | - | done |
| C3 | Move by 24-entry transition table per move, unrolled | `cubepos.w` 519-540 | branch-free move | AVX-512/AVX2 `apply_move` (`simd.rs:195`) | yes | - | done (SIMD) |
| C4 | `movepc`: move as premultiplication on slot->cubie form | `cubepos.w` 889-1075 | the "front" end of bidirectional search | `premove_inv` (`simd.rs:207`): compose with a table cube | yes | - | done |
| C5 | Inverse by scatter | `cubepos.w` 837-887 | per node (1558) | only at the root (`simd.rs:225`); maintained incrementally afterwards | yes | - | done (stronger) |
| C6 | canon_into48/96 with early exit | `cubepos.w` 1709-1811 | symmetry canonicalization | only needed for A20 (`cubie.rs` has `symmetries`/`conjugate`) | for A20 | - | (part of A20) |
| C7 | kocsymm move tables / 16-way canon with `minbits`/`minmap` fast path | `kocsymm.w` 128-510 | two-phase coordinates | cocsep packed entry gives class, symmetry, depth in one 3-byte load | - | - | done (analog) |

## D. Lookup scheduling, memory, threads, compiler, I/O

| # | technique | where in nxopt | what it does | ours | applies? | gain | verdict |
|---|---|---|---|---|---|---|---|
| D1 | Prefetch a side's probes before resolving them | `lookup3_prefetch` 1480-1539, `lookup_prefetch` 1408-1429 | 2-3 misses overlapped per node | 24 lanes per thread, one miss in flight each (`run_lanes` 1408) | yes | - | done (far deeper MLP) |
| D2 | Prefetch hint (T0 default; T1 on Windows) | 892-897 | - | NTA (`solver.rs:141-146`); T0/T1 measured -1% (`neg/m6-prefetch-t0-t1`) | yes | - | done |
| D3 | Table memory: one malloc per corner row | 749 | - | one mapping on 1 GiB hugetlbfs pages (`tables.rs:234-290`) | - | TLB | done (stronger) |
| D4 | Threads: one cube per thread, no per-cube parallelism | `dowork` 1836-1884, 2051-2058 | - | stream lanes + share mode for latency (`lanes/share.rs`) | - | - | done (stronger) |
| D5 | Worker padding against false sharing | `int buf[64]` 1885 | - | lanes are thread-local `Vec<Lane>`; shared state is one atomic counter | yes | - | done |
| D6 | Global mutex for work handout and ordered output | `getwork` 1929, `report` 1253-1277 | - | atomic counter; bench records under a mutex once per cube | - | - | done |
| D7 | `-O3 -march=native`, `-DHALF` compile-time metric | makefile-nxopt | - | `target-cpu=native`, fat LTO, cgu=1 (Cargo.toml); flag sweeps negative (`neg/a7-*`) | yes | - | done |
| D8 | Input: stdin positions (Singmaster or moves) or random walk | `parseposition` 1887-1921, `getwork` 1929-1944 | - | `solve` CLI, `bench` random/stream/file | - | - | done |

## E. Table generation (not on the solve path)

| # | technique | where in nxopt | what it does | ours | applies? | gain | verdict |
|---|---|---|---|---|---|---|---|
| E1 | Affine GF(2) EO move tables (`emove`/`emap`: base ^ per-bit xor, deduplicated) | 839-891 | move a whole EO row with xors | `gen.rs` decodes/encodes cubes per entry | yes | generation time only | n/a for solve speed |
| E2 | Prefetch ring during generation (PS=16) | 900-992 | MLP in BFS | `gen.rs` prefetches in `backward_chunk` (232) | yes | gen only | done |
| E3 | mod-3 depth codes during BFS, then rebase to 2-bit codes | 1139-1163 | 2 bits per entry during the BFS | `gen.rs` 2-bit BFS states (unvisited/new/frontier/old) then final codes | yes | gen only | done (analog) |
| E4 | Lock-free row parallelism with neighbour locks, shuffled row order | `getgenwork` 1050-1087, shuffle 1041 | - | atomics per word (`gen.rs` `mark_new`, `lower_nibble`) | yes | gen only | done (analog) |
| E5 | Group minimum maintained in the same pass | 950-954 | - | `lower_nibble` (`gen.rs:49`) | yes | - | done |
| E6 | TESTTABLE: average probe value statistics | 1837-1864 | table quality metric | - (node counts used instead) | diagnostic | - | n/a |

## F. Ideas from the idea list

These rows follow the numbering of the idea list assessed in the [ideas assessment](ideas-assessment.md).

| # | idea | ours | measurement | verdict |
|---|---|---|---|---|
| 5.2 | BPMX / pathmax, full early return on both sides | in-tree: `note_value` (lanes.rs:1088) on both sides and on the corner depth, learned bounds at pop, `lb > r` early return (lanes.rs:950) | job-level pathmax (the only open case): 0 skippable jobs on random:1000:2026 (M4) | done |
| 5.4 | Canonical sequences beyond commuting order (DFA) | commuting order only | perfect duplicate elimination is capped at 0.48% of nodes, 0.0035% within jobs (M5) | not applicable (< 1%) |
| 3.6 | Small exact in-cache table | cocsep depth precheck | (class, eo11) projection of h11, 3.5 MB: prunes 0.16% of lookups (M6) | neg |
| B3 / nxopt | Empirical base | base 10 | base 11 +38% / 9 +268% nodes (M2) | done (10) |

## M. Measurements and prototypes

All counts: `count.sh` (8 threads, deterministic), lengths identical unless noted.

**M1, A10 axis inheritance** (nodes identical by construction; lookups):

| set | main e7b094e lookups/cube | axis (v2) lookups/cube | change |
|---|---|---|---|
| random:1000:2026 | 2588380 | 2526492 | -2.39% (v1 with root relays: -2.72%) |
| random:20000:99 | 2607693 | 2545909 | -2.37% |

nodes/cube 3058198 / 3080725 unchanged; 59.6% of inherited values prune. Throughput, 1T paired, 3 pairs:
v1 -14.7%, v2 -11.7%, v2a -8.4%, v2b -7.5%, consumer only -5.6%, pop relay only -1.8%,
v3 (known flag in `skip` bits) -15.6/-15.0%. `stream_thread` spill references: 157 (main), 189
(consumer only), 316 (v2), 319 (v3). The ceiling (~2.4% of lookups) is below the cost of every
form. Negative.

**M2, B3 base** (simulated from the base-10 table on random:1000:2026; the nissy-core inventory's exact base-11 emulation: +10.2% nodes):

| base | rule | nodes/cube | lookups/cube |
|---|---|---|---|
| 10 (main) | - | 3058198 | 2588380 |
| 11 | exact 11 -> max(group min, corner depth); >= 14 not representable (pessimistic) | 4233702 (+38%) | 3700617 (+43%) |
| 9 | 13 -> 12; exact 10 stays code 0 (pessimistic) | 11241867 (+268%) | 7451042 |

The 4-bit group minimum is the true minimum of its 62 entries and does not depend on the base.
Re-basing only changes which exact values fall into code 0 and get replaced by max(min, cd), which
for an 11 is usually 9-10. Free encoding idea (only with a regenerated table, tiny gain): effective
base = max(base, group min), so groups with min >= 11 get exact 11/12/13 and >= 14.

**M3, A8 tie-break by bounds** (nxopt's rule, count*64 + bound): random:20000:99 nodes 3080725 -> 3080063
(-0.02%), lookups -0.02%. Neutral, as a2 found.

**M4, 5.2 BPMX / pathmax.** In-tree upward BPMX is complete. Every child value on either side, and
the corner depth too, raises the node bound (`note_value`: `lb = max(lb, v - 1)`, the face is dropped
if v > r). Exhausted children return their learned bound (pop path). The node returns as soon as
`lb > r` (top of `advance`), on both sides. Downward pathmax cannot prune in fixed-threshold IDA*,
since the parent passed, so `parent - 1 <= r - 1`. It cannot restrict half turns either: that needs
the side's own coset value. Job-level pathmax was the only open case, because jobs are extended
whatever their parents' bounds. Measured with path bound = max over prefixes of (bound - distance):
0 jobs skippable on random:1000:2026, although 53 per cube drop by >= 2 on the inverse side.

**M5, 5.4 duplicates beyond commuting order** (reference solver, random:200:2026, every generated
node hashed with its remaining depth; a node is a duplicate if the same position was generated
before with at least as many moves left):

| depth from root | nodes | within the job | within the iteration (any job) |
|---|---|---|---|
| 5 | 102.6M | 0 | 0.21% |
| 6 | 302.7M | 0.000% | 0.42% |
| 7 | 371.5M | 0.000% | 0.55% |
| 8 | 137.6M | 0.003% | 0.62% |
| 9 | 29.1M | 0.043% | 0.57% |
| 10 | 4.3M | 0.114% | 0.53% |
| all (957M) | | **0.0035%** | **0.48%** |

Perfect duplicate elimination saves at most 0.48% of nodes, and inside jobs 0.0035%. A DFA cannot
reach the 1% bar. twsearch's `--newcanon` is not in the twsearch checkout that was read (Rust port only:
`canonical_fsm.rs` builds the commuting-class FSM, the same as ours).

**M6, 3.6 small in-cache table.** Candidate: the exact projection of h11, min over (msplit, eslot)
per (cocsep class, eo11), 3393 x 2048 = 6.9M entries (3.5 MB at 4 bits), built from the group
minima. Values: 8: 28%, 9: 62%, 10: 6.6%, max 11. On random:1000:2026 it would prune 0.16% of the
h11 lookups before the DRAM access. Lookups happen at r1 = 11 (53%), 12 (30%), 10 (12%), where
only values >= 11..13 prune, and no cache-sized coset space reaches those distances. Negative.

## Notes: why the mapping holds

* Kernel conventions (`simd.rs`): a child moves `a -> a m` and `b -> m^-1 b`. The other side's value
  only depends on `H m^-1 b` (H contains the half turns): X2 keeps it, X and X' share it.
* nxopt's `skipval` chains: a U move then a D move keeps the UD-axis probe on that end because both
  are in nxopt's subgroup. In H48 only half turns are in H, but X (face F) and Y (opposite face)
  commute, so `Y^-1 X b = X Y^-1 b = X2 (X^-1 Y^-1 b)`: the same coset as `Y^-1 X^-1 b`. Two-move
  sequences on one axis therefore share the other side's value in 3 classes (quarter-quarter,
  quarter-half = the X child's own, half-quarter = the node's own Y child). Moves on other axes do
  not commute with X2, so nothing else is shared; and the moved side's cosets `H a m` share nothing.
* A4 has no analog: a tight H48 probe forbids only the 6 half turns at its end; nothing forbids the
  quarter turns without probing the children.
