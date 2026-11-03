//! Cooperative batch and latency mode: the lanes of all threads share the
//! work of a set of active cubes.
//!
//! In stream mode (`run_lanes`) every lane solves its own cube: ideal for an
//! endless stream, but a finite batch waits for its slowest cubes, each in a
//! single lane. Here:
//!
//! * A cube's search is the same sequence of iterations (target T) of sorted
//!   jobs as in stream mode, but the job list lives in a shared `CubeCtx`.
//!   Threads claim jobs in chunks (one atomic per chunk, smaller chunks as
//!   the iteration runs out) and start them in their lanes at job
//!   boundaries.
//! * Iteration T+1 is built (jobs extended, sorted by the statistics of
//!   iteration T) by the thread that finishes the last job of iteration T. A
//!   solution found in iteration T is therefore optimal and is reported at
//!   once; lanes still running jobs of the cube drop them (lanes are checked
//!   once per round after a cube is solved). If threads are waiting for work,
//!   a long job list is extended and sorted by all of them (`build_task`).
//!   Jobs whose bounds exceed the iteration are sorted last and not claimed.
//! * A lane whose job ends takes, in order: a piece donated to its thread,
//!   another job of the same cube, a job its thread already claimed, a fresh
//!   cube (at most `max_active` cubes are in progress), jobs of the oldest
//!   cube with unclaimed jobs, or part of the remaining moves of a busy lane
//!   of its thread: the upper half of the faces left at its shallowest node
//!   with two faces left (cube48opt's `split_ida_node`). Otherwise it goes
//!   idle.
//! * A thread without active lanes asks for work in its mailbox; busy
//!   threads check once per round whether some thread is waiting and donate
//!   it a split of one of their lanes (a copy of the path to the split
//!   node). Waiting threads spin briefly, then yield.
//! * The job buffers of reported cubes are reused by new ones, the other
//!   build buffers by later builds: builds touch no fresh pages.
//!
//! Each job runs exactly once; its statistic is the sum over its pieces. A
//! cube's node count includes work done on it after its solution was found
//! by another lane. The search kernel is the one of stream mode: `advance`
//! of a `Lane::shared` lane only differs in returning at the end of a job,
//! and everything here runs at job boundaries or once per round.

use super::{job_weight, Job, Lane, Node, Phase, Step, MAXD};
use crate::cubie::CubieCube;
use crate::simd::Cube;
use crate::solver::{Solver, Stats, ALLOWED_AFTER, ALL_MOVES};
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Parents per task of a parallel job extension, and the smallest job lists
/// extended or sorted in parallel (when threads are waiting for work).
const EXT_CHUNK: usize = 16;
const EXT_MIN: usize = 128;
/// (Extensions by two moves: one parent per task.)
const EXT_MIN2: usize = 16;
const SORT_MIN: usize = 8192;
/// Most jobs a thread claims at once.
const CHUNK_MAX: u32 = 64;
/// Smallest remaining depth of a node split for another lane of the same
/// thread, and for another thread.
const SPLIT_R_LOCAL: u8 = 4;
const SPLIT_R_REMOTE: u8 = 5;
const NONE: u32 = u32::MAX;

/// Called once per cube: (index, solution, stats, seconds from its start to
/// its solution).
pub type Done<'d> = dyn FnMut(usize, Vec<u8>, Stats, f64) + 'd;

/// Iteration data, written only by the thread that builds an iteration, while
/// no job of the cube is outstanding (nobody else reads it then).
struct Inner {
    /// The jobs of the current iteration, the first `nf` feasible (claimed),
    /// the others with bounds beyond it.
    jobs: Vec<Job>,
    nf: usize,
    /// Statistic (f32 bits) of each job of the current iteration, summed
    /// over the job's pieces.
    stats: Vec<AtomicU32>,
    target: u8,
    /// Buffers of the build in progress (from `Pool::bufs`).
    b: Bufs,
    /// When the current iteration's build started, its jobs were extended
    /// and the stages of a parallel sort ended (for `Pool::trace`).
    t0: Instant,
    t1: Instant,
    t2: Instant,
    ts: [Instant; 3],
}

/// Most jobs of an iteration (the canonical sequences of 4 moves), and the
/// capacity of every job buffer (`Lane::extend_jobs` reserves 15 children
/// per parent). Job buffers are allocated at full size and then only
/// swapped around, and the other buffers of a build are reused through
/// `Pool::bufs`: builds never touch fresh pages (whose faults, ~1 us per
/// 4 KiB, took most of the time of a parallel build).
const MAX_JOBS: usize = 43254;
const JOBS_CAP: usize = 3240 * 15;

/// Buffers of a build. Parallel extension (`build_task`): the first child
/// of each chunk of `EXT_CHUNK` parents; the children go to `out`. Sort (a
/// sample sort; the serial sort uses `keys` and `keys_tmp`): the keys and
/// their buckets, the splitters, the keys scattered to `keys_tmp` by
/// bucket, the counts (then positions) of the buckets per chunk, the start
/// of each bucket; the sorted jobs go to `out`, which is then swapped with
/// the cube's jobs. Tasks write through the `*_ptr` copies of the buffer
/// addresses.
#[derive(Default)]
struct Bufs {
    /// From `Pool::bufs` (those of staged builds; serial builds use the
    /// builder's own).
    pooled: bool,
    /// Moves added by the parallel extension.
    ext_lev: u8,
    /// The sort keys take the statistics from `Inner::stats` (the jobs are
    /// those of the last iteration).
    direct: bool,
    ext_off: Vec<u32>,
    out: Vec<Job>,
    keys: Vec<u64>,
    keys_tmp: Vec<u64>,
    bids: Vec<u8>,
    splitters: Vec<u64>,
    hist: Vec<u32>,
    groups: Vec<u32>,
    out_ptr: usize,
    keys_ptr: usize,
    tmp_ptr: usize,
    bids_ptr: usize,
    hist_ptr: usize,
}

impl Bufs {
    fn sized() -> Bufs {
        Bufs {
            out: Vec::with_capacity(JOBS_CAP),
            keys: Vec::with_capacity(MAX_JOBS),
            keys_tmp: Vec::with_capacity(MAX_JOBS),
            bids: Vec::with_capacity(MAX_JOBS),
            ..Bufs::default()
        }
    }
}

/// A reader-writer spin lock. The threads of a pool have cores of their
/// own and hold it briefly: waiting must not sleep (with `std`'s lock,
/// readers queued behind a writer sleep in the kernel, for tens of us).
struct SpinRw<T> {
    /// Readers, plus `SPIN_W` while a writer holds or waits for the lock.
    state: AtomicU32,
    data: UnsafeCell<T>,
}

const SPIN_W: u32 = 1 << 31;

// (A lock.)
unsafe impl<T: Send + Sync> Sync for SpinRw<T> {}

struct SpinRead<'l, T>(&'l SpinRw<T>);
struct SpinWrite<'l, T>(&'l SpinRw<T>);

impl<T> SpinRw<T> {
    fn new(v: T) -> SpinRw<T> {
        SpinRw {
            state: AtomicU32::new(0),
            data: UnsafeCell::new(v),
        }
    }

    fn read(&self) -> SpinRead<'_, T> {
        let mut i = 0;
        loop {
            let s = self.state.load(Relaxed);
            if s & SPIN_W == 0
                && self
                    .state
                    .compare_exchange_weak(s, s + 1, Acquire, Relaxed)
                    .is_ok()
            {
                return SpinRead(self);
            }
            spin_wait(&mut i);
        }
    }

    /// (New readers wait while a writer waits for the others to leave.)
    fn write(&self) -> SpinWrite<'_, T> {
        let mut i = 0;
        while self.state.fetch_or(SPIN_W, Acquire) & SPIN_W != 0 {
            while self.state.load(Relaxed) & SPIN_W != 0 {
                spin_wait(&mut i);
            }
        }
        while self.state.load(Acquire) != SPIN_W {
            spin_wait(&mut i);
        }
        SpinWrite(self)
    }
}

/// One wait of a spin lock: spins, then yields (if threads outnumber the
/// cores, the holder may be waiting for one).
#[inline(always)]
fn spin_wait(i: &mut u32) {
    *i += 1;
    if *i < 256 {
        std::hint::spin_loop();
    } else {
        std::thread::yield_now();
    }
}

impl<T> std::ops::Deref for SpinRead<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: no writer while a reader holds the lock.
        unsafe { &*self.0.data.get() }
    }
}

impl<T> Drop for SpinRead<'_, T> {
    fn drop(&mut self) {
        self.0.state.fetch_sub(1, Release);
    }
}

impl<T> std::ops::Deref for SpinWrite<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: the writer holds the lock alone.
        unsafe { &*self.0.data.get() }
    }
}

impl<T> std::ops::DerefMut for SpinWrite<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the writer holds the lock alone.
        unsafe { &mut *self.0.data.get() }
    }
}

impl<T> Drop for SpinWrite<'_, T> {
    fn drop(&mut self) {
        self.0.state.store(0, Release);
    }
}

/// A value on cache lines of its own (atomics written by many threads).
#[repr(align(128))]
struct Pad<T>(T);

impl<T> std::ops::Deref for Pad<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

/// A cube being solved by any number of lanes.
struct CubeCtx {
    index: usize,
    started: Instant,
    root_c: Cube,
    root_ci: Cube,
    /// Solved (no more iterations; running jobs are dropped). (Written
    /// once: with the read-mostly fields.)
    done: AtomicBool,
    trace: bool,
    /// Jobs of the current iteration: `count << 32 | next unclaimed`.
    claim: Pad<AtomicU64>,
    /// Tasks of a parallel build: `stage << 56 | count << 32 | next
    /// unclaimed`, and the number of tasks of the stage not yet done.
    task: Pad<AtomicU64>,
    task_left: Pad<AtomicU32>,
    /// Jobs (and donated pieces) of the current iteration not yet finished,
    /// counted down by each thread when its claimed jobs are all done.
    outstanding: Pad<AtomicU64>,
    /// 1 until solved, plus one per thread working on the cube; the thread
    /// that drops the last one reports the cube.
    parts: Pad<AtomicU32>,
    /// Written now and then (not to disturb the read-mostly fields).
    rare: Pad<Rare>,
    /// (solution, seconds).
    result: Mutex<(Vec<u8>, f64)>,
    inner: UnsafeCell<Inner>,
    /// (trace) Per build task (stage - 1) * 256 + c: thread + 1, start and
    /// end (ms, f32 bits).
    tlog: Box<[[AtomicU32; 3]]>,
}

