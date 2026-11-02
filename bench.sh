#!/bin/bash
# Standard benchmark: 1000 uniformly random states (seed 2026), 48 threads,
# h11 table; prints the median solves/sec of 3 runs.
# Usage: ./bench.sh [table] [runs]
TABLE=${1:-/mnt/huge/h11}
RUNS=${2:-3}
cargo build --release -q || exit 1
for i in $(seq $RUNS); do
  ./target/release/bench $TABLE random:1000:2026 1000 48 2>/dev/null | tee /dev/stderr | grep solves/sec | awk '{print $2}'
done | sort -n | awk '{a[NR]=$1} END {print "MEDIAN solves/sec", a[int((NR+1)/2)]}'
