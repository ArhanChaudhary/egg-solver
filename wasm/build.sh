#!/bin/bash
# Build the WebAssembly modules into wasm/dist/:
#   cubeopt.wasm          one thread (wasm32-wasip1), runs anywhere
#   cubeopt-threads.wasm  threads (wasm32-wasip1-threads): Bun, or a
#                         cross-origin isolated page
# Needs the targets: rustup target add wasm32-wasip1 wasm32-wasip1-threads
# (SIMD128 and the memory limit are set in .cargo/config.toml.)
set -euo pipefail
cd "$(dirname "$0")/.."
target_dir=${CARGO_TARGET_DIR:-target}
mkdir -p wasm/dist
for target in wasm32-wasip1 wasm32-wasip1-threads; do
    cargo build -p cubeopt-wasm --profile wasm-release --target $target
done
cp "$target_dir/wasm32-wasip1/wasm-release/cubeopt_wasm.wasm" wasm/dist/cubeopt.wasm
cp "$target_dir/wasm32-wasip1-threads/wasm-release/cubeopt_wasm.wasm" wasm/dist/cubeopt-threads.wasm
ls -l wasm/dist/*.wasm