#[derive(Default)]
struct Rare {
    /// On `Pool::open` (changed under its write lock).
    listed: AtomicBool,
    /// Summed by the threads leaving the cube.
    nodes: AtomicU64,
    lookups: AtomicU64,
    /// (trace) ns from `started` when the claims of the iteration ran out,
    /// nodes and jobs of the iteration.
    exhausted: AtomicU64,
    it_nodes: AtomicU64,
    it_jobs: AtomicU64,
    it_donated: AtomicU64,
    it_splits: AtomicU64,
}

// `inner` is only written by the builder while nobody else accesses it.
unsafe impl Sync for CubeCtx {}

impl Inner {
    fn new(jobs: Vec<Job>, target: u8) -> Inner {
        let t0 = Instant::now();
        Inner {
            jobs,
            nf: 0,
            stats: Vec::new(),
            target,
            b: Bufs::default(),
            t0,
            t1: t0,
            t2: t0,
            ts: [t0; 3],
        }
    }
}

impl CubeCtx {
    /// The current iteration. Only while the caller has outstanding jobs.
    #[inline(always)]
    fn inner(&self) -> &Inner {
        // SAFETY: the iteration cannot end (and be rebuilt) while one of
        // its jobs is outstanding.
        unsafe { &*self.inner.get() }
    }

    fn unclaimed(&self) -> u32 {
        let w = self.claim.load(Relaxed);
        ((w >> 32) as u32).saturating_sub(w as u32)
    }

    fn task_unclaimed(&self) -> bool {
        let w = self.task.load(Relaxed);
        (w as u32) < (w >> 32) as u32 & 0xFF_FFFF
    }

    /// Unclaimed jobs or build tasks.
    fn open(&self) -> bool {
        !self.done.load(Relaxed) && (self.unclaimed() > 0 || self.task_unclaimed())
    }

    /// Claim up to `k` jobs of the current iteration: (first, count).
    fn claim(&self, k: u32) -> Option<(u32, u32)> {
        let w = self.claim.load(Acquire);
        // (Checked first so that exhausted counters are not bumped forever.)
        if w as u32 >= (w >> 32) as u32 || self.done.load(Relaxed) {
            if self.trace && self.rare.exhausted.load(Relaxed) == 0 {
                let _ = self.rare.exhausted.compare_exchange(
                    0,
                    self.started.elapsed().as_nanos() as u64,
                    Relaxed,
                    Relaxed,
                );
            }
            return None;
        }
        let w = self.claim.fetch_add(k as u64, AcqRel);
        let (n, next) = ((w >> 32) as u32, w as u32);
        (next < n).then(|| (next, k.min(n - next)))
    }
}

/// `*st += v` for f32 bits.
fn add_f32(st: &AtomicU32, v: f32) {
    let mut x = st.load(Relaxed);
    while let Err(y) =
        st.compare_exchange_weak(x, (f32::from_bits(x) + v).to_bits(), Relaxed, Relaxed)
    {
        x = y;
    }
}

/// Part of a job's tree donated to another thread: the path from the job
/// root, whose last node holds the donated moves (the others have none).
struct Piece {
    ctx: Arc<CubeCtx>,
    job: u32,
    nodes: Vec<Node>,
}

/// Mailbox states: the owner has work, waits for a piece, a donor is
/// writing one, a piece is there.
const IDLE: u8 = 0;
const WANT: u8 = 1;
const FILL: u8 = 2;
const FULL: u8 = 3;

/// A thread's mailbox: a thread without work asks for a piece (`WANT`); a
/// busy thread takes the request (`FILL`) and delivers (`FULL`).
struct Mailbox {
    state: AtomicU8,
    /// (Uncontended: the donor writes it, then the owner takes it.)
    piece: Mutex<Option<Piece>>,
}

/// Shared state of a batch: the cube source and the active cubes.
pub struct Pool<'a> {
    cubes: &'a (dyn Fn(usize) -> Option<CubieCube> + Sync),
    max_active: usize,
    nthreads: u32,
    next: Pad<AtomicUsize>,
    no_fresh: AtomicBool,
    /// Cubes started and not yet solved.
    active: Pad<AtomicUsize>,
    /// Cubes whose current iteration may have unclaimed jobs, oldest first.
    open: Pad<SpinRw<Vec<Arc<CubeCtx>>>>,
    nopen: Pad<AtomicUsize>,
    /// Per thread: a piece donated to it while it has no active lane.
    mail: Vec<Pad<Mailbox>>,
    tids: AtomicUsize,
    /// Threads waiting for a piece (mailboxes in state `WANT`).
    hungry: Pad<AtomicUsize>,
    /// Bumped whenever new work may be available.
    epoch: Pad<AtomicU64>,
    /// Bumped whenever a cube is solved (lanes may have jobs to drop).
    solved: Pad<AtomicU64>,
    /// Buffers of reported cubes, for new ones, and of builds.
    spare: Pad<Mutex<Vec<Inner>>>,
    bufs: Pad<Mutex<Vec<Bufs>>>,
    counters: Mutex<[u64; NCOUNT]>,
    /// Print a line per iteration and solution (env `CUBE_TRACE`).
    trace: bool,
    /// (Tests) Pause after publishing a fresh cube, so that other threads
    /// can solve and report it before its creator goes on.
    #[cfg(test)]
    pause_after_publish: bool,
}

/// Event counters (summed over threads; see `Pool::counters`).
const NCOUNT: usize = 13;
const COUNT_NAMES: [&str; NCOUNT] = [
    "rounds",
    "lane_visits",
    "jobs",
    "fresh",
    "open",
    "pieces",
    "splits",
    "donated",
    "builds",
    "idle_waits",
    "refills",
    "refill_fails",
    "open_scans",
];
const C_ROUNDS: usize = 0;
const C_VISITS: usize = 1;
const C_JOBS: usize = 2;
const C_FRESH: usize = 3;
const C_OPEN: usize = 4;
const C_PIECES: usize = 5;
const C_SPLITS: usize = 6;
const C_DONATED: usize = 7;
const C_BUILDS: usize = 8;
const C_IDLE: usize = 9;
const C_REFILL: usize = 10;
const C_REFILL_FAIL: usize = 11;
const C_SCANS: usize = 12;

impl<'a> Pool<'a> {
    /// Solve `cubes(i)` for i = 0, 1, ... until it returns None, with at most
    /// `max_active` cubes in progress at once (1: one cube at a time with
    /// all threads), by `nthreads` threads calling [`Pool::run`].
    pub fn new(
        cubes: &'a (dyn Fn(usize) -> Option<CubieCube> + Sync),
        max_active: usize,
        nthreads: usize,
    ) -> Pool<'a> {
        Pool {
            cubes,
            max_active: max_active.max(1),
            nthreads: nthreads.max(1) as u32,
            next: Pad(AtomicUsize::new(0)),
            no_fresh: AtomicBool::new(false),
            active: Pad(AtomicUsize::new(0)),
            open: Pad(SpinRw::new(Vec::new())),
            nopen: Pad(AtomicUsize::new(0)),
            mail: (0..nthreads.max(1))
                .map(|_| {
                    Pad(Mailbox {
                        state: AtomicU8::new(IDLE),
                        piece: Mutex::new(None),
                    })
                })
                .collect(),
            tids: AtomicUsize::new(0),
            hungry: Pad(AtomicUsize::new(0)),
            epoch: Pad(AtomicU64::new(0)),
            solved: Pad(AtomicU64::new(0)),
            spare: Pad(Mutex::new(Vec::new())),
            bufs: Pad(Mutex::new(Vec::new())),
            counters: Mutex::new([0; NCOUNT]),
            trace: std::env::var_os("CUBE_TRACE").is_some(),
            #[cfg(test)]
            pause_after_publish: false,
        }
    }

    /// Work on the pool with `nlanes` interleaved lanes until all cubes are
    /// solved or `stop` is set. Calls `done(index, solution, stats,
    /// seconds)` for the cubes whose last worker is this thread.
    pub fn run(
        &self,
        s: &Solver,
        nlanes: usize,
        stop: &AtomicBool,
        mut done: impl FnMut(usize, Vec<u8>, Stats, f64),
    ) {
        let mut w = Worker {
            pool: self,
            s,
            id: self.tids.fetch_add(1, Relaxed),
            done: &mut done,
            lanes: (0..nlanes.max(1)).map(|_| SLane::new()).collect(),
            nactive: 0,
            locals: Vec::new(),
            scratch: Box::new(Lane::new()),
            bufs: Bufs::sized(),
            epoch: 0,
            solved: 0,
            split_wait: 0,
            split_backoff: 0,
            donate_wait: 0,
            donate_backoff: 0,
            count: [0; NCOUNT],
        };
        w.run(stop);
        let mut c = self.counters.lock().unwrap();
        for (a, b) in c.iter_mut().zip(w.count) {
            *a += b;
        }
    }

    /// Event counters of the finished threads, e.g. for tuning: rounds, the
    /// mean number of active lanes per round, jobs started, ...
    pub fn counters(&self) -> String {
        let c = *self.counters.lock().unwrap();
        let mut out = format!(
            "active lanes/round {:.2}",
            c[C_VISITS] as f64 / c[C_ROUNDS].max(1) as f64
        );
        for (n, v) in COUNT_NAMES.iter().zip(c) {
            out += &format!(" {n} {v}");
        }
        out
    }

    /// Make sure `ctx` is on the open list (new jobs or build tasks).
    fn publish(&self, ctx: &Arc<CubeCtx>) {
        let mut o = self.open.write();
        // Drop the cubes with nothing left to claim (else nobody may: when
        // no lane is idle).
        if o.len() >= 64 && o.len().is_power_of_two() {
            o.retain(keep_open);
        }
        // (`listed` changes only under the write lock.)
        if !ctx.rare.listed.load(Relaxed) {
            ctx.rare.listed.store(true, Relaxed);
            o.push(ctx.clone());
        }
        self.nopen.store(o.len(), Relaxed);
        drop(o);
        self.epoch.fetch_add(1, Release);
    }

    /// The oldest cube with unclaimed jobs (`task`: build tasks).
    fn find_open(&self, task: bool) -> Option<Arc<CubeCtx>> {
        if self.nopen.load(Relaxed) == 0 {
            return None;
        }
        let o = self.open.read();
        let mut dead = 0;
        let mut found = None;
        for c in o.iter() {
            if !c.open() {
                dead += 1;
                continue;
            }
            if if task {
                c.task_unclaimed()
            } else {
                c.unclaimed() > 0
            } {
                found = Some(c.clone());
                break;
            }
        }
        drop(o);
        if dead > 0 {
            // (Not a try: under the read locks of the threads looking for
            // work it would rarely succeed, and the dead cubes would keep
            // them all taking the lock.)
            let mut o = self.open.write();
            o.retain(keep_open);
            self.nopen.store(o.len(), Relaxed);
        }
        found
    }

    fn take_bufs(&self) -> Bufs {
        let mut b = self.bufs.lock().unwrap().pop().unwrap_or_else(Bufs::sized);
        b.pooled = true;
        b.direct = false;
        b
    }

    /// Give back the buffers of a staged build (if any).
    fn put_bufs(&self, b: Bufs) {
        if b.pooled {
            self.bufs.lock().unwrap().push(b);
        }
    }

    fn finished(&self) -> bool {
        self.no_fresh.load(Acquire) && self.active.load(Acquire) == 0
    }
}

