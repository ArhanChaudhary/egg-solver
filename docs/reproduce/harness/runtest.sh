#!/bin/bash

# Previous run names remain aliases for recorded benchmark matrices.
case "${1:-}" in cubeoptrs*) set -- "eggsolver${1#cubeoptrs}" "${@:2}" ;; esac

if [ -z $ntest ]; then
	ntest=10
fi

if [ -z $ngroup ]; then
	ngroup=1
fi

if [ -z $nthread ]; then
	nthread=1
fi

if [ -z $maxl ]; then
	maxl=21
fi

# Build variants (env): vcube_bin=vc-optimal-native, h48_dir=nissy-core-native select the
# -O3 -march=native builds next to the shipped ones (final-compare.sh's "native" build column);
# nxopt_suffix=-pgo, vcube_bin=vc-optimal-pgo, h48_dir=nissy-core-pgo the profile-guided builds.
# The raw solver output of every run is appended to $rawlog when it is set (final-compare.sh
# uses it to cross-check solution lengths); the summary format below is unchanged.
function raw() {
	if [ -n "$rawlog" ]; then tee -a "$rawlog"; else cat; fi
}

function nxopt() (
	echo "test nxopt$2 $1"
	dataset=$(pwd)/dataset/$1.txt
	cd cube20src
	cat $dataset | head -n$ntest |\
	./nxopt$2${nxopt_suffix} -t $nthread - | raw |\
	awk '/Solved/ {
		cnt += 1;
		node += $8;
		tt += $(10);
		printf("%03d %.3fM nodes, %0.3f ns/node\n", $2 - 1, $8/1e6, $(10)*1e9/$8/'$nthread');
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node/'$nthread', tt/cnt/'$nthread');
	}'
)

function vcube() (
	echo "test vcube$2 $1"
	dataset=$(pwd)/dataset/$1.txt
	cd vcube
	cat $dataset | head -n$ntest |\
	./${vcube_bin:-vc-optimal} -c $2 -w $nthread 2>/dev/null | raw |\
	awk '/^[0-9]/ {
		cnt += 1;
		node += $3;
		tt += $2;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", $1, $3/1e6, $2*1e9/$3/'$nthread');
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node/'$nthread', tt/cnt/'$nthread');
	}'
)

function reid() (
	echo "test reid $1"
	dataset=$(pwd)/dataset/$1.txt
	cd reid
	(echo "R"; cat $dataset | head -n$ntest) |\
	sed "s/1/ /g" | sed "s/3/'/g" |\
	./twist |\
	awk 'v==1 {print; v=0;} /:/ {v=1}' |\
	./solve |\
	awk '@load "time"; /solution found/ {
		cnt += 1;
		if (cnt == 1) {
			tref = gettimeofday();
		} else {
			gsub(",", "", $5);
			node += $5
			tinc = gettimeofday() - tref;
			tref = gettimeofday();
			tt += tinc;
			printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 2, $5/1e6, tinc*1e9/$5);
			system("");
		}
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node, tt/cnt);
	}'
)

function h48() (
	echo "test h48$2 $1"
	dataset=$(pwd)/dataset/$1.txt
	cd ${h48_dir:-nissy-core}
	cat $dataset | head -n$ntest |\
	python3 cmd.py h48$2 $nthread 2>&1 | raw |\
	awk '/Nodes visited/ {
		cnt += 1;
		node += $5;
		curnode = $5;
	} /^Time/ {
		tt += $2;
		curtt = $2;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, curnode/1e6, curtt*1e9/curnode);
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node, tt/cnt);
	}'
)

function nissy-classic() (
	echo "test nissy-classic $1"
	dataset=$(pwd)/dataset/$1.txt
	cd nissy-classic
	cat $dataset | head -n$ntest | sed "s/1/ /g" | sed "s/3/'/g" | awk '{print "solve optimal -t '$nthread' "$0}' |\
	./nissy 2>&1 |\
	awk '/Search done/ {
		curnode = $3;
		curtt = $7 / 1000000;
		cnt += 1;
		tt += curtt;
		node += curnode;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, curnode/1e6, curtt*1e9/curnode);
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node, tt/cnt);
	}'
)

# (The cubeopt30 / cube48opt10 binaries compared here parse only -l -m -n -t -v: -g is ignored,
# so they have no concurrent-cube mode; keep ngroup=1.)
function cubeopt() (
	echo "test cube$3$2 $1"
	dataset=$(pwd)/dataset/$1.txt
	cd ../cubeopt
	cat $dataset | head -n$ntest |\
	./cube$3$2 -t $nthread -g $ngroup -m $maxl - | raw |\
	awk '/(finished|unsolved)/ {
		split($5, s, "/");
		split($7, t, "=");
		cnt += 1;
		node += s[1];
		tt += t[2] / 1000;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, s[1]/1e6, t[2]*1e6/(s[1] + 1)/'$ngroup');
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node/'$ngroup', tt/cnt);
	}'
)

