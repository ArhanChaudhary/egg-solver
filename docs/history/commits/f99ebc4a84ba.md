# Packed 3-byte cocsep entries on a huge page; shorter index chain

Commit `f99ebc4a84ba2c4f4acaa16b266be3bf21defbf6` · 2026-11-03T01:47:35+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

Packed 3-byte cocsep entries on a huge page; shorter index chain

The hot cocsep data is now packed to 3 bytes per entry,
class<<10 | ttrep<<4 | depth, read as an unaligned u32 at 3i (459 KB
instead of 612 KB of L2 footprint). The packed table lives in its own
2 MiB anonymous mapping so it is backed by one transparent huge page (one
TLB entry instead of ~150). The u32 `data` table stays for gen.rs/tests.

index_from_parts computes (class*70+msplit) << (k+9) | (eo>>(11-k))<<9 |
eslot directly instead of building the h11 key and reducing it, cutting
the chain after the RANK8 load from ~8 ops to 4. The unit test checks it
against reduce_key for every k in 0..=11.

Search is unchanged (depth18: 3607979 nodes/cube, lengths 18:100).

Benchmark (bench stream 20 30 48 16, interleaved A/B, 3 runs each):

| build                 | run 1 | run 2 | run 3 | median |
|-----------------------|-------|-------|-------|--------|
| main ae4fc49          | 598.5 | 598.4 | 594.9 | 598.4  |
| packed cocsep         | 616.8 | 613.1 | 614.9 | 614.9  |

+2.8% solves/sec.
