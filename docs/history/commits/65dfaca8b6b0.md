# NEG gen: FxHashSet for the dfs prefix set (hashbrown's set adopted instead)

Commit `65dfaca8b6b08feb11b9b4404fe43b0f0d2323ac` · 2026-11-05T05:18:58+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/d1-fx-dfs`

## Original experiment record

NEG gen: FxHashSet for the dfs prefix set (hashbrown's set adopted instead)

The DFS generator's set of the prefix's last two levels (gen/dfs.rs), our
own open-addressing set, as rustc-hash's FxHashSet (std's HashSet with the
Fx hash), as the user suggested. Just as fast as the hashbrown set that was
adopted (gen 2 8 at 8 threads, compute from gen's log, without the file
write; outputs byte-identical, md5 01e161c964d6):
| rep | main | FxHashSet | hashbrown (foldhash) |
|-----|------|-----------|----------------------|
| 1 | 5.2 s | 4.0 s | 4.0 s |
| 2 | 5.6 s | 3.9 s | 4.0 s |
| 3 | 5.3 s | 4.1 s | 4.2 s |
(gen 0 7 --dfs, its one pass: 1.3 s -> 0.6 s for both.)

Not chosen because FxHashSet is std's HashSet: its hashbrown code is the
copy that std's HashMap in symstart's dedup also instantiates, so with a
second user LTO inlines that shared code differently in the solver:
Lane::extend_jobs changes, and so does bench's thread closure (the stream
loop's caller), frame 0x700 -> 0x740 (the stack-frame hazard). The
crates.io hashbrown set is a separate copy: with it every cubeopt, bench
and solve function outside gen is the same code as on main.
