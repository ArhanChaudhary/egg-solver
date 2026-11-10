# cube48opt-rust benchmark setup

Source: [github.com/2017YANR02/cuberoot.me](https://github.com/2017YANR02/cuberoot.me), `solver/experiments/cube48opt-rust` at 1effb34 (2026-11-05), built from a read-only local copy.

This page records how cube48opt-rust was built, given its tables and run for the [historical multi-solver comparison](../../comparisons/RESULTS.md). It was written for the comparison harness before the measured runs; the harness function it describes is in [`runtest.sh`](../../reproduce/harness/runtest.sh). Paths on the measurement machine are written as placeholders.

What it is:
- A Rust port of Chen Shuang's cube48opt1..9. The Opt backend reads cube48opt's own `h48prunNN.dat` tables and the WASM's `small-memory.bin`.
- **Opt10** is its own EO10 extension. Its table is cube48opt10's table, see §2.
- An **H48 backend** reads nissy-core's `h48hN` files.
- The crate has no dependencies and no build.rs, so builds are offline and reproducible.

All files are under one working directory, written `$D` below (binaries in `$D/bin`, tables in `$D/optN`), except Opt10's table, which is the comparison's raw cube48opt10 table `h48prun31h1.dat`, used in place.

## 1. Builds

Every binary is built from inside its folder with `cargo build --release --locked --bin cube48opt-research --target-dir TARGET/<name> [--features F]`. `RUSTFLAGS` is per row below.

| `$D/bin/` name | target dir | RUSTFLAGS | features | what it is |
|---|---|---|---|---|
| `c48rs-opt7` | default | (none) | default | Opt1-7 generic search. x86-64 baseline with runtime AVX2/AVX-512 dispatch: the README's portable build (README:71) |
| `c48rs-opt7-nat` | native | `-C target-cpu=native` | default | the same with target-cpu=native. **A build inside their repo gets this by default**: `solver/.cargo/config.toml`, which is in their git tree but not in our sparse checkout, sets it for every non-wasm target |
| `c48rs-opt7-pfnat` | opt7pf-native | native | `opt1-7-native-prefetch` | their optional x86 table prefetch for Opt1-7 ("unmeasured" upstream) |
| `c48rs-opt7-pgo` | pgo-opt7-nat | native + PGO | default | see §6 |
| `c48rs-opt8` | opt8svc | `-C target-cpu=x86-64` | `opt8-native-search,opt8-native-quarter,opt9-native-fixed-contexts,opt8-native-prefetch` | their production Linux service build (`native-service/build.mjs:16,29`; it is musl-static there) |
| `c48rs-opt8-nat` | opt8svc-native | native | same | |
| `c48rs-opt8-pgo` | pgo-opt8-nat | native + PGO | same | see §6 |
| `c48rs-opt10` | opt10-shim | (none) | `opt10-native-coset` | the official Opt10 feature set (README; `opt10-release.json`) with the **loader-only shim** of §2 |
| `c48rs-opt10-nat` | opt10-shim-native | native | same | |
| `c48rs-opt10-pf` / `-pfnat` | opt10pf-shim[-native] | none / native | `+ opt10-native-prefetch-x86` | On x86 the official Opt10 set issues **no** table prefetch: their prefetch is aarch64-only unless this feature is on (`src/eo_table.rs:484-498`; unmeasured upstream). It belongs in the screening |
| `c48rs-h7..h10` (`-nat`) | default / native | | default | H48 backend (`solve-h48 N`) on nissy files |

The loader-only shim of §2 is a small patch to cube48opt-rust's table loader; it is not published.

## 2. Tables

| row | file | how made | identity |
|---|---|---|---|
| opt7 | `$D/opt7/h48prun31h7.dat` (3,891,363,840 B) + `$D/opt7/small-memory.bin` | their documented `cube48opt-research init --variant 7 --threads 8` (the pinned cube48opt7.wasm generator via Node 26): 177 s wall, 4.0 GB peak | SHA-256 b9032e3c… = their published Opt7 hash |
| opt8 | `$D/opt8/h48prun31h8.dat` (7,782,727,680 B) + `$D/opt8/small-memory.bin` | `init --variant 8 --threads 48` (one exclusive hold): 365 s wall, 8.3 GB peak | SHA-256 0c742e0a… = their published Opt8 hash |
| opt10 | cube48opt10's raw `h48prun31h1.dat` (31,130,910,720 B) + side header `$D/opt10/h48prun31h1.header` (1 KiB) + `$D/opt9-small/small-memory.bin` | the comparison's cube48opt10 table, unchanged on disk | see below |
| h7, h8, h9, h10 (H48 backend) | nissy's `h48hN` files | the comparison's nissy tables | their loader checks the exact size, the three headers and nissy's pinned code distribution. The comparison's h48h7 passes |

Why not our tables:
- Our h7/h8/h10 have the same sizes and the same 16-byte-group layout, but they are **not byte-identical** to cube48opt's: the index order differs. Our h7 XOR is 0x99293616, cube48opt's Opt7 is 0xe6843b35. The comparison's [byte check](../../data/comparison/results.md#cube48opt10-s-table-vs-egg-solver-h10) found the same for h10 vs cube48opt10.
- So Opt7/Opt8 were generated with their own generator.

**Opt10: same bytes, different load path (a labelled deviation).**
- cube48opt-rust's Opt10 file format is a 1 KiB header plus a 31,130,910,720-byte payload (`src/eo_table.rs:17-147`).
- Streaming cube48opt10's raw `h48prun31h1.dat` and setting every 4-bit block minimum above the base to the base changed 1,267,916,448 blocks, all of them the sentinel 15, which is never read: such a block has no code-0 cell. The result has SHA-256 `2ce3d57dbefc8d4fb557568398c280ebbd827f5e19125c1cdcc9d5f79032c3e8`, **equal to their published official Opt10 payload** (`opt10-development.md:39`).
- So their own Opt10 table is cube48opt10's table with the unread minima written as 10 (their generator's initial value, `eo_table.rs:166-176`).
- Running it as shipped would need a 31 GB copy with a header. Instead, the shim build reads the header from `CUBEOPT_EO_HEADER` (a side file) and the payload from the raw file. It rewrites those minimum nibbles in memory before their own unchanged checks: payload SHA-256 against the header (2ce3d57d…), the full distribution and the snapshot hash.
- **Every load therefore re-proves byte identity with their own table.**
- **Search code is byte-identical.** Both builds were disassembled with addresses and RIP offsets normalized: 959 of 962 functions are identical, including every `search::*`, `native_parallel::*` and Opt10 oracle function. Only `Opt10Solver::open`, `Opt11Solver::open` (the inlined loader) and `run` differ. The same holds for the `-pf` pair. Code addresses shift, as they do in any rebuild.
- The side header was made by a small script (not published); it holds the distribution, the payload SHA and the Opt9 snapshot SHA.
- Their own generator (`generate-opt10`) is the fallback, but it needs 31.4 GB RAM (1G pool) for ~1 h at 16 threads. It's not needed, since the bytes are proven equal.

