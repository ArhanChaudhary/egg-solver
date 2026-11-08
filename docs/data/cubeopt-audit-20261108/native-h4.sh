#!/bin/bash
set -euo pipefail
R=/data/agents/cubeopt-current
for base in 8 9; do
 flock /tmp/cube-turn.lock flock /tmp/cube-bench.lock /data/agents/f1/tools/bin/capmem 4G /data2/build/egg-solver-target/release/gen 4 "$base" "$R/egg-h4b$base" -t 48 > "$R/egg-h4b$base-generate.log" 2>&1
 flock /tmp/cube-turn.lock flock /tmp/cube-bench.lock /data/agents/f1/tools/bin/capmem 4G /data2/build/egg-solver-target/release/examples/table_bounds "$R/egg-h4b$base" < "$R/bound-corpus.txt" > "$R/egg-h4b$base-bounds.csv"
 flock /tmp/cube-turn.lock flock /tmp/cube-bench.lock /data/agents/f1/tools/bin/capmem 4G /data/agents/nxopt32/bin/egg-solve --table "$R/egg-h4b$base" --load copy -t 48 -c 0 --json < /home/super/egg-solver/testdata/random_state.txt > "$R/egg-h4b$base-solves.jsonl" 2> "$R/egg-h4b$base-solves.stderr"
done
