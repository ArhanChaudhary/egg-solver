# NEGATIVE l6: early-builds (not merged)

Commit `e8db5ed5c11b56b3bbba61f64a336b2ba24078f5` · 2026-11-04T08:04:29+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/l6-early-builds`

## Original experiment record

NEGATIVE l6: early-builds (not merged)

NEGATIVE: share: build the next iteration while the last jobs of the current one run (early builds)

On top of the l6 series (S4 = "share: the solver as a parameter of the visit loop"); share.rs only.

Idea: with few cubes in progress, when the claims of iteration T run out, start building T+1 in
stages (extension + sort) from the statistics as they are; jobs still running have the statistic
RUNNING (-0.0, which sorts first). A per-cube phase word (STAGED / BUILT / ENDED, NO_EARLY) makes
whoever sets the second of BUILT and ENDED publish T+1; the build state lives outside `Inner`
(which lanes read during the tail); the build holds a `parts` reference; build tasks of a solved
cube are drained as no-ops.

Result: no gain, and a rare crash left unresolved.
- solve -c 1 (48 threads, all 500 random_state cubes, one run each): 2.49 2.56 2.61 2.55 2.55
  2.59 ms/cube (S4-era code without early builds: ~2.33-2.42); -c 1 -n 50 screening run on
  78ce33e: 2.571 ms (S4: 2.359 2.417); -c 0 1.330 (S4 1.352 1.361); bench share 839.5 (S4 844.7
  846.1).
- Traces: early extensions for iteration 15 are slow (0.6-0.8 ms) because during the tail of
  iteration 14 few threads are fully idle (threads with any active lane do not run build tasks,
  and the idle ones take donated pieces first), and the T+1 order puts the jobs still running
  first, which costs some of the final iteration's ordering.
- Bugs found and fixed on the way (all in the early-build path): lost ENDED when an attempt for a
  j
