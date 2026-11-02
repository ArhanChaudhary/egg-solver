#!/bin/bash
# Standard benchmark: 1000 uniformly random states (seed 2026), 48 threads,
# h11 table. Primary metric: steady-state throughput (solves/sec with all 48
# threads busy = 48 / mean per-solve time); batch wall-clock solves/sec is
# printed too. Prints the median over runs.
# Usage: ./bench.sh [table] [runs]
TABLE=${1:-/mnt/huge/h11}
RUNS=${2:-3}
cargo build --release -q || exit 1
OUT=$(for i in $(seq $RUNS); do ./target/release/bench $TABLE random:1000:2026 1000 48 2>/dev/null; done)
echo "$OUT" | grep -E "nodes/cube|lengths" | tail -2
echo "$OUT" | grep throughput | awk '{print $2}' | sort -n | awk '{a[NR]=$1} END {print "MEDIAN throughput (steady-state solves/sec)", a[int((NR+1)/2)]}'
echo "$OUT" | grep solves/sec | awk '{print $2}' | sort -n | awk '{a[NR]=$1} END {print "MEDIAN batch solves/sec", a[int((NR+1)/2)]}'
