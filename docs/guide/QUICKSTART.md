# Build and solve a cube

[Documentation](../README.md) · [Tables and memory](OVERVIEW.md) · [Full command reference](../reference/USER_GUIDE.md)

Run these commands from a source checkout containing `Cargo.toml`. Rust and Cargo are required. The release build targets the build machine's CPU by default.

## First solve

```sh
cargo build --release --locked
mkdir -p tables
target/release/gen 0 auto tables/h0 -t 2
target/release/solve --fast --table tables/h0 -t 2 "R U R' U' F2"
```

h0 contains about 29 MiB of pruning data and keeps this example small. A possible shortest solution is `F2 U R U' R'`; a different shortest sequence is also valid. The solver verifies the returned moves.

## Arbitrary states and batches

Choose a [larger table that fits in RAM](OVERVIEW.md#tables-and-memory) for arbitrary random states. Generate it once, then reuse it:

```sh
target/release/gen 7 auto tables/h7
target/release/gen check tables/h7
target/release/solve --table tables/h7 < testdata/random_state.txt
```

Each input line is a scramble. The default batch mode allows workers to share searches across the input. Add `-c 1` to finish one cube before starting the next. `-t N` chooses worker threads; `-l N` chooses interleaved lanes per worker. `--json` emits one result per cube and a final total.

## One solve and table loading

`--fast` requires an explicit `--table` and one scramble, either as arguments or as exactly one nonblank stdin line. Empty input is an error; there is no random fallback or automatic warm-up. `--warmup SCRAMBLE` explicitly adds a warm-up solve.

`--load copy` reads the table into private RAM before searching. Mapping and Linux shared-memory modes are also available. See [load modes](../reference/USER_GUIDE.md#table-files) for residency, page size and backing-file requirements. Search timers exclude table loading; startup is a separate cost.
