# egg-solver

An optimal 3x3x3 Rubik's cube solver: IDA* with an H48 pruning table
(`hK`, K = 0..11: the larger K, the faster the solves and the more memory),
interleaved lanes to hide table misses, and SIMD cube operations (x86 AVX2 /
AVX-512, aarch64 NEON, or a portable scalar fallback).

## Running on a laptop

The fastest table, h11, needs 58 GiB of RAM. Smaller tables give the same
(optimal) solutions, just more slowly: each step down halves the memory and
roughly doubles the time per cube. Pick the largest table that fits in your
RAM with a few GB to spare:

| RAM     | table | size     | generation | cubes/s, 1 core | cubes/s, 8 cores |
|---------|-------|---------:|-----------:|----------------:|-----------------:|
| 4 GB    | h6    | 1.8 GiB  | 40 s       | 1.2             | 9.3              |
| 8 GB    | h7    | 3.6 GiB  | 1 min      | 2.2             | 17               |
| 16 GB   | h8    | 7.3 GiB  | 5.5 min    | 4.0             | 29               |
| 32 GB   | h9    | 14.5 GiB | ~7 min     | ~7.5 (est.)     | ~55 (est.)       |
| 64 GB   | h10   | 29 GiB   | ~9 min     | ~14 (est.)      | ~105 (est.)      |
| 96+ GB  | h11   | 58 GiB   | ~25 min    | 25.6            | 197              |

Measured on 8 cores of a Zen 5 server (one L3 domain, about a laptop's
worth): generation including writing the file to a slow disk (~140 MB/s),
and solves of random positions per second (an average: the hardest
positions take several times longer than typical ones). h9 and h10 are
estimates: their speeds follow the measured h8 -> h11 trend, and their
generation costs about h8's plus the writing (h9 was generated here in 4
parts, in 13 minutes). h11 is generated differently (by BFS), needs its 58
GiB of RAM to generate too, and takes 4.4 minutes on 48 cores.

An 8-12 core Apple M-series laptop should be in the same range: its cores
are about as fast on this kind of code (expect within a factor of ~1.5
either way). The table must fit in RAM: one that does not is read from
disk on every miss, hundreds of times slower.

Build (tuned for the CPU you build on; on Apple Silicon this uses NEON):

```sh
cargo build --release
```

Generate a table once (`auto` picks the best base value for the size). It
needs about the table's size in RAM plus 0.3 GB; `--parts 2` halves that,
at up to twice the time:

```sh
mkdir -p ~/cube-tables
target/release/gen 8 auto ~/cube-tables/h8
```

Before it starts, `gen` checks that the disk (on hugetlbfs: the free huge
pages) and the memory are enough, and stops with the numbers if not
(`--force` skips the checks); `solve` and `bench` check the memory before
they copy a table into it.

Optionally tune the lanes per thread for your machine (about two minutes;
saved next to the table as `h8.tune`, used by `solve` and `bench`):

```sh
target/release/bench ~/cube-tables/h8 tune
```

Solve scrambles from stdin, one per line:

```sh
echo "R U R' U' F2 D L2" | target/release/solve ~/cube-tables/h8
target/release/solve ~/cube-tables/h8 < testdata/random_state.txt
```

The default solves the whole input as a batch on all cores (highest
throughput); `solve -c 1` solves one cube at a time with all cores (lowest
latency), `-t` sets the threads.

Solve one supplied scramble with all selected threads. `--table` is required;
no directories are searched. With no scramble arguments, exactly one nonblank
line is read from stdin. Empty input is an error before loading a table.

```sh
target/release/solve --fast --table tables/h7 "R U R' U' F2"
printf '%s\n' "R U R' U' F2" | target/release/solve --fast --table tables/h7
```

Warm-up solves are disabled by default. `--warmup SCRAMBLE` explicitly adds
one; reported solve time excludes table loading and that optional warm-up.

