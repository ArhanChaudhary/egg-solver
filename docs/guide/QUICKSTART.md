# Build it and solve a cube

[Documentation home](../README.md) · Previous: [Overview](OVERVIEW.md) · Next: [Comparisons](../comparisons/README.md)

Run these commands from the extracted source directory containing `Cargo.toml`. Rust and Cargo are required. Cargo downloads the locked dependencies if they are not already cached.

```sh
cargo build --release --locked
mkdir -p tables
target/release/gen 0 auto tables/h0 -t 2
printf "%s\n" "R U R' U' F2" | target/release/solve tables/h0 --load copy -t 2 -c 1
```

This small example uses h0 to keep setup lightweight. The source ZIP was independently extracted, built for generic x86-64, and exercised with this scramble. It returned the five-move solution `F2 U R U' R'`, and its internal solution check passed. A correct solver may return a different shortest sequence.

Use h0 for short examples. For arbitrary random states, choose a larger table that fits your RAM. For example, a machine with enough free RAM for h7 can generate and use it with:

```sh
target/release/gen 7 auto tables/h7
target/release/solve tables/h7 -c 1 < testdata/random_state.txt
```

`-c 1` solves one cube at a time using the selected workers. Omit it for the default batch mode. `-t N` sets the worker count. `--json` emits structured results; `--help` lists the options.

## Loading tables into RAM

`--load copy` reads the table into private RAM before the search. Mapping and shared-memory modes are also available. Ensure the table fits comfortably in available memory: the operating system can otherwise introduce paging. Table-load time is excluded from the main solver comparison timings.

After generating a table, verify its stored checksum and distribution:

```sh
target/release/gen check tables/h7
```

For the one-cube convenience command, specify your table explicitly:

```sh
target/release/solve --fast --table tables/h7 "R U R' U' F2"
```

`--fast` also accepts one nonblank stdin line. It requires an explicit `--table`,
rejects empty input, and performs no warm-up unless `--warmup SCRAMBLE` is supplied.

The full [user guide](../reference/USER_GUIDE.md) covers lane tuning, table sizing, load modes, environment variables, WebAssembly and shared memory. The [packaging validation record](../evidence/validation/continuation/source-release.json) identifies the exact source and fresh-build checks.