/// `retain` predicate of the open list: drops (and unlists) the cubes with
/// nothing left to claim.
fn keep_open(c: &Arc<CubeCtx>) -> bool {
    let k = c.open();
    if !k {
        c.rare.listed.store(false, Relaxed);
    }
    k
}

/// Solve all cubes of `cubes` (until it returns None) with `threads` threads
/// of `nlanes` lanes, at most `max_active` cubes at a time; `done` is called
/// once per cube (from any thread).
pub fn solve_all(
    s: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    threads: usize,
    nlanes: usize,
    max_active: usize,
    done: &(dyn Fn(usize, Vec<u8>, Stats, f64) + Sync),
) {
    let pool = Pool::new(cubes, max_active, threads);
    let stop = AtomicBool::new(false);
    std::thread::scope(|sc| {
        for _ in 0..threads.max(1) {
            sc.spawn(|| pool.run(s, nlanes, &stop, done));
        }
    });
}

/// A thread's share of a cube: jobs claimed and the stats of its lanes.
struct Local {
    ctx: Arc<CubeCtx>,
    /// Claimed jobs not yet started: `next..end`.
    next: u32,
    end: u32,
    /// Claimed jobs and received pieces not yet finished.
    pending: u32,
    /// Jobs and pieces to count down from `outstanding` once none is
    /// pending.
    claimed: u64,
    nodes: u64,
    lookups: u64,
    /// (trace) Of the current iteration.
    it_nodes: u64,
    it_jobs: u64,
}

struct SLane {
    l: Lane,
    /// `Worker::locals` slot of the cube of the current job (NONE if idle).
    slot: u32,
    /// Index of the current job in the cube's job list.
    job: u32,
    /// The job's tree is shared with other lanes (split, donated or a
    /// piece): its statistic is added atomically.
    split: bool,
    /// Lane counters at the start of the job.
    n0: u64,
    k0: u64,
}

impl SLane {
    fn new() -> SLane {
        let mut l = Lane::new();
        l.shared = true;
        l.jobs.push(Job {
            moves: [0; 4],
            len: 0,
            ln: 0,
            li: 0,
            stat: 0.0,
        });
        SLane {
            l,
            slot: NONE,
            job: 0,
            split: false,
            n0: 0,
            k0: 0,
        }
    }
}

struct Worker<'w, 'a> {
    pool: &'w Pool<'a>,
    s: &'w Solver<'w>,
    /// Mailbox index (none if beyond `Pool::nthreads`).
    id: usize,
    done: &'w mut Done<'w>,
    /// The active lanes are `lanes[..nactive]`.
    lanes: Vec<SLane>,
    nactive: usize,
    locals: Vec<Option<Local>>,
    /// Builds iterations (its job buffers), and the buffers of serial builds.
    scratch: Box<Lane>,
    bufs: Bufs,
    epoch: u64,
    /// `Pool::solved` when the lanes were last checked for solved cubes.
    solved: u64,
    /// No split attempt before round `split_wait` (after failures).
    split_wait: u32,
    split_backoff: u32,
    /// No donation attempt before round `donate_wait` (after failures).
    donate_wait: u32,
    donate_backoff: u32,
    count: [u64; NCOUNT],
}

/// Build the next iteration of `ctx` (the first one if `first`) with `sc`'s
/// extension, and publish it. If threads are waiting for work, a long job
/// list is extended and sorted by all of them instead (`build_task`):
/// returns true if that was started (the caller then helps).
fn build(
    sc: &mut Lane,
    own: &mut Bufs,
    s: &Solver,
    pool: &Pool,
    ctx: &Arc<CubeCtx>,
    first: bool,
) -> bool {
    let t0 = Instant::now();
    // SAFETY: no job of the cube is outstanding: nobody else accesses
    // `inner` until the iteration (or a build task) is published.
    let inner = unsafe { &mut *ctx.inner.get() };
    if pool.trace && !first {
        let ex = ctx.rare.exhausted.swap(0, Relaxed) as f64 * 1e-6;
        let dt = (t0 - inner.t2).as_secs_f64();
        let nd = ctx.rare.it_nodes.swap(0, Relaxed);
        eprintln!("trace cube {} T {} ran {:.3} .. {:.3} ms, claims out at {ex:.3}, nodes {nd} jobs {} donated {} splits {} ({:.2} Gnodes/s)", ctx.index, inner.target, (inner.t2 - ctx.started).as_secs_f64() * 1e3, (t0 - ctx.started).as_secs_f64() * 1e3, ctx.rare.it_jobs.swap(0, Relaxed), ctx.rare.it_donated.swap(0, Relaxed), ctx.rare.it_splits.swap(0, Relaxed), nd as f64 / dt * 1e-9);
    }
    inner.t0 = t0;
    inner.t1 = t0;
    if !first {
        inner.target += 1;
    }
    let t = inner.target;
    // (As in `start_iteration`: the stack depth stays below the target.)
    assert!((t as usize) < MAXD);
    let jl = s.job_len(t);
    let parallel = pool.hungry.load(Relaxed) != 0;
    let len = inner.jobs[0].len as usize;
    // Sorted by the statistics as they are (not copied to the jobs first)?
    let direct = !first && len >= jl && jl > 0 && parallel && inner.jobs.len() >= SORT_MIN;
    if !first && !direct {
        copy_stats(inner);
    }
    if len < jl {
        // (`sc` gets the cube's buffer and ours, and returns them.)
        sc.root_c = ctx.root_c;
        sc.root_ci = ctx.root_ci;
        std::mem::swap(&mut sc.jobs, &mut inner.jobs);
        std::mem::swap(&mut sc.jobs_tmp, &mut own.out);
        let par = loop {
            let (n, lev) = (sc.jobs.len(), jl - sc.jobs[0].len as usize);
            if lev == 0 || parallel && (lev == 1 && n >= EXT_MIN || lev == 2 && n >= EXT_MIN2) {
                break lev;
            }
            sc.extend_jobs(s);
        };
        std::mem::swap(&mut sc.jobs, &mut inner.jobs);
        std::mem::swap(&mut sc.jobs_tmp, &mut own.out);
        if par > 0 {
            inner.b = pool.take_bufs();
            // Parallel extension: chunk c of the parents writes its
            // children (`par` moves longer) to `out[ext_off[c]..ext_off[c +
            // 1]]`, in the order of serial `extend_jobs`.
            let b = &mut inner.b;
            b.ext_lev = par as u8;
            b.ext_off.clear();
            let mut off = 0u32;
            let chunk = if par == 1 { EXT_CHUNK } else { 1 };
            for ch in inner.jobs.chunks(chunk) {
                b.ext_off.push(off);
                for j in ch {
                    off += nchildren(j, par);
                }
            }
            b.ext_off.push(off);
            b.out.clear();
            assert!(off as usize <= b.out.capacity());
            b.out_ptr = b.out.as_mut_ptr() as usize;
            open_stage(pool, ctx, ST_EXT, b.ext_off.len() - 1);
            return true;
        }
    }
    inner.t1 = Instant::now();
    if jl > 0 && inner.jobs.len() >= SORT_MIN && parallel {
        inner.b = pool.take_bufs();
        inner.b.direct = direct;
        start_sort(pool, inner, ctx);
        return true;
    }
    // (Serial builds use this thread's buffers: those of the pool were last
    // written by other threads.)
    inner.nf = if jl > 0 {
        sort_serial(&mut inner.jobs, own, t)
    } else {
        inner.jobs.len()
    };
    finish_build(pool, ctx);
    false
}

/// The statistics of the last iteration to its jobs.
fn copy_stats(inner: &mut Inner) {
    let n = inner.jobs.len();
    for i in 0..n {
        // (Written by other threads: fetched well ahead.)
        if i % 16 == 0 {
            prefetch(&inner.stats[(i + 512).min(n - 1)], false);
        }
        inner.jobs[i].stat = f32::from_bits(inner.stats[i].load(Relaxed));
    }
}

