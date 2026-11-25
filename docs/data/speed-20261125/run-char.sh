#!/bin/bash
# egg-solver's own speed on this machine: one cube at a time (-c 1) at 1, 8, 14 and 48 threads
# and a batch (-c 0) at 48 threads, on testdata/random_state.txt (the first 100 states at 1
# thread, all 500 otherwise), two rounds. Tables copied into RAM (the default load) on
# transparent huge pages. Run under flock -x /tmp/cube-bench.lock.
#   run-char.sh OUTDIR TABLE...   (TABLE: h6 .. h11 in /data3/build/tables)
OUT=$1; shift
S=/data3/build/perf/bin/17386a2-native/solve; T=/data3/build/tables; D=/data3/build/perf/char
mkdir -p $OUT
for k in "$@"; do
  # (pages and lanes, once per table)
  echo "R U" | $S -v --table $T/$k -t 48 -c 1 2>&1 >/dev/null | grep -E "pages|lanes per thread|loaded" > $OUT/$k-setup.txt
  for round in 1 2; do
    if [ $round = 1 ]; then order="1 8 14 48 b48"; else order="b48 48 14 8 1"; fi
    for cfg in $order; do
      case $cfg in
        b48) args="-t 48 -c 0"; set=all500 ;;
        1) args="-t 1 -c 1"; set=first100 ;;
        *) args="-t $cfg -c 1"; set=all500 ;;
      esac
      f=$OUT/$k-$cfg-r$round.jsonl
      $S --json --table $T/$k $args < $D/$set.txt > $f 2> $OUT/$k-$cfg-r$round.err
      echo "$k $cfg r$round $(date -u +%T) $(tail -1 $f | cut -c1-200)" >> $OUT/progress.log
    done
  done
done
echo "done $(date -u +%T)" >> $OUT/progress.log
