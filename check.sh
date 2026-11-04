#!/bin/bash
# fmt + clippy + tests; fails on any problem.
set -euo pipefail
cd "$(dirname "$0")"
cargo fmt --check
cargo clippy --release --all-targets -- -D warnings
# A private log, so concurrent runs (several worktrees) never read each other's output.
log=$(mktemp -t cube-test.XXXXXX.log)
trap 'rm -f "$log"' EXIT
cargo test --release 2>&1 | tee "$log" | grep -E "^test result"
! grep -qE "FAILED|panicked" "$log"
echo CHECK OK