/// The number of jobs `lev` (at most 2) moves longer than `j`
/// (`extend_jobs`).
fn nchildren(j: &Job, lev: usize) -> u32 {
    /// By the face of the last move (6: none), and moves added.
    static N: [[u32; 3]; 7] = {
        let mut n = [[1u32; 3]; 7];
        let mut f = 0;
        while f < 7 {
            let mm = if f == 6 { ALL_MOVES } else { ALLOWED_AFTER[f] };
            n[f][1] = mm.count_ones();
            let mut m = 0;
            n[f][2] = 0;
            while m < 18 {
                if mm & (1 << m) != 0 {
                    n[f][2] += ALLOWED_AFTER[m / 3].count_ones();
                }
                m += 1;
            }
            f += 1;
        }
        n
    };
    let f = if j.len == 0 {
        6
    } else {
        (j.moves[j.len as usize - 1] / 3) as usize
    };
    N[f][lev]
}

/// Sort `jobs` for iteration `t` with the buffers of `b` (an LSD radix sort
/// of the keys of `sort_key`, then a gather); returns the number of
/// feasible jobs.
fn sort_serial(jobs: &mut Vec<Job>, b: &mut Bufs, t: u8) -> usize {
    let n = jobs.len();
    let (keys, tmp) = (&mut b.keys, &mut b.keys_tmp);
    keys.clear();
    keys.extend((0..n).map(|i| sort_key(jobs, &[], false, t, i)));
    tmp.clear();
    tmp.resize(n, 0);
    // (The indices, in the low bits, are in order.)
    for (shift, bits) in [
        (KEY_SHIFT, 11u32),
        (KEY_SHIFT + 11, 11),
        (KEY_SHIFT + 22, 11),
    ] {
        let mask = (1u64 << bits) - 1;
        let mut cnt = [0u32; 2048];
        for &k in keys.iter() {
            cnt[((k >> shift) & mask) as usize] += 1;
        }
        if cnt.iter().any(|&c| c as usize == n) {
            continue; // all keys share this digit
        }
        let mut sum = 0;
        for c in cnt.iter_mut() {
            (*c, sum) = (sum, sum + *c);
        }
        for &k in keys.iter() {
            let d = ((k >> shift) & mask) as usize;
            tmp[cnt[d] as usize] = k;
            cnt[d] += 1;
        }
        std::mem::swap(keys, tmp);
    }
    b.out.clear();
    b.out.extend(keys.iter().map(|&k| jobs[key_index(k)]));
    std::mem::swap(jobs, &mut b.out);
    keys.partition_point(|&k| k >> 63 == 0)
}

/// Build stages done in parallel (`build_task`).
const ST_EXT: u64 = 1;
const ST_KEYS: u64 = 2;
const ST_SCATTER: u64 = 3;
const ST_BUCKETS: u64 = 4;
/// Parallel sort: jobs per chunk, buckets (between splitters taken from a
/// sample of `SAMPLE` keys per bucket).
const SORT_CHUNK: usize = 2048;
const NBUCKETS: usize = 64;
const SAMPLE: usize = 16;

/// Open build stage `stage` with `n` tasks (and list the cube: it may have
/// been dropped from the open list while the last stage's tasks ran).
fn open_stage(pool: &Pool, ctx: &Arc<CubeCtx>, stage: u64, n: usize) {
    ctx.task_left.store(n as u32, Relaxed);
    ctx.task.store(stage << 56 | (n as u64) << 32, Release);
    pool.publish(ctx);
}

/// Set up the parallel sort of `inner.jobs`, the same order as
/// `sort_jobs`: by `!job_key << 32 | index`. Keys are counted per chunk by
/// their top bits (`ST_KEYS`), scattered to their buckets (`ST_SCATTER`),
/// and runs of buckets are sorted and gathered (`ST_BUCKETS`).
fn start_sort(pool: &Pool, inner: &mut Inner, ctx: &Arc<CubeCtx>) {
    let nch = sort_setup(inner);
    open_stage(pool, ctx, ST_KEYS, nch);
}

/// Sort key of job `i` in iteration `t` (with the statistic from `stats` if
/// `direct`), in increasing order: the jobs whose bounds exceed the
/// iteration (they would be skipped) last, then by decreasing `job_key`
/// (as `sort_jobs`), then by index: `infeasible << 63 | !job_key << 31 |
/// i`. The feasible jobs are in the order of `sort_jobs`, and only they
/// are claimed.
#[inline(always)]
fn sort_key(jobs: &[Job], stats: &[AtomicU32], direct: bool, t: u8, i: usize) -> u64 {
    let j = &jobs[i];
    let stat = if direct {
        f32::from_bits(stats[i].load(Relaxed))
    } else {
        j.stat
    };
    let infeasible = j.ln.max(j.li) as u32 + j.len as u32 > t as u32;
    (infeasible as u64) << 63 | ((!(stat * job_weight(j)).to_bits() as u64) << KEY_SHIFT) | i as u64
}

const KEY_SHIFT: u32 = 31;

#[inline(always)]
fn key_index(k: u64) -> usize {
    (k & ((1 << KEY_SHIFT) - 1)) as usize
}

/// Splitters of the buckets and buffers of a parallel sort; returns the
/// number of `ST_KEYS` tasks.
fn sort_setup(inner: &mut Inner) -> usize {
    let n = inner.jobs.len();
    let t = inner.target;
    let nch = n.div_ceil(SORT_CHUNK);
    let at = |x: usize| x * n / (NBUCKETS * SAMPLE);
    let direct = inner.b.direct;
    for x in 0..NBUCKETS * SAMPLE {
        // (The jobs, or statistics, were just written by other threads.)
        prefetch(&inner.jobs[at(x)], false);
        if direct {
            prefetch(&inner.stats[at(x)], false);
        }
    }
    let b = &mut inner.b;
    let mut sample = std::mem::take(&mut b.splitters);
    sample.clear();
    sample.extend(
        (0..NBUCKETS * SAMPLE).map(|x| sort_key(&inner.jobs, &inner.stats, direct, t, at(x))),
    );
    sample.sort_unstable();
    sample.dedup();
    let len = sample.len();
    for i in 1..NBUCKETS {
        sample[i - 1] = sample[i * len / NBUCKETS];
    }
    sample.truncate(NBUCKETS - 1);
    b.splitters = sample;
    b.hist.clear();
    b.hist.resize(nch * NBUCKETS, 0);
    for v in [&mut b.keys, &mut b.keys_tmp] {
        v.clear();
        assert!(n <= v.capacity());
    }
    b.bids.clear();
    b.out.clear();
    assert!(n <= b.bids.capacity() && n <= b.out.capacity());
    b.keys_ptr = b.keys.as_mut_ptr() as usize;
    b.tmp_ptr = b.keys_tmp.as_mut_ptr() as usize;
    b.bids_ptr = b.bids.as_mut_ptr() as usize;
    b.hist_ptr = b.hist.as_mut_ptr() as usize;
    b.out_ptr = b.out.as_mut_ptr() as usize;
    nch
}

/// Task `c` of sort stage `stage`.
///
/// # Safety
/// Tasks of a stage run concurrently only with each other, after
/// `sort_setup` and the previous stages.
unsafe fn sort_task(inner: &Inner, stage: u64, c: usize) {
    let jobs = &inner.jobs;
    let b = &inner.b;
    match stage {
        ST_KEYS => {
            let lo = c * SORT_CHUNK;
            let hi = (lo + SORT_CHUNK).min(jobs.len());
            let (keys, bids) = (b.keys_ptr as *mut u64, b.bids_ptr as *mut u8);
            let mut h = [0u32; NBUCKETS];
            for i in lo..hi {
                let k = sort_key(jobs, &inner.stats, b.direct, inner.target, i);
                let bk = b.splitters.partition_point(|&s| s <= k);
                *keys.add(i) = k;
                *bids.add(i) = bk as u8;
                h[bk] += 1;
            }
            let hist = (b.hist_ptr as *mut u32).add(c * NBUCKETS);
            std::ptr::copy_nonoverlapping(h.as_ptr(), hist, NBUCKETS);
        }
        ST_SCATTER => {
            let lo = c * SORT_CHUNK;
            let hi = (lo + SORT_CHUNK).min(jobs.len());
            let (keys, tmp) = (b.keys_ptr as *const u64, b.tmp_ptr as *mut u64);
            let bids = b.bids_ptr as *const u8;
            let off = std::slice::from_raw_parts_mut(
                (b.hist_ptr as *mut u32).add(c * NBUCKETS),
                NBUCKETS,
            );
            for i in lo..hi {
                let o = &mut off[*bids.add(i) as usize];
                *tmp.add(*o as usize) = *keys.add(i);
                *o += 1;
            }
        }
        _ => {
            let (lo, hi) = (b.groups[c] as usize, b.groups[c + 1] as usize);
            let run = std::slice::from_raw_parts_mut((b.tmp_ptr as *mut u64).add(lo), hi - lo);
            // (Keys are distinct: the index breaks ties, as in a stable
            // sort.)
            run.sort_unstable();
            let out = (b.out_ptr as *mut Job).add(lo);
            for (p, &k) in run.iter().enumerate() {
                if let Some(&k) = run.get(p + 16) {
                    prefetch(&jobs[key_index(k)], false);
                }
                *out.add(p) = jobs[key_index(k)];
            }
        }
    }
}

/// All tasks of sort stage `stage` are done: the next stage and its number
/// of tasks, or None when the jobs are sorted.
fn sort_stage_done(inner: &mut Inner, stage: u64) -> Option<(u64, usize)> {
    inner.ts[(stage - ST_KEYS) as usize] = Instant::now();
    let b = &mut inner.b;
    match stage {
        ST_KEYS => {
            // Counts to the first position of each (chunk, bucket); a
            // bucket is a task of `ST_BUCKETS`.
            let nch = b.hist.len() / NBUCKETS;
            b.groups.clear();
            let mut pos = 0u32;
            for bk in 0..NBUCKETS {
                b.groups.push(pos);
                for ch in 0..nch {
                    let x = &mut b.hist[ch * NBUCKETS + bk];
                    (*x, pos) = (pos, pos + *x);
                }
            }
            b.groups.push(pos);
            Some((ST_SCATTER, nch))
        }
        ST_SCATTER => Some((ST_BUCKETS, NBUCKETS)),
        _ => {
            let n = inner.jobs.len();
            // SAFETY: the tasks wrote all of `out` (and sorted `keys_tmp`).
            unsafe {
                b.out.set_len(n);
                inner.nf = std::slice::from_raw_parts(b.tmp_ptr as *const u64, n)
                    .partition_point(|&k| k >> 63 == 0);
            }
            std::mem::swap(&mut inner.jobs, &mut b.out);
            None
        }
    }
}

