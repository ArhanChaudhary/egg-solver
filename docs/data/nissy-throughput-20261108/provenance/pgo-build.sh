#!/bin/bash
# pgo-build.sh gen|use: profile-guided builds of the open-source competitors, with their shipped
# flags otherwise (-march=native did not help any of them).
#   gen: instrumented binaries (nxopt31/22-pgogen, vcube/vc-optimal-pgogen, nissy-core-pgo with a
#        -fprofile-generate nissy.so); then pgo-train.sh runs them.
#   use: the PGO binaries nxopt31-pgo, nxopt22-pgo, vcube/vc-optimal-pgo, nissy-core-pgo.
# Plain (racy) counters: with -fprofile-update=atomic, 48 training threads contending on shared
# counters ran nxopt31 so slowly that no cube finished in 10 min; races only blur the counts,
# which -fprofile-correction tolerates. Training uses 8 threads (the multithreaded path).
set -e
F=/data/agents/f1
H=$F/cube_solver_test
D=$F/pgo
mode=$1
NXF="-DPS=16 -std=c++11 -O3 -g -march=native -DHALF -Wall -DTHREADS"   # makefile-nxopt's CXXFLAGS
case $mode in
gen) GCCP="-fprofile-generate=$D/gcda -fprofile-update=single"; CLANGP="-fprofile-instr-generate -fprofile-update=single"; SUF=-pgogen ;;
use) GCCP="-fprofile-use=$D/gcda -fprofile-correction -Wno-missing-profile"; CLANGP="-fprofile-instr-use=$D/vcube.profdata"; SUF=-pgo ;;
*) echo "usage: pgo-build.sh gen|use"; exit 2 ;;
esac

# nxopt: objects at fixed paths, so that the use build finds the gen build's .gcda files.
cd $H/cube20src
for v in 31:"-DEO3 -DEP1" 22:"-DEO2 -DEP2"; do
	n=${v%%:*}; defs=${v#*:}; o=$D/obj/nxopt$n; mkdir -p $o
	nice -n 39 g++ $NXF $defs $GCCP -c nxopt.cpp -o $o/nxopt.o
	nice -n 39 g++ $NXF $defs $GCCP -c cubepos.cpp -o $o/cubepos.o
	g++ $NXF $GCCP -o nxopt$n$SUF $o/nxopt.o $o/cubepos.o -lpthread
	ln -sfn /data/competitors/nxopt/nxopth${n}b.dat nxopth${n}b.dat
done

# vcube: its CMake build (clang, -O3 -mavx2 ...) with the profile flags added.
b=$D/build-vcube-$mode; rm -rf $b; mkdir -p $b
CPPUTEST_HOME=/nonexistent cmake -S $H/vcube -B $b -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DCMAKE_CXX_FLAGS="$CLANGP" > /dev/null
nice -n 39 make -C $b -j8 vc-optimal > /dev/null
cp $b/vc-optimal $H/vcube/vc-optimal$SUF

# nissy: build.sh's flags (cc -O3 -mavx2, THREADS=48), with the profile flags, in one folder for
# both steps (gcc names the .gcda files after the object's path).
if [ $mode = gen ]; then
	rm -rf $H/nissy-core-pgo
	rsync -a --exclude tables --exclude '*.o' --exclude python/nissy.so $H/nissy-core/ $H/nissy-core-pgo/
	ln -sfn /data2/competitors/h48 $H/nissy-core-pgo/tables
fi
cd $H/nissy-core-pgo && rm -f nissy.o python/nissy.so
THREADS=48 OPTIMIZE="-O3 $GCCP" PATH=$F/tools/bin:$PATH nice -n 39 ./build.sh python > $D/nissy-$mode.log 2>&1
ls -la $H/cube20src/nxopt*$SUF $H/vcube/vc-optimal$SUF $H/nissy-core-pgo/python/nissy.so
