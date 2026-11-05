# cubeopt

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

### Table files

A table file holds the table data, then a 4 KiB trailer (k, base,
distribution, checksum): `gen check <table>` verifies a table against it.
Tables are the same on every machine and backend: you can generate one on a
big machine and copy it. `CUBEOPT_LOAD=map|copy` sets how a table is
loaded (map the file, or copy it into memory with transparent huge pages;
the default is copy on Linux, map elsewhere), and `CUBEOPT_PREFAULT=0`
(with map) reads pages on first use instead of the whole table at start.

### Portable builds

`target-cpu=native` (in `.cargo/config.toml`, x86 only) builds for the CPU
of the build machine. For binaries that run on other machines:

```sh
RUSTFLAGS="-C target-cpu=x86-64-v4" cargo build --release  # x86 with AVX-512
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release  # x86 with AVX2 + BMI2
RUSTFLAGS="-C target-cpu=x86-64" cargo build --release     # any x86-64 (scalar, slower)
cargo build --release --target aarch64-apple-darwin        # Apple Silicon (NEON)
cargo build --release --features scalar                    # the scalar backend, any target
```

## WebAssembly

The solver also compiles to WebAssembly (SIMD128) and runs in browsers and
in Node: the same search on the same table files, with the same solutions
and node counts as natively, about 5.7 times slower than the native x86
build (see "Speed").

### Build

```sh
rustup target add wasm32-wasip1 wasm32-wasip1-threads
wasm/build.sh
```

This makes `wasm/dist/cubeopt.wasm` (one thread; runs anywhere) and
`wasm/dist/cubeopt-threads.wasm` (threads, see below), ~380 KB each. Both
are WASI modules with a small C ABI (`wasm/src/lib.rs`); `wasm/cubeopt.mjs`
wraps them for JavaScript and provides the few WASI functions they import,
so neither a WASI runtime nor wasm-bindgen is needed.

```js
import { Cubeopt } from './wasm/cubeopt.mjs';
const solver = await Cubeopt.create({ wasm: 'wasm/dist/cubeopt.wasm' });
await solver.loadTable('h6');             // a path (Node), URL, File or bytes
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

### Node

```sh
node wasm/demo.mjs --table ~/cube-tables/h6 < testdata/random_state.txt
node wasm/demo.mjs --gen 4:8 --random 5:1             # h4 base 8, 5 random states
node wasm/demo.mjs --table ~/cube-tables/h6 --threads 8 < scrambles.txt
node wasm/demo.mjs --table ~/cube-tables/h6 --batch < scrambles.txt   # throughput
```

### Browser

```sh
node wasm/serve.mjs --tables ~/cube-tables    # then open http://localhost:8000/
```

`wasm/index.html` generates h0..h4 in the page or loads a table file (file
picker, or a URL such as `/tables/h6`), and solves scrambles, facelet
strings or random states. Any static server works for one thread;
`serve.mjs` also sends the headers the threads build needs.

### Tables

wasm32 has at most 4 GiB of memory, for the table and everything else
(tens of MB; ~0.2 GB with 8 threads). Small tables can be generated in the module; larger ones are made
by the native `gen` (see "Running on a laptop") and loaded from the file.
Time per random-state cube, one at a time, in V8 (Node 26; Chrome has the
same engine), mean and median of 10 cubes (h4: 5), on Zen 5 cores (8
threads: one L3 domain):

| table     | size     | runs in                     | made by                    | 1 thread         | 8 threads        |
|-----------|---------:|-----------------------------|----------------------------|------------------|------------------|
| h0..h3    | ≤ 0.2 GiB| any browser                 | the module: h0 in 2.5 s    | short scrambles  |                  |
| h4 base 8 | 0.45 GiB | any browser                 | the module: 75 s (8 threads: 13 s) | 21.6 s, 20.7 s |    |
| h5        | 0.9 GiB  | desktop browsers            | `gen 5 auto`               | 19.4 s, 10.3 s   |                  |
| h6        | 1.8 GiB  | desktop browsers            | `gen 6 auto`               | 12.6 s, 5.0 s    | 1.6 s, 0.65 s    |
| h7        | 3.6 GiB  | Node; Chrome (tested headless) | `gen 7 auto`            | 6.7 s, 2.9 s     | 0.93 s, 0.44 s   |

Loading from a file takes seconds (from a local server into headless
Chromium: h6 3-13 s, h7 15 s; `verify` adds a checksum pass). The 4 GiB are
the engine's limit for wasm32; a browser tab may allow less (mobile ones
much less). h8 and larger would need memory64 (64-bit WebAssembly memory),
which this build does not use.

### Threads

`cubeopt-threads.wasm` is built for `wasm32-wasip1-threads` (stable Rust):
std threads work through wasi-threads, so the work-sharing mode (`solve`,
all threads on one cube) and the stream mode (`solveMany`) run as natively.
`cubeopt.mjs` implements wasi-threads' `thread-spawn` with a pool of
workers (`cubeopt-thread.mjs`, Web Workers or Node `worker_threads`) that
share the module's memory. That needs `SharedArrayBuffer`:

- In browsers, a cross-origin isolated page, served with the headers
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp` (`serve.mjs` sends them).
- The solver must run in a worker, not on the page's main thread: a solve
  waits for its threads (`Atomics.wait`), which the main thread may not do.
  `index.html` runs it in `demo-worker.mjs`.
- Node needs nothing special.

`Cubeopt.create({ wasm, threads })` starts the pool once (threads + 1
workers); a call then uses all of them. Other hosts must call the module's
`_initialize` export once, on the main thread, before anything else (as
`cubeopt.mjs` does): it sets up wasi-libc's record of the main thread,
without which threads end up waiting forever.

### Speed

100 depth-17 cubes on h6 (14.8M nodes each), one thread, `solveMany` /
`bench` batch mode, on one Zen 5 core:

| build                              | time    | Mnodes/s |
|------------------------------------|--------:|---------:|
| native x86 (AVX-512, `simd.rs`)    | 21.7 s  | 68       |
| native scalar (`--features scalar`)| 74.5 s  | 20       |
| wasm in wasmtime 44                | 122 s   | 12.2     |
| wasm in V8 (Node 26)               | 124 s   | 11.9     |

(NEON was only run under qemu here, which says nothing about speed.) Two
things matter for wasm speed, both handled:

- No prefetch instruction: `prefetch::read_once` is a plain load there,
  which starts the cache miss just as well (+53% in wasmtime, +45% in V8).
  Lanes still pay off: 1 lane 200.6 s, 8 lanes 123.8 s, flat up to 64; the
  module uses 16.
- V8 (browsers, Node) starts new code in its baseline compiler and gives a
  function optimized code only from its next call: a long first solve would
  run slowly to its end (187.7 s instead of 129.5 s here). `loadTable` and
  `generateTable` therefore run a short warm-up (`warmUp`, ~0.6 s), and all
  exports share one copy of the search loop.

With 8 threads (`cubeopt-threads.wasm`, one L3 domain), one cube at a time
gets 7.2-7.8 times the one-thread speed (h6: 88.6 Mnodes/s; in headless
Chromium 0.66 s instead of 5.15 s for the same cube), and generating h4
base 8 takes 13 s instead of 75 s.

### Tests

`check.sh` runs the library tests in wasm under wasmtime (v44 or older:
later versions dropped wasi-threads; `WASMTIME=path`) and `wasm/test.mjs`
(both modules in Node), when installed. The node counts match native ones
(h5, 20 random states: identical nodes, lookups and lengths in x86,
scalar, wasmtime and Node).