/// Do a task of `ctx`'s parallel build, if one is left (false otherwise);
/// the last task of a stage opens the next one or publishes the iteration.
fn build_task(sc: &mut Lane, s: &Solver, pool: &Pool, ctx: &Arc<CubeCtx>, tid: usize) -> bool {
    let w = ctx.task.load(Acquire);
    if (w as u32) >= (w >> 32) as u32 & 0xFF_FFFF {
        return false;
    }
    let w = ctx.task.fetch_add(1, AcqRel);
    let (stage, n, c) = (w >> 56, (w >> 32) as usize & 0xFF_FFFF, w as u32 as usize);
    if c >= n {
        return false;
    }
    // The stage cannot end while this task is not done; tasks write
    // disjoint parts of the buffers (through the `*_ptr` copies), which are
    // read only in later stages.
    let inner = ctx.inner();
    let tq0 = if pool.trace {
        (Instant::now() - ctx.started).as_secs_f32() * 1e3
    } else {
        0.0
    };
    // SAFETY: see above.
    unsafe {
        match stage {
            ST_EXT => {
                let chunk = if inner.b.ext_lev == 1 { EXT_CHUNK } else { 1 };
                let lo = c * chunk;
                let hi = (lo + chunk).min(inner.jobs.len());
                sc.root_c = ctx.root_c;
                sc.root_ci = ctx.root_ci;
                sc.jobs.clear();
                sc.jobs.extend_from_slice(&inner.jobs[lo..hi]);
                for _ in 0..inner.b.ext_lev {
                    sc.extend_jobs(s);
                }
                let b = &inner.b;
                let (o, e) = (b.ext_off[c] as usize, b.ext_off[c + 1] as usize);
                assert_eq!(sc.jobs.len(), e - o);
                let out = (b.out_ptr as *mut Job).add(o);
                std::ptr::copy_nonoverlapping(sc.jobs.as_ptr(), out, e - o);
            }
            _ => sort_task(inner, stage, c),
        }
    }
    if pool.trace {
        let tq1 = (Instant::now() - ctx.started).as_secs_f32() * 1e3;
        if let Some(e) = ctx
            .tlog
            .get((stage as usize - 1) * 256 + c)
            .filter(|_| c < 256)
        {
            e[0].store(tid as u32 + 1, Relaxed);
            e[1].store(tq0.to_bits(), Relaxed);
            e[2].store(tq1.to_bits(), Relaxed);
        }
    }
    if ctx.task_left.fetch_sub(1, AcqRel) == 1 {
        // SAFETY: all tasks of the stage are done, the next one is not open.
        let inner = unsafe { &mut *ctx.inner.get() };
        match stage {
            ST_EXT => {
                let b = &mut inner.b;
                // SAFETY: the tasks wrote all of `out`.
                unsafe { b.out.set_len(*b.ext_off.last().unwrap() as usize) };
                std::mem::swap(&mut inner.jobs, &mut b.out);
                inner.t1 = Instant::now();
                if inner.jobs.len() >= SORT_MIN {
                    start_sort(pool, inner, ctx);
                } else {
                    inner.nf = sort_serial(&mut inner.jobs, &mut inner.b, inner.target);
                    finish_build(pool, ctx);
                }
            }
            _ => match sort_stage_done(inner, stage) {
                Some((next, n)) => open_stage(pool, ctx, next, n),
                None => finish_build(pool, ctx),
            },
        }
    }
    true
}

/// Publish the (extended and sorted) jobs of the new iteration of `ctx`.
fn finish_build(pool: &Pool, ctx: &Arc<CubeCtx>) {
    // SAFETY: as in `build`.
    let inner = unsafe { &mut *ctx.inner.get() };
    pool.put_bufs(std::mem::take(&mut inner.b));
    let t = inner.target;
    let t2 = Instant::now();
    let n = inner.jobs.len();
    inner.stats.clear();
    inner.stats.resize_with(n, || AtomicU32::new(0));
    // (The infeasible jobs, at the end, keep the statistic 0.)
    let nf = inner.nf;
    // At least one job is claimed (an infeasible one is skipped): the
    // iteration ends when the claimed jobs end.
    let nc = nf.max(1) as u64;
    ctx.outstanding.store(nc, Relaxed);
    ctx.rare.exhausted.store(0, Relaxed);
    inner.t2 = Instant::now();
    ctx.claim.store(nc << 32, Release);
    pool.publish(ctx);
    if pool.trace {
        let ms = |t: Instant| (t - ctx.started).as_secs_f64() * 1e3;
        let st = if inner.ts[2] > inner.t1 {
            let [a, b, c] = inner.ts.map(ms);
            format!(" (parallel: keys {a:.3} scatter {b:.3} buckets {c:.3})")
        } else {
            String::new()
        };
        eprintln!(
            "trace cube {} T {t} jobs {n} feasible {nf} build {:.3} .. {:.3} ms (extended {:.3} sorted {:.3}){st}",
            ctx.index,
            ms(inner.t0),
            ms(Instant::now()),
            ms(inner.t1),
            ms(t2)
        );
        for st in 1..5usize {
            let v: Vec<(u32, f32, f32)> = ctx.tlog[(st - 1) * 256..st * 256]
                .iter()
                .filter(|e| e[0].load(Relaxed) != 0)
                .map(|e| {
                    (
                        e[0].swap(0, Relaxed),
                        f32::from_bits(e[1].load(Relaxed)),
                        f32::from_bits(e[2].load(Relaxed)),
                    )
                })
                .collect();
            if v.is_empty() {
                continue;
            }
            let mut th: Vec<u32> = v.iter().map(|x| x.0).collect();
            th.sort();
            th.dedup();
            let first = v.iter().map(|x| x.1).fold(f32::MAX, f32::min);
            let last = v.iter().map(|x| x.2).fold(0f32, f32::max);
            let sum: f32 = v.iter().map(|x| x.2 - x.1).sum();
            let mx = v.iter().map(|x| x.2 - x.1).fold(0f32, f32::max);
            eprintln!("  stage {st} tasks {} threads {} first {first:.3} last {last:.3} sum {sum:.3} max {mx:.3}", v.len(), th.len());
        }
    }
}

#[inline(always)]
fn prefetch<T>(x: &T, write: bool) {
    // SAFETY: prefetches have no effect on the program state.
    unsafe {
        use std::arch::x86_64::*;
        if write {
            _mm_prefetch(x as *const T as *const i8, _MM_HINT_ET0);
        } else {
            _mm_prefetch(x as *const T as *const i8, _MM_HINT_T0);
        }
    }
}

/// Round `round` is at or after round `t` (wrapping).
fn after(round: u32, t: u32) -> bool {
    round.wrapping_sub(t) < u32::MAX / 2
}

fn two_mut<T>(v: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert!(a != b);
    if a < b {
        let (x, y) = v.split_at_mut(b);
        (&mut x[a], &mut y[0])
    } else {
        let (x, y) = v.split_at_mut(a);
        (&mut y[0], &mut x[b])
    }
}

/// Moves of `todo` to give away: the upper half of its faces, never the face
/// `cur` of the child in progress (an X' child relies on the other-side value
/// noted by its X sibling) and never all of them.
fn split_moves(todo: u32, cur: u8) -> Option<u32> {
    let mut faces = 0u32;
    for f in 0..6 {
        if todo & (7 << (3 * f)) != 0 {
            faces |= 1 << f;
        }
    }
    let mut n = faces.count_ones() / 2;
    let mut give = 0;
    for f in (0..6u8).rev() {
        if n == 0 {
            break;
        }
        if faces & (1 << f) != 0 && f != cur {
            give |= todo & (7 << (3 * f));
            n -= 1;
        }
    }
    (give != 0).then_some(give)
}

impl Worker<'_, '_> {
    fn run(&mut self, stop: &AtomicBool) {
        let mut round = 0u32;
        loop {
            if self.nactive == 0 && !self.idle_wait(stop) {
                break;
            }
            if !self.rounds(stop, &mut round) {
                break;
            }
        }
        // (Only with `stop` can jobs be left; their cubes are not reported.)
        for slot in 0..self.locals.len() {
            if matches!(&self.locals[slot], Some(l) if l.pending == 0) {
                self.release(slot);
            }
        }
        self.unwant();
    }

    /// Visit the active lanes round-robin (as `run_lanes` does) until none
    /// is left (true) or `stop` is set (false). Job ends are handled by
    /// `on_end` (which may swap another lane into position i), the rest
    /// once per round by `maintain`. (Kept apart from the cold code.)
    #[inline(never)]
    fn rounds(&mut self, stop: &AtomicBool, round: &mut u32) -> bool {
        let s = self.s;
        while self.nactive > 0 {
            let mut i = 0;
            let mut n = self.nactive;
            while i < n {
                // SAFETY: i < nactive <= lanes.len().
                let l = unsafe { &mut self.lanes.get_unchecked_mut(i).l };
                let v = l.pending_value(s);
                let st = match l.phase {
                    Phase::WaitOther => {
                        if l.resolve_other(v, s) {
                            i += 1;
                            continue;
                        }
                        l.advance(s)
                    }
                    Phase::WaitMoved => l.resolve_moved(v, s),
                };
                match st {
                    Step::Pending => i += 1,
                    st => {
                        if self.on_end(i, matches!(st, Step::Found)) {
                            i += 1;
                        }
                        n = self.nactive;
                    }
                }
            }
            *round = round.wrapping_add(1);
            self.count[C_ROUNDS] += 1;
            self.count[C_VISITS] += n as u64;
            if n < self.lanes.len()
                || self.pool.solved.load(Relaxed) != self.solved
                || self.pool.hungry.load(Relaxed) != 0
            {
                self.maintain(*round);
            }
            if stop.load(Relaxed) {
                return false;
            }
        }
        true
    }

