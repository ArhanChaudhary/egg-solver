# Assessment of ~/ideas.txt (Rokicki and Arhan brainstorm, 2026-11-04)

Copy of the ideas: `$P/ideas.txt`. Our solver: H48 h11, 2-bit entries, base 10, 4-bit group minima; IDA* with 24 lanes per thread.

| # | Idea | Status in our solver | Action |
|---|---|---|---|
| 1.1–1.5 | Faster table generation (scanning, batched writes, symmetry at the root) | Affects only generation time. Our limit is RAM, not generation time. | Use in the new any-k `gen` for laptop tables (p10). |
| 2.1 | nxopt 2-bit + per-line min + base | Done. cube48opt-style 16-byte groups with a 4-bit min; base 10 confirmed best for h11 (x-nx, x-h48 simulations: base 11 +38%, base 9 +268% nodes). | none |
| 2.2–2.4, 2.6 | Denser or entropy-coded tables (trits 1.25x, bitmaps, tANS ~1.5x, lossy) | They only pay if the freed memory buys a bigger effective table, and H48 tops out at h11 (full EO). | **x-h48:** feasibility study of a "bigger effective table within ~65 GiB": a coset refinement × an encoding, with the expected nodes gain and decode cost. |
| 2.7 | Non-power-of-two sizes without division | N/A: exact index, no hashing. | none |
| 3.1–3.4 | One big table; mean h is the key metric; nodes × size ≈ constant | Known; it guides the study above. | — |
| 3.6 | Small exact in-cache table that catches many lookups (untried) | We have only the cocsep corner-depth precheck. | **x-nx:** measure the fraction of h11 lookups a cache-resident projection table (≤ L2/L3 size) would prune. |
| 3.8 | Table checksum before/after runs | We have table-meta validation, tabcheck and a read-only mapping. | Optional `--verify-table` (xxhash) for the final comparison (p10). |
| 4.1, 4.5, 4.6, 4.7 | Inverse probe; rotated probes; fast representative; mirroring | Done, or N/A for a 48-symmetric table. | — |
| 4.2–4.4 | Merge the inverse into the table (96-way) | It needs a coset structure closed under inversion. The H48 coset of p and of p⁻¹ differ, and the mod-3 encoding is invalid on the inverse side. | Part of x-h48's bigger-table study. |
| 4.8 | Symmetric inputs: search the first levels slowly, drop symmetric duplicates (symmetry² speedup) | x-h48 is implementing symmetric start. | Use Rokicki's deeper version; we report one solution, so the all-solutions caveat doesn't apply. |
| 4.9 | Symmetry duplicates during search | Not worth it (confirmed). | — |
| 5.2 | BPMX / pathmax (full: return early from the parent) | We have sibling-face skip and back-jumps (lanes.rs doc). | **x-nx:** verify we do the *full* BPMX early return on both sides, including against premoves (the inverse side); if not, measure it. |
| 5.3 | Continue in the direction with fewer allowed moves | Done (side choice by popcount, inverse-first). | — |
| 5.4 | Canonical sequences beyond commuting (twsearch `--newcanon`, longer identities) | Commuting order only. Depth-4 duplicates 0.035%, depth-5 0.43% (x-vc); an earlier estimate said ≤ 0.5%. | **x-nx:** measure duplicate sequences in the real search tree at depths 5–10 (positions reached by >1 canonical sequence within a job) and read twsearch's newcanon (~/ref/twsearch/src). Implement a DFA if it pays ≥ 1%. |
| 5.5, 6.1, 6.2, 6.3 | Preallocated stacks; deep splits; microthreading; huge pages | Done: lanes = microthreading; 1 GiB pages; 2–4-move job prefixes. | — |
| 7.2 | min(v, v−3) mod-3 fix; SIMD inverse | Done (x-vc: our move 4.6 Ir, child cubes 12.5 Ir, vs vcube's 4.4 and 53.8). | — |
| 7.4 | Ranking: popcount Lehmer, half-split tables, digit parallelism | Our index chain is 45 Ir and 15.2 ns latency (x-vc). | **x-vc:** apply these to shorten the index chain (coordinated with m7). |
| 7.1, 7.5 | Throughput over many positions; Apple Silicon differences | — | Laptop/NEON port (p10). |
