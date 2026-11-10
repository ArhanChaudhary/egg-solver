# Assessment of optimization ideas from Tom Rokicki and Arhan Chaudhary (2026-11-04)

This table assesses a numbered list of optimization ideas from Tom Rokicki and Arhan Chaudhary. The list itself is not published; the first column keeps its numbering, which the other inventories cite as “idea 2.4” and similar. The Action column records the follow-up planned at the time, with the recorded outcome where one of the other inventories measured it.

The solver at the time: H48 h11, 2-bit entries, base 10, 4-bit group minima; IDA* with 24 lanes per thread.

| # | Idea | Status in our solver | Action |
|---|---|---|---|
| 1.1–1.5 | Faster table generation (scanning, batched writes, symmetry at the root) | Affects only generation time. Our limit is RAM, not generation time. | Use in the new any-k `gen` for laptop-sized tables. |
| 2.1 | nxopt 2-bit + per-line min + base | Done. cube48opt-style 16-byte groups with a 4-bit min; base 10 confirmed best for h11 (simulations in the [nxopt](x-nxopt-inventory.md) and [nissy-core](x-h48-inventory.md) inventories: base 11 +38%, base 9 +268% nodes). | none |
| 2.2–2.4, 2.6 | Denser or entropy-coded tables (trits 1.25x, bitmaps, tANS ~1.5x, lossy) | They only pay if the freed memory buys a bigger effective table, and H48 tops out at h11 (full EO). | Feasibility study of a "bigger effective table within ~65 GiB": a coset refinement × an encoding, with the expected nodes gain and decode cost. Outcome: infeasible ([nissy-core inventory](x-h48-inventory.md), feasibility section). |
| 2.7 | Non-power-of-two sizes without division | N/A: exact index, no hashing. | none |
| 3.1–3.4 | One big table; mean h is the key metric; nodes × size ≈ constant | Known; it guides the study above. | — |
| 3.6 | Small exact in-cache table that catches many lookups (untried) | We have only the cocsep corner-depth precheck. | Measure the fraction of h11 lookups a cache-resident projection table (≤ L2/L3 size) would prune. Outcome: 0.16% of lookups; negative ([nxopt inventory](x-nxopt-inventory.md), M6). |
| 3.8 | Table checksum before/after runs | We have table-meta validation, tabcheck and a read-only mapping. | Optional `--verify-table` (xxhash) for the final comparison. |
| 4.1, 4.5, 4.6, 4.7 | Inverse probe; rotated probes; fast representative; mirroring | Done, or N/A for a 48-symmetric table. | — |
| 4.2–4.4 | Merge the inverse into the table (96-way) | It needs a coset structure closed under inversion. The H48 coset of p and of p⁻¹ differ, and the mod-3 encoding is invalid on the inverse side. | Part of the bigger-table feasibility study above. |
| 4.8 | Symmetric inputs: search the first levels slowly, drop symmetric duplicates (symmetry² speedup) | Symmetric start was being implemented (later merged as f07b109). | Use Rokicki's deeper version; we report one solution, so the all-solutions caveat doesn't apply. |
| 4.9 | Symmetry duplicates during search | Not worth it (confirmed). | — |
| 5.2 | BPMX / pathmax (full: return early from the parent) | We have sibling-face skip and back-jumps (lanes.rs doc). | Verify that the *full* BPMX early return happens on both sides, including against premoves (the inverse side); if not, measure it. Outcome: in-tree BPMX is complete; job-level pathmax would skip 0 jobs ([nxopt inventory](x-nxopt-inventory.md), M4). |
| 5.3 | Continue in the direction with fewer allowed moves | Done (side choice by popcount, inverse-first). | — |
| 5.4 | Canonical sequences beyond commuting (twsearch `--newcanon`, longer identities) | Commuting order only. Depth-4 duplicates 0.035%, depth-5 0.43% ([vcube inventory](x-vcube-inventory.md)); an earlier estimate said ≤ 0.5%. | Measure duplicate sequences in the real search tree at depths 5–10 (positions reached by >1 canonical sequence within a job) and read twsearch's newcanon source. Implement a DFA if it pays ≥ 1%. Outcome: at most 0.48% of nodes, below the 1% threshold ([nxopt inventory](x-nxopt-inventory.md), M5). |
| 5.5, 6.1, 6.2, 6.3 | Preallocated stacks; deep splits; microthreading; huge pages | Done: lanes = microthreading; 1 GiB pages; 2–4-move job prefixes. | — |
| 7.2 | min(v, v−3) mod-3 fix; SIMD inverse | Done (vcube inventory: our move 4.6 Ir, child cubes 12.5 Ir, vs vcube's 4.4 and 53.8). | — |
| 7.4 | Ranking: popcount Lehmer, half-split tables, digit parallelism | Our index chain is 45 Ir and 15.2 ns latency ([vcube inventory](x-vcube-inventory.md)). | Apply these to shorten the index chain. Outcome: the index has no permutation ranks for popcount Lehmer ranking, digit-parallel ranking was already in use, and the half-split variant was negative ([vcube inventory](x-vcube-inventory.md), section F). |
| 7.1, 7.5 | Throughput over many positions; Apple Silicon differences | — | Laptop/NEON port. |