Every binary has `--help` and `--version`. `--json` prints JSON lines
(`solve`: one per cube, then the total; `bench`; `gen check`). `-v` logs to
stderr how the table was loaded (map or copy, page sizes) and the threads
and lanes (`RUST_LOG=debug` for more). In batch mode the table can also come from
`EGG_SOLVER_TABLE` (with `--fast`, use `--table` explicitly), the threads from `EGG_SOLVER_THREADS`. `bench` has
subcommands (`bench stream|batch|tune <table> ...`, see `bench --help`); its
positional form (`bench <table> stream 20 30 48`) still works. Shell
completions: e.g. `solve --completions bash` (or zsh, fish, elvish,
powershell). The binaries need the default `cli` feature; the library
builds without it (`default-features = false`).

### Table files

A table file holds the table data, then a 4 KiB trailer (k, base,
distribution, checksum): `gen check <table>` verifies a table against it.
Tables are the same on every machine and backend: you can generate one on a
big machine and copy it. `--load map|copy|shm` (or `EGG_SOLVER_LOAD`) sets how
a table is loaded (map the file, copy it into memory with transparent huge
pages, or attach its shared memory segment, below; the default is the
table's segment if there is one, else copy on Linux, map elsewhere), and
`--prefault 0` (or `EGG_SOLVER_PREFAULT=0`; with map and shm) reads pages on
first use instead of the whole table at start.

### Shared memory

As with vcube's `--shm`, a table can be loaded once into a System V shared
memory segment, which stays in memory after the process exits (until reboot
or until it is removed); later runs attach it read-only and start at once
(Linux only). h7 attaches in 0.00 s on 1 GiB pages, against 28 s to copy it
from a ~140 MB/s disk, and solves as fast as from a hugetlbfs map of the
same table:

```sh
solve --shm-load tables/h8           # make the segment from the file (or check it), and exit
solve tables/h8 < scrambles          # attaches it (the default --load auto does)
solve --shm tables/h8 < scrambles    # the same; makes the segment first if there is none
solve --shm-remove tables/h8         # remove it (as ipcrm -M KEY)
```

`--shm` (the same as `--load shm`, or `EGG_SOLVER_LOAD=shm`) also works with
`bench` and `egg-solver`; `--load map` and `--load copy` never use a segment.

- **Keys:** ASCII "co" (`0x636f`), then the table's k and base, a byte each:
  `ipcs -m` lists h7 base 9 as `0x636f0709`, h11 base 10 as `0x636f0b0a`.
  `--shm-key` (or `EGG_SOLVER_SHM_KEY`) sets another, e.g. for two tables of the
  same k and base.
- **Checked on every attach:** a segment holds the file's bytes, and in its
  last 4 KiB the file's trailer (k, base, size, distribution, checksum),
  written last, once the copy's checksum is checked. Each run compares it
  with the table file's trailer: a segment of another table under the key
  (say an older table of the same k and base) is refused by `--shm`, with how
  to remove it, and passed over by `auto`, which loads the file. A segment
  whose load was interrupted has no trailer: `--shm` removes it and loads
  again (or waits while another process is loading it). The table needs a
  checksum in its trailer (`gen stamp` adds one). A loaded segment is
  read-only: `ipcs -m` shows perms `400` (`600` while loading).
- **Pages:** a new segment gets 1 GiB pages, else 2 MiB, else 4 KiB pages
  (`--shm-pages 1g,2m,4k`, or `EGG_SOLVER_SHM_PAGES`); `-v` logs which, and why
  larger ones could not be had. On 4 KiB pages the search is much slower
  (h7: 27% fewer solves/s at 1 thread, 32% at 48 threads, than on 1 GiB
  pages; attaching maps 0.95 million pages, 0.6 s), and every run warns. Huge-page segments need, once (root):
  1. the permission, which is what enables 1 GiB and 2 MiB segments: a
     group of yours in `vm.hugetlb_shm_group`, e.g. `sudo sysctl
     vm.hugetlb_shm_group=$(id -g)` (a file in `/etc/sysctl.d` keeps it after
     reboot), or CAP_IPC_LOCK. Without it the kernel refuses (EPERM; older
     kernels also allowed them within the memlock limit, `ulimit -l`: kernel
     7.0 on the test server refused even 2 MiB with an 11.8 GiB limit);
  2. free pages in the kernel's hugetlb pool: `echo N | sudo tee
     /sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages` for 1 GiB
     pages (most reliably reserved at boot: `hugepagesz=1G hugepages=N`), or
     `hugepages-2048kB` for 2 MiB pages.

  A segment on 4 KiB pages gets transparent huge pages where
  `/sys/kernel/mm/transparent_hugepage/shmem_enabled` is `advise` (not tried
  here: it is `never` on the test server).
- **Preflight:** before it copies anything, `--shm` checks the segment
  against `kernel.shmmax` (bytes per segment), `kernel.shmall` (pages in all
  segments) and `kernel.shmmni`, then the permission and the pool's free
  pages for huge pages, or free memory for 4 KiB pages (as for a copy), and
  says what to raise. A run that attaches a segment needs no free memory.
- **Persistence:** a segment stays until reboot or removal, also when no
  process uses it. `ipcs -m` lists segments; `solve --shm-remove TABLE` or
  `ipcrm -M 0x636f0709` removes one (its memory is freed when the last
  process detaches).

```text
$ solve --shm-load /mnt/huge/h7
segment 0x636f0709 shmid 42: h7 base 9, 4.00 GiB on 1 GiB pages, made from /mnt/huge/h7 in 0.10s
$ ipcs -m
------ Shared Memory Segments --------
key        shmid      owner      perms      bytes      nattch     status
0x636f0709 42         super      400        4294967296 0
```

### Portable builds

`target-cpu=native` (in `.cargo/config.toml`, x86 only) builds for the CPU
of the build machine. For binaries that run on other machines:

```sh
RUSTFLAGS="-C target-cpu=x86-64-v4" cargo build --release  # x86 with AVX-512
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release  # x86 with AVX2 + BMI2
RUSTFLAGS="-C target-cpu=x86-64" cargo build --release     # any x86-64 (portable backend)
cargo build --release --target aarch64-apple-darwin        # Apple Silicon (NEON)
cargo build --release --features scalar                    # the scalar backend, any target
```

## WebAssembly

The solver also compiles to WebAssembly (SIMD128) and runs in browsers and
in [Bun](https://bun.sh) (tested: 1.4.4): the same search on the same table
files, with the same solutions and node counts as natively, about 5.7 times
slower than the native x86 build (see "Speed").

### Build

```sh
rustup target add wasm32-wasip1 wasm32-wasip1-threads
wasm/build.sh
```

This makes `wasm/dist/egg-solver.wasm` (one thread; runs anywhere) and
`wasm/dist/egg-solver-threads.wasm` (threads, see below), ~380 KB each. Both
are WASI modules with a small C ABI (`wasm/src/lib.rs`); `wasm/egg-solver.mjs`
wraps them for JavaScript and provides the few WASI functions they import,
so neither a WASI runtime nor wasm-bindgen is needed.

```js
import { EggSolver } from './wasm/egg-solver.mjs';
const solver = await EggSolver.create({ wasm: 'wasm/dist/egg-solver.wasm' });
await solver.loadTable('h6');             // a path (Bun), URL, File or bytes
// or: await solver.generateTable(4, 8);   // h4 base 8, made in the module
solver.solve("R U R' U' F2 D L2");         // { moves, length, nodes, lookups, seconds }
solver.solve('UUFUUFUUFRRRRRRRRRFFDFFDFFDDDBDDBDDBLLLLLLLLLUBBUBBUBB');  // facelets
solver.solveMany(cubes);                   // many cubes, for throughput
```

A cube is a scramble or a facelet string: 54 stickers, face by face in the
order U R F D L B, each face row by row as seen from outside (U with B at
the top, D with F at the top, the others with U at the top), any six
symbols for the colors (as in Kociemba's solvers). `solve` puts all lanes
(and threads) on one cube; `solveMany` gives each lane its own cube.

`egg-solver.mjs` uses web APIs only (Worker, fetch, Blob, Atomics), plus Bun's
`Bun.file` for local paths, so the same file serves browsers and Bun.

### Bun

```sh
bun wasm/demo.mjs --table ~/cube-tables/h6 < testdata/random_state.txt
bun wasm/demo.mjs --gen 4:8 --random 5:1             # h4 base 8, 5 random states
bun wasm/demo.mjs --table ~/cube-tables/h6 --threads 8 < scrambles.txt
bun wasm/demo.mjs --table ~/cube-tables/h6 --batch < scrambles.txt   # throughput
```

### Browser

```sh
bun wasm/serve.mjs --tables ~/cube-tables    # then open http://localhost:8000/
```

`wasm/index.html` generates h0..h4 in the page or loads a table file (file
picker, or a URL such as `/tables/h6`), and solves scrambles, facelet
strings or random states, in plain web JavaScript (`demo-worker.mjs`,
`egg-solver.mjs`). Any static server works for one thread; `serve.mjs` (a
`Bun.serve` server) also sends the headers the threads build needs.

### Tables

wasm32 has at most 4 GiB of memory, for the table and everything else
(tens of MB; ~0.2 GB with 8 threads). Each table allocation must also fit below
Rust's `isize::MAX` limit (2 GiB on wasm32), so h0 through h6 are supported;
h7 is rejected. Small tables can be generated in the module; larger ones are made
by the native `gen` (see "Running on a laptop") and loaded from the file.
Time per random-state cube, one at a time, mean and median of 10 cubes
(h4: 5), on Zen 5 cores (8 threads: one L3 domain), in V8 (Chrome's engine;
measured in Node 26) and in Bun (JavaScriptCore, Safari's engine):

| table     | size     | runs in                     | made by                    | 1 thread: V8     | 1 thread: Bun    | 8 threads: V8    | 8 threads: Bun   |
|-----------|---------:|-----------------------------|----------------------------|------------------|------------------|------------------|------------------|
| h0..h3    | ≤ 0.2 GiB| any browser                 | the module: h0 in 2.5 s    | short scrambles  |                  |                  |                  |
| h4 base 8 | 0.45 GiB | any browser                 | the module: 75 s (8 threads: 13 s) | 21.6 s, 20.7 s |          |                  |                  |
| h5        | 0.9 GiB  | desktop browsers            | `gen 5 auto`               | 19.4 s, 10.3 s   |                  |                  |                  |
| h6        | 1.8 GiB  | desktop browsers            | `gen 6 auto`               | 12.6 s, 5.0 s    | 12.2 s, 4.9 s    | 1.6 s, 0.65 s    | 1.6 s, 0.65 s              |

Loading from a file takes seconds (from a local server into headless
Chromium: h6 3-13 s; `verify` adds a checksum pass). The 4 GiB are
the engine's limit for wasm32; a browser tab may allow less (mobile ones
much less). h8 and larger would need memory64 (64-bit WebAssembly memory),
which this build does not use.

### Threads

`egg-solver-threads.wasm` is built for `wasm32-wasip1-threads` (stable Rust):
std threads work through wasi-threads, so the work-sharing mode (`solve`,
all threads on one cube) and the stream mode (`solveMany`) run as natively.
`egg-solver.mjs` implements wasi-threads' `thread-spawn` with a pool of
workers (`egg-solver-thread.mjs`, Web Workers in browsers and in Bun) that
share the module's memory. That needs `SharedArrayBuffer`:

- In browsers, a cross-origin isolated page, served with the headers
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp` (`serve.mjs` sends them).
- The solver must run in a worker, not on the page's main thread: a solve
  waits for its threads (`Atomics.wait`), which the main thread may not do.
  `index.html` runs it in `demo-worker.mjs`.
- Bun needs nothing special (its main thread may wait).

`EggSolver.create({ wasm, threads })` starts the pool once (threads + 1
workers); a call then uses all of them. Other hosts must call the module's
`_initialize` export once, on the main thread, before anything else (as
`egg-solver.mjs` does): it sets up wasi-libc's record of the main thread,
without which threads end up waiting forever.

### Speed

100 depth-17 cubes on h6 (14.8M nodes each), one thread, `solveMany` /
`bench` batch mode, on one Zen 5 core:

| build                              | time    | Mnodes/s |
|------------------------------------|--------:|---------:|
| native x86 (AVX-512, `simd.rs`)    | 21.7 s  | 68       |
| wasm in wasmtime 44                | 122 s   | 12.2     |
| wasm in V8 (Node 26)               | 124 s   | 11.9     |
| wasm in Bun 1.4.4 (JavaScriptCore) | 127 s   | 11.7     |

(NEON was only run under qemu here, which says nothing about speed.) Two
things matter for wasm speed, both handled:

- No prefetch instruction: `prefetch::read_once` is a plain load there,
  which starts the cache miss just as well (+53% in wasmtime, +45% in V8).
  Lanes still pay off: 1 lane 200.6 s, 8 lanes 123.8 s, flat up to 64; the
  module uses 16.
- V8 (Chrome) starts new code in its baseline compiler and gives a function
  optimized code only from its next call: a long first solve would run
  slowly to its end (187.7 s instead of 129.5 s here). `loadTable` and
  `generateTable` therefore run a short warm-up (`warmUp`, ~0.6 s), and all
  exports share one copy of the search loop. JavaScriptCore (Bun, Safari)
  moves a running loop to optimized code: in Bun the warm-up is skipped
  (126.5 and 127.2 s without it, 127.0 and 127.9 s with it).

With 8 threads (`egg-solver-threads.wasm`, one L3 domain), one cube at a time
gets 7.2-7.8 times the one-thread speed (h6: 88.6 Mnodes/s in V8, 90.7 in
Bun; in headless Chromium 0.63-0.66 s instead of 5.15 s for the same cube),
and generating h4 base 8 takes 13 s instead of 75 s.

### Tests

`scripts/check.sh` runs the library tests in wasm under wasmtime (v44 or older:
later versions dropped wasi-threads; `WASMTIME=path`) and `wasm/test.mjs`
(both modules in Bun), when installed. The node counts match native ones
(h5, 20 random states: identical nodes, lookups and lengths in x86,
scalar, wasmtime and V8; h6, 100 depth-17 cubes: the same in Bun).


### Compatibility with existing installations

Settings use `EGG_SOLVER_*`; the corresponding `CUBEOPT_*` names remain fallback aliases.
An explicitly set modern name takes precedence. Existing table files and shared-memory
segments remain readable, and pruning-table data does not need to be regenerated.

### Library memory safety contracts

`Cube::from_bytes`, symmetry-coordinate lookups, and `cube_from_edge_key`
return `Option` for invalid input. Table and solver internals are exposed
through read-only accessors. `H48Table::from_memory` checks the metadata
and allocation size; `get` returns `None` for an invalid index.

`H48Table::open_copy` loads private RAM and is safe. APIs that can map files
or attach shared segments are unsafe: the backing storage must remain
unchanged and untruncated while the table exists. This includes changes by
other processes. The command-line mapped/shared modes require the same
condition; `--load copy` avoids that dependence after loading. File-backed
generation requires exclusive access to its output paths.

The current packed-coordinate and table encodings require a little-endian
target. Big-endian builds are rejected at compile time.
