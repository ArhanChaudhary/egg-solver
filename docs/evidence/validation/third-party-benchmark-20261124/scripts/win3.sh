#!/bin/bash
# Window 3: what the final `bench tune` (with the 1% rule) saves for h6..h9. All 48 cores; run
# under flock -x. usage: win3.sh OUT
OUT=$1; P=/data3/build/perf; B=$P/bin/tune-fix-native
for K in h6 h7 h8 h9; do
  T=$P/tune-tables/$K; rm -f $T.tune
  echo "== tune $K $(date -u +%T)" >> $OUT
  $B/bench tune $T >> $OUT 2>/dev/null
  echo "-- file:" >> $OUT; cat $T.tune >> $OUT; rm -f $T.tune
done
echo "done $(date -u +%T)" >> $OUT
