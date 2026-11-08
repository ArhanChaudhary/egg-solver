#!/bin/bash
# pgo-train.sh: profile training runs under locked.sh (bench lock shared, CPUs 0-7, bounded), 8
# threads each (the multithreaded path). Training cubes: random_state.txt lines 301-400
# (dataset/train_pgo.txt), disjoint from the evaluated cubes (the first 200); the same tables as
# the evaluation. The instrumented binaries are 1.5-3x slower, so the sets are small.
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
rm -rf $D/gcda $D/vcube-*.profraw
echo "nxopt31 $(date +%T)"; head -24 $T | run 6G $H/cube20src ./nxopt31-pgogen -t 8 - > $D/train-nxopt31.log 2>&1
echo "nxopt22 $(date +%T)"; head -24 $T | run 11G $H/cube20src ./nxopt22-pgogen -t 8 - > $D/train-nxopt22.log 2>&1
echo "vcube $(date +%T)"
head -24 $T | LLVM_PROFILE_FILE=$D/vcube-112-%p.profraw run 6G $H/vcube ./vc-optimal-pgogen -c 112 -w 8 > $D/train-vcube112.log 2>&1
head -24 $T | LLVM_PROFILE_FILE=$D/vcube-208-%p.profraw run 11G $H/vcube ./vc-optimal-pgogen -c 208 -w 8 > $D/train-vcube208.log 2>&1
llvm-profdata-18 merge -o $D/vcube.profdata $D/vcube-*.profraw
echo "nissy $(date +%T)"
head -20 $T | run 7G $H/nissy-core-pgo python3 cmd.py h48h7 8 > $D/train-h48h7-8.log 2>&1
tail -n +21 $T | head -3 | run 7G $H/nissy-core-pgo python3 cmd.py h48h7 1 > $D/train-h48h7-1.log 2>&1
echo "solved: nxopt31 $(grep -c '^Solved' $D/train-nxopt31.log) nxopt22 $(grep -c '^Solved' $D/train-nxopt22.log)" \
	"vcube112 $(grep -cE '^[0-9]+ [0-9.]+ ' $D/train-vcube112.log) vcube208 $(grep -cE '^[0-9]+ [0-9.]+ ' $D/train-vcube208.log)" \
	"h48h7 $(grep -c '^Time' $D/train-h48h7-8.log)+$(grep -c '^Time' $D/train-h48h7-1.log)"
grep -h 's wall' $D/train-*.log
find $D/gcda -name '*.gcda' | wc -l; ls -la $D/vcube.profdata
echo "== done $(date +%T)"
