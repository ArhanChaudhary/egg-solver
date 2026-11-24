#!/bin/bash
# Lists of all optimal solutions on h7 with 16 threads on 16 pinned cores: egg-solver (current
# and the benchmarked c3eb909), nissy-core master as benchmarked (its limit of 999999 is cut to
# 63 solutions per cube) and with that cut removed. usage: win-lists.sh CORES OUT
CORES=$1; OUT=$2
E=/data3/build/perf/bin; T=/data3/build/tables/h7; P=/data3/build/perf; S=/data2/tools/nissy-q/scrambles
egg() { # label build file args...
  local label=$1 v=$2 f=$3; shift 3
  local t0=$(date +%s.%N)
  taskset -c $CORES $E/$v/solve --table $T -t 16 -c 1 "$@" < $f > $P/tmp-egg.out 2>/dev/null
  echo "egg $v $label $(basename $f .txt): $(grep ^total $P/tmp-egg.out | sed -E 's/total ([0-9]+) cubes wall ([0-9.]+) .*nodes\/cube ([0-9]+).*/\1 cubes, \2 s, \3 nodes\/cube/'), solutions $(grep -c '^solution ' $P/tmp-egg.out), truncated cubes $(grep -c '^truncated' $P/tmp-egg.out)" >> $OUT
}
nissy() { # label dir file
  local label=$1 d=$2 f=$3
  (cd $P/$d && taskset -c $CORES ./solve_file h48h7 $f 999999 0 > $P/tmp-nissy.out 2> $P/tmp-nissy.err)
  echo "nissy $label $(basename $f .txt): $(grep 'Total time' $P/tmp-nissy.out), solutions $(grep -c -E "^[UDRLFB]['2]?( [UDRLFB]['2]?)+$" $P/tmp-nissy.out)" >> $OUT
}
echo "start $(date -u +%T)" >> $OUT
egg one dc153b3-native $S/scrambles-18.txt
egg all-optimal dc153b3-native $S/scrambles-18.txt --all-optimal
egg all-optimal c3eb909-native $S/scrambles-18.txt --all-optimal
nissy capped nissy16 $S/scrambles-18.txt
nissy nocap nissy16-nocap $S/scrambles-18.txt
egg one dc153b3-native $P/sets/s19-sub.txt
egg all-optimal dc153b3-native $P/sets/s19-sub.txt --all-optimal
egg all-optimal-max63 dc153b3-native $P/sets/s19-sub.txt --all-optimal --max-solutions 63
nissy capped nissy16 $P/sets/s19-sub.txt
nissy nocap nissy16-nocap $P/sets/s19-sub.txt
egg one dc153b3-native $P/sets/s20-sub.txt
egg all-optimal dc153b3-native $P/sets/s20-sub.txt --all-optimal
egg all-optimal-max63 dc153b3-native $P/sets/s20-sub.txt --all-optimal --max-solutions 63
nissy capped nissy16 $P/sets/s20-sub.txt
nissy nocap nissy16-nocap $P/sets/s20-sub.txt
egg all-optimal dc153b3-native $S/superflip.txt --all-optimal
egg all-optimal c3eb909-native $S/superflip.txt --all-optimal
nissy capped nissy16 $S/superflip.txt
echo "done $(date -u +%T)" >> $OUT