function kocpy() (
	echo "test kocpy $1"
	dataset=$(pwd)/dataset/$1.txt
	cd RubiksCube-OptimalSolver
	cat $dataset | head -n$ntest |\
	pypy3 cmd.py |\
	awk '/total time/ {
		cnt += 1;
		node += $7;
		tt += $3;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, $7/1e6, $3*1e9/$7);
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node, tt/cnt);
	}'
)

function kocce() (
	echo "test kocce$2 $1"
	dataset=$(pwd)/dataset/$1.txt
	cd CubeExplorer
	make
	cmd="./CommandMain"
	arg=''
	if [[ "$2" =~ ^1$ ]]; then
		arg='-b'
	elif [[ "$2" =~ ^2$ ]]; then
		cmd=./CommandMainHuge
		arg='-b'
	fi
	cat $dataset | head -n$ntest |\
	$cmd $arg -t $nthread |\
	awk '/Solved/ {
		cnt += 1;
		node += $6;
		tt += $8/'$nthread';
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, $6/1e6, $8*1e9/$6/'$nthread');
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node, tt/cnt);
	}'
)

function stickersolve() (
	echo "test stickersolve $1"
	dataset=$(pwd)/dataset/$1.txt
	cd ../stickersolve/build
	cat $dataset | head -n$ntest |\
	../stickersolve -t $nthread - 2>/dev/null |\
	awk '/Solution found/ {
		split($5, s, "/");
		split($7, t, "=");
		cnt += 1;
		node += $4;
		tt += $7;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, $4/1e6, $7*1e6/$4);
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e3/node, tt/cnt);
	}'
)

function mf3color() {
	echo "test mf3color"
	dataset=$(pwd)/dataset/$1.txt
	cd 3ColorCube
	python3 -m http.server -b localhost 8000 2>&1 > /dev/null &
	jpid="$!"
	cat $dataset | head -n$ntest |\
	sed "s/1/ /g" | sed "s/3/'/g" |\
	node mf3color.benchmark.js -t $nthread |\
	awk '/Solved/ {
		cnt += 1;
		node += $3;
		tt += $5 / 1000;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, $3/1e6, $5*1e6/$3);
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node, tt/cnt);
	}'
	kill $jpid
}

function optjava() {
	echo "test optjava $1"
	dataset=$(pwd)/dataset/$1.txt
	cd optjava
	cat $dataset | head -n$ntest | awk '{print 1; print; print 1}' |\
	java -Xmx8g -cp out cube.app.Main 2>&1 |\
	awk '/total time/ {
		cnt += 1;
		node += $7;
		tt += $3;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", cnt - 1, $7/1e6, $3*1e9/$7);
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.3fs\n\n", node/1e6/cnt, tt*1e9/node, tt/cnt);
	}'
}

