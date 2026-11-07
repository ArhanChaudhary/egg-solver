# Merge work/s8: readability sweep (names, docs, clippy pedantic, std idioms, get_unchecked, gain comments)

Commit `c9a1c4ddfcc0c0307caf8dab6b5a3ed52c63bcec` · 2026-11-04T10:32:43+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

Merge work/s8: readability sweep (names, docs, clippy pedantic, std idioms, get_unchecked, gain comments)

The user asked for descriptive names and short inline docs a beginner
to intermediate reader can follow (no walls of text), clippy pedantic,
std APIs instead of C idioms, get_unchecked at the index instead of
assert_unchecked + indexing, and a comment at every optimization with
its measured gain. Ten commits (details and per-commit tables in each):

 1. lints: clippy::pedantic enforced via [lints.clippy] (inline_always
    allowed); std idioms (isolate_lowest_one, is_multiple_of,
    unbounded_shr masks, carrying_mul); three idioms that changed the
    asm keep the old form with a comment.
 2-5. cubie/coord, simd/tables/gen/lib, solver (search split into
    steps), bins/tests/examples: names and docs.
 6-8. lanes.rs: field/item names, local names, docs (share.rs updated
    mechanically on top of l6's names).
 9. get_unchecked at the index (resolve_other keeps assert_unchecked:
    with get_unchecked there the share loop had +39 instructions and was
    -1.6% at 1T, all 8 layout pairs negative). Stream loop 1468 -> 1417
    instructions, 5 -> 2 bounds checks; share loop 1285 -> 1229, 3 -> 0.
10. inline gain comments (lanes, solver, coord, simd, tables, gen), every
    number from git history or notes, "(not measured separately)" where
    none exists.

Rename map for agents' patches: $P/s8-rename-map.{md,sed},
s8-lanes-locals-spec.py, s8-fnrename.py.

Verified: check.sh at all 10 commits; verify.sh identical to main
(3058198 / 2588380; depth18 3644301 / 3084160); stream and share loops
instruction-identical for commits 1-8; .text byte-identical 5-8 and 9-10.

Base 55e048e -> this, every binary built in one folder:

| test                                   | result                                      |
|----------------------------------------|---------------------------------------------|
| 1T stream, layout-averaged (8 pairs)   | median -0.20%, mean +0.17% (layout noise)   |
| 1T share solve, layout-averaged        | median -0.48%, mean -0.49% (noise ~1.6%)    |
| 48T bench share (2 pairs)              | 940.1 -> 938.7, 923.5 -> 933.5 solves/s     |
| 48T stream (on f2cb1f4, commit 8)      | 1083.2 -> 1086.5 (+0.3%)                    |
| 48T solve -c 1 (5 pairs)               | +11.2 +4.4 -2.0 +2.4 +0.8% (noisy measure)  |
| solve -c 0 / -c 1                      | BAD 0, per-cube lengths identical           |

Neutral within noise, as expected for a refactor whose hot loops are
instruction-identical (commits 1-8) or shorter (commit 9).
