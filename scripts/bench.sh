#!/bin/bash
# Repeated stream throughput measurements. Table selection is explicit.
# Usage: scripts/bench.sh TABLE [runs=3] [lanes=24] [threads=online CPUs]
set -euo pipefail
cd "$(dirname "$0")/.."
table=${1:?usage: scripts/bench.sh TABLE [runs] [lanes] [threads]}
runs=${2:-3}
lanes=${3:-24}
threads=${4:-$(getconf _NPROCESSORS_ONLN)}
for value in "$runs" "$threads"; do
    if [[ ! $value =~ ^[1-9][0-9]*$ ]]; then
        echo 'runs and threads must be positive integers' >&2
        exit 2
    fi
done
if [[ ! $lanes =~ ^[0-9]+$ ]]; then
    echo 'lanes must be a nonnegative integer' >&2
    exit 2
fi
cargo build --release --locked --bin bench
out=$(mktemp)
trap 'rm -f "$out"' EXIT
for ((i = 0; i < runs; i++)); do
    "${CARGO_TARGET_DIR:-target}/release/bench" "$table" stream 20 30 "$threads" "$lanes" | tee -a "$out"
done
awk '/^throughput / {print $2}' "$out" | sort -n | awk '
    {a[NR]=$1}
    END {
        if (!NR) exit 1
        median = NR % 2 ? a[(NR+1)/2] : (a[NR/2]+a[NR/2+1])/2
        printf "MEDIAN throughput (solves/sec) %.6f\n", median
    }'
