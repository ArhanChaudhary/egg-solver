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

`auto` chooses the program's recommended compression base. The base changes pruning bounds, not data capacity. Generate once and reuse the file. The `--parts N` option generates row ranges sequentially to reduce peak memory in depth-first generation; it does not apply to h11's BFS generation. Memory and disk preflight checks run before generation.

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

Batch mode also accepts a positional table path. `EGG_SOLVER_TABLE` can supply it in batch mode, and `EGG_SOLVER_THREADS` can supply the thread count. `--fast` still requires a command-line `--table`.

Search timing excludes table loading. The batch total measures completion of the whole batch; individual cube durations can overlap. See [timing definitions](../comparisons/METHODS.md) before interpreting them as throughput.

## Tune and benchmark

```sh
target/release/bench tables/h7 tune
target/release/bench --help
```

The tune command writes a `<TABLE>.tune` record for its measured configurations. `solve` and `bench` use a matching record unless `-l` is explicit. The built-in lane candidates end at 48, but explicit larger lane values are supported. Tuning a stream does not guarantee the best finite-batch or single-cube setting. The [measured lane study](../performance/PARALLELISM.md) explains that distinction.

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

## Shared memory on Linux

```sh
target/release/solve --shm-load tables/h7
target/release/solve --shm --table tables/h7 < testdata/random_state.txt
target/release/solve --shm-remove tables/h7
```

A segment persists until removal or reboot. An existing matching segment avoids reloading the table into a fresh private allocation. Metadata is checked on attachment; a mismatched segment is rejected by explicit shared mode. Keep the segment and table immutable during use.

`--shm-pages 1g` requests only 1 GiB pages and fails if unavailable. The default page preference is `1g,2m,4k`; `-v` reports the selected size and fallback reasons. Huge-page allocation needs both permission (the configured huge-page group or `CAP_IPC_LOCK`) and enough free reserved pages. The program's preflight reports relevant limits. `--shm-key` selects another segment key when needed.

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

`solve` puts the selected workers on one cube; `solveMany` uses independent cube lanes. Inputs can be scrambles or 54-sticker facelet strings in U R F D L B face order, each face viewed from outside. The wrapper's facelet parser checks the representation.

### Bun

```sh
bun wasm/demo.mjs --table tables/h6 < testdata/random_state.txt
bun wasm/demo.mjs --table tables/h6 --threads 8 --batch < scrambles.txt
```

### Browser

```sh
bun wasm/serve.mjs --tables tables
```

Open the local URL printed by the server, or the public site, <https://egg-solver.arhan.sh/>. On load the page starts with every core the browser reports and generates h0, the smallest table, so **Solve** works immediately. **Restart** applies a different thread count and rebuilds the selected table. The page can also generate h1–h4 or load a compatible table. Its recommended h4 option uses base 9; the explicit base-8 option trades weaker search pruning for faster generation. [The h4 study](../comparisons/TABLE_ANALYSIS.md) records that tradeoff.

### Tables

The wasm32 build supports h0–h6. Each Rust allocation must fit below `isize::MAX` (2 GiB), so h7 is rejected even though wasm32 has a larger overall address space. The table and the module's other allocations must also fit the host's memory limits. Large tables are normally generated with native `gen` and then loaded.

### Threads

Browser threads need `SharedArrayBuffer` and cross-origin isolation. The local server supplies the required COOP/COEP headers, and the public site, egg-solver.arhan.sh, receives them from Cloudflare because GitHub Pages cannot set headers. The solver runs in a Web Worker because its thread waits cannot run on a browser's main thread. Without isolation, such as on a plain `github.io` deployment, the page uses one thread. Bun supports the threaded wrapper without those HTTP headers.

The wrapper initializes the module and worker pool. Other hosts must perform the module's required initialization before calling its C ABI. See `wasm/src/lib.rs` and the JavaScript wrapper when embedding it directly.

<a id="speed"></a>

### Performance records

Historical V8/Bun timings, generation times and engine warm-up experiments are preserved in the [dated reference](USER_GUIDE_20261107.md#speed). They are not predictions for a current browser or laptop. [Performance studies](../performance/README.md) indexes the measured native and WebAssembly evidence.

## Development and publishing

[Development checks](../guide/DEVELOPMENT.md) covers checks, Miri and stress runs. [Publishing](../guide/PUBLISHING.md) describes the Pages workflow and local site build. Shell completions are available through `--completions` (bash, zsh, fish, elvish and powershell).
