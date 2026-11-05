#!/bin/bash
# fmt + clippy + tests; fails on any problem.
set -euo pipefail
cd "$(dirname "$0")"
cargo fmt --all --check
cargo clippy --release --all-targets -- -D warnings
# A private log, so concurrent runs (several worktrees) never read each other's output.
log=$(mktemp -t cube-test.XXXXXX.log)
trap 'rm -f "$log"' EXIT
# The last test run's log shows no failure. (Called as `no_failure || exit 1`:
# `set -e` ignores a failing `! grep`.)
no_failure() { ! grep -qE "FAILED|panicked" "$log"; }
cargo test --release 2>&1 | tee "$log" | grep -E "^test result"
no_failure || exit 1
# The other backends (src/portable/) must keep compiling and passing: aarch64
# NEON (macOS and Linux), the scalar fallback (any target; forced here by the
# `scalar` feature) and wasm (SIMD128: WASI with and without threads, and the
# library for wasm32-unknown-unknown). Their library tests run with the
# scalar backend here, with NEON under qemu and with wasm under wasmtime
# (wasi-threads: up to v44) when installed, and the wasm module (wasm/) gets a
# smoke test in Node. Skipped (with a note) where a target or tool is missing;
# CHECK_BACKENDS=0 skips all.
if [ "${CHECK_BACKENDS:-1}" != 0 ]; then
    installed=$(rustup target list --installed 2>/dev/null || true)
    for target in aarch64-apple-darwin aarch64-unknown-linux-gnu wasm32-unknown-unknown \
        wasm32-wasip1 wasm32-wasip1-threads; do
        if ! grep -qx "$target" <<< "$installed"; then
            echo "check: target $target not installed: not checked" >&2
        elif [ $target = wasm32-unknown-unknown ]; then
            cargo clippy --release --lib --target $target -- -D warnings
        else
            cargo clippy --release --all-targets --target $target -- -D warnings
        fi
        if [[ $target == wasm32-wasip1* ]] && grep -qx "$target" <<< "$installed"; then
            cargo clippy --release -p cubeopt-wasm --target $target -- -D warnings
        fi
    done
    if grep -qx wasm32-wasip1-threads <<< "$installed"; then
        # (Every wasm run has a time limit: a hang fails the check at once
        # instead of holding the build lock.)
        wasmtime=${WASMTIME:-wasmtime}
        export CARGO_TARGET_WASM32_WASIP1_THREADS_RUNNER="timeout -s KILL 600 $wasmtime run -S threads=y -W threads=y,shared-memory=y --dir /tmp"
        cargo test --release --lib --target wasm32-wasip1-threads --no-run
        if command -v "$wasmtime" > /dev/null &&
            cargo test --release --lib --target wasm32-wasip1-threads -- --list > /dev/null 2>&1; then
            if ! cargo test --release --lib --target wasm32-wasip1-threads > "$log" 2>&1; then
                tail -20 "$log" >&2
                echo "check: wasm library tests failed (or hung: killed after 600 s)" >&2
                exit 1
            fi
            grep -E "^test result" "$log"
        else
            echo "check: wasmtime with wasi-threads (v44 or older; WASMTIME=path) missing: wasm tests not run" >&2
        fi
        if grep -qx wasm32-wasip1 <<< "$installed" && command -v node > /dev/null; then
            wasm/build.sh > /dev/null
            if ! timeout -s KILL 300 node wasm/test.mjs; then
                echo "check: wasm/test.mjs failed (or hung: killed after 300 s)" >&2
                exit 1
            fi
        else
            echo "check: node or target wasm32-wasip1 missing: wasm module not tested" >&2
        fi
    fi
    cargo clippy --release --all-targets --features scalar -- -D warnings
    cargo test --features scalar --lib 2>&1 | tee "$log" | grep -E "^test result"
    no_failure || exit 1
    if grep -qx aarch64-unknown-linux-gnu <<< "$installed" &&
        command -v qemu-aarch64 > /dev/null && command -v aarch64-linux-gnu-gcc > /dev/null; then
        neon_tests() {
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
                CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="qemu-aarch64 -L /usr/aarch64-linux-gnu" \
                cargo test --target aarch64-unknown-linux-gnu --lib -- "$@" 2>&1 | tee "$log" | grep -E "^test result"
        }
        # qemu-user (8.2) crashes ("QEMU internal SIGSEGV {code=MAPERR,
        # addr=0x20}", a fault in its own code) when the guest reads
        # /proc/self/maps, which qemu emulates, while other guest threads map
        # or unmap memory (in 8-20% of the runs, from the test below; a
        # 25-line program does it too): that test runs alone. Should qemu
        # crash anyway, with no test failed, the step runs once more.
        maps_test=tables::tests::opened_table_file_is_read_only
        for filter in --skip --exact; do
            if ! neon_tests $filter $maps_test; then
                if ! grep -q "QEMU internal SIGSEGV" "$log" || ! no_failure; then
                    exit 1
                fi
                echo "check: qemu-aarch64 crashed (QEMU internal SIGSEGV): NEON tests run again" >&2
                neon_tests $filter $maps_test
            fi
            no_failure || exit 1
        done
        # (The test alone did run.)
        grep -q "test result: ok. 1 passed" "$log" || exit 1
    else
        echo "check: qemu-aarch64 or aarch64-linux-gnu-gcc missing: NEON tests not run" >&2
    fi
fi
echo CHECK OK
