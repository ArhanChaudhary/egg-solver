#!/bin/bash
# Run the unit tests that Miri can interpret: it checks unsafe code for
# undefined behavior (bounds, alignment, provenance, uninitialized memory)
# and the share protocol's threads for data races and leaks.
#   ./miri.sh [seeds]   (default 0..8: scheduler seeds of the share tests)
# Covered: the cubie model, SIMD cube operations, coordinates and the hot
# index path (its inline asm has `cfg(miri)` intrinsic equivalents), the
# job sorts, and the work-sharing protocol on a tiny search. Not covered:
# `Cocsep::generate` and the table generator (too slow), the table files.
# Slow: about 20 minutes with 6 cores. Not part of check.sh.
set -euo pipefail
cd "$(dirname "$0")"
seeds=${1:-0..8}
strict="-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target/miri}
run() { # <miri flags> <test filters...>
    local flags=$1
    shift
    MIRIFLAGS="$flags" cargo +nightly miri test --lib -q -- "$@" 2>&1 | grep -E "^test |test result|^error" || true
}
cargo +nightly miri test --lib --no-run -q 2>&1 | grep -E "^error" && exit 1
logs=$(mktemp -d)
# (One process per group, in parallel: Miri runs one thread at a time.)
run "$strict" cubie::tests::symmetry > "$logs/1" &
run "$strict" cubie::tests --skip symmetry > "$logs/2" &
run "$strict" simd:: coord:: solver:: tables:: > "$logs/3" &
run "$strict" lanes::tests:: > "$logs/4" &
run "$strict" lanes::share::tests::staged > "$logs/5" &
run "$strict -Zmiri-many-seeds=$seeds" lanes::share::tests::share_ > "$logs/6" &
wait
cat "$logs"/*
rm -r "$logs"
