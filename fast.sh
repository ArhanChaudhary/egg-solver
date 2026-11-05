#!/usr/bin/env bash
# fast.sh: the fastest optimal solve of one random scramble this machine can do.
#
#   ./fast.sh                       # a random 25-move scramble
#   ./fast.sh "R U R' U' F2 D L'"   # your own scramble (R R' R2, or R1 R3 as in cube_solver_test)
#
# How it is fast:
#   - the largest pruning table available (h11, 58 GiB, on 1 GiB hugepages: fewest nodes per solve);
#   - share mode with -c 1: every thread works on the one cube (lowest latency, not throughput);
#   - a warm-up cube first in the same process, so threads, page tables and TLBs are hot when the
#     real cube starts; the time shown is the cube's own solve time (table load excluded).
#
# Settings (environment): CUBEOPT_TABLE=path (override the table), THREADS=n (default: all CPUs),
# FAST_NOLOCK=1 (do not wait for running benchmarks; see below).
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

# ---- the largest table: hK with the biggest K among the usual places
table=${CUBEOPT_TABLE:-}
if [ -z "$table" ]; then
    for k in 11 10 9 8 7 6 5 4 3 2 1 0; do
        for dir in /mnt/huge /data2/tables /data/tables tables; do
            if [ -f "$dir/h$k" ]; then table=$dir/h$k; break 2; fi
        done
    done
fi
[ -n "$table" ] || { echo "no table found; generate one, e.g.: target/release/gen 8 auto tables/h8" >&2; exit 1; }

# ---- the solver
if [ ! -x target/release/solve ]; then
    echo "building solve (release)..." >&2
    cargo build --release --bin solve -q
fi

# ---- the scramble: given, or 25 random moves (no face twice in a row, no X Y X on opposite faces)
if [ $# -gt 0 ]; then
    scramble="$*"
else
    faces=(U R F D L B) suffix=("" "2" "'")   # U/D, R/L, F/B are opposite pairs (index mod 3)
    scramble="" prev=-1 prev2=-1
    for _ in $(seq 25); do
        while :; do
            f=$(( $(od -An -N2 -tu2 /dev/urandom) % 6 ))
            [ "$f" -eq "$prev" ] && continue
            [ "$prev" -ge 0 ] && [ $((f % 3)) -eq $((prev % 3)) ] && [ "$f" -eq "$prev2" ] && continue
            break
        done
        scramble+="${faces[f]}${suffix[$(( $(od -An -N2 -tu2 /dev/urandom) % 3 ))]} "
        prev2=$prev prev=$f
    done
    scramble=${scramble% }
fi
threads=${THREADS:-$(nproc)}
warmup="R U2 F' L D2 B R' U F2 D' L2 B' U R2 F D L' B2 U' R F' D2 L B U2"

# ---- do not share the machine with a running benchmark (both results would be wrong)
run() { target/release/solve "$table" -t "$threads" -c 1; }
if [ -z "${FAST_NOLOCK:-}" ] && [ -e /tmp/cube-bench.lock ]; then
    echo "waiting for running benchmarks to finish (FAST_NOLOCK=1 skips this)..." >&2
    exec 9>>/tmp/cube-turn.lock; flock 9                       # queue fairly with the bench scripts
    exec 8>>/tmp/cube-bench.lock; flock 8; flock -u 9          # exclusive: nothing else runs
fi

echo "table    $table"
echo "threads  $threads (all on this one cube)"
echo "scramble $scramble"
out=$(printf '%s\n%s\n' "$warmup" "$scramble" | run)
# line format: solved <index> <length> <nodes> <seconds> <solution>
line=$(awk '$1 == "solved" && $2 == 1' <<< "$out")
[ -n "$line" ] || { echo "$out" >&2; echo "no solution line for the scramble" >&2; exit 1; }
read -r _ _ len nodes secs solution <<< "$line"
echo "solution $solution"
awk -v l="$len" -v n="$nodes" -v s="$secs" 'BEGIN {
    printf "length   %d moves (optimal, verified)\n", l
    printf "time     %.3f ms   (%.1f M nodes, %.0f M nodes/s)\n", s * 1000, n / 1e6, n / s / 1e6 }'
