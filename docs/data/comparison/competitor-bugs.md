# Bugs and issues found in other solvers (for the final report; the user will report them upstream)

Source commits read: nissy-core 1f7e8e9 (2026-09-17), vcube 9f5bc2c (2024-04-18), cube20src/nxopt f430921 (2025-07-18), cube_solver_test b9cca71 (2026-02-01); cubeopt30/cube48opt10 = the prebuilt binaries in ~/ (no source).

Each entry: project, file:line at the commit we read, what is wrong, a repro or proof, impact, and a suggested fix.

| # | project | severity | summary |
|---|---|---|---|
| 1 | nissy-core (H48) | correctness (admissibility) | h11 generator skips the last hash-map slot: 11,546 h11 entries are one level too high |
| 2 | cube_solver_test (h48cmd.py) | harness | per-cube lines are mis-paired (stdout is buffered, nissy logs to stderr); the Avg line is unaffected |
| 3 | cube_solver_test (h48cmd.py) | harness / memory | `bytearray(f.read())` holds the table twice (2x RAM); h48h8 does not fit in ~16 GiB free |
| 4 | cube_solver_test (runtest.sh) | harness | passes `-g $ngroup` to cubeopt/cube48opt, but the published binaries accept only `-l -m -n -t -v` (no `-g`); the option is ignored |
| 5 | vcube | performance | when MAP_HUGETLB (1G, then 2M) fails, falls back to a plain mmap with no madvise(MADV_HUGEPAGE): 4K pages, ~10% slower on THP=madvise systems |
| 6 | nxopt | performance | the table is one malloc per corner row with no MADV_HUGEPAGE: 4K pages on THP=madvise systems, ~11% slower |

## 1. nissy-core: h11 table slightly inadmissible (generator skips the last hash slot)

- **Where:** h48/gendata_h48.h:307-316 (`gendata_h48_runthread`) with h48/map.h:68-87 (`h48map_nextkvpair` post-increments the slot index).
  - Commit: nissy-core 1f7e8e9.
- **What:** the loop `kv = nextkvpair(map, next); if (*next == capacity) break;` breaks before processing the entry in the last slot (`capacity - 1`).
  - With the fixed capacity 10000019 and nissy's hash, that slot holds the depth-8 h11 entry 189880194105 (class 2675, no self-symmetry). Its 4-level DFS never runs: the loop visits 7,437,854 of 7,437,855 short entries.
  - The same loop shape in `gendata_h48short` (gendata_h48.h:62-65) is harmless, because that entry's value is 8 (the last BFS level).
- **Proof:**
  - nissy's own depth-≤8 BFS equals ours per depth (1/1/4/34/333/3815/45382/548562/6839723).
  - Modelling nissy's values in the 49,206 entries within 4 moves of the skipped root gives ours − nissy = [+159, +1689, +7691, −9539] per 2-bit code. That exactly matches the difference between our exact h11 distribution [1150763320, 12045847349, 91163441021, 136418085910] and nissy's checkdata.h.
  - Scripts: /data/agents/x-h48/drv/{short,lastslot,skipped}.c.
- **Impact:** 11,546 entries are over-estimated by one level (159 true ≤10 coded 1, 1848 true 11 coded 2, 9539 true 12 coded 3). The table is therefore not admissible: a solve whose optimal path passes through one of these positions near the end could return a non-optimal solution. The probability is tiny on random cubes but nonzero.
- **Fix:** after `h48map_nextkvpair`, test `kv.key == MAP_KEYMASK` while holding the mutex. That sentinel means the iterator is exhausted; every non-sentinel entry, including the last occupied slot, must be processed. Regenerate h11 and update checkdata.h.
- **To check in the final comparison:** whether h9/h10 (other k) are affected; the length cross-check on h48h11 runs.

## 2–4. cube_solver_test harness issues (found by f1 while extending it)

- **2. h48cmd.py output pairing.** Python's stdout is block-buffered, while nissy prints its stats to stderr, so runtest.sh's awk pairs "Nodes visited" and "Time" lines from different cubes. Per-cube lines are wrong; totals and Avg are right. Fix: `flush=True` / `sys.stdout.flush()` after each solve. Patch in /data/agents/f1/cube_solver_test-cubeoptrs.patch.
- **3. h48cmd.py double table copy.** `bytearray(f.read())` keeps the bytes object and the bytearray (2x the table) alive, so h48h8+ needs twice the RAM. Fix: `buf = bytearray(size); f.readinto(buf)`. Patch: same file.
- **4. runtest.sh `-g`.** cubeopt()/cube48opt() pass `-g $ngroup`, but the published cubeopt30/cube48opt10 binaries parse only `-l -m -n -t -v` (from their disassembly), so ngroup has no effect. Either the binaries or the script are out of date.

