# User guide

[Documentation](../README.md) · [Quick start](../guide/QUICKSTART.md) · [Benchmarks](../comparisons/README.md)

Commands below run from the project root. The examples use the executables under `target/release/`. Every binary has `--help`; use it for the complete option list.

## Build

```sh
cargo build --release --locked
```

The default x86 release targets the build machine's CPU. Build on the machine that will run the program, or choose a [portable target](#portable-builds). The library can be built without the CLI dependencies using `--no-default-features`.

<a id="running-on-a-laptop"></a>

## Choose and generate a table

See [table sizes](../guide/OVERVIEW.md#tables-and-memory). Choose a table that fits available RAM with room for the process and operating system. Generation and solve speed depend on CPU, memory and storage; server measurements are not laptop performance predictions.

```sh
mkdir -p tables
target/release/gen 7 auto tables/h7
target/release/gen check tables/h7
```

`auto` chooses the program's recommended compression base. The base changes pruning bounds, not data capacity. Generate once and reuse the file. The `--parts N` option generates row ranges sequentially to reduce peak memory in depth-first generation; it does not apply to h11's BFS generation. Memory and disk preflight checks run before generation; on hugetlbfs the disk check counts free huge pages, and `--force` skips the checks.

## Solve scrambles

```sh
# One cube, using all selected threads:
target/release/solve --fast --table tables/h7 "R U R' U' F2"
# The same scramble from stdin:
printf '%s\n' "R U R' U' F2" | target/release/solve --fast --table tables/h7
# Batch: one scramble per input line:
target/release/solve --table tables/h7 < testdata/random_state.txt
# Complete one cube before starting the next:
target/release/solve --table tables/h7 -c 1 < testdata/random_state.txt
```

`--fast` requires an explicit `--table`. It does not search directories for tables. With no scramble arguments it reads exactly one nonblank stdin line. Empty input is an error, not a request for a random cube. An explicitly supplied empty scramble argument represents the solved cube. Warm-up is optional: `--warmup SCRAMBLE` adds one solve before the measured solve.

| Option | Meaning |
|---|---|
| `-t N` / `--threads N` | Worker threads; default is the program's detected thread count |
| `-l N` / `--lanes N` | Interleaved contexts per worker; otherwise use a matching tune record or table-dependent default |
| `-c 0` | Allow the complete input batch in progress; the default |
| `-c 1` | Complete one input before starting the next; workers still cooperate within a cube |
| `-n N` | Read at most N scrambles in batch mode |
| `--json` | Per-cube JSON and a batch total; `--fast` emits one JSON result |
| `-v` | Loading and configuration diagnostics on stderr |

Batch mode also accepts a positional table path. `EGG_SOLVER_TABLE` can supply it in batch mode, and `EGG_SOLVER_THREADS` can supply the thread count. `--fast` still requires a command-line `--table`. Each `EGG_SOLVER_*` setting also accepts the matching `CUBEOPT_*` name from the project's earlier name as a fallback; an explicitly set `EGG_SOLVER_*` name takes precedence.

Search timing excludes table loading. The batch total measures completion of the whole batch; individual cube durations can overlap. See [timing definitions](../comparisons/METHODS.md) before interpreting them as throughput.

## Tune and benchmark

```sh
target/release/bench tables/h7 tune
target/release/bench --help
```

The tune command writes a `<TABLE>.tune` record for its measured configurations. A table on hugetlbfs (any mount, not only `/mnt/huge`) cannot hold small files, so its record goes to `tables/<NAME>.hugemeta.tune` in the build checkout, beside the optional `tables/<NAME>.hugemeta` metadata sidecar; rebuild on each machine so that path is local, and keep table names distinct across hugetlbfs mounts. `solve` and `bench` use a matching record unless `-l` is explicit. The built-in lane candidates end at 48, but explicit larger lane values are supported. Tuning a stream does not guarantee the best finite-batch or single-cube setting. The [measured lane study](../performance/PARALLELISM.md) explains that distinction.

The historical positional benchmark syntax remains supported. Use `bench --help` and each subcommand's help for named syntax, warm-up and measurement windows. Record the actual command with benchmark output.

## Table files

A table has pruning data and a 4 KiB metadata trailer containing k, base, distribution and checksum. Padding can separate data and trailer. `gen check TABLE` verifies stored integrity; a checksum alone does not prove that every bound is admissible.

| Load mode | Storage used during search | Notes |
|---|---|---|
| `--load copy` | Private RAM populated before searching | Does not keep reading the original file for lookups; still requires enough physical RAM |
| `--load map` | Read-only file mapping | Resident hugetlbfs data is RAM-backed; an ordinary file mapping can fault pages from disk |
| `--load shm` / `--shm` | Linux System V shared-memory segment | Reuse an existing segment or create one from the file |
| `--load auto` | Existing matching shared segment; otherwise the platform default | On Linux: map RAM-backed files, copy ordinary files; otherwise follow the platform default |

`EGG_SOLVER_LOAD` sets the mode. Mapping and shared-memory storage must remain unchanged while the solver uses it. Prefaulting touches mapped/shared pages at startup; `--prefault 0` defers that work until first use. Prefaulting is not a guarantee that an ordinary mapping can never be paged out.

The 1 GiB-page benchmark policy describes those recorded experiments. It is not a requirement to reserve huge pages for the small quick-start example.

Table files and shared-memory segments created before the project was renamed remain usable; the pruning data does not need to be regenerated.

## Shared memory on Linux

```sh
target/release/solve --shm-load tables/h7
target/release/solve --shm --table tables/h7 < testdata/random_state.txt
target/release/solve --shm-remove tables/h7
```

A segment persists until removal or reboot. An existing matching segment avoids reloading the table into a fresh private allocation. Metadata is checked on attachment; a mismatched segment is rejected by explicit shared mode. Keep the segment and table immutable during use.

A segment holds the table file's bytes, with the file's trailer (k, base, size, distribution and checksum) in its last 4 KiB. The trailer is written last, after the copied data's checksum is checked, so a segment without one is still loading or was interrupted. `--load auto` passes over a segment that does not match the table file and loads the file instead. A loaded segment is read-only: `ipcs -m` shows perms `400` (`600` while loading).

The segment key is the ASCII bytes "co" (`0x636f`, a legacy prefix kept for compatibility), then the table's k and base, one byte each: `ipcs -m` shows h7 base 9 as `0x636f0709` and h11 base 10 as `0x636f0b0a`. `--shm-key` (or `EGG_SOLVER_SHM_KEY`) selects another key, for example for two tables with the same k and base.

`--shm-pages 1g` requests only 1 GiB pages and fails if unavailable. The default page preference is `1g,2m,4k`; `-v` reports the selected size and fallback reasons. Huge-page allocation needs both permission (the configured huge-page group or `CAP_IPC_LOCK`) and enough free reserved pages. The program's preflight reports relevant limits: `kernel.shmmax`, `kernel.shmall` and `kernel.shmmni`, then the huge-page permission and free pool pages, or free memory for 4 KiB pages. Attaching an existing segment needs no free memory.

On a typical Linux host, the one-time root setup for huge-page segments looks like this; adapt it to the host and the loader's diagnostic:

```sh
# Permission: a group of yours as the huge-page shared-memory group
sudo sysctl vm.hugetlb_shm_group=$(id -g)
# Free pages: reserve N 1 GiB pages (or use hugepages-2048kB for 2 MiB pages)
echo N | sudo tee /sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages
```

A file in `/etc/sysctl.d` keeps the group setting after reboot. 1 GiB pages are most reliably reserved at boot with the kernel parameters `hugepagesz=1G hugepages=N`.

`ipcs -m` lists segments. Removal releases memory after the last process detaches. These system settings depend on the host; read the loader's diagnostic before changing them.

## Portable builds

On x86, replace the native target flags to build for another CPU:

```sh
RUSTFLAGS="-C target-cpu=x86-64-v4" cargo build --release --locked
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release --locked
RUSTFLAGS="-C target-cpu=x86-64" cargo build --release --locked
cargo build --release --locked --target aarch64-apple-darwin
```

The targets above require AVX-512-class features, AVX2/BMI2-class features, baseline x86-64, or Apple Silicon respectively. Cross-compilation also needs the target toolchain and linker. The portable fallback is available through the library's `scalar` feature for correctness/backend checks.

## WebAssembly

```sh
rustup target add wasm32-wasip1 wasm32-wasip1-threads
wasm/build.sh
```

This creates `wasm/dist/egg-solver.wasm` and `egg-solver-threads.wasm`. The wrapper supplies the required WASI interface for browsers and Bun:

```js
import { EggSolver } from './wasm/egg-solver.mjs';
const solver = await EggSolver.create({ wasm: 'wasm/dist/egg-solver.wasm' });
await solver.loadTable('h6');
// Or generate a small table with the recommended base:
// await solver.generateTable(4);
solver.solve("R U R' U' F2");
```

`solve` puts the selected workers on one cube and returns `{ moves, length, nodes, lookups, seconds }`; `solveMany` uses independent cube lanes. `loadTable` accepts a path (Bun), a URL, a `File` or `Blob`, or the file's bytes. Inputs can be scrambles or 54-sticker facelet strings in U R F D L B face order, each face viewed from outside and read row by row (U with B at the top, D with F at the top, the side faces with U at the top); any six symbols can stand for the colors. The wrapper's facelet parser checks the representation.

### Bun

```sh
bun wasm/demo.mjs --table tables/h6 < testdata/random_state.txt
bun wasm/demo.mjs --gen 4:8 --random 5:1
bun wasm/demo.mjs --table tables/h6 --threads 8 --batch < scrambles.txt
```

The second command generates h4 with base 8 in memory and solves five random states.

### Browser

```sh
bun wasm/serve.mjs --tables tables
```

Open the local URL printed by the server, or the public site, <https://egg-solver.arhan.sh/>. On load the page starts with every core the browser reports and generates h0, the smallest table, so **Solve** works immediately. The page shows only the scramble box, **Solve** and **Random cube**; a status line reports progress. The collapsed **Settings** section holds the thread count and table choice (**Apply** restarts only if the thread count changed), loading a table file or URL, and lanes per thread. The page can also generate h1–h4 or load a compatible table. Its recommended h4 option uses base 9; the explicit base-8 option trades weaker search pruning for faster generation. [The h4 study](../comparisons/TABLE_ANALYSIS.md) records that tradeoff.

### Tables

The wasm32 build supports h0–h6. Each Rust allocation must fit below `isize::MAX` (2 GiB), so h7 is rejected even though wasm32 has a larger overall address space. The table and the module's other allocations must also fit the host's memory limits. Large tables are normally generated with native `gen` and then loaded.

### Threads

Browser threads need `SharedArrayBuffer` and cross-origin isolation. The local server supplies the required COOP/COEP headers, and the public site, egg-solver.arhan.sh, receives them from Cloudflare because GitHub Pages cannot set headers. The solver runs in a Web Worker because its thread waits cannot run on a browser's main thread. Without isolation, such as on a plain `github.io` deployment, the page uses one thread. Bun supports the threaded wrapper without those HTTP headers.

The wrapper initializes the module and worker pool. Other hosts must perform the module's required initialization before calling its C ABI. See `wasm/src/lib.rs` and the JavaScript wrapper when embedding it directly.

### Tests

`scripts/check.sh` runs the library tests in WebAssembly under wasmtime and `wasm/test.mjs` (both modules in Bun) when those tools are installed. The wasmtime runner needs wasi-threads support (wasmtime 44 or older; set `WASMTIME=path` to choose a binary). Missing tools are reported, not counted as passing.

### Performance records

Historical V8/Bun timings, generation times and engine warm-up experiments are kept under [Historical measurements](#speed). They are not predictions for a current browser or laptop. [Performance studies](../performance/README.md) indexes the measured native and WebAssembly evidence.

## Library use

`Cube::from_bytes`, symmetry-coordinate lookups and `cube_from_edge_key` return `Option` for invalid input. Table and solver internals are exposed through read-only accessors. `H48Table::from_memory` checks the metadata and allocation size; `get` returns `None` for an invalid index.

`H48Table::open_copy` loads the table into private RAM and is safe. APIs that map files or attach shared segments are `unsafe`: the backing storage must remain unchanged and untruncated while the table exists, including changes by other processes. The command-line mapped and shared modes require the same condition; `--load copy` avoids that dependence after loading. File-backed generation requires exclusive access to its output paths. [Safety](../safety/README.md) describes the undefined-behavior evidence and its scope.

The packed-coordinate and table encodings require a little-endian target; big-endian builds are rejected at compile time.

## Historical measurements

These records were measured during development on the project's Zen 5 server and were previously kept in an earlier edition of this guide (dated 2026-11-07). Each block links the record of the commit that introduced the figures. Later source changes, lane defaults and engines can give different results; they are not predictions for another machine.

<details>
<summary>Native table sizes, generation times and solve rates (recorded 2026-11-04)</summary>

| RAM     | table | size     | generation | cubes/s, 1 core | cubes/s, 8 cores |
|---------|-------|---------:|-----------:|----------------:|-----------------:|
| 4 GB    | h6    | 1.8 GiB  | 40 s       | 1.2             | 9.3              |
| 8 GB    | h7    | 3.6 GiB  | 1 min      | 2.2             | 17               |
| 16 GB   | h8    | 7.3 GiB  | 5.5 min    | 4.0             | 29               |
| 32 GB   | h9    | 14.5 GiB | ~7 min     | ~7.5 (est.)     | ~55 (est.)       |
| 64 GB   | h10   | 29 GiB   | ~9 min     | ~14 (est.)      | ~105 (est.)      |
| 96+ GB  | h11   | 58 GiB   | ~25 min    | 25.6            | 197              |

Measured on 8 cores of a Zen 5 server (one L3 domain): generation including writing the file to a slow disk (~140 MB/s), and solves of random positions per second (an average: the hardest positions take several times longer than typical ones). h9 and h10 are estimates: their speeds follow the measured h8 -> h11 trend, and their generation costs about h8's plus the writing (h9 was generated in 4 parts, in 13 minutes). h11 is generated differently (by BFS), needs its 58 GiB of RAM to generate too, and took 4.4 minutes on 48 cores. Generation needed about the table's size in RAM plus 0.3 GB; `--parts 2` halved that, at up to twice the time. A table that does not fit in RAM is read from disk on every miss, hundreds of times slower. Source: [d4125fd](https://github.com/ArhanChaudhary/egg-solver/commit/d4125fd510afd197cc495d22f43ab1f257644d90).

</details>

<details>
<summary>Shared-memory attach time and page size (recorded 2026-11-05)</summary>

h7 attached in 0.00 s on 1 GiB pages, against 28 s to copy it from a ~140 MB/s disk, and solved as fast as from a hugetlbfs map of the same table. On 4 KiB pages the search was much slower (h7: 27% fewer solves/s at 1 thread, 32% at 48 threads, than on 1 GiB pages; attaching mapped 0.95 million pages in 0.6 s). The solver warns when a segment is on 4 KiB pages. Source: [6ae0bb2](https://github.com/ArhanChaudhary/egg-solver/commit/6ae0bb2388e3dd928bb2ad50ffe99d0de7d727e7).

```text
$ solve --shm-load /mnt/huge/h7
segment 0x636f0709 shmid 42: h7 base 9, 4.00 GiB on 1 GiB pages, made from /mnt/huge/h7 in 0.10s
$ ipcs -m
------ Shared Memory Segments --------
key        shmid      owner      perms      bytes      nattch     status
0x636f0709 42         super      400        4294967296 0
```

</details>

<details>
<summary>WebAssembly timings (recorded 2026-11-04 and 2026-11-05)</summary>

<a id="speed"></a>

Time per random-state cube, one at a time, mean and median of 10 cubes (h4: 5), on Zen 5 cores (8 threads: one L3 domain), in V8 (Chrome's engine; measured in Node 26) and in Bun (JavaScriptCore, Safari's engine):

| table     | size     | runs in                     | made by                    | 1 thread: V8     | 1 thread: Bun    | 8 threads: V8    | 8 threads: Bun   |
|-----------|---------:|-----------------------------|----------------------------|------------------|------------------|------------------|------------------|
| h0..h3    | ≤ 0.2 GiB| any browser                 | the module: h0 in 2.5 s    | short scrambles  |                  |                  |                  |
| h4 base 8 | 0.45 GiB | any browser                 | the module: 75 s (8 threads: 13 s) | 21.6 s, 20.7 s |          |                  |                  |
| h5        | 0.9 GiB  | desktop browsers            | `gen 5 auto`               | 19.4 s, 10.3 s   |                  |                  |                  |
| h6        | 1.8 GiB  | desktop browsers            | `gen 6 auto`               | 12.6 s, 5.0 s    | 12.2 s, 4.9 s    | 1.6 s, 0.65 s    | 1.6 s, 0.65 s    |

Loading from a file took seconds (from a local server into headless Chromium: h6 3-13 s; `verify` adds a checksum pass).

100 depth-17 cubes on h6 (14.8M nodes each), one thread, `solveMany` / `bench` batch mode, on one Zen 5 core:

| build                              | time    | Mnodes/s |
|------------------------------------|--------:|---------:|
| native x86 (AVX-512, `simd.rs`)    | 21.7 s  | 68       |
| wasm in wasmtime 44                | 122 s   | 12.2     |
| wasm in V8 (Node 26)               | 124 s   | 11.9     |
| wasm in Bun 1.4.4 (JavaScriptCore) | 127 s   | 11.7     |

The WebAssembly build was about 5.7 times slower than the native x86 build in this record, with the same solutions and node counts. NEON was only run under qemu, which says nothing about speed. Two things mattered for WebAssembly speed:

- WebAssembly has no prefetch instruction: `prefetch::read_once` is a plain load there, which starts the cache miss just as well (+53% in wasmtime, +45% in V8). Lanes still paid off: 1 lane 200.6 s, 8 lanes 123.8 s, flat up to 64; the module used 16.
- V8 (Chrome) starts new code in its baseline compiler and gives a function optimized code only from its next call: a long first solve would run slowly to its end (187.7 s instead of 129.5 s). `loadTable` and `generateTable` therefore run a short warm-up (`warmUp`, then ~0.6 s), and all exports share one copy of the search loop. JavaScriptCore (Bun, Safari) moves a running loop to optimized code: in Bun the warm-up is skipped (126.5 and 127.2 s without it, 127.0 and 127.9 s with it).

With 8 threads (`egg-solver-threads.wasm`, one L3 domain), one cube at a time got 7.2-7.8 times the one-thread speed (h6: 88.6 Mnodes/s in V8, 90.7 in Bun; in headless Chromium 0.63-0.66 s instead of 5.15 s for the same cube), and generating h4 base 8 took 13 s instead of 75 s.

Node counts matched the native ones (h5, 20 random states: identical nodes, lookups and lengths natively, in wasmtime and in V8; h6, 100 depth-17 cubes: the same in Bun). Sources: [db0b8ac](https://github.com/ArhanChaudhary/egg-solver/commit/db0b8ac510361f067f8c08497a63e50c66d7061f) and [cc43b77](https://github.com/ArhanChaudhary/egg-solver/commit/cc43b7779fc239e7d5b6f6252ff4b33fcb584280).

</details>

## Development and publishing

[Development checks](../guide/DEVELOPMENT.md) covers checks, Miri and stress runs. [Publishing](../guide/PUBLISHING.md) describes the Pages workflow and local site build. Shell completions are available through `--completions` (bash, zsh, fish, elvish and powershell).
