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
cp wasm/index.html wasm/demo-worker.mjs wasm/egg-solver.mjs wasm/egg-solver-thread.mjs "$output/"
cp wasm/dist/egg-solver.wasm wasm/dist/egg-solver-threads.wasm "$output/dist/"
cp -R docs "$output/docs"
# Also works at /REPOSITORY/, without hardcoded account or repository names.
python3 - "$output/index.html" <<'PY'
from pathlib import Path
import sys
p = Path(sys.argv[1])
s = p.read_text().replace('<body>', '<body>\n<nav><a href="docs/index.html">Documentation and benchmarks</a></nav>', 1)
p.write_text(s)
PY
printf 'Pages site: %s\n' "$output"
