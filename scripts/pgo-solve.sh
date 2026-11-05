#!/bin/bash
# Profile-guided (PGO) build of the `solve` binary (share mode only; `bench`
# and the default `cargo build` are not affected). 1T `solve -c 1`: -2.1%
# ms/cube (8 of 8 pairs); 48T `-c 1` neutral (pb1, 66c4f03).
#
#   scripts/pgo-solve.sh <table> [threads]
#
# 1. Builds `solve` instrumented (-Cprofile-generate) in $OUT/gen.
# 2. Trains it on <table> with `threads` threads (default 8): `-c 1` on 150
#    and `-c 0` on 300 scrambles of 40 random moves, made here from a fixed
#    seed. They share no cube with the benchmark sets (cube_solver_test's
#    datasets, testdata/, `bench`'s random:N:SEED and stream cubes).
# 3. Merges the profile with llvm-profdata (rustup component llvm-tools) and
#    builds `solve` with -Cprofile-use: $OUT/solve.
#
# The profile flags are added to the repo's own rustflags (.cargo/config.toml:
# target-cpu=native on x86-64). Env: OUT (default target/pgo-solve);
# PGO_PROFILE=<file.profdata> skips steps 1-2 and reuses that profile;
# CUBEOPT_SHARE_PAD as for any build (src/hot.rs; default here 0, see below).
set -euo pipefail
cd "$(dirname "$0")/.."
TABLE=${1:?usage: scripts/pgo-solve.sh <table> [threads]}
THREADS=${2:-8}
OUT=${OUT:-$PWD/target/pgo-solve}
PROFDATA=$(ls "$(rustc --print sysroot)"/lib/rustlib/*/bin/llvm-profdata 2>/dev/null | head -1 || true)
if [ ! -x "$PROFDATA" ]; then
  echo "llvm-profdata not found: rustup component add llvm-tools" >&2
  exit 1
fi
# The PGO build's share loop (`share::Worker::rounds`) at offset 0 of its
# 64-byte line (src/hot.rs): PGO rewrites the loop, and its best pad moved
# from main's 16 to 0 (1T `solve -c 1` on 585b3b6, PGO against main at pad
# 16, ms/cube, 2 pairs each: pad 0 -2.0 -3.4%, 16 +0.6 +0.5%, 32 -2.0 -1.7%,
# 48 -2.2 -2.9%; on 3a9a50a pad 0 was the best too).
export CUBEOPT_SHARE_PAD=${CUBEOPT_SHARE_PAD:-0}
# Extra rustflags for every target (joined with the repo's per-target flags).
flags() { echo "target.'cfg(all())'.rustflags=[\"$1\"]"; }

# `n` scrambles of 40 random moves (no face twice in a row), in the
# `R1 U3 F2` notation, from a Park-Miller generator with a fixed seed
# (integer math exact in any awk).
scrambles() {
  awk -v n="$1" 'BEGIN {
    x = 20261105; split("U R F D L B", face, " ")
    for (i = 0; i < n; i++) {
      line = ""; last = 0
      for (m = 0; m < 40; m++) {
        do { x = (x * 16807) % 2147483647; f = x % 6 + 1 } while (f == last)
        last = f; x = (x * 16807) % 2147483647
        line = line (m ? " " : "") face[f] (x % 3 + 1)
      }
      print line
    }
  }'
}

if [ -z "${PGO_PROFILE:-}" ]; then
  PROF=$OUT/prof
  rm -rf "$PROF" && mkdir -p "$PROF"
  cargo build --release -q --bin solve --target-dir "$OUT/gen" \
    --config "$(flags "-Cprofile-generate=$PROF")"
  scrambles 300 > "$PROF/train.txt"
  LLVM_PROFILE_FILE="$PROF/solve-%p.profraw" "$OUT/gen/release/solve" "$TABLE" \
    -t "$THREADS" -c 1 -n 150 < "$PROF/train.txt" > "$PROF/train-c1.log"
  LLVM_PROFILE_FILE="$PROF/solve-%p.profraw" "$OUT/gen/release/solve" "$TABLE" \
    -t "$THREADS" -c 0 < "$PROF/train.txt" > "$PROF/train-c0.log"
  "$PROFDATA" merge -o "$PROF/solve.profdata" "$PROF"/solve-*.profraw
  PGO_PROFILE=$PROF/solve.profdata
fi
cargo build --release -q --bin solve --target-dir "$OUT/use" \
  --config "$(flags "-Cprofile-use=$(realpath "$PGO_PROFILE")")"
cp "$OUT/use/release/solve" "$OUT/solve"
echo "built $OUT/solve"