## 3. Run

CLI and output (`src/main.rs`):
- **Input:** stdin, one `id<TAB>scramble` per line. The id is `[A-Za-z0-9_-]+`; moves are `U U2 U'`. The harness's `R1/R3` notation needs `1→""`, `3→'`.
- **Command:**
  - Opt7/8: `CUBEOPT_VARIANT=N solve SMALL PDB 8 20 0`.
  - Opt10: `CUBEOPT_BACKEND=opt10 [CUBEOPT_EO_HEADER=…] solve-opt10 OPT9_SMALL TABLE 8 20 0`.
  - H48: `solve-h48 N FILE 8 20 0`.
  - The three numbers are 8 interleaved contexts per thread, max depth 20, and **0 = no node limit**. The default is 50M nodes, after which it gives up with `cancelled:true`. Parallel search requires 0.
- **Threads:**
  - `CUBEOPT_SEARCH_THREADS=t` is one cube at a time on t threads (latency). It is **capped at 18** (`native_parallel.rs:13`). Workers are spawned per cube and the depths are barrier-synchronized.
  - `CUBEOPT_SOLVE_THREADS=t` is t cubes at once, one per thread, up to 256 (throughput).
  - The two can't both be >1. Neither sets CPU affinity.
- **Output:** one JSON line per cube. `seconds` is measured around the solve only (`main.rs:323-325`); the table load is excluded. There are also `length`, `solution`, `nodes`, `probes` and other fields, and the solution is replayed internally.
- **Node:** every search-frame state it inspects, counted before its bound check. This includes depth-0 leaves and states whose inverse bound came from the half-turn cache (`search.rs:1366`). It is close to "children generated" (ours), not to "lookups": they report ~1.2 probes/node.
- **SIMD:** `CUBEOPT_SIMD=auto` picks AVX2 even when AVX-512 is present, by upstream choice. On this Zen 5, AVX-512 is the same speed at 1T: Opt7, 5 cubes, 73.29 vs 73.15 ns/node. Keep auto.
- **Load:**
  - Opt7/8 read the whole table into a `Vec<u32>` and, on the first run, validate the checksum and distribution. A `<table>.verified` file next to the table then skips validation; the folder is writable.
  - Opt10 reads 31 GB from disk and SHA-256s it on every run: **about 4 min per run**, outside `seconds`.
  - H48 mmaps the nissy file read-only and scans its distribution on every start.

**Harness function:** a `c48rs` function for the comparison harness's [`runtest.sh`](../../reproduce/harness/runtest.sh):
- It adds `c48rs <dataset> <v> [batch]` and the names `c48rs<v>` (latency) and `c48rsbatch<v>` (throughput), for `<v>` = `opt7|opt8|opt9|opt10|h7..h10`.
- It prints the harness's `NNN x.xxxM nodes, y ns/node` per-cube lines and `Avg …, tt=…`. tt is the mean per-cube seconds for latency (like h48/cubeopt), and Σseconds/cubes/threads for batch (like nxopt/vcube).
- Env: `c48rs_build` = `nat` | `pgo` | `pf` | `pfnat` (binary suffix), `c48rs_threads` (search threads instead of min(nthread, 18)), `c48rs_simd`, `c48rs_dir`, `c48rs_opt10`, `c48rs_h48`.
- **Length cross-check:** the raw log has JSON lines `{"id":"<cube index>", … "length":N, …}`. The harness's raw-log parser needs a JSON case: id = cube index from 0, length = `"length"`.

## 4. Pages (natively and through hugeshim)

| backend | native page type here (THP=madvise) | THP column | 1G column |
|---|---|---|---|
| Opt7/8/9/10 (`Vec<u32>` by malloc) | **4 KiB**. The only madvise is an off-by-default Opt8 feature, which upstream measured and kept off | `hugerun.sh thp` works: Opt7 3,799,040 of 3,805,624 KB AnonHugePages; Opt8 7,600,128 of 7,605,832 KB | `hugerun.sh hugetlb` with `HUGESHIM_ARENA` works: Opt7 hugetlb 4,194,304 KB, Opt10 30,408,704 KB |
| H48 backend (`mmap(PROT_READ, MAP_PRIVATE)` of the file) | 4 KiB page cache | not possible with hugeshim: a file mapping, not anonymous | not possible (exact-size check; hugetlbfs files are page-rounded) |

Arena settings: `HUGESHIM_ARENA_MIN=1073741824` and `HUGESHIM_ONLY=<comm>`, where comm is the binary name cut to 15 characters (e.g. `c48rs-opt7-pfna`). Arena sizes:

| table | HUGESHIM_ARENA |
|---|---|
| Opt7 | 4 GiB |
| Opt8 | 8 GiB |
| Opt10 | 30 GiB (tested). The Vec is 28.99 GiB, so 29 should fit, but that's untested |

1T on 5 random_state cubes (default build): THP vs 4K is Opt7 −8.0% ns/node (67.4 vs 73.2) and Opt8 −15.8% (63.6 vs 75.5). So the shipped 4K page type is clearly slower here.

## 5. Smoke results (all lengths equal egg-solver's, 181 + 5 cubes, 0 mismatches)

| run | cubes | pages | build | nodes/cube | ns/node | tt (s/cube) |
|---|---:|---|---|---:|---:|---:|
| opt7 1T | 5 | 4K | default | 56.76M | 73.15 | 4.152 |
| opt7 1T | 5 | 4K | nat | 56.76M | 71.22 | 4.042 |
| opt7 1T | 5 | THP | default | 56.76M | 67.36 | 3.823 |
| opt8 1T | 5 | 4K | default | 16.32M | 75.51 | 1.232 |
| opt8 1T | 5 | THP | default | 16.32M | 63.59 | 1.038 |
| h7 (H48 backend, nissy h48h7) 1T | 5 | 4K | default | 50.98M | 118.46 | 6.039 |
| opt7 latency, 18 threads | 10 | THP | default | 43.28M | 4.20 | 0.1820 |
| opt7 latency, 17 threads | 10 | THP | default | 41.70M | 4.27 | 0.1782 |
| opt7 latency, 16 threads | 10 | THP | default | 44.07M | 4.50 | 0.1981 |
| opt7 latency, 18 threads | 10 | 1G | default | 43.38M | 4.15 | 0.1801 |
| opt7 batch 48 | 48 | THP | default | 51.72M | 1.37 | 0.0709 |
| opt8 latency, 18 threads | 10 | THP | default | 13.96M | 4.48 | 0.0626 |
| opt8 batch 48 | 48 | THP | default | 17.57M | 1.45 | 0.0255 |
| opt10 latency, 18 threads | 5 | 1G | default (shim) | 3.52M | 5.55 | 0.0196 |

## 6. Fairness

1. **Page type.**
   - Natively the Opt backends get 4 KiB pages here: a malloc'd Vec with no madvise.
   - The H48 backend gets 4 KiB page cache (a file mmap).
   - For Opt7/8/10 both columns work through the harness's hugeshim: `thp` mode (glibc.malloc.hugetlb=1 madvises the Vec's mmapped chunk) and `hugetlb` mode with an arena (Rust's System allocator calls malloc through the PLT, so the shim's malloc takes it).
   - Measured placements are in §4. Treat them like nxopt/nissy: THP = hugeshim thp, 1G = hugeshim arena.
   - The H48 backend can only run on 4K, so it has no row in a same-page comparison. Use it only as a note: same table as nissy's h48h7 run, their search; 1T 118 ns/node on 4K.
2. **Builds.** Screen shipped vs native vs PGO, as for the other solvers; all are built (§1). For Opt7 and Opt10 also screen the `pf` variants: their own optional x86 prefetch features, unmeasured upstream and off in their official sets. The 48T screening belongs to the comparison runs.
3. **PGO.**
   - Built with a PGO script (`pgo-c48.sh`, not published): `-Cprofile-generate` on the native build, then training with the harness's PGO training cubes, `dataset/train_pgo.txt` (random_state lines 301-400, disjoint from the evaluated cubes):
     - 16 cubes with `CUBEOPT_SEARCH_THREADS=8`;
     - 4 cubes single-thread;
     - 16 cubes batch-8;
     - all on the same table as the evaluation.
   - Then `llvm-profdata merge` (rustup's llvm-tools) and `-Cprofile-use`.
   - Opt10 PGO is not built; training needs the 1 GiB-page slot (about 5 min plus a 4 min load).
4. **Threads.**
   - Latency rows can't use more than 18 threads (`MAX_SEARCH_THREADS`, `native_parallel.rs:13`). Label their 48T latency rows "18 threads (its cap)".
   - In the Opt7 build (generic oracle, static prefix shards `key % threads`), 17 threads were not slower than 18 in the smoke run: 0.178 vs 0.182 s/cube on 10 cubes, i.e. noise-level. 16 was slower (0.198). Use 18.
   - Opt8 and Opt10 use dynamic prefix claims (their features `opt8-native-search` / `opt10-native-search`).
   - Throughput rows use `CUBEOPT_SOLVE_THREADS=48` (one cube per thread), as nxopt/vcube do.
5. **Timing.** Per-cube `seconds` excludes the table load and includes per-cube thread spawn and depth barriers. That is its own definition, as for h48/cubeopt.
6. **Lengths:** every one of the 186 smoke cubes (latency, batch, 1T, 4K/THP/1G, Opt7/8/10, H48 backend) has egg-solver's length. The first solutions are even the same move sequences.

### Build screening

A 1T screening (THP, 5 random_state cubes, 2 rounds, rotated pairs in two CCD slots) was planned here; its table was never filled in on this page. The comparison's own [build screening](../../data/comparison/results.md#build-screening-shipped-flags-vs-o3-march-native-and-vs-pgo) covers these builds and selected Opt7 `pfpgo`, Opt8 `native` and Opt10 `pfnative`.

## 7. Comparison rows

Format: `name threads dataset n pages build repeat`. All rows below use `build` = the screening winner (written B), plus `shipped` for the record.

```
# latency, ~3.9 GB (vs egg-solver h7, 1.00x)
c48rsopt7        48 random_state  200 1g   B 1..3      # 18 search threads (its cap)
c48rsopt7        48 random_state  200 thp  B 1..3
c48rsopt7         1 random_state   50 1g   B 1        # or 2 x 25 chunks
c48rsopt7         1 random_state   50 thp  B 1
# latency, ~7.8 GB (vs egg-solver h8, 1.00x)
c48rsopt8        48 random_state  200 1g   B 1..3
c48rsopt8        48 random_state  200 thp  B 1..3
c48rsopt8         1 random_state   50 1g   B 1
c48rsopt8         1 random_state   50 thp  B 1
# latency, large (1G only): Opt10 31.13 GB vs egg-solver h10 31.13 GB (1.00x)
c48rsopt10       48 random_state  200 1g   B 1..3
c48rsopt10        1 random_state   50 1g   B 1
# throughput (one cube per thread)
c48rsbatchopt7   48 random_state  200 1g   B 1..3
c48rsbatchopt7   48 random_state  200 thp  B 1..3
c48rsbatchopt8   48 random_state  200 1g   B 1..3
c48rsbatchopt8   48 random_state  200 thp  B 1..3
c48rsbatchopt10  48 random_state  200 1g   B 1..3
# (1T throughput = the 1T latency rows: one cube on one thread)
# wiki rows (1T, nodes): 15f/16f/17f (100) and inf (50)
c48rsopt7         1 random_move_15f 100 thp B 1
c48rsopt7         1 random_move_16f 100 thp B 1
c48rsopt7         1 random_move_17f 100 thp B 1
c48rsopt7         1 random_state_from50 50 thp B 1
(same four for c48rsopt8 thp, and for c48rsopt10 1g)
```

The comparison's driver script (`final-compare.sh`) needs these entries (the names follow the harness function; `$D`, `$CUBE48OPT10` and `$NISSY` stand for local directories):

```
capfor:    c48rs*opt7) 7G ;; c48rs*opt8) 11G ;; c48rs*opt10) 8G ;; c48rs*h7) 6G ;; c48rs*h8) 10G ;;
           # (thp/4K: the table is anon memory; 1g: hugetlb is not counted, but the 31 GB Opt10
           # read goes through the page cache inside the cgroup, which it reclaims: 8G worked)