    fn mailbox(&self) -> Option<&Mailbox> {
        self.pool.mail.get(self.id).map(|m| &m.0)
    }

    /// Ask for a piece.
    fn want(&self) {
        if let Some(m) = self.mailbox() {
            if m.state.load(Relaxed) == IDLE {
                m.state.store(WANT, Release);
                self.pool.hungry.fetch_add(1, Relaxed);
            }
        }
    }

    /// Withdraw a request for a piece (one already on its way is taken by
    /// `piece` later).
    fn unwant(&self) {
        if let Some(m) = self.mailbox() {
            if m.state
                .compare_exchange(WANT, IDLE, AcqRel, Relaxed)
                .is_ok()
            {
                self.pool.hungry.fetch_sub(1, Relaxed);
            }
        }
    }

    /// Lane `i`'s job ended (`found`: with a solution): account for it and
    /// `restart` the lane. Returns false if there was no work: the lane was
    /// deactivated (another lane took its place).
    #[inline(never)]
    fn on_end(&mut self, i: usize, found: bool) -> bool {
        if found {
            self.report(i);
        }
        let slot = self.end_job(i, !found);
        if self.restart(i, Some(slot)) {
            return true;
        }
        self.deactivate(i);
        false
    }

    /// Give lane `i` (not running a job) new work and advance it to its
    /// first lookup, past jobs that end before (as stream mode does within
    /// `advance`). Returns false if there is no work. `prefer`: the slot of
    /// its last job.
    fn restart(&mut self, i: usize, mut prefer: Option<usize>) -> bool {
        loop {
            if !self.refill(i, prefer) {
                return false;
            }
            let found = match self.lanes[i].l.advance_cold(self.s) {
                Step::Pending => return true,
                st => matches!(st, Step::Found),
            };
            if found {
                self.report(i);
            }
            prefer = Some(self.end_job(i, !found));
        }
    }

    /// Record lane `i`'s solution if it is the first for its cube.
    fn report(&mut self, i: usize) {
        let sl = &self.lanes[i];
        let ctx = &self.locals[sl.slot as usize].as_ref().unwrap().ctx;
        if ctx.done.swap(true, AcqRel) {
            return;
        }
        let sol = sl.l.reconstruct();
        let dt = ctx.started.elapsed().as_secs_f64();
        if self.pool.trace {
            // SAFETY: (trace) the iteration is still on.
            let t2 = unsafe { (*ctx.inner.get()).t2 };
            eprintln!(
                "trace cube {} found {:.3} ms ({:.3} ms into T {})",
                ctx.index,
                dt * 1e3,
                (Instant::now() - t2).as_secs_f64() * 1e3,
                sl.l.target
            );
        }
        *ctx.result.lock().unwrap() = (sol, dt);
        self.pool.active.fetch_sub(1, AcqRel);
        self.pool.epoch.fetch_add(1, Release);
        self.pool.solved.fetch_add(1, Release);
        // The "unsolved" part (this thread still holds one).
        ctx.parts.fetch_sub(1, AcqRel);
    }

    /// Account for the end of lane `i`'s job; returns its slot.
    fn end_job(&mut self, i: usize, stat: bool) -> usize {
        let sl = &mut self.lanes[i];
        let slot = sl.slot as usize;
        sl.slot = NONE;
        let loc = self.locals[slot].as_mut().unwrap();
        loc.nodes += sl.l.stats.nodes - sl.n0;
        loc.lookups += sl.l.stats.lookups - sl.k0;
        loc.it_nodes += sl.l.stats.nodes - sl.n0;
        loc.it_jobs += 1;
        if stat {
            let st = &loc.ctx.inner().stats[sl.job as usize];
            if sl.split {
                add_f32(st, sl.l.heur);
            } else {
                // (Nobody else writes it; it was 0.)
                st.store(sl.l.heur.to_bits(), Relaxed);
            }
        }
        loc.pending -= 1;
        slot
    }

    fn deactivate(&mut self, i: usize) {
        self.nactive -= 1;
        self.lanes.swap(i, self.nactive);
    }

    /// Give lane `li` (not running a job) new work; `prefer`: the slot of
    /// its last job.
    fn refill(&mut self, li: usize, prefer: Option<usize>) -> bool {
        // (A piece first: its job holds up the iteration.)
        if matches!(self.mailbox(), Some(m) if m.state.load(Relaxed) == FULL) && self.piece(li) {
            // (The cube of the lane's last job may have nothing pending
            // any more: its jobs are to be counted down.)
            if let Some(slot) = prefer {
                self.tidy(slot);
            }
            return true;
        }
        if let Some(slot) = prefer {
            if self.continue_local(li, slot) {
                return true;
            }
        }
        for slot in 0..self.locals.len() {
            if matches!(&self.locals[slot], Some(l) if l.next < l.end)
                && self.continue_local(li, slot)
            {
                return true;
            }
        }
        self.count[C_REFILL] += 1;
        let ok = self.fresh(li) || self.open_cube(li) || self.split(li);
        self.count[C_REFILL_FAIL] += !ok as u64;
        ok
    }

    /// Start a job of the cube in `slot` in lane `li`, claiming more jobs if
    /// needed. Releases the slot if nothing is left to do there.
    fn continue_local(&mut self, li: usize, slot: usize) -> bool {
        loop {
            if self.take(li, slot) {
                return true;
            }
            self.settle(slot);
            if !self.claim(slot) {
                break;
            }
        }
        if self.locals[slot].as_ref().unwrap().pending == 0 {
            self.release(slot);
        }
        false
    }

    /// Count down the jobs of `slot` if none is pending, and leave the cube
    /// then.
    fn tidy(&mut self, slot: usize) {
        self.settle(slot);
        if self.locals[slot].as_ref().unwrap().pending == 0 {
            self.release(slot);
        }
    }

    /// Start the next claimed job of `slot` in lane `li`.
    fn take(&mut self, li: usize, slot: usize) -> bool {
        let loc = self.locals[slot].as_mut().unwrap();
        if loc.next < loc.end && loc.ctx.done.load(Relaxed) {
            // (dropped)
            loc.pending -= loc.end - loc.next;
            loc.next = loc.end;
        }
        while loc.next < loc.end {
            let ji = loc.next;
            loc.next += 1;
            let ctx = &*loc.ctx;
            let inner = ctx.inner();
            let sl = &mut self.lanes[li];
            let l = &mut sl.l;
            l.root_c = ctx.root_c;
            l.root_ci = ctx.root_ci;
            l.target = inner.target;
            l.jobs[0] = inner.jobs[ji as usize];
            l.job_pos = 0;
            if l.setup_job() {
                // The statistic is written at the end of the job, the next
                // job is read when it starts.
                prefetch(&inner.stats[ji as usize], true);
                if loc.next < loc.end {
                    prefetch(&inner.jobs[loc.next as usize], false);
                }
                self.count[C_JOBS] += 1;
                sl.slot = slot as u32;
                sl.job = ji;
                sl.split = false;
                sl.n0 = l.stats.nodes;
                sl.k0 = l.stats.lookups;
                return true;
            }
            // infeasible (statistic 0, as in stream mode)
            loc.pending -= 1;
        }
        false
    }

    /// Claim more jobs of the cube in `slot` (none must be unstarted).
    fn claim(&mut self, slot: usize) -> bool {
        let loc = self.locals[slot].as_mut().unwrap();
        debug_assert!(loc.next >= loc.end);
        let k = (loc.ctx.unclaimed() / (2 * self.pool.nthreads)).clamp(1, CHUNK_MAX);
        let Some((p, c)) = loc.ctx.claim(k) else {
            return false;
        };
        loc.next = p;
        loc.end = p + c;
        loc.pending += c;
        loc.claimed += c as u64;
        prefetch(&loc.ctx.inner().jobs[p as usize], false);
        true
    }

    /// If no job of `slot` is pending, count its finished jobs down from
    /// the cube's outstanding jobs, and build the next iteration if they
    /// were the last.
    fn settle(&mut self, slot: usize) {
        let loc = self.locals[slot].as_mut().unwrap();
        if loc.ctx.done.load(Relaxed) {
            // drop the unstarted jobs
            loc.pending -= loc.end - loc.next;
            loc.next = loc.end;
        }
        if loc.pending != 0 || loc.claimed == 0 {
            return;
        }
        let c = loc.claimed;
        loc.claimed = 0;
        if self.pool.trace {
            loc.ctx
                .rare
                .it_nodes
                .fetch_add(std::mem::take(&mut loc.it_nodes), Relaxed);
            loc.ctx
                .rare
                .it_jobs
                .fetch_add(std::mem::take(&mut loc.it_jobs), Relaxed);
        }
        // (Release: the statistics of the jobs; Acquire: everybody else's.)
        if loc.ctx.outstanding.fetch_sub(c, AcqRel) == c && !loc.ctx.done.load(Acquire) {
            let ctx = loc.ctx.clone();
            self.count[C_BUILDS] += 1;
            if build(
                &mut self.scratch,
                &mut self.bufs,
                self.s,
                self.pool,
                &ctx,
                false,
            ) {
                while build_task(&mut self.scratch, self.s, self.pool, &ctx, self.id) {}
            }
        }
    }

    /// Leave the cube in `slot` (nothing pending).
    fn release(&mut self, slot: usize) {
        let loc = self.locals[slot].take().unwrap();
        debug_assert!(loc.pending == 0 && loc.claimed == 0);
        let ctx = loc.ctx;
        ctx.rare.nodes.fetch_add(loc.nodes, Relaxed);
        ctx.rare.lookups.fetch_add(loc.lookups, Relaxed);
        if ctx.parts.fetch_sub(1, AcqRel) == 1 {
            // Nobody works on the cube any more (the open list only reads
            // its atomics): recycle its buffers.
            // SAFETY: see above.
            let inner = std::mem::replace(unsafe { &mut *ctx.inner.get() }, Inner::new(vec![], 0));
            let mut spare = self.pool.spare.lock().unwrap();
            if spare.len() < self.pool.nthreads as usize {
                spare.push(inner);
            }
            drop(spare);
            let (sol, dt) = std::mem::take(&mut *ctx.result.lock().unwrap());
            let st = Stats {
                nodes: ctx.rare.nodes.load(Relaxed),
                lookups: ctx.rare.lookups.load(Relaxed),
            };
            (self.done)(ctx.index, sol, st, dt);
        }
    }

