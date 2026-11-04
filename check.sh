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
# The other backends (src/portable/) must keep compiling and passing: aarch64
# NEON (macOS and Linux), the scalar fallback (any target; forced here by the
# `scalar` feature) and wasm (library only). Their library tests run with the
# scalar backend here, and with NEON under qemu when it is installed. Skipped
# (with a note) where a target or tool is missing; CHECK_BACKENDS=0 skips all.
if [ "${CHECK_BACKENDS:-1}" != 0 ]; then
    installed=$(rustup target list --installed 2>/dev/null || true)
    for target in aarch64-apple-darwin aarch64-unknown-linux-gnu wasm32-unknown-unknown; do
        if ! grep -qx "$target" <<< "$installed"; then
            echo "check: target $target not installed: not checked" >&2
        elif [ $target = wasm32-unknown-unknown ]; then
            cargo clippy --release --lib --target $target -- -D warnings
        else
            cargo clippy --release --all-targets --target $target -- -D warnings
        fi
    done
    cargo clippy --release --all-targets --features scalar -- -D warnings
    cargo test --features scalar --lib 2>&1 | tee "$log" | grep -E "^test result"
    ! grep -qE "FAILED|panicked" "$log"
    if grep -qx aarch64-unknown-linux-gnu <<< "$installed" &&
        command -v qemu-aarch64 > /dev/null && command -v aarch64-linux-gnu-gcc > /dev/null; then
        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="qemu-aarch64 -L /usr/aarch64-linux-gnu" \
            cargo test --target aarch64-unknown-linux-gnu --lib 2>&1 | tee "$log" | grep -E "^test result"
        ! grep -qE "FAILED|panicked" "$log"
    else
        echo "check: qemu-aarch64 or aarch64-linux-gnu-gcc missing: NEON tests not run" >&2
    fi
fi
echo CHECK OK
