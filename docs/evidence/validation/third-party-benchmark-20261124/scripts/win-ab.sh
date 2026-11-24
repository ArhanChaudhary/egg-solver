#!/bin/bash
# Code A/B at 1 thread on one pinned core: three commits x two target CPUs, same table, same
# lanes, interleaved. usage: win-ab.sh CORE TABLE SET LANES ROUNDS OUT
CORE=$1; T=$2; SET=$3; L=$4; ROUNDS=$5; OUT=$6
B=/data3/build/perf/bin
for mode in 1 0; do
  for r in $(seq 1 $ROUNDS); do
    for v in 28a1953-native c3eb909-native dc153b3-native 28a1953-znver4 c3eb909-znver4 dc153b3-znver4 dc153b3-znver4 c3eb909-znver4 28a1953-znver4 dc153b3-native c3eb909-native 28a1953-native; do
      res=$(taskset -c $CORE $B/$v/solve --table $T -t 1 -c $mode -l $L < $SET 2>/dev/null | tail -1 | sed -E 's/total [0-9]+ cubes wall ([0-9.]+) .*nodes\/cube ([0-9]+).*/\1 \2/')
      echo "$(basename $T) $(basename $SET .txt) c$mode l$L $v $res" >> $OUT
    done
  done
done
echo "done $(date -u +%T)" >> $OUT
