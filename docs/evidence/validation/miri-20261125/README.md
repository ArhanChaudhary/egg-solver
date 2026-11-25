# Miri runs of November 25, 2026: the generators' progress reporter

[Safety overview](../../../safety/README.md) · [Undefined-behavior evidence](../../../safety/NO_UB.md) · [Miri rerun of November 23](../miri-20261123/README.md)

Two commits of November 25 changed library code for the page's loading bar, all of it safe code: `17386a2` added the bar, and the commit after it made the bar fill without threads and made the generator report every step. Together:

- `gen::progress::Overall`, a reporter that turns the generators' steps into one number in an atomic;
- in the prefix of `gen::dfs`, progress reports, and a merge of the threads' sorted parts in place of sorting their concatenation (tables made by the old and the new code were compared byte for byte: h4 base 8, QTM h5 base 10, h6 base 9, all identical);
- a hook that WebAssembly builds without threads use to call out to their host (`cfg(target_family = "wasm")` only);
- in `lanes::share`, four atomics that the search writes when an iteration is published and when jobs are claimed. These exist only on WebAssembly targets (`cfg(target_family = "wasm")`), so no native build, and no Miri run, contains them.

The full Miri catalogues were not rerun on it. The two unit tests of `Overall` were run under Miri (miri 0.1.0 of 2026-11-02, `-Zmiri-symbolic-alignment-check -Zmiri-num-cpus=4`) on the x86 backend (`-C target-cpu=x86-64-v3`) and on the portable one (`-C target-cpu=x86-64`), and passed on both: `gen::progress::tests::overall_follows_a_dfs_generation` and `overall_follows_a_bfs_generation`. The logs are `progress-tests-x86.log` and `progress-tests-portable.log`. They were made on the working tree of the second commit before it was committed (its first line says so), and replace the logs of the same tests on the first.

These are finite runs of unit tests in an interpreter, not a proof that the program has no undefined behavior. The WebAssembly-only atomics are covered by `wasm/test.mjs` (the words are read after a table and after a solve) and by the page's browser test, not by Miri.
