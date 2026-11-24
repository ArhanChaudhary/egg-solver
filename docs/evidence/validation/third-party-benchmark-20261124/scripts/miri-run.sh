#!/bin/bash
# Targeted Miri runs of the unit tests added in 5bfcd2a, on both backends:
# the shared-search test under 16 scheduler seeds, the tune-record tests once.
cd /data3/egg-solver-tune || exit 1
export PATH=$HOME/.cargo/bin:$PATH
strict="-Zmiri-symbolic-alignment-check -Zmiri-num-cpus=4"
for spec in "x86 x86-64-v3" "portable x86-64"; do set -- $spec
  log=/data3/build/perf/miri/tune-tests-$1.log
  {
    echo "commit $(git rev-parse --short HEAD); RUSTFLAGS=-C target-cpu=$2; MIRIFLAGS=$strict (+ -Zmiri-many-seeds=0..16 for the shared-search test)"
    cargo +nightly miri --version
    export RUSTFLAGS="-C target-cpu=$2" CARGO_TARGET_DIR=/data3/build/perf/target-miri-$1
    MIRIFLAGS="$strict -Zmiri-many-seeds=0..16" cargo +nightly miri test --lib -- lanes::share::tests::share_run_returns_the_work_of_its_lanes --exact --test-threads=1
    echo "exit status $?"
    MIRIFLAGS="$strict" cargo +nightly miri test --lib -- tables::file::tests::tune_files_are_read_by_mode tables::file::tests::nearest_thread_count_is_by_ratio tables::file::tests::default_lanes_use_the_tune_file --test-threads=1
    echo "exit status $?"
  } > $log 2>&1
done
grep -E "^test |test result|exit status|error" /data3/build/perf/miri/tune-tests-*.log | cut -c1-200
