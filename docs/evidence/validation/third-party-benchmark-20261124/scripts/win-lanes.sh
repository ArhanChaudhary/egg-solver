#!/bin/bash
# Lane sweep at 1 thread on one pinned core, one cube at a time (-c 1) and batch (-c 0).
# usage: win-lanes.sh CORE BUILD OUT  (tables and sets fixed below)
CORE=$1; V=$2; OUT=$3
B=/data3/build/perf/bin/$V; T=/data3/build/tables; S=/data2/tools/nissy-q/scrambles
for spec in "h6 17" "h7 17" "h8 18" "h9 18"; do set -- $spec
  for mode in 1 0; do
    for L in 8 12 16 20 24 32 40 48 48 40 32 24 20 16 12 8; do
      res=$(taskset -c $CORE $B/solve --table $T/$1 -t 1 -c $mode -l $L < $S/scrambles-$2.txt 2>/dev/null | tail -1 | sed -E 's/total [0-9]+ cubes wall ([0-9.]+) .*nodes\/cube ([0-9]+).*/\1 \2/')
      echo "$1 s$2 c$mode l$L $V $res" >> $OUT
    done
  done
done
echo "done $(date -u +%T)" >> $OUT
