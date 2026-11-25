#!/bin/bash
# M2 unit (run under flock -x /tmp/m2-bench.lock): build 17386a2, then egg-solver's speed on
# testdata/random_state.txt: one cube at a time at 1 thread (first 50 states, a performance
# core), 4 threads (the performance cores) and 8 threads (first 200), and a batch at 8 threads
# (first 200); h6 and h7 copied into RAM.
set -u
cd ~/egg; export RUSTUP_HOME=$PWD/rustup CARGO_HOME=$PWD/cargo; export PATH=$CARGO_HOME/bin:$PATH
O=out/char; mkdir -p $O; echo "start $(date -u +%T) load $(cut -d' ' -f1-3 /proc/loadavg) avail $(awk '/MemAvailable/{print int($2/1024)}' /proc/meminfo) MB" > $O/log.txt
if [ ! -x bin/17386a2/solve ]; then
  rm -rf wt && cp -r src/17386a2 wt && (cd wt && CARGO_TARGET_DIR=$HOME/egg/target cargo build -q --release --locked --bin solve 2>&1 | tail -3)
  mkdir -p bin/17386a2 && cp target/release/solve bin/17386a2/ && echo "built $(date -u +%T)" >> $O/log.txt
fi
head -50 scr/random_state.txt > scr/first50.txt; head -200 scr/random_state.txt > scr/first200.txt
S=bin/17386a2/solve
for k in h6 h7; do
  echo "R U" | $S -v --table tables/$k -t 8 -c 1 2>&1 >/dev/null | grep -E "pages|lanes per thread|loaded" > $O/$k-setup.txt
  for spec in "1 1 4 first50" "4 1 4-7 first200" "8 1 0-7 first200" "8 0 0-7 first200"; do set -- $spec
    taskset -c $3 $S --json --table tables/$k -t $1 -c $2 < scr/$4.txt > $O/$k-t$1-c$2.jsonl 2> $O/$k-t$1-c$2.err
    echo "$k t$1 c$2 $(date -u +%T) $(tail -1 $O/$k-t$1-c$2.jsonl | cut -c1-160)" >> $O/log.txt
  done
done
echo "end $(date -u +%T) load $(cut -d' ' -f1-3 /proc/loadavg)" >> $O/log.txt
