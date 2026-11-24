#!/bin/bash
# Build solve/bench/gen of several egg-solver commits, each for target-cpu native (this Zen 5)
# and znver4 (Sebastiano's Ryzen 7 7700), in one worktree path and one target path (so that
# embedded paths do not differ between the builds). Binaries go to perf/bin/<commit>-<cpu>/.
set -u
export PATH=$HOME/.cargo/bin:$PATH
W=/data3/build/perf/wt; T=/data3/build/perf/target; B=/data3/build/perf/bin
[ -d $W ] || git -C /data3/egg-solver worktree add -q --detach $W dc153b3
for cpu in native znver4; do
  for commit in 28a1953 c3eb909 dc153b3; do
    out=$B/$commit-$cpu; [ -x $out/solve ] && continue
    git -C $W checkout -q --detach $commit || exit 1
    if [ $cpu = native ]; then unset RUSTFLAGS; else export RUSTFLAGS="-C target-cpu=$cpu"; fi
    cd $W && CARGO_TARGET_DIR=$T flock -s /tmp/cube-bench.lock nice -n 10 cargo build -q --release --locked --bin solve --bin bench --bin gen 2>&1 | tail -3
    mkdir -p $out && cp $T/release/solve $T/release/bench $T/release/gen $out/ && echo "built $commit $cpu $(date -u +%T)"
  done
done
git -C $W checkout -q --detach dc153b3
