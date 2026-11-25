# Miri runs of November 25, 2026: the generators' progress reporter

[Safety overview](../../../safety/README.md) · [Undefined-behavior evidence](../../../safety/NO_UB.md) · [Miri rerun of November 23](../miri-20261123/README.md)

The commit that added the page's loading bars changed library code in three places, all of it safe code:

- `gen::progress::Overall`, a reporter that turns the generators' steps into one number in an atomic;
- two calls to the existing progress hooks in the prefix loop of `gen::dfs`;
- in `lanes::share`, four atomics that the search writes when an iteration is published and when jobs are claimed. These exist only on WebAssembly targets (`cfg(target_family = "wasm")`), so no native build, and no Miri run, contains them.

The full Miri catalogues were not rerun on it. The two unit tests of `Overall` were run under Miri (miri 0.1.0 of 2026-11-02, `-Zmiri-symbolic-alignment-check -Zmiri-num-cpus=4`) on the x86 backend (`-C target-cpu=x86-64-v3`) and on the portable one (`-C target-cpu=x86-64`), and passed on both: `gen::progress::tests::overall_follows_a_dfs_generation` and `overall_follows_a_bfs_generation`. The logs are `progress-tests-x86.log` and `progress-tests-portable.log`. They were made on the working tree of that commit before it was committed, which their first line says.

These are finite runs of unit tests in an interpreter, not a proof that the program has no undefined behavior. The WebAssembly-only atomics are covered by `wasm/test.mjs` (the words are read after a table and after a solve) and by the page's browser test, not by Miri.
