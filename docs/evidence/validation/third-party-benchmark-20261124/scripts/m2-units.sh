#!/bin/bash
# M2 timed units for the third-party benchmark question. usage: m2-units.sh a|b
# (run under flock -x /tmp/m2-bench.lock; P-cores are cpu4-7)
cd ~/egg; U=$1; O=out/m2-$U.txt; : > $O; START=$(date +%s)
left() { [ $(( $(date +%s) - START )) -lt ${1:-1020} ]; }
run() { # cores threads build table set mode lanes
  local res=$(taskset -c $1 bin/$3/solve --table tables/$4 -t $2 -c $6 -l $7 < scr/scrambles-$5.txt 2>/dev/null | tail -1 | sed -E 's/total [0-9]+ cubes wall ([0-9.]+) .*nodes\/cube ([0-9]+).*/\1 \2/')
  echo "$4 s$5 c$6 l$7 t$2 cores$1 $3 $res" >> $O
}
echo "start $(date -u +%T) load $(cut -d' ' -f1-3 /proc/loadavg) avail $(awk '/MemAvailable/{print int($2/1024)}' /proc/meminfo) MB" >> $O
if [ $U = a ]; then
  for mode in 1 0; do for r in 1 2 3; do for v in 28a1953 c3eb909 dc153b3 dc153b3 c3eb909 28a1953; do left && run 4 1 $v h6 17 $mode 40; done; done; done
  echo "ab done $(date -u +%T)" >> $O
  for mode in 1 0; do for L in 8 12 16 20 24 32 40 48 48 40 32 24 20 16 12 8; do left && run 4 1 dc153b3 h6 17 $mode $L; done; done
  echo "lanes h6 done $(date -u +%T)" >> $O
  for L in 8 12 16 20 24 32 40 48 48 40 32 24 20 16 12 8; do left && run 4 1 dc153b3 h7 17 1 $L; done
else
  for spec in "4-7 4" "0-7 8"; do set -- $spec
    for mode in 1 0; do for L in 16 24 32 40 40 32 24 16; do left 600 && run $1 $2 dc153b3 h7 18 $mode $L; done; done
    for mode in 1 0; do for v in 28a1953 c3eb909 dc153b3 dc153b3 c3eb909 28a1953; do left 600 && run $1 $2 $v h7 18 $mode 40; done; done
  done
  echo "threads done $(date -u +%T)" >> $O
  for t in h6 h7; do
    echo "== tune $t $(date -u +%T)" >> $O
    bin/dc153b3/bench tune tables/$t >> $O 2>/dev/null; cat tables/$t.tune >> $O 2>/dev/null; rm -f tables/$t.tune
  done
fi
echo "end $(date -u +%T) load $(cut -d' ' -f1-3 /proc/loadavg)" >> $O
