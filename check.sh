#!/bin/bash
# fmt + clippy + tests; fails on any problem.
set -euo pipefail
cd "$(dirname "$0")"
cargo fmt --check
cargo clippy --release --all-targets -- -D warnings
cargo test --release 2>&1 | tee /tmp/cube-test.log | grep -E "^test result" 
! grep -qE "FAILED|panicked" /tmp/cube-test.log
echo CHECK OK
