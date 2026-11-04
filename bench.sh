#!/bin/bash
# Standard benchmark: steady-state optimal solves/sec on an endless stream
# of uniformly random states (seed 2026), 48 threads, h11 table: solves
# completed between t=20s and t=50s (24 interleaved lanes per thread by
# default; lanes=0 is the recursive solver). Prints the median over runs.
# Usage: ./bench.sh [table] [runs] [lanes]
TABLE=${1:-/mnt/huge/h11}
RUNS=${2:-3}
LANES=${3:-24}
cargo build --release -q || exit 1
OUT=$(for i in $(seq $RUNS); do ./target/release/bench $TABLE stream 20 30 48 $LANES 2>/dev/null; done)
echo "$OUT" | grep -E "nodes/cube" | tail -1
echo "$OUT" | grep throughput | awk '{print $2}' | sort -n | tr '\n' ' ' | awk '{n=split($0,a," "); printf "runs: %s\nMEDIAN throughput (solves/sec) %s\n", $0, a[int((n+1)/2)]}'