# egg-solver (Rust; H48-style hN tables, IDA* with 24-40 interleaved lanes per thread).
#   eggsolverh7 .. eggsolverh11       `solve -c 1`: one cube at a time on all nthread threads
#                                     (per-cube latency, as h48 and cube48opt measure it).
#   eggsolverthph7 ..                 the same with the table copied into anonymous THP
#                                     (madvise) memory instead of mapped from hugetlbfs.
#   eggsolverbatchh7 ..               throughput: `solve -c 0`, the whole input in flight at once
#                                     (comparable to nxopt -t N / vcube -w N, which solve one cube
#                                     per thread). Report separately from the latency runs.
#   eggsolverstreamh7 ..              throughput: `bench stream` (endless random states, steady
#                                     state over $eggsolver_window s; ignores the dataset).
#   eggsolverbatchthphN, eggsolverstreamthphN: the same on a THP copy of the table.
# tt: latency modes = mean of the per-cube solve times (as h48/cubeopt); batch = wall / cube;
# stream = 1 / throughput. The table load is never included.
# ns/node: per cube, its time / its nodes; Avg: total wall / total nodes (all threads).
# Env: eggsolver_bin (folder with solve and bench; default $HOME/egg-solver/target/release),
#      eggsolver_tables (folder with the hN table files; default /mnt/huge),
#      eggsolver_lanes (lanes per thread; default the table's tuned or untuned default),
#      eggsolver_window ("warmup window" seconds for stream; default "10 30").
function eggsolver() (
	eggsolver_lanes=${eggsolver_lanes:-${cubeoptrs_lanes:-}}
	eggsolver_window=${eggsolver_window:-${cubeoptrs_window:-10 30}}
	echo "test eggsolver$3$2 $1"
	dataset=$(pwd)/dataset/$1.txt
	bin=${eggsolver_bin:-${cubeoptrs_bin:-$HOME/egg-solver/target/release}}
	table=${eggsolver_tables:-${cubeoptrs_tables:-/mnt/huge}}/$2
	load=${EGG_SOLVER_LOAD-${CUBEOPT_LOAD-auto}}
	[[ "$3" == *thp ]] && load=copy
	if [[ "$3" == stream* ]]; then
		EGG_SOLVER_LOAD=$load CUBEOPT_LOAD=$load $bin/bench $table stream ${eggsolver_window:-10 30} $nthread $eggsolver_lanes 2>&1 | raw |\
		awk '/^nodes\/cube/ { node = $2; } /^throughput/ { tp = $2; } END {
			if (tp > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.6fs\n\n", node/1e6, 1e9/(tp*node), 1/tp);
		}'
		return
	fi
	conc=1
	[[ "$3" == batch* ]] && conc=0
	cat $dataset | head -n$ntest |\
	EGG_SOLVER_LOAD=$load CUBEOPT_LOAD=$load $bin/solve $table -t $nthread -c $conc ${eggsolver_lanes:+-l $eggsolver_lanes} 2>&1 | raw |\
	awk -v conc=$conc '/^solved/ {
		cnt += 1;
		node += $4;
		sec += $5;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", $2, $4/1e6, $5*1e9/$4);
		system("");
	} /^total/ {
		tt = $7;
	} END {
		# Latency (-c 1): the mean of the per-cube times, measured as h48 and cubeopt measure
		# theirs (around each solve, table load excluded). Batch (-c 0): wall / cube.
		if (conc == 1 && cnt > 0) tt = sec / cnt;
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.6fs\n\n", node/1e6/cnt, tt*1e9*cnt/node, tt);
	}'
)

# cube48opt-rust (github.com/2017YANR02/cuberoot.me, solver/experiments/cube48opt-rust, 1effb34): a
# Rust port of cube48opt1..9 with its own Opt10 and a nissy-H48 backend. Names:
#   c48rs<v>       latency: one cube at a time on CUBEOPT_SEARCH_THREADS = min(nthread, 18) threads
#                  (18 is the solver's cap, src/native_parallel.rs:13);
#   c48rsbatch<v>  throughput: CUBEOPT_SOLVE_THREADS = nthread cubes at once, one per thread;
#   <v> = opt7|opt8|opt9|opt10 (cube48opt tables + the WASM's small-memory.bin; opt10 is its own
#   EO10 format) or h7..h10 (its H48 backend on nissy's h48hN files).
# Input id<TAB>scramble in U U2 U' notation; 8 interleaved contexts, depth 20, no node limit. Output:
# one JSON line per cube; "seconds" is measured around the solve (no table load), "nodes" are the
# states it inspected. tt: latency = mean per-cube seconds (as h48/cubeopt); batch = sum of per-cube
# seconds / threads (as nxopt/vcube).
# Env: c48rs_dir (bin/ and tables; default /mnt/egg/super/xcr/cube48opt-rust), c48rs_build
#      (""|nat|pgo: binary suffix, e.g. c48rs-opt7-nat), c48rs_threads (search threads instead of min(nthread, 18)),
#      c48rs_simd (CUBEOPT_SIMD: auto|avx2|avx512|scalar; default auto = AVX2 on this machine).
function c48rs() (
	echo "test c48rs$3$2 $1"
	dataset=$(pwd)/dataset/$1.txt
	d=${c48rs_dir:-/mnt/egg/super/xcr/cube48opt-rust}
	bin=$d/bin/c48rs-$2${c48rs_build:+-$c48rs_build}
	case $2 in
	# Opt10: cube48opt10's own raw table (byte-identical to cube48opt-rust's Opt10 payload once its
	# unused block minima 15 are written as 10), read by the loader-only shim build
	# (CUBEOPT_EO_HEADER = its 1 KiB header in a side file; see xcr-bench-entry.md).
	opt10) cmd="solve-opt10 $d/opt9-small/small-memory.bin ${c48rs_opt10:-/data2/competitors/cube48opt10/h48prun31h1.dat}"
		export CUBEOPT_EO_HEADER=$d/opt10/h48prun31h1.header ;;
	opt*) cmd="solve $d/$2/small-memory.bin $d/$2/h48prun31h${2#opt}.dat" ;;
	h*) cmd="solve-h48 ${2#h} ${c48rs_h48:-/data2/competitors/h48}/h48$2" ;;
	esac
	if [ "$3" = batch ]; then
		par="CUBEOPT_SOLVE_THREADS=$nthread CUBEOPT_SEARCH_THREADS=1"; div=$nthread
	else
		st=${c48rs_threads:-$(( nthread < 18 ? nthread : 18 ))}
		par="CUBEOPT_SOLVE_THREADS=1 CUBEOPT_SEARCH_THREADS=$st"; div=1
	fi
	cat $dataset | head -n$ntest |
	awk '{ gsub(/1/, ""); gsub(/3/, "\047"); printf("%d\t%s\n", NR - 1, $0) }' |
	env $par CUBEOPT_VARIANT=${2#opt} CUBEOPT_BACKEND=$2 CUBEOPT_SIMD=${c48rs_simd:-auto} \
		$bin $cmd 8 20 0 2>&1 | raw |
	awk -v div=$div '/^\{"id"/ {
		match($0, /"id":"[0-9]+"/); id = substr($0, RSTART + 6, RLENGTH - 7) + 0;
		match($0, /"seconds":[0-9.]+/); sec = substr($0, RSTART + 10, RLENGTH - 10) + 0;
		match($0, /"nodes":[0-9]+/); n = substr($0, RSTART + 8, RLENGTH - 8) + 0;
		cnt += 1; node += n; tt += sec;
		printf("%03d %.3fM nodes, %0.3f ns/node\n", id, n/1e6, sec*1e9/(n + 1)/div);
		system("");
	} END {
		if (cnt > 0) printf("Avg %.3fM nodes, %0.3f ns/node, tt=%0.6fs\n\n", node/1e6/cnt, tt*1e9/node/div, tt/cnt/div);
	}'
)