## 5–6. Huge-page handling (performance, not correctness)

- **5. vcube** (src/alloc.cpp): tries MAP_HUGETLB 1G, then 2M (both fail when the hugetlb pool has no free pages), then falls back to a plain mmap with no madvise. On THP=madvise systems the table ends up on 4K pages. Measured with an LD_PRELOAD madvise shim: 1T −10%, 48T −9% sec/solve.
  - Fix: `madvise(p, len, MADV_HUGEPAGE)` on the fallback mapping.
- **6. nxopt** (cube20src nxopt.w): one malloc per corner row (114 × 17.5 MiB + smaller), no madvise, so 4K pages on THP=madvise. With glibc.malloc.hugetlb=1 + mmap_threshold=32 MiB: 1T −11%, 48T −11%.
  - Fix: one aligned allocation for the whole table + MADV_HUGEPAGE.

## f1: exact locations for #2-#6, and new findings #7-#10 (f1, 2026-11-04)

Exact locations for the entries above (code as read; harness = ~/ref/cube_solver_test b9cca71):
- **#2:** h48cmd.py:31 `print (scramble)` and :34 `print('Time: ...')` go to Python's stdout, which is block-buffered when piped. nissy's stats go to stderr: python/nissy_module.c:447-449 `log_stdout()` (misnamed) does `fprintf(stderr, ...)` and is installed at :455. runtest.sh:85 merges them with `2>&1`.
  - Mechanism, reproduced: `python3 -c 'import sys; [ (sys.stderr.write("Nodes visited: %d\n" % i), print("Time: %d" % i)) for i in range(2)]' 2>&1 | cat` prints both "Nodes" lines before both "Time" lines. runtest.sh's awk (h48(), runtest.sh:86-97) then pairs every Time with the last cube's nodes.
  - Fix in our copy: `sys.stdout.flush()` after each cube.
- **#3:** h48cmd.py:17 `solver_data = bytearray(f.read())`.
- **#4:** runtest.sh:124 `./cube$3$2 -t $nthread -g $ngroup -m $maxl -`.
  - Binaries: the option switch in main does `sub $0x6c,%eax; cmp $0xa,%al` (cube48opt10 at 0x2a79, cubeopt30 at 0x2a99), then a jump table (0x183f4 / 0x193f4). That table has cases only for l, m, n, t and v: `-l` scramble length (default 15), `-m` max depth (20), `-n` count (100), `-t` threads, `-v` debug.
  - `-g` is skipped; its value "1" briefly becomes the mode character, and the final "-" sets the stdin mode again. So `-g` is harmless but has no effect.
- **#5:** vcube src/alloc.cpp:45-57 (`alloc::huge_impl`: `map_huge(n, 30)` -> `map_huge(n, 21)` -> `map_huge(n, 0)`; `map_huge` at :36-43 adds no madvise). The SysV-shm path is at :77-89 (the `-S` option).
  - strace: `mmap(3072 MiB, MAP_HUGETLB) = -1`, `mmap(2484 MiB, MAP_HUGETLB) = -1`, then a plain 2482 MiB mmap.
  - 200-cube numbers: /data/agents/f1/final/results.md (phase A).
- **#6:** cube20src nxopt.w:749 `firstlevs[j].p = (unsigned int *)malloc(MEMSIZE * (size_t)siz);` (nxopt.cpp:837 after ctangle). It allocates one block per distinct corner row.
  - glibc serves each block with its own mmap: strace shows 114 mmaps of 17.5 MiB plus smaller ones, and no madvise.
  - `glibc.malloc.hugetlb=1` alone does not help: blocks under 2 MiB are separate small mmaps, and with 700 KiB blocks the test showed 0% THP.
  - It also needs `glibc.malloc.mmap_threshold=33554432`, so that the rows come from the madvised heap.

| # | project | severity | summary |
|---|---|---|---|
| 7 | nissy-core (H48 solve) | nondeterminism (uninitialized read) | task `rank` is never initialized, so the depth >= 16 task sort reads stack garbage; node counts (and times) vary between identical 1-thread runs |
| 8 | nissy-core (python binding) | memory leak | `gendata()` copies its PyMem_Malloc buffer into a new bytearray and never frees it: 2x the table, and the buffer leaks |
| 9 | nissy-core (build.sh) | build pitfall | THREADS defaults to `nproc`, i.e. the CPUs of the build's affinity mask; a build under taskset/cpuset caps every later solve (and gendata) at that count, with only a log line |
| 10 | nissy-core (H48 gendata) | performance (minor) | the progress logger sleeps a fixed ~10%-of-expected-time interval and joins only afterwards: up to ~3.5 min of idle wait after h48h7's workers finished (8 cores) |

