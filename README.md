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
