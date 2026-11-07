# cubie: the cube model's constants from build.rs, a Move type; no panics in the model

Commit `ae25e0a20feddf89178fb4d9d50223f949133633` · 2026-11-05T20:40:45+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

cubie: the cube model's constants from build.rs, a Move type; no panics in the model

The user asked that the code have no intentional panicking paths. The
cube model and the checked move functions had several (`expect`,
`unwrap`, `unreachable!`, `assert!(mv < 18)`):

* build.rs derives the moves, the 48 symmetries, their inverses and the
  facelet tables of the pieces from the sticker model (cubie::model, now
  returning None instead of panicking: the build fails with a message),
  and writes them to $OUT_DIR/cubie_consts.rs, which the library includes
  under cfg(cubie_generated). The library no longer derives anything at
  run time (move_cubes, symmetries, symmetry_inverses are constants), and
  facelet strings are read and written through the tables. The SIMD
  constants (consts.rs) are byte-identical.
* `Move`, an enum of the 18 moves: `Cube::apply_move`,
  `premove_inverse` and `CubieCube::apply_move` take one, so they index
  the move tables without a check or an assert. `Move::lowest(mask)`
  replaces `trailing_zeros` in the reference solver's and the job
  extensions' mask loops (the same test as `mask != 0`).
  `CubieCube::apply_moves` returns None for a number that is no move.
* CubieCube's fields are crate-private: every way to make one gives a
  valid cube, so its pieces index the tables in range.
* symstart's move conjugates are the generated SYM_MOVE_MAP (they were
  the same table, computed at run time with an expect); class_key folds
  instead of min().unwrap().
* Rng::below_u8/below_usize: casts of values below n instead of
  try_from().expect().
