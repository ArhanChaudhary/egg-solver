#!/bin/bash
# Window 2: what the new `bench tune` picks, against real `solve` times over lane counts, and
# (AB=1) `bench --share` and `solve` of the fix against main's. All 48 cores; run under flock -x.
# usage: win2.sh TABLE(h7|h9) SET(19) THREADLIST("24 48") OUT [AB]
K=$1; SET=$2; TL=$3; OUT=$4; AB=${5:-0}
P=/data3/build/perf; B=$P/bin/tune-fix-native; O=$P/bin/dc153b3-native; T=$P/tune-tables/$K; S=/data2/tools/nissy-q/scrambles/scrambles-$SET.txt
echo "== tune $K $(date -u +%T)" >> $OUT
rm -f $T.tune
$B/bench tune $T >> $OUT 2>/dev/null
echo "-- file:" >> $OUT; cat $T.tune >> $OUT
cp $T.tune $P/out-2/$K.tune; rm -f $T.tune
echo "== sweep $K s$SET $(date -u +%T)" >> $OUT
for t in $TL; do for mode in 1 0; do for L in 8 12 16 20 24 32 40 48 48 40 32 24 20 16 12 8; do
  res=$($B/solve --table $T -t $t -c $mode -l $L < $S 2>/dev/null | tail -1 | sed -E 's/total [0-9]+ cubes wall ([0-9.]+) .*nodes\/cube ([0-9]+).*/\1 \2/')
  echo "$K s$SET c$mode l$L t$t $res" >> $OUT
done; done; done
if [ $AB = 1 ]; then
  echo "== ab $(date -u +%T)" >> $OUT
  for r in 1 2 3 4; do for v in dc153b3-native tune-fix-native tune-fix-native dc153b3-native; do
    for spec in "48 1" "48 0" "1 1"; do set -- $spec
      [ $1 = 1 ] && SS=/data2/tools/nissy-q/scrambles/scrambles-17.txt || SS=$S
      res=$($P/bin/$v/bench batch --share=$2 -t $1 -l 24 $T $SS 2>/dev/null | grep -E "^cubes " | head -1)
      echo "benchshare t$1 share$2 $v $res" >> $OUT
      res=$($P/bin/$v/solve --table $T -t $1 -c $2 -l 24 < $SS 2>/dev/null | tail -1 | sed -E 's/total [0-9]+ cubes wall ([0-9.]+) .*nodes\/cube ([0-9]+).*/\1 \2/')
      echo "solve t$1 c$2 $v $res" >> $OUT
    done
  done; done
fi
echo "done $(date -u +%T)" >> $OUT
