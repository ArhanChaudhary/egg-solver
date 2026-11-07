#!/bin/bash
# Check every Miri-enabled library/binary unit test, plus 16 scheduling
# seeds for shared-search tests. Compilation, worker and coverage failures
# all fail this script. Raw logs are retained for successful and failed runs.
#   scripts/miri.sh [seeds]   (default 0..16, exclusive upper bound)
# Miri cannot execute native inline assembly or Linux shared-memory calls.
# The hot index path has cfg(miri) intrinsic equivalents. Expensive tests
# and file tests explicitly ignored under cfg(miri) are listed in the logs.
# On x86 use the supported AVX2/BMI2 path; native AVX-512 needs native checks.
set -euo pipefail
cd "$(dirname "$0")/.."
seeds=${1:-0..16}
toolchain=${MIRI_TOOLCHAIN:-nightly}
# The shipped parking_lot dependency uses exposed provenance. Preserve
# Miri's default warnings and all UB checks; strict mode rejects its tagged
# pointers as unsupported. Set MIRI_STRICT_PROVENANCE=1 for supported tests.
strict="-Zmiri-symbolic-alignment-check -Zmiri-num-cpus=4"
if [ "${MIRI_STRICT_PROVENANCE:-0}" = 1 ]; then
    strict="$strict -Zmiri-strict-provenance"
fi
strict="$strict ${MIRI_EXTRA_FLAGS:-}"
if [ "${MIRI_RUSTFLAGS+x}" = x ]; then
    export RUSTFLAGS=$MIRI_RUSTFLAGS
elif rustc +"$toolchain" -vV | sed -n 's/^host: //p' | grep -q '^x86_64-'; then
    export RUSTFLAGS='-C target-cpu=x86-64-v3'
fi
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target/miri}
logs=${MIRI_LOG_DIR:-$(mktemp -d -t egg-solver-miri.XXXXXX)}
mkdir -p "$logs"
trap 'miri_result=$?; if [ "$miri_result" -ne 0 ]; then echo "Miri failed; full logs: $logs" >&2; fi' EXIT
printf 'Miri logs: %s\nToolchain: %s\nRUSTFLAGS: %s\nMIRIFLAGS: %s\nScheduler seeds: %s\n' \
    "$logs" "$toolchain" "${RUSTFLAGS:-}" "$strict" "$seeds"
cargo +"$toolchain" miri --version
rustc +"$toolchain" -vV
# These commands must succeed directly; filtering output must never turn a
# compiler failure into a successful prerequisite check.
cargo +"$toolchain" miri test --lib --bins --no-run > "$logs/build.log" 2>&1
MIRIFLAGS="$strict" cargo +"$toolchain" miri test --lib --bins -- --list > "$logs/catalogue.log" 2>&1
run() { # <miri flags> <test filters...>
    local flags=$1
    shift
    MIRIFLAGS="$flags" cargo +"$toolchain" miri test --lib -- "$@" --test-threads=1
}
# Run independent heavy test families concurrently, preserving every status.
# The datasets and checks are unchanged; this avoids a single group adding
# several long interpreted tests before another group can finish.
parallel_group() { # <group number> <family>
    local group=$1 family=$2
    local -a children=() shards=()
    local shard=0 group_failed=0
    shard_run() {
        shard=$((shard + 1))
        local path="$logs/$group-$shard.log"
        shards+=("$path")
        run "$strict" "$@" > "$path" 2>&1 & children+=("$!")
    }
    if [ "$family" = coordinates ]; then
        shard_run simd:: coord:: solver:: tables:: \
            --skip portable_check::coord::cross_check::portable_matches_x86 \
            --skip portable_check::coord::tests::pext_replacements_match \
            --skip tables::file::tests::checksum_sees_every_byte
        shard_run portable_check::coord::cross_check::portable_matches_x86
        shard_run portable_check::coord::tests::pext_replacements_match
        shard_run tables::file::tests::checksum_sees_every_byte
    else
        shard_run --skip cubie::tests --skip simd:: --skip coord:: --skip solver:: --skip tables:: \
            --skip lanes::tests:: --skip lanes::share::tests::staged \
            --skip lanes::share::tests::share_ --skip lanes::share::tests::first_pass_ \
            --skip lanes::share::tests::split_runs_cover_each_child_once --skip symstart::
        shard_run lanes::share::tests::split_runs_cover_each_child_once
        shard_run symstart:: --skip symstart::tests::superflip_jobs
        shard_run symstart::tests::superflip_jobs
    fi
    for child in "${children[@]}"; do
        if ! wait "$child"; then group_failed=1; fi
    done
    cat "${shards[@]}"
    [ "$group_failed" -eq 0 ]
}
pids=()
run "$strict" cubie::tests::symmetry > "$logs/1.log" 2>&1 & pids+=("$!")
run "$strict" cubie::tests --skip cubie::tests::symmetry > "$logs/2.log" 2>&1 & pids+=("$!")
parallel_group 3 coordinates > "$logs/3.log" 2>&1 & pids+=("$!")
run "$strict" lanes::tests:: > "$logs/4.log" 2>&1 & pids+=("$!")
run "$strict" lanes::share::tests::staged > "$logs/5.log" 2>&1 & pids+=("$!")
run "$strict -Zmiri-many-seeds=$seeds" lanes::share::tests::share_ > "$logs/6.log" 2>&1 & pids+=("$!")
run "$strict" lanes::share::tests::first_pass_ > "$logs/7.log" 2>&1 & pids+=("$!")
# Catch every remaining module/test, including newly added tests. The
# catalogue/coverage comparison below also catches accidentally empty groups.
parallel_group 8 remaining > "$logs/8.log" 2>&1 & pids+=("$!")
MIRIFLAGS="$strict" cargo +"$toolchain" miri test --bins -- --test-threads=1 > "$logs/9.log" 2>&1 & pids+=("$!")
failed=0
for miri_pid in "${pids[@]}"; do
    if ! wait "$miri_pid"; then failed=1; fi
done
for group in {1..9}; do
    printf '\nMiri group %s\n' "$group"
    cat "$logs/$group.log"
done
[ "$failed" -eq 0 ] || exit 1
LC_ALL=C awk '/: test$/ {sub(/: test$/, ""); print}' "$logs/catalogue.log" | LC_ALL=C sort -u > "$logs/expected.txt"
# Parallel Miri seeds can interleave test-start lines before printing `ok`.
# Only reach here after every harness succeeded; account for each started
# (or ignored) test even when several starts share an output line.
LC_ALL=C awk '{for (i = 1; i + 2 <= NF; ++i) if ($i == "test" && $(i + 2) == "...") print $(i + 1)}' "$logs"/{1..9}.log | \
    LC_ALL=C sort -u > "$logs/accounted.txt"
[ -s "$logs/expected.txt" ] || { echo 'Miri catalogue unexpectedly empty' >&2; exit 1; }
LC_ALL=C comm -23 "$logs/expected.txt" "$logs/accounted.txt" > "$logs/missing.txt"
if [ -s "$logs/missing.txt" ]; then
    echo 'Miri test coverage is incomplete:' >&2
    cat "$logs/missing.txt" >&2
    exit 1
fi
echo "MIRI CHECK OK; all catalogued unit tests accounted for; logs: $logs"
