#!/bin/bash
# pgo-train2.sh: vcube and nissy training (nxopt's is done by pgo-train.sh). Instrumented vcube with
# 8 threads managed 2 cubes in 7 min (counter cache lines shared by the threads), so its training
# uses 2 threads on 6 cubes per table; nissy 8 threads on 10 cubes + 1 thread on 2.
P=/tmp/claude-1000/-home-super/059de2d4-71d2-490d-a4e0-08a2f8815035/scratchpad
F=/data/agents/f1
H=$F/cube_solver_test
D=$F/pgo
C=$F/tools/bin/capmem
T=$H/dataset/train_pgo.txt
run() { # memcap folder cmd...
	local m=$1 dir=$2; shift 2
	( cd $dir && LOCK_TIMEOUT=900 $P/bin/locked.sh /usr/bin/time -f "%e s wall, %U s user" $C $m "$@" )
}
rm -f $D/vcube-*.profraw
echo "vcube $(date +%T)"
head -6 $T | LLVM_PROFILE_FILE=$D/vcube-112-%p.profraw run 6G $H/vcube ./vc-optimal-pgogen -c 112 -w 2 > $D/train-vcube112.log 2>&1
head -6 $T | LLVM_PROFILE_FILE=$D/vcube-208-%p.profraw run 11G $H/vcube ./vc-optimal-pgogen -c 208 -w 2 > $D/train-vcube208.log 2>&1
llvm-profdata-18 merge -o $D/vcube.profdata $D/vcube-*.profraw
echo "nissy $(date +%T)"
head -10 $T | run 7G $H/nissy-core-pgo python3 cmd.py h48h7 8 > $D/train-h48h7-8.log 2>&1
tail -n +11 $T | head -2 | run 7G $H/nissy-core-pgo python3 cmd.py h48h7 1 > $D/train-h48h7-1.log 2>&1
echo "solved: vcube112 $(grep -cE '^[0-9]+ [0-9.]+ ' $D/train-vcube112.log) vcube208 $(grep -cE '^[0-9]+ [0-9.]+ ' $D/train-vcube208.log)" \
	"h48h7 $(grep -c '^Time' $D/train-h48h7-8.log)+$(grep -c '^Time' $D/train-h48h7-1.log)"
grep -h 's wall' $D/train-*.log
find $D/gcda -name '*.gcda' | wc -l; ls -la $D/vcube.profdata
echo "== done $(date +%T)"
