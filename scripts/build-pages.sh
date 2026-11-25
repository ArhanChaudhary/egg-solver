#!/bin/bash
# Build the browser solver and copy only public site assets to a new directory.
set -euo pipefail
cd "$(dirname "$0")/.."
output=${1:?usage: scripts/build-pages.sh OUTPUT_DIRECTORY}
if [ -e "$output" ]; then
    echo "output already exists: $output (choose a new directory)" >&2
    exit 2
fi
bun scripts/render-docs.mjs
python3 scripts/check-docs.py
wasm/build.sh
mkdir -p "$output/dist"
cp wasm/index.html wasm/demo-worker.mjs wasm/egg-solver.mjs wasm/egg-solver-thread.mjs \
    wasm/cube-model.mjs wasm/cube-input.mjs wasm/cube-3d.mjs wasm/coi-sw.js "$output/"
cp wasm/dist/egg-solver.wasm wasm/dist/egg-solver-threads.wasm "$output/dist/"
cp -R docs "$output/docs"
printf 'Pages site: %s\n' "$output"
