#!/bin/bash
# What `bench tune` picks (default 6 s windows): twice on h6, once on h7, h8, h9, on one group
# of 8 cores (so "all threads" is 8). usage: win-tune.sh CORES OUT
CORES=$1; OUT=$2; B=/data3/build/perf/bin/dc153b3-native; T=/data3/build/perf/tune-tables
for t in h6 h6 h7 h8 h9; do
  echo "== tune $t $(date -u +%T)" >> $OUT
  taskset -c $CORES $B/bench tune $T/$t >> $OUT 2>/dev/null
  cat $T/$t.tune >> $OUT 2>/dev/null; rm -f $T/$t.tune
done
echo "done $(date -u +%T)" >> $OUT