    /// The slot of `ctx`, joining it if needed (None if it is finished).
    fn local(&mut self, ctx: &Arc<CubeCtx>) -> Option<usize> {
        for (i, l) in self.locals.iter().enumerate() {
            if matches!(l, Some(l) if Arc::ptr_eq(&l.ctx, ctx)) {
                return Some(i);
            }
        }
        // (Like upgrading a weak reference: never revive a reported cube.)
        let mut p = ctx.parts.load(Acquire);
        loop {
            if p == 0 {
                return None;
            }
            match ctx.parts.compare_exchange_weak(p, p + 1, AcqRel, Acquire) {
                Ok(_) => break,
                Err(q) => p = q,
            }
        }
        let loc = Local {
            ctx: ctx.clone(),
            next: 0,
            end: 0,
            pending: 0,
            claimed: 0,
            nodes: 0,
            lookups: 0,
            it_nodes: 0,
            it_jobs: 0,
        };
        Some(match self.locals.iter().position(|l| l.is_none()) {
            Some(i) => {
                self.locals[i] = Some(loc);
                i
            }
            None => {
                self.locals.push(Some(loc));
                self.locals.len() - 1
            }
        })
    }

    /// Start a fresh cube in lane `li`.
    fn fresh(&mut self, li: usize) -> bool {
        let pool = self.pool;
        let s = self.s;
        while !pool.no_fresh.load(Relaxed) {
            // (A load first: idle threads call this often.)
            if pool.active.load(Relaxed) >= pool.max_active {
                return false;
            }
            if pool.active.fetch_add(1, AcqRel) >= pool.max_active {
                pool.active.fetch_sub(1, AcqRel);
                return false;
            }
            let i = pool.next.fetch_add(1, Relaxed);
            let Some(cube) = (pool.cubes)(i) else {
                pool.no_fresh.store(true, Release);
                pool.active.fetch_sub(1, AcqRel);
                pool.epoch.fetch_add(1, Release);
                return false;
            };
            let started = Instant::now();
            let c = Cube::from_cubie(&cube);
            if c.is_solved() {
                pool.active.fetch_sub(1, AcqRel);
                (self.done)(i, vec![], Stats::default(), 0.0);
                continue;
            }
            let ci = c.inverse();
            let (ln, li_) = (s.lower_bound(c), s.lower_bound(ci));
            let spare = pool.spare.lock().unwrap().pop();
            let mut inner = spare.unwrap_or_else(|| Inner::new(Vec::with_capacity(JOBS_CAP), 0));
            inner.target = ln.max(li_).max(1);
            let ctx = Arc::new(CubeCtx {
                index: i,
                started,
                root_c: c,
                root_ci: ci,
                claim: Pad(AtomicU64::new(0)),
                task: Pad(AtomicU64::new(0)),
                task_left: Pad(AtomicU32::new(0)),
                outstanding: Pad(AtomicU64::new(0)),
                done: AtomicBool::new(false),
                trace: pool.trace,
                parts: Pad(AtomicU32::new(1)),
                rare: Pad(Rare::default()),
                result: Mutex::new((vec![], 0.0)),
                tlog: (0..1024).map(|_| Default::default()).collect(),
                inner: UnsafeCell::new(inner),
            });
            {
                // SAFETY: nobody else knows the cube yet.
                let inner = unsafe { &mut *ctx.inner.get() };
                inner.jobs.clear();
                inner.jobs.push(Job {
                    moves: [0; 4],
                    len: 0,
                    ln,
                    li: li_,
                    stat: 0.0,
                });
                inner.stats.clear();
                inner.t0 = started;
                inner.t1 = started;
            }
            // Join before `build` publishes the cube: from then on another
            // thread may solve and report it, and joining would fail.
            let slot = self.local(&ctx).unwrap();
            if build(&mut self.scratch, &mut self.bufs, s, pool, &ctx, true) {
                while build_task(&mut self.scratch, s, pool, &ctx, self.id) {}
            }
            #[cfg(test)]
            if pool.pause_after_publish {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            self.count[C_FRESH] += 1;
            if self.continue_local(li, slot) {
                return true;
            }
        }
        false
    }

    /// Start a job of the oldest cube with unclaimed jobs in lane `li`.
    fn open_cube(&mut self, li: usize) -> bool {
        for _ in 0..4 {
            self.count[C_SCANS] += 1;
            let Some(ctx) = self.pool.find_open(false) else {
                return false;
            };
            if let Some(slot) = self.local(&ctx) {
                if self.continue_local(li, slot) {
                    self.count[C_OPEN] += 1;
                    return true;
                }
            }
        }
        false
    }

    /// Start the piece in this thread's mailbox in lane `li`.
    fn piece(&mut self, li: usize) -> bool {
        let Some(m) = self.mailbox() else {
            return false;
        };
        if m.state.load(Acquire) != FULL {
            return false;
        }
        let p = m.piece.lock().unwrap().take().unwrap();
        m.state.store(IDLE, Release);
        {
            // (A solved cube's outstanding count no longer matters.)
            if p.ctx.done.load(Relaxed) {
                return false;
            }
            let Some(slot) = self.local(&p.ctx) else {
                return false;
            };
            let loc = self.locals[slot].as_mut().unwrap();
            loc.pending += 1;
            loc.claimed += 1;
            let inner = p.ctx.inner();
            let sl = &mut self.lanes[li];
            let l = &mut sl.l;
            l.jobs[0] = inner.jobs[p.job as usize];
            l.job_pos = 0;
            l.target = inner.target;
            l.root_c = p.ctx.root_c;
            l.root_ci = p.ctx.root_ci;
            let d = p.nodes.len() - 1;
            l.stack[..=d].copy_from_slice(&p.nodes);
            l.sp = d;
            l.heur = 0.0;
            sl.slot = slot as u32;
            sl.job = p.job;
            sl.split = true;
            sl.n0 = l.stats.nodes;
            sl.k0 = l.stats.lookups;
            self.count[C_PIECES] += 1;
            true
        }
    }

    /// The lane and node to split for another lane: the active lane (other
    /// than `exclude`) with the shallowest node (remaining depth at least
    /// `min_r`) that has moves of two faces left. (lane, depth, moves to
    /// give away.)
    fn find_split(&self, min_r: u8, exclude: usize) -> Option<(usize, usize, u32)> {
        let mut best = None;
        let mut best_r = min_r - 1;
        for v in 0..self.nactive {
            let sl = &self.lanes[v];
            if v == exclude || sl.slot == NONE {
                continue;
            }
            let l = &sl.l;
            for d in 0..=l.sp {
                let n = &l.stack[d];
                if n.r <= best_r {
                    break;
                }
                if n.lb > n.r || n.todo == 0 {
                    continue;
                }
                // (An active lane's pending child is `k_m`.)
                let cur = if d < l.sp {
                    l.stack[d + 1].m / 3
                } else {
                    l.k_m / 3
                };
                if let Some(give) = split_moves(n.todo, cur) {
                    best = Some((v, d, give));
                    best_r = n.r;
                    break;
                }
            }
        }
        best
    }

    /// Give lane `li` part of a busy lane's remaining moves.
    fn split(&mut self, li: usize) -> bool {
        let Some((v, d, give)) = self.find_split(SPLIT_R_LOCAL, li) else {
            return false;
        };
        let (vl, tl) = two_mut(&mut self.lanes, v, li);
        tl.l.stack[..=d].copy_from_slice(&vl.l.stack[..=d]);
        for n in &mut tl.l.stack[..d] {
            n.todo = 0;
        }
        tl.l.stack[d].todo = give;
        vl.l.stack[d].todo &= !give;
        tl.l.sp = d;
        tl.l.heur = 0.0;
        tl.l.jobs[0] = vl.l.jobs[0];
        tl.l.job_pos = 0;
        tl.l.target = vl.l.target;
        tl.l.root_c = vl.l.root_c;
        tl.l.root_ci = vl.l.root_ci;
        tl.slot = vl.slot;
        tl.job = vl.job;
        tl.split = true;
        vl.split = true;
        tl.n0 = tl.l.stats.nodes;
        tl.k0 = tl.l.stats.lookups;
        let loc = self.locals[vl.slot as usize].as_mut().unwrap();
        loc.pending += 1;
        if self.pool.trace {
            loc.ctx.rare.it_splits.fetch_add(1, Relaxed);
        }
        self.count[C_SPLITS] += 1;
        true
    }

    /// Donate part of a busy lane's remaining moves to a hungry thread.
    fn donate(&mut self, round: u32) {
        if !after(round, self.donate_wait) {
            return;
        }
        let Some((v, d, give)) = self.find_split(SPLIT_R_REMOTE, usize::MAX) else {
            self.donate_backoff = (self.donate_backoff * 2).clamp(4, 256);
            self.donate_wait = round.wrapping_add(self.donate_backoff);
            return;
        };
        self.donate_backoff = 0;
        // a thread waiting for a piece
        let pool = self.pool;
        let nm = pool.mail.len();
        let Some(m) = (1..nm).map(|k| &pool.mail[(self.id + k) % nm].0).find(|m| {
            m.state.load(Relaxed) == WANT
                && m.state
                    .compare_exchange(WANT, FILL, AcqRel, Relaxed)
                    .is_ok()
        }) else {
            return;
        };
        pool.hungry.fetch_sub(1, Relaxed);
        let sl = &mut self.lanes[v];
        sl.split = true;
        let mut nodes = sl.l.stack[..=d].to_vec();
        for n in &mut nodes[..d] {
            n.todo = 0;
        }
        nodes[d].todo = give;
        sl.l.stack[d].todo &= !give;
        let ctx = self.locals[sl.slot as usize].as_ref().unwrap().ctx.clone();
        // (The job is outstanding, so the iteration cannot end before the
        // piece is counted.)
        ctx.outstanding.fetch_add(1, Relaxed);
        if pool.trace {
            ctx.rare.it_donated.fetch_add(1, Relaxed);
        }
        *m.piece.lock().unwrap() = Some(Piece {
            ctx,
            job: sl.job,
            nodes,
        });
        m.state.store(FULL, Release);
        self.count[C_DONATED] += 1;
    }

    /// Once per round if lanes are idle, a cube was solved or other threads
    /// are hungry.
    #[inline(never)]
    fn maintain(&mut self, round: u32) {
        let solved = self.pool.solved.load(Acquire);
        if solved != self.solved {
            self.solved = solved;
            self.cancel_done();
        }
        if self.nactive < self.lanes.len() {
            let e = self.pool.epoch.load(Acquire);
            if e != self.epoch || after(round, self.split_wait) {
                self.epoch = e;
                self.fill_idle(round);
            }
        }
        if self.pool.hungry.load(Relaxed) != 0 {
            self.donate(round);
        }
    }

    /// Give idle lanes work.
    fn fill_idle(&mut self, round: u32) {
        while self.nactive < self.lanes.len() {
            if !self.restart(self.nactive, None) {
                // back off from splitting (busy lanes may have nothing to
                // split for a while)
                self.split_backoff = (self.split_backoff * 2).clamp(4, 256);
                self.split_wait = round.wrapping_add(self.split_backoff);
                return;
            }
            self.nactive += 1;
        }
        self.split_backoff = 0;
    }

    /// Drop the jobs of solved cubes.
    fn cancel_done(&mut self) {
        let mut i = self.nactive;
        while i > 0 {
            i -= 1;
            let slot = self.lanes[i].slot as usize;
            if !self.locals[slot].as_ref().unwrap().ctx.done.load(Relaxed) {
                continue;
            }
            let slot = self.end_job(i, false);
            if !self.restart(i, Some(slot)) {
                self.deactivate(i);
            }
        }
    }

    /// No lane is active: find work or wait for some. Returns false when
    /// the pool is finished (or `stop` is set).
    #[inline(never)]
    fn idle_wait(&mut self, stop: &AtomicBool) -> bool {
        let pool = self.pool;
        loop {
            if stop.load(Relaxed) || pool.finished() {
                return false;
            }
            let e = pool.epoch.load(Acquire);
            // help extending job lists
            while let Some(ctx) = pool.find_open(true) {
                while build_task(&mut self.scratch, self.s, pool, &ctx, self.id) {}
            }
            self.fill_idle(0);
            if self.nactive > 0 {
                self.unwant();
                return true;
            }
            self.want();
            self.count[C_IDLE] += 1;
            let full = || matches!(self.mailbox(), Some(m) if m.state.load(Relaxed) == FULL);
            // Spin a little, then yield: a thread of ours with work may
            // share the core (until the scheduler moves one of us).
            for i in 0..1 << 12 {
                if pool.epoch.load(Relaxed) != e || full() {
                    break;
                }
                if i < 1 << 10 {
                    std::hint::spin_loop();
                } else {
                    std::thread::yield_now();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::{Cocsep, H48Table};

    /// Solve `cubes` with a pool of `threads` threads of `nlanes` lanes and at
    /// most `max_active` cubes in progress; checks each cube is reported once.
    fn solve_with_pool(
        solver: &Solver,
        cubes: &[CubieCube],
        threads: usize,
        nlanes: usize,
        max_active: usize,
        pause_after_publish: bool,
    ) -> Vec<Vec<u8>> {
        let get = |i: usize| cubes.get(i).copied();
        let mut pool = Pool::new(&get, max_active, threads);
        pool.pause_after_publish = pause_after_publish;
        let solutions = Mutex::new(vec![None; cubes.len()]);
        let stop = AtomicBool::new(false);
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    // A panic stops the other threads (else they could wait
                    // for its jobs forever and the test would hang).
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        pool.run(solver, nlanes, &stop, |i, solution, _, _| {
                            let mut all = solutions.lock().unwrap();
                            assert!(all[i].is_none(), "cube {i} reported twice");
                            all[i] = Some(solution);
                        });
                    }));
                    if let Err(e) = result {
                        stop.store(true, Relaxed);
                        std::panic::resume_unwind(e);
                    }
                });
            }
        });
        let solutions = solutions.into_inner().unwrap();
        solutions
            .into_iter()
            .enumerate()
            .map(|(i, s)| s.unwrap_or_else(|| panic!("cube {i} not reported")))
            .collect()
    }

    /// A fresh cube solved and reported by other threads as soon as it is
    /// published is still handled by its creator (which must have joined it).
    #[test]
    fn share_fresh_cube_solved_by_others_at_once() {
        let corners = Cocsep::truncated(4);
        let table = H48Table::zeroed(0, 2);
        let solver = Solver::new(&corners, &table);
        let cubes: Vec<CubieCube> = (0..6)
            .map(|m| CubieCube::SOLVED.apply_move(3 * m))
            .collect();
        for (threads, max_active) in [(4, 1), (3, 2)] {
            let solutions = solve_with_pool(&solver, &cubes, threads, 1, max_active, true);
            for (cube, solution) in cubes.iter().zip(&solutions) {
                assert_eq!(cube.apply_moves(solution), CubieCube::SOLVED);
                assert_eq!(solution.len(), 1);
            }
        }
    }

    /// Work sharing over a tiny search (claims, builds, splits, donations,
    /// cube hand-over): every cube reported once, as short as `Solver::solve`.
    #[test]
    fn share_protocol_with_tiny_search() {
        // Corner depths cut at 4 and base 2: lookups at remaining depth <= 6,
        // where local splits (4) and donations (5) happen; jobs from target 6.
        let corners = Cocsep::truncated(4);
        let table = H48Table::zeroed(0, 2);
        let solver = Solver::new(&corners, &table);
        let mut rng = crate::cubie::Rng(77);
        let count = if cfg!(miri) { 5 } else { 64 };
        let cubes: Vec<CubieCube> = (0..count)
            .map(|i| {
                let moves: Vec<u8> = (0..i % 7)
                    .map(|_| u8::try_from(rng.below(18)).unwrap())
                    .collect();
                CubieCube::SOLVED.apply_moves(&moves)
            })
            .collect();
        let optimal: Vec<usize> = cubes.iter().map(|c| solver.solve(c).0.len()).collect();
        // (threads, lanes per thread, cubes in progress)
        let configs: &[(usize, usize, usize)] = if cfg!(miri) {
            &[(3, 2, usize::MAX), (2, 1, 1)]
        } else {
            &[
                (2, 1, 1),
                (3, 2, usize::MAX),
                (4, 3, 2),
                (5, 1, 1),
                (8, 2, 3),
                (16, 1, 4),
            ]
        };
        for &(threads, nlanes, max_active) in configs {
            let solutions = solve_with_pool(&solver, &cubes, threads, nlanes, max_active, false);
            for (i, (cube, solution)) in cubes.iter().zip(&solutions).enumerate() {
                assert_eq!(cube.apply_moves(solution), CubieCube::SOLVED, "cube {i}");
                assert_eq!(
                    solution.len(),
                    optimal[i],
                    "cube {i}: {threads} threads, {nlanes} lanes, {max_active} cubes"
                );
            }
        }
    }

    /// The staged sort (its tasks run in any order within a stage) gives
    /// the order of `sort_jobs`.
    #[test]
    fn staged_sort_matches_sort_jobs() {
        let mut rng = crate::cubie::Rng(11);
        for n in [8192usize, 20000, 43254] {
            let jobs: Vec<Job> = (0..n)
                .map(|i| {
                    let r = rng.next_u64();
                    // many ties and a wide range of keys
                    let stat = match r % 5 {
                        0 => 0.0,
                        1 => (r >> 8) as u8 as f32,
                        _ => f32::from_bits((r >> 16) as u32 % 0x4000_0000 + 0x0080_0000),
                    };
                    Job {
                        moves: [(r >> 40) as u8 % 18, 0, 0, (i % 251) as u8],
                        len: 1,
                        ln: (i / 251) as u8,
                        li: 0,
                        stat,
                    }
                })
                .collect();
            let mut want = jobs.clone();
            super::super::sort_jobs(&mut want, &mut Vec::new());
            // all feasible
            let mut serial = jobs.clone();
            assert_eq!(sort_serial(&mut serial, &mut Bufs::sized(), 255), n);
            let id = |j: &Job| (j.moves, j.ln, j.stat.to_bits());
            assert!(serial.iter().map(id).eq(want.iter().map(id)), "n = {n}");
            // the feasible ones in that order, then the others
            let t = 100;
            let feasible = |j: &&Job| (j.ln as usize) < t;
            let want_t: Vec<Job> = want
                .iter()
                .filter(feasible)
                .chain(want.iter().filter(|j| !feasible(j)))
                .copied()
                .collect();
            let nf = want.iter().filter(feasible).count();
            let mut serial = jobs.clone();
            assert_eq!(sort_serial(&mut serial, &mut Bufs::sized(), t as u8), nf);
            assert!(serial.iter().map(id).eq(want_t.iter().map(id)), "n = {n}");
            let mut inner = Inner::new(jobs, t as u8);
            inner.b = Bufs::sized();
            let mut ntasks = sort_setup(&mut inner);
            let mut stage = ST_KEYS;
            loop {
                for c in (0..ntasks).rev() {
                    // SAFETY: one task at a time.
                    unsafe { sort_task(&inner, stage, c) };
                }
                match sort_stage_done(&mut inner, stage) {
                    Some((next, k)) => (stage, ntasks) = (next, k),
                    None => break,
                }
            }
            assert!(
                inner.jobs.iter().map(id).eq(want_t.iter().map(id)),
                "n = {n}"
            );
            assert_eq!(inner.nf, nf);
        }
    }
}