### 7. nissy-core: uninitialized task rank, so node counts are nondeterministic
- **Where:** src/solvers/h48/solve.h:578 `solve_h48_task_t tasks[H48_STARTING_CUBES];` is a stack array. `solve_h48_maketasks` (:487-:492) sets cube, pval, moves and tmask, but not `rank`.
  - `rank` is written only after a task is searched (:443-446).
  - From depth 16 (`H48_SORT_TASKS_MIN_DEPTH`) the tasks are sorted by rank (:688). Tasks skipped so far (`pval + 4 > depth`, :419-421) still hold whatever was on the stack.
- **Repro:** the same h48h7 cube at `threads = 1`, in four runs (random_state #0): 38,739,453 / 38,726,886 / 38,725,684 / 38,726,886 nodes. Cube #1: 7,090,571 vs 7,090,825.
  - Solution lengths are unaffected: every depth is still searched completely before the next.
  - What changes is when the first solution is found inside the last depth. That changes nodes and time.
- **Fix:** `tasks[*ntasks].rank = 0;` in maketasks (or zero the array).

### 8. nissy-core python binding: gendata leaks and doubles memory
- **Where:** python/nissy_module.c:247 `buf = PyMem_Malloc(size);` then :254 `return PyByteArray_FromStringAndSize((char *)buf, size);`. That call copies the buffer, and `buf` is never freed.
- **Impact:** 2x the table during generation, and the original copy stays leaked: about 15 GB for h48h8 and 121 GB for h48h11.
  - The harness's h48cmd.py generates through this path when the table file is missing.
- **Fix:** allocate the bytearray first (`PyByteArray_FromStringAndSize(NULL, size)`) and generate into `PyByteArray_AS_STRING`. Or free `buf` after the copy.
- **Workaround used here:** /data/agents/f1/tools/src/h48gen.c (one malloc, fwrite).

### 9. nissy-core build.sh: thread maximum taken from the affinity mask
- **Where:** build.sh:44 `THREADS=${THREADS-...}`, :61-65 `detectthreads` -> `portable_nproc` -> `nproc` (:50). `nproc` honours `sched_getaffinity`.
  - The value becomes the compile-time `-DTHREADS`.
  - At solve time, src/nissy.c:349-353 clamps any larger request to it with only a LOG line.
- **Repro:** `taskset -c 0-7 ./build.sh python` gives THREADS=8. Then `nissy.solve(..., threads=48)` logs "[solve] Selected number of threads (48) is above the maximum value (8), using 8 threads instead".
  - With runtest.sh's `2>&1 | awk` that line is never seen. Our 48T smoke runs of h48h7 ran on 8 threads.
  - The final runs were rebuilt with THREADS=48, and init_h48.sh now passes `THREADS=$(nproc --all)`.
- **Fix:** `nproc --all` (or `getconf _NPROCESSORS_ONLN`), or a much larger default maximum: THREADS only sizes arrays.

### 10. nissy-core gendata: progress logger delays the end
- **Where:** src/solvers/h48/gendata_h48.h:262-281.
  - The main thread sleeps `sleeptime = 100*(nshort - velocity)/velocity` ms between progress lines (about 10% of the expected total, estimated from the first second).
  - It reaches `wrapthread_join` (:287) only after its current sleep.
- **Observed:** h48h7 on 8 cores. The workers finished, and h48gen sat in `hrtimer_nanosleep` with 1 thread and 0% CPU for about 3.5 min before writing. Wall 21.5 min against 17 min of compute (8144 s of CPU).
- **Fix:** wait on the workers with a timed join or a condition variable, instead of a fixed sleep.

### Watch list (no bug confirmed)
- **cubeopt30:** its table-generating run solved its one cube (random_state #1, 18 moves, 33.3M nodes) in 10.36 s at `-t 48`, which is 311 ns/node. That is far slower than its README class. It may be first-touch or compaction right after generation; the 200-cube phase-A run will tell.
  - Note: its cube numbers start at 1 ("Cube1").
- **nissy h48h8 generation:** at 48 threads it reached only ~15% after 5 min (3.5 CPU-h; ~21-24 CPU-h projected, ~30 min). h48h7 took 2.3 CPU-h. Base 10 (h8..h11) makes the depth-8 DFS ~10x more work.
  - Not a bug, but h48h9/h10/h11 generation will take >= 30 min each at 48 threads.

### f1 update (2026-11-05): cubeopt30 watch item resolved
- The 10.36 s for one cube came from the run that had just generated the table. In the 200-cube 48T run (table loaded from file), the same cube (Cube1) took 166.7 ms, and the mean was 0.250 s/cube on 1 GiB pages.
- No bug. It looks like a one-off cost right after generation (THP compaction / first touch).

## shm1: vcube shared memory (found while porting `--shm`, 2026-11-05)

| # | project | severity | summary |
|---|---|---|---|
| 11 | vcube (`--shm`) | robustness (silent slow or stale table) | a `--shm` load that fails or is interrupted after creating the segment leaves it zero-filled or partly filled, and every later run attaches it with no check; the key names only the coordinate and base, so a regenerated table is also shadowed silently |

### 11. vcube: unchecked SysV shared-memory segments
- **Where** (vcube 9f5bc2c, as read; not run here):
  - src/nxprune.cpp:130-162 `prune_base::loadShared(key, filename)`: `alloc::shared(sz, key, true)` creates the segment (IPC_CREAT|IPC_EXCL), then `fopen` (:144), `fread` (:149) and the size check (:154). On any failure it returns `false` and the created segment stays, zero-filled or partly filled. Ctrl-C during the `fread` of a 23 GiB table does the same.
  - src/vc-optimal.cpp:337 `if (!P.loadShared(shm_key))`: every run, with or without `--shm`, first attaches any segment under the key and uses it at once (`setPrune(mem)`, nxprune.cpp:159). There is no size, header or checksum check, and no "load complete" mark.
  - src/vc-optimal.cpp:84-87 (`S()`): the key is `0x76630000 | Base << 8 | (EP + 1) << 4 | (EO + 1) * 4`. It names the coordinate and base, not the table, so a table regenerated after a generator fix (same file name) is shadowed by the old segment until `ipcrm` or reboot.
  - src/alloc.cpp:77-89 `shared_impl`: the creating run keeps its read-write attachment for the search (no `SHM_RDONLY` re-attach).
- **Impact:** in a zero-filled segment every entry reads 0, so `fetch()` (nxprune.h:357-362) returns the stripe minimum 0 for every position. The bounds are still admissible, so solutions stay optimal, but the search is unpruned: in practice a hang for 17-20 move positions. A partly filled segment gives weaker bounds for the rest. Neither is reported.
  - vcube's README ("Shared memory") mentions removing a segment "which is corrupt because it was interrupted while loading", but nothing detects it. The user has to know to run `ipcrm`.
- **Fix:** write a small header after the data, last (the table's size and a checksum or generator version), and check it on every attach; remove the segment (IPC_RMID) when the load fails. egg-solver's `--shm` (shm1) writes the file's trailer last and compares it on every attach, and removes a segment whose loader died.

## Continuation review (2026-11-05)

The source mechanisms for #1–#3 and #5–#11 were checked again against the exact commits named above, using `git show` rather than the harness's modified working files. Source excerpts with full commit IDs are retained in `/data/agents/codex-resume/competitor-source-evidence.txt`. The performance numbers and the large-table numerical proof remain the original recorded measurements; this source review does not claim to have rerun those experiments. #4's harness invocation was rechecked; the binary option-switch evidence remains the original disassembly.

**Correction to #1's suggested fix:** changing `*next == capacity` to `*next > capacity` is insufficient and can loop forever: `h48map_nextkvpair` stops advancing at `capacity`. In this source the exhausted iterator returns `kv.key == MAP_KEYMASK`; check that sentinel under the mutex, and process every non-sentinel entry, including the last occupied slot. Regenerate the table after fixing the loop.

The distance-20 interference validation also requires serial nissy timings: h48h7 took 210.867 s/cube alone and 217.062 s/cube in the parallel block (+2.94%, above the agreed 2% limit). Parallel distance-20 timings are excluded from the final comparison and preserved as rejected evidence.

## Pinned source links

- [nissy-core source snapshot](https://github.com/sebastianotronto/nissy-core/tree/1f7e8e937d0e71e7eb3b5deab38b34d0a0159d88)
- [vcube source snapshot](https://github.com/Voltara/vcube/tree/9f5bc2cce18f29437879ace825f22917f6705378)
- [cube20src source snapshot](https://github.com/rokicki/cube20src/tree/f430921a48306874864b9f2ac8a059b43492fd3a)
- [cube_solver_test source snapshot](https://github.com/cs0x7f/cube_solver_test/tree/b9cca71cdf3f6e57bd3c7bfdc2491e5b806dda31)
