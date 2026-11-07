#!/bin/bash
# Long cross-check of the solving modes (not part of check.sh): solves a cube
# set in stream mode, share mode (batch and one cube at a time) and with the
# recursive solver, and the first cubes with an independent plain IDA*
# (`examples/xsolve.rs`, mode `ref`). Every solution is verified, every cube
# must have the same length in every mode, and no run may hang.
#   scripts/stress.sh <table> [set] [threads] [lanes]
#     set: random:N:SEED (default random:5000:1), moves:N:LEN:SEED,
#          conj:N:SEED or a file of scrambles (see examples/xsolve.rs)
#   env: RUN (command prefix, e.g. "taskset -c 8-15"), TIMEOUT (s, per run),
#        NREF (cubes for the reference IDA*, default 200; 0 = none)
set -uo pipefail
cd "$(dirname "$0")/.."
table=${1:?usage: scripts/stress.sh TABLE [set] [threads] [lanes]}
set=${2:-random:5000:1}
threads=${3:-8}
lanes=${4:-20}
timeout=${TIMEOUT:-3600}
nref=${NREF:-200}
cargo build --release --example xsolve -q || exit 1
xsolve=${CARGO_TARGET_DIR:-target}/release/examples/xsolve
out=$(mktemp -d)
status=0
run() { # <name> <set> <mode> <threads> <lanes>
    ${RUN:-} timeout "$timeout" "$xsolve" "$table" "$2" "$3" "$4" "$5" > "$out/$1" 2> "$out/$1.err"
    local rc=$?
    echo "$1: rc $rc, $(tail -1 "$out/$1")"
    if [ $rc != 0 ]; then
        status=1
        [ $rc = 124 ] && echo "$1: TIMEOUT (hang?)"
        grep -m3 "panicked\|BAD\|MISSING" "$out/$1" "$out/$1.err"
    fi
}
# per-cube lengths of run $2 against run $1
compare() {
    local diff
    diff=$(join <(awk '/^[0-9]+ /{print $1, $2}' "$out/$1" | sort) \
        <(awk '/^[0-9]+ /{print $1, $2}' "$out/$2" | sort) | awk '$2 != $3')
    if [ -n "$diff" ]; then
        echo "$2 vs $1: LENGTH MISMATCH (cube, lengths):"
        echo "$diff" | head
        status=1
    fi
}
run stream "$set" stream "$threads" "$lanes"
run share0 "$set" share:0 "$threads" "$lanes"
run share1 "$set" share:1 "$threads" "$lanes"
run share2 "$set" share:2 3 3
run rec "$set" rec "$threads" 0
for r in share0 share1 share2 rec; do compare stream $r; done
if [ "$nref" -gt 0 ] && [[ $set == random:* ]]; then
    # (random:M:SEED is the first M cubes of random:N:SEED.)
    spec=${set#random:}
    n=${spec%%:*}
    run ref "random:$((n < nref ? n : nref)):${spec#*:}" ref "$threads" 0
    compare stream ref
fi
rm -r "$out"
[ $status = 0 ] && echo "STRESS OK" || echo "STRESS FAILED"
exit $status
