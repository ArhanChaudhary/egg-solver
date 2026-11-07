# lints: clippy pedantic, std idioms instead of C idioms

Commit `b2f2c9a3dd1fd832b204ff7cf7ec42d64eb69fc5` · 2026-11-04T10:31:18+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

lints: clippy pedantic, std idioms instead of C idioms

Cargo.toml turns on `clippy::pedantic` (as warnings; check.sh's
`-D warnings` makes them errors), with `inline_always` allowed. The
crate-wide `needless_range_loop`/`manual_is_multiple_of` allows are gone.
Every warning is fixed:
- lossless casts use `T::from`; off the hot path, narrowing uses
  `try_from` (`?` with `io::Error::other` where an `io::Result` is
  returned); sign changes use `cast_signed`/`cast_unsigned`; pointer casts
  use `.cast()`/`ptr::from_ref`.
- hot kernels (`coord::cocsep*`, `Solver::value`, `Lane::advance`,
  `push_advance`, the AVX-512 helpers, ...) keep their casts under a
  function-level `#[expect(clippy::cast_*, reason = "...")]`. So do the
  const table builders: there is no `From`/`try_from` in const fns.
- `#[must_use]`, `# Panics`/`# Errors` docs, explicit imports instead of
  wildcards, `&mut v` loops, literal separators, `sort_unstable`, `map_or`,
  and a generic `arg` in bench (no f64 round trip for integer arguments).
- share.rs is pedantic-clean on main already (l6-share-3). Temporary:
  `advance`, `setup_run` and `Solver::dfs` get
  `expect(many_single_char_names)` until their rename commits.

Std idioms, each checked with cargo asm (the hot loops are unchanged):
- `x &= x - 1` -> `x ^= x.isolate_lowest_one()` (blsr either way).
- `x % n == 0` -> `x.is_multiple_of(n)`.
- `(1 << k) - 1` -> `uN::MAX.unbounded_shr(N - k)` (ALL_MOVES, eo_mask,
  split_index, sort_jobs).
- `Rng::below`: `next_u64().carrying_mul(n, 0).1`, the high half of the
  128-bit product.
- `quant`: a saturating `u8::try_from(..).unwrap_or(u8::MAX)` instead of a
  clamp and a cast.
- kept, with a comment saying why: `_bzhi_u32` in `advance` (the std form
  `todo & u32::MAX.unbounded_shr(32 - tz)` adds a test and a cmov at both
  inlined sites); `r1.wrapping_sub(1) >= maxval` (`!(1..=maxval).contains(&r1)`
  is the same test, but changes the register allocation of the whole stream
  and share functions); `max_dec` stays an i8 max (an i32 max adds register
  moves), with a debug assertion and a doc saying why `v - 1` cannot wrap
  (bounds are below 16).

Verification (this commit built fresh from its own tree):
- check.sh passes (fmt, clippy with pedantic, tests).
- verify.sh output identical to main 55e048e: nodes/cube 3058198
  lookups/cube 2588380, lengths 15:1 16:35 17:270 18:662 19:32;
  depth18 nodes/cube 3644301 lookups/cube 3084160, lengths 18:100.
- cargo asm: the stream loop (bench::stream_thread) and the share
  loop (Worker::rounds) are instruction-identical to main 55e048e
  (normalised for labels; symbols differ only by renames).

Perf, solves/s (stream, 24 lanes; a pure refactor, so neutral is
expected). Measured before the rebases onto 595e424, abce87c and
55e048e (bench's thread functions outlined; c8's child ordering;
l6's share.rs pass): main f2cb1f4 and this commit on f2cb1f4,
built in the same folder. The series as a whole is measured on
55e048e (layout-averaged) in the last commits.

| threads, build | runs | median |
|---|---|---|
| 1T main | 25.533 25.233 25.367 25.467 25.467 | 25.467 |
| 1T this commit | 25.367 24.883 25.067 25.200 25.250 | 25.200 (-1.0%) |
| 48T main | 1083.233 1078.067 1093.300 | 1083.233 |
| 48T this commit | 1074.500 1082.533 1084.700 | 1082.533 (-0.1%) |

1T: 5 rounds, the 5 builds of a round at once on separate L3 domains
(start order rotated per round). 48T: 3 interleaved rounds (ab48.sh).
The hot loops are instruction-identical to main, so a difference here is
code placement: layout alone moves 1T results by ~1.6% on this machine.