function runtest() {
	if [[ "$1" =~ ^nxopt([0-9]{2})$ ]]; then
		nxopt $2 ${BASH_REMATCH[1]}
	elif [[ "$1" =~ ^vcube([0-9]{3})$ ]]; then
		vcube $2 ${BASH_REMATCH[1]}
	elif [[ "$1" =~ ^reid$ ]]; then
		reid $2
	elif [[ "$1" =~ ^kocpy$ ]]; then
		kocpy $2
	elif [[ "$1" =~ ^kocce([012])$ ]]; then
		kocce $2 ${BASH_REMATCH[1]}
	elif [[ "$1" =~ ^stickersolve$ ]]; then
		stickersolve $2
	elif [[ "$1" =~ ^h48(h[0-9]{1,2})$ ]]; then
		h48 $2 ${BASH_REMATCH[1]}
	elif [[ "$1" =~ ^nissy-classic$ ]]; then
		nissy-classic $2
	elif [[ "$1" =~ ^mf3color$ ]]; then
		mf3color $2
	elif [[ "$1" =~ ^cubeopt([0-9]{2})$ ]]; then
		cubeopt $2 ${BASH_REMATCH[1]} "opt"
	elif [[ "$1" =~ ^cube48opt([0-9]{1,2})$ ]]; then
		cubeopt $2 ${BASH_REMATCH[1]} "48opt"
	elif [[ "$1" =~ ^cubenp([0-9]{2})$ ]]; then
		cubeopt $2 ${BASH_REMATCH[1]} "np"
	elif [[ "$1" =~ ^eggsolver(h[0-9]{1,2})$ ]]; then
		eggsolver $2 ${BASH_REMATCH[1]} ""
	elif [[ "$1" =~ ^eggsolver(thp|batch|batchthp|stream|streamthp)(h[0-9]{1,2})$ ]]; then
		eggsolver $2 ${BASH_REMATCH[2]} ${BASH_REMATCH[1]}
	elif [[ "$1" =~ ^c48rs(batch)?(opt[0-9]{1,2}|h[0-9]{1,2})$ ]]; then
		c48rs $2 ${BASH_REMATCH[2]} "${BASH_REMATCH[1]}"
	elif [[ "$1" =~ ^optjava$ ]]; then
		optjava $2 ${BASH_REMATCH[1]}
	fi
}

mkdir -p results/

runtest $1 $2 | tee results/"$1"_"$nthread"_"$2".txt

# runtest nxopt11 random_move_15f
# runtest vcube104 random_move_15f
# runtest h48h0 random_move_15f
# runtest kocpy random_move_15f
# runtest reid random_move_15f
# ntest=500 nthread=12 bash runtest.sh vcube112 random_move_16f
# ntest=200 nthread=48 bash runtest.sh eggsolverh11 random_state