tablefile: c48rs*opt7) $D/opt7/h48prun31h7.dat ;;
           c48rs*opt8) $D/opt8/h48prun31h8.dat ;;
           c48rs*opt10) $CUBE48OPT10/h48prun31h1.dat ;;
           c48rs*h[0-9]*) $NISSY/h48h${1##*h} ;;
pagemode:  c48rs*: 1g -> hugetlb, thp -> thp (as nxopt/nissy)
arena_for: c48rs*opt7) "$((4 * g)) $g c48rs-opt7" ;; c48rs*opt8) "$((8 * g)) $g c48rs-opt8" ;;
           c48rs*opt10) "$((30 * g)) $g c48rs-opt10" ;;
           # HUGESHIM_ONLY must name the binary actually run: c48rs-opt7, c48rs-opt7-nat,
           # c48rs-opt7-pgo, c48rs-opt7-pfna (15-char comm of -pfnat) ...; the harness's inner() appends "-pgo"
           # already, add "-nat" and "-pfna" the same way
build:     native -> c48rs_build=nat; pgo -> c48rs_build=pgo; pf -> pf; pfnative -> pfnat
```

**Expected run times** (smoke-based; THP/1G):

| row | per cube | 200 cubes (48T) | 50 cubes (1T) | load per run |
|---|---:|---:|---:|---|
| Opt7 latency | 0.18 s (18 threads) | ~40 s | ~3.5 min (3.8 s/cube) | ~5-10 s (3.9 GB from the data disk; the first run per table validates, ~10 s more) |
| Opt8 latency | 0.063 s | ~15 s | ~1 min (1.0 s/cube) | ~10-15 s |
| Opt10 latency | 0.02 s | ~5 s | ~30 s (est.) | **~4 min**: a 31 GB read from disk plus SHA-256 and distribution, every run (the page cache can't hold it) |
| Opt7 / Opt8 batch | 0.071 / 0.025 s per cube of throughput | ~15 s / ~5 s | | as above |
| wiki 15f/16f/17f (1T, 100 cubes) | Opt7 ~0.1-0.5 s, Opt8 less | | ~1-3 min per row | |
