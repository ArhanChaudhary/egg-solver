# share: few cubes in progress; data structures

Commit `47b5da2fc3dff3b55df05926caf80274548a1593` · 2026-11-03T23:36:45+00:00

This commit is reachable from the released main history. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: Historical ancestor; no current branch tip

## Original experiment record

share: few cubes in progress; data structures

With few cubes in progress (at most threads / 8: one cube at a time with
all threads, or the end of a batch) the iterations are short and their
ends set the latency. Traces of one cube at a time: iteration 15 (243
jobs) spent ~230 us of ~270 us in its tail, iteration 16 ~100-300 us.

- Longest prefixes early: as soon as stream mode's prefixes are 2 moves
  (iteration 15 with h11), the jobs get 4 (43254 jobs): more and smaller
  jobs share an iteration better, and the extension is done once (by
  two moves in stages, one parent per task; the counts per parent come
  from a table).
- Builds are staged whenever few cubes are in progress (threads are
  about to wait), not only once threads wait: iteration 15's
  three-level extension took 1.4 ms serially when the last thread of
  iteration 14 found nobody hungry yet.
- Threads stay with (at most 2) unsolved cubes they have nothing to do
  on, and claim their next jobs there without searching the open list:
  no Arc clone, `parts` CAS, node-count RMWs and list lock per thread and
  iteration. Kept cubes are left once solved.

Data structures and synchronisation (from the pass over share.rs):

- Mailbox pieces: `Mutex<Option<Piece>>` with a `Vec<Node>` allocated
  per donation is replaced by a slot with a fixed `[Node; MAXD]` path,
  accessed as the IDLE/WANT/FILL/FULL state machine orders it (the donor
  that moved WANT to FILL writes it, the owner reads it once FULL).
- Claimed jobs are prefetched when claimed (their lines and their
  statistics', for writing): they were written by other threads (the
  sorted list by the build's tasks), and lanes start them one after
  another; before, only the next job was fetched.
- Rarely written fields of a cube (`listed`, the node counts summed by
  leaving threads, trace counters) and the pool's buffer free lists are
  on lines of their own, away from read-mostly fields.
- Checked, unchanged: the open list is read on refills that find no
  claimed or kept work (in a batch ~30 scans per cube), so a seqlock or
  epoch-published slots would not pay; `spare` (twice per cube),
  `bufs` (twice per staged build), `result` (once per cube) and
  `counters` (at the end) are uncontended mutexes. The per-job
  statistics are written by the thread that claimed the chunk (16 per
  line, chunks of up to 64). No SeqCst; on x86-64 the other orderings
  cost nothing (plain loads and stores, locked RMWs either way).

Ablations of the data-structure changes: at the end.

Measured on 2dbe81d (opt/r7-fresh-join = main cb8cb70 + opt/l6-share-a +
the r7 fresh-join fix) (h11 table; random_state = cube_solver_test
dataset/random_state.txt, 500 cubes). 48 threads: paired, i.e. in each
of 5 short exclusive holds every binary of the stack ran each test back
to back (the machine drifts by more than these differences between
holds); 1 thread: 3 runs pinned to free L3 slots. Every solution
verified; bench share lengths 15:1 16:35 17:270 18:662 19:32 in every
run. Values per run, and medians:

  48 threads                                  main -> this
    solve -c 0 (500), ms/cube    median   1.223 ->   1.230  (+0.6%)
      runs before: 1.254 1.193 1.223 1.214 1.255
      runs after:  1.230 1.232 1.231 1.165 1.218
    solve -c 1 -n 50, ms/cube    median   2.278 ->   2.401  (+5.4%)
      runs before: 2.319 2.180 2.273 2.278 2.332
      runs after:  2.640 2.401 2.823 2.319 2.400
    bench share 1000, solves/s   median   866.7 ->   869.2  (+0.3%)
      runs before: 865.8 866.7 871.7 871.6 865.9
      runs after:  869.2 871.5 869.5 869.0 863.9
  1 thread
    solve -c 0 -n 100, ms/cube   median   50.41 ->   50.11  (-0.6%)
      runs before: 50.20 50.71 50.41
      runs after:  50.11 50.46 49.58
    solve -c 1 -n 10, ms/cube    median   38.05 ->   38.05  (+0.0%)
      runs before: 38.35 38.05 38.05
      runs after:  38.21 37.88 38.05

The 4-move prefixes are dropped again by "share: no early 4-move
prefixes" (the last commit of this series): they cost more than they
saved for one cube at a time (-c 1 above).

Ablations of the data-structure changes: each removed from the final
stack (on 78ce33e, where the stack measured -c 0 1.352 1.361 ms, -c 1
2.359 2.417 ms, share 844.7 846.1/s), one 48-thread run each (screening:
the noise is +-3-5% here, so none is separated from the others; none is a
clear loss, and each removes work from a job or donation path):

                             -c 0 ms   -c 1 -n 50 ms   share /s
  without claim prefetch     1.387     3.059           798.6
  without kept cubes         1.295     2.462           841.1
  with the Mutex+Vec piece   1.289     2.407           827.0
  without the padding        1.391     2.461           786.6
