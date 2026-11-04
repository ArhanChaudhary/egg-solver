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
//! * A cube's first iterations, up to the first one with 3-move jobs (16
//!   with h11), run as one pass on those jobs: each job once per iteration,
//!   claimed in the order of the iterations but without barriers between
//!   them. A solution is reported once the iterations below its length have
//!   all ended.
//! * Iteration T+1 is built (jobs extended, sorted by the statistics of
//!   iteration T) by the thread that finishes the last job of iteration T. A
//!   solution found in iteration T is therefore optimal and is reported at
//!   once; lanes still running jobs of the cube drop them (lanes are checked
//!   once per round after a cube is solved). If threads are waiting for work,
//!   a long job list is extended and sorted by all of them (`build_task`).
//!   Jobs whose bounds exceed the iteration are sorted last and not claimed.
//! * With few cubes in progress (at most threads / 8, e.g. one at a time),
//!   builds are always staged and threads stay with the cubes they work on.
//! * A lane whose job ends takes, in order: a piece donated to its thread,
//!   another job of the same cube, a job its thread already claimed, a fresh
//!   cube (at most `max_active` cubes are in progress), jobs of the oldest
//!   cube with unclaimed jobs, or (while its thread has at most
//!   `TAIL_LANES` active lanes) part of the remaining moves of a busy lane
//!   of its thread: the upper half of the faces left at its shallowest node
//!   with two faces left (cube48opt's `split_ida_node`). Otherwise it goes
//!   idle.
//! * A thread without active lanes asks for work in its mailbox; busy
//!   threads check once per round whether some thread is waiting and donate
//!   it a split of one of their lanes (a copy of the path to the split
//!   node; the piece's slot is written and read in the order of the
//!   mailbox states). Waiting threads spin briefly, then yield.
//! * The job buffers of reported cubes are reused by new ones, the other
//!   build buffers by later builds: builds touch no fresh pages.
//!
//! Each job runs exactly once; its statistic is the sum over its pieces. A
//! cube's node count includes work done on it after its solution was found
//! by another lane. The search kernel is the one of stream mode: `advance`
//! of a `Lane::shared` lane only differs in returning at the end of a job,
//! and everything here runs at job boundaries or once per round.
//!
//! Gains, 48 threads, h11, `random_state`: the mode itself (l5) took a batch
//! from 7.86 ms to 1.26 ms per cube (bench random:1000 from 187 to 795
//! solves/s), and one cube at a time from 306 ms (one lane) to 3.6 ms.
//! l6-share-a and l6-share-2 brought one cube at a time to ~2.2 ms. The
//! comments at each optimization give its own measured gain.

use super::{job_weight, moves_after, Job, Lane, Node, Phase, Step, MAXD, MAX_JOB_LEN};
use crate::cubie::CubieCube;
use crate::simd::Cube;
use crate::solver::{Solver, Stats, ALLOWED_AFTER, ALL_MOVES};
use crate::symstart::SymFilter;
use std::cell::UnsafeCell;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize};
use std::sync::{Arc, Mutex, PoisonError};
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
/// Most active lanes of a thread for it to split one of them for an idle
/// one (`Worker::split`).
const TAIL_LANES: usize = 8;
/// `SharedLane::slot` of an idle lane.
const NO_SLOT: usize = usize::MAX;

/// Called once per cube: (index, solution, stats, seconds from its start to
/// its solution).
pub type Done<'d> = dyn FnMut(usize, Vec<u8>, Stats, f64) + 'd;

/// Iteration data, written only by the thread that builds an iteration, while
/// no job of the cube is outstanding (nobody else reads it then).
struct Inner {
    /// The jobs of the current iteration, sorted. Only the first `feasible`
    /// can hold a solution of length `target`, and only they are claimed.
    jobs: Vec<Job>,
    feasible: usize,
    /// Statistic (f32 bits) of each job of the current iteration, summed
    /// over the job's pieces.
    stats: Vec<AtomicU32>,
    target: u8,
    /// Iterations that the current one stands for: 1, or in a cube's first
    /// pass (`build`) one phase per iteration up to `target`. Claim `entry`
    /// is job `entry % phase_entries` in phase `entry / phase_entries`
    /// (`entry_job`), whose target is `phase_target`.
    phases: u32,
    phase_entries: u32,
    /// The jobs are new (the cube's first build): sorted by their bounds.
    sort_by_bounds: bool,
    /// The jobs record their leads (`lead_word`): the next iteration, on
    /// the same jobs, is split. `leads_target`: the last iteration that did.
    record_leads: bool,
    leads_target: u8,
    /// Per job, its lead from the last iteration that recorded them (indexed
    /// like `jobs` and `stats`; `MAX_JOBS` long).
    leads: Vec<AtomicU32>,
    /// A split iteration (`Bufs::rest_children`): claim entry e runs
    /// `runs[e]`, job `runs[e] as u32` (an index of `jobs`, which keep the
    /// last iteration's order) with the root children `runs[e] >> 32`.
    split: bool,
    runs: Vec<u64>,
    /// Split iterations: leads with codes above it run first.
    lead_threshold: u8,
    /// The tags (`lead_tag`) of the leads that this iteration's build reads
    /// and that its jobs write; the cube's part of them (set per cube).
    read_tag: u32,
    write_tag: u32,
    tag_base: u32,
    /// Split iterations: the first-part runs (`runs[..first_runs]`).
    first_runs: u32,
    /// Buffers of the build in progress (from `Pool::bufs`).
    b: Bufs,
    /// (trace) When the current iteration's build started, its jobs were
    /// extended, the stages of a parallel sort ended, and it was published.
    build_started: Instant,
    extended: Instant,
    sort_stages_ended: [Instant; 3],
    published: Instant,
}

/// Most jobs of an iteration (the canonical sequences of 4 moves), and the
/// capacity of every job buffer (`Lane::extend_jobs` reserves 15 children
/// per parent). Job buffers are allocated at full size and then only
/// swapped around, and the other buffers of a build are reused through
/// `Pool::bufs`: builds never touch fresh pages (whose faults, ~1 us per
/// 4 KiB, took most of the time of a parallel build).
// No fresh pages in builds: part of l6-share-a (-38.8% -c 1 latency at 48T,
// with the staged builds on all threads).
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
    extension_moves: usize,
    /// The sort keys take the statistics from `Inner::stats` (the jobs are
    /// those of the last iteration).
    stats_from_atomics: bool,
    child_offsets: Vec<u32>,
    out: Vec<Job>,
    keys: Vec<u64>,
    keys_tmp: Vec<u64>,
    key_buckets: Vec<u8>,
    splitters: Vec<u64>,
    bucket_counts: Vec<u32>,
    bucket_starts: Vec<u32>,
    /// Split iterations: per job the root children of its rest run (all,
    /// or those outside its lead group if it has a first-part run); per
    /// chunk and lead bucket the first-part runs (then their positions);
    /// the number of first-part runs.
    rest_children: Vec<u32>,
    lead_counts: Vec<u32>,
    first_runs: usize,
    out_ptr: TaskPtr<Job>,
    keys_ptr: TaskPtr<u64>,
    keys_tmp_ptr: TaskPtr<u64>,
    key_buckets_ptr: TaskPtr<u8>,
    bucket_counts_ptr: TaskPtr<u32>,
    rest_children_ptr: TaskPtr<u32>,
    lead_counts_ptr: TaskPtr<u32>,
    runs_ptr: TaskPtr<u64>,
}

/// The start of a build buffer that the tasks of a stage write at once, each
/// its own part (through a raw pointer: references to the whole buffer
/// would alias).
#[derive(Clone, Copy)]
struct TaskPtr<T>(*mut T);

// SAFETY: the build stages order the accesses (`build_task`).
unsafe impl<T> Send for TaskPtr<T> {}
unsafe impl<T> Sync for TaskPtr<T> {}

impl<T> Default for TaskPtr<T> {
    fn default() -> Self {
        TaskPtr(std::ptr::null_mut())
    }
}

impl Bufs {
    /// The buffers of staged builds (`Pool::bufs`), touched (`touched`).
    fn sized() -> Bufs {
        Bufs {
            out: touched(JOBS_CAP, EMPTY_JOB),
            keys: touched(MAX_JOBS, 0),
            keys_tmp: touched(MAX_JOBS, 0),
            key_buckets: touched(MAX_JOBS, 0),
            rest_children: touched(MAX_JOBS, 0),
            ..Bufs::default()
        }
    }

    /// A worker's own buffers (serial builds). Its job buffer is touched
    /// (job buffers change hands), its sort keys grow when needed (serial
    /// sorts of long job lists only run with many cubes in progress, one
    /// thread each). Its key buckets (43 KB) are touched too, though serial
    /// sorts do not use them: then the thread's malloc arena grows when the
    /// thread starts. The arenas also serve the big buffers (strace: glibc
    /// raised its mmap threshold when a 4.9 MB block was freed at startup),
    /// and otherwise they grew page by page (mprotect) during the first
    /// cubes' builds: the parallel extension to 4-move jobs of the first 50
    /// cubes still took 0.18-0.26 ms instead of 0.05 (solve -c 1,
    /// `random_move_17f`, 48T; 0.04 ms without this touch but with glibc's
    /// `top_pad` at 4 MB). All sort buffers touched: the same, but a
    /// 48-thread pool took 0.73 ms instead of 0.42 ms more to start.
    fn own() -> Bufs {
        Bufs {
            out: touched(JOBS_CAP, EMPTY_JOB),
            key_buckets: touched(MAX_JOBS, 0),
            ..Bufs::default()
        }
    }
}

/// An empty vector of `capacity` elements whose pages were all written once.
/// Vectors only reserved left the pages of their unused tail fresh, and job
/// buffers change hands (cube, builder, pooled buffers): a pool's first
/// cubes took page faults in their parallel builds, where faulting threads
/// wait for each other (the extension to 4-move jobs took 0.26 ms instead of
/// 0.02 ms for about half of the first 50 cubes of `solve -c 1` at 48T).
// With `WARM_BUFFERS` and `Bufs::own` (c11): -c 1 at 48T -8.1% on 17-move
// scrambles, -9.6% on depth-17 states, -0.7% on 500 random states (7 paired
// holds); a 48-thread pool starts 0.42 ms later.
fn touched<T: Clone>(capacity: usize, value: T) -> Vec<T> {
    let mut v = Vec::with_capacity(capacity);
    // (Miri: not needed, and slow.)
    if !cfg!(miri) {
        v.resize(capacity, value);
        std::hint::black_box(&mut v);
        v.clear();
    }
    v
}

/// A job slot's value before it is written.
const EMPTY_JOB: Job = Job {
    moves: [0; 4],
    len: 0,
    normal_bound: 0,
    inverse_bound: 0,
    stat: 0.0,
};

/// Touched buffers made for each of the first threads of a pool when it
/// starts (`Pool::run`), so that the first cubes find them in `Pool::spare`
/// and `Pool::bufs` (with -c 1, two cubes overlap and builds are one at a
/// time).
const WARM_BUFFERS: usize = 3;

/// A reader-writer spin lock. The threads of a pool have cores of their
/// own and hold it briefly: waiting must not sleep (with `std`'s lock,
/// readers queued behind a writer sleep in the kernel, for tens of us).
// Instead of std's RwLock: part of l6-share-a (-38.8% -c 1 latency at 48T).
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
            let state = self.state.load(Relaxed);
            if state & SPIN_W == 0
                && self
                    .state
                    .compare_exchange_weak(state, state + 1, Acquire, Relaxed)
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
// Yields after 256 spins: with pure spinning and more threads than cores,
// the pool stalled (l6-share-a; a fix, not measured as a gain).
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
// Padding of the shared atomics, rarely written fields and free lists
// (l6-share-2; screening run without it: bench share 787 vs 845/s at 48T).
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
    /// The scrambled cube and its inverse.
    root: Cube,
    root_inverse: Cube,
    /// Symmetric-start filter of the jobs (`symstart`).
    sym_filter: SymFilter,
    /// Solved (no more iterations; running jobs are dropped). (Written
    /// once: with the read-mostly fields.)
    done: AtomicBool,
    /// In a first pass (`Inner::phases`): per phase, the entries (and
    /// donated pieces) not yet ended, counted down by each thread when its
    /// own have all ended (`Worker::phase_ended`).
    phase_left: Pad<[AtomicU64; MAX_PHASES]>,
    trace: bool,
    /// Jobs of the current iteration: `count << 32 | next unclaimed`. One
    /// `fetch_add` claims a chunk; the count in the same word tells a late
    /// claimer that it got nothing (no separate, racing check).
    // Jobs claimed in chunks by one atomic: part of l5's share mode (batch
    // 7.86 -> 1.26 ms/cube at 48T; not measured alone).
    claim: Pad<AtomicU64>,
    /// Tasks of a parallel build: `stage << 56 | count << 32 | next
    /// unclaimed`, and the number of tasks of the stage not yet done.
    task: Pad<AtomicU64>,
    task_left: Pad<AtomicU32>,
    /// Jobs (and donated pieces) of the current iteration not yet finished,
    /// counted down by each thread when its claimed jobs are all done: the
    /// thread that brings it to 0 knows the whole iteration ended (the
    /// barrier), and builds the next one.
    outstanding: Pad<AtomicU64>,
    /// 1 until solved, plus one per thread working on the cube; the thread
    /// that drops the last one reports the cube.
    parts: Pad<AtomicU32>,
    /// Written now and then (not to disturb the read-mostly fields).
    rarely_written: Pad<RarelyWritten>,
    /// (solution, seconds).
    result: Mutex<(Vec<u8>, f64)>,
    /// A first pass's shortest solution so far.
    pass: Mutex<PassState>,
    inner: UnsafeCell<Inner>,
    /// (trace) Per build task (stage - 1) * 256 + task: thread + 1, start and
    /// end (ms, f32 bits).
    task_log: Box<[[AtomicU32; 3]]>,
    /// (trace) Per thread: ns from `started` when it last finished its
    /// claimed jobs (`settle`).
    thread_done_ns: Box<[AtomicU64]>,
}

#[derive(Default)]
struct RarelyWritten {
    /// On `Pool::open` (changed under its write lock).
    listed: AtomicBool,
    /// Summed by the threads leaving the cube.
    nodes: AtomicU64,
    lookups: AtomicU64,
    /// (trace) ns from `started` when the claims of the iteration ran out,
    /// nodes and jobs of the iteration.
    claims_ran_out_ns: AtomicU64,
    iteration_nodes: AtomicU64,
    iteration_jobs: AtomicU64,
    iteration_donations: AtomicU64,
    iteration_splits: AtomicU64,
    /// The length of a first pass's shortest solution so far (`u8::MAX`
    /// if none): entries whose target is as long are dropped.
    pass_limit: AtomicU8,
}

/// The solutions of a cube's first pass (`Inner::phases`): the shortest
/// found so far (empty if none) until it is reported, its phase, and the
/// number of leading phases whose entries have all ended. A solution is
/// optimal once the phases below its own ended.
#[derive(Default)]
struct PassState {
    candidate: Vec<u8>,
    candidate_phase: u32,
    phases_ended: u32,
}

// `inner` is only written by the builder while nobody else accesses it.
unsafe impl Sync for CubeCtx {}

impl Inner {
    /// An iteration's buffers at full size, touched (`touched`).
    fn touched() -> Inner {
        let mut inner = Inner::new(touched(JOBS_CAP, EMPTY_JOB), 0);
        inner.stats.reserve_exact(MAX_JOBS);
        if !cfg!(miri) {
            inner.stats.resize_with(MAX_JOBS, || AtomicU32::new(0));
            inner.stats.clear();
        }
        inner.leads.resize_with(MAX_JOBS, || AtomicU32::new(0));
        inner.runs = touched(2 * MAX_JOBS, 0);
        inner
    }

    fn new(jobs: Vec<Job>, target: u8) -> Inner {
        let now = Instant::now();
        Inner {
            jobs,
            feasible: 0,
            stats: Vec::new(),
            target,
            phases: 1,
            phase_entries: 0,
            sort_by_bounds: false,
            record_leads: false,
            leads_target: 0,
            leads: Vec::new(),
            split: false,
            runs: Vec::new(),
            lead_threshold: u8::MAX,
            read_tag: 0,
            write_tag: 0,
            tag_base: 0,
            first_runs: 0,
            b: Bufs::default(),
            build_started: now,
            extended: now,
            sort_stages_ended: [now; 3],
            published: now,
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
        let (count, next) = claim_fields(self.claim.load(Relaxed));
        count.saturating_sub(next)
    }

    fn task_unclaimed(&self) -> bool {
        let (_, count, next) = task_fields(self.task.load(Relaxed));
        next < count
    }

    /// Unclaimed jobs or build tasks.
    fn open(&self) -> bool {
        !self.done.load(Relaxed) && (self.unclaimed() > 0 || self.task_unclaimed())
    }

    /// Claim up to `chunk` jobs of the current iteration: (first, count).
    fn claim(&self, chunk: u32) -> Option<(u32, u32)> {
        let (count, next) = claim_fields(self.claim.load(Acquire));
        // (Checked first so that exhausted counters are not bumped forever.)
        if next >= count || self.done.load(Relaxed) {
            if self.trace && self.rarely_written.claims_ran_out_ns.load(Relaxed) == 0 {
                let ns = u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX);
                let _ = self
                    .rarely_written
                    .claims_ran_out_ns
                    .compare_exchange(0, ns, Relaxed, Relaxed);
            }
            return None;
        }
        let (count, next) = claim_fields(self.claim.fetch_add(u64::from(chunk), AcqRel));
        (next < count).then(|| (next, chunk.min(count - next)))
    }
}

/// The job count and the next unclaimed job of a claim word.
#[expect(clippy::cast_possible_truncation, reason = "the halves of the word")]
fn claim_fields(word: u64) -> (u32, u32) {
    ((word >> 32) as u32, word as u32)
}

/// The stage, task count and next unclaimed task of a task word.
#[expect(clippy::cast_possible_truncation, reason = "fields of the word")]
fn task_fields(word: u64) -> (usize, usize, usize) {
    (
        (word >> 56) as usize,
        (word >> 32) as usize & 0xFF_FFFF,
        word as u32 as usize,
    )
}

/// `*stat += v` for f32 bits.
fn add_f32(stat: &AtomicU32, v: f32) {
    let mut old = stat.load(Relaxed);
    while let Err(now) =
        stat.compare_exchange_weak(old, (f32::from_bits(old) + v).to_bits(), Relaxed, Relaxed)
    {
        old = now;
    }
}

/// Part of a job's tree donated to another thread: the path from the job
/// root (`nodes[..=depth]`), whose last node holds the donated moves (the
/// others have none).
struct Piece {
    ctx: Option<Arc<CubeCtx>>,
    job: u32,
    /// The phase of the donor's entry (`Inner::phases`).
    phase: u8,
    depth: usize,
    nodes: [Node; MAXD],
}

/// Mailbox states: the owner has work, waits for a piece, a donor is
/// writing one, a piece is there. Only the owner moves IDLE -> WANT and
/// FULL -> IDLE, and only the one donor whose CAS won moves WANT -> FILL ->
/// FULL: the piece slot never has two writers.
const IDLE: u8 = 0;
const WANT: u8 = 1;
const FILL: u8 = 2;
const FULL: u8 = 3;

/// A thread's mailbox: a thread without work asks for a piece (`WANT`); a
/// busy thread takes the request (`FILL`) and delivers (`FULL`).
// A fixed piece slot instead of a Mutex<Option<Piece>> with a Vec per
// donation (l6-share-2; screening run with the Mutex: bench share 827 vs
// 845/s at 48T).
struct Mailbox {
    state: AtomicU8,
    /// Written only by the donor that moved `state` from `WANT` to `FILL`,
    /// then read by the owner once it is `FULL` (ordered by `state`).
    piece: UnsafeCell<Piece>,
}

// `piece` is accessed as described above.
unsafe impl Sync for Mailbox {}

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
    open_count: Pad<AtomicUsize>,
    /// Per thread: a piece donated to it while it has no active lane.
    mail: Vec<Pad<Mailbox>>,
    next_thread_id: AtomicUsize,
    /// Threads waiting for a piece (mailboxes in state `WANT`).
    hungry: Pad<AtomicUsize>,
    /// Bumped whenever new work may be available.
    epoch: Pad<AtomicU64>,
    /// Bumped whenever a cube is solved or a first pass finds a shorter
    /// solution (lanes may have jobs to drop).
    solved: Pad<AtomicU64>,
    /// Buffers of reported cubes, for new ones, and of builds.
    spare: Pad<Mutex<Vec<Inner>>>,
    bufs: Pad<Mutex<Vec<Bufs>>>,
    counters: Mutex<[u64; NCOUNT]>,
    /// Trace each iteration and solution (env `CUBE_TRACE` set): times,
    /// threads' finish times and the solving job. `CUBE_TRACE` = 1 plus the
    /// bits `TRACE_*` adds more, at a cost.
    trace: bool,
    trace_bits: u32,
    /// (trace) The lines, printed when the pool is dropped: printed at once,
    /// with the counters, they took -c 1 from 2.0 to 2.9 ms per cube at 48
    /// threads (timestamps alone, buffered: ~2%).
    trace_out: Mutex<String>,
    /// (trace) ns spent in `idle_wait`, summed over threads.
    idle_ns: AtomicU64,
    started: Instant,
    /// Iterations a cube's first pass stands for, at most (`build`).
    first_pass_span: u8,
    /// (Tests) Pause after publishing a fresh cube, so that other threads
    /// can solve and report it before its creator goes on.
    #[cfg(test)]
    pause_after_publish: bool,
}

/// `CUBE_TRACE` bits: nodes, jobs, splits and donations per iteration (atomics
/// shared by all threads: +35% -c 1 latency at 48 threads), build tasks, time
/// in `idle_wait`.
const TRACE_COUNTERS: u32 = 2;
const TRACE_TASKS: u32 = 4;
const TRACE_IDLE: u32 = 8;

/// Event counters (summed over threads; see `Pool::counters`).
const NCOUNT: usize = 14;
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
    "pass_solutions",
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
const C_PASS_SOLUTIONS: usize = 13;

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
            nthreads: u32::try_from(nthreads.max(1)).unwrap_or(u32::MAX),
            next: Pad(AtomicUsize::new(0)),
            no_fresh: AtomicBool::new(false),
            active: Pad(AtomicUsize::new(0)),
            open: Pad(SpinRw::new(Vec::new())),
            open_count: Pad(AtomicUsize::new(0)),
            mail: (0..nthreads.max(1))
                .map(|_| {
                    Pad(Mailbox {
                        state: AtomicU8::new(IDLE),
                        piece: UnsafeCell::new(Piece {
                            ctx: None,
                            job: 0,
                            phase: 0,
                            depth: 0,
                            nodes: [Lane::new().stack[0]; MAXD],
                        }),
                    })
                })
                .collect(),
            next_thread_id: AtomicUsize::new(0),
            hungry: Pad(AtomicUsize::new(0)),
            epoch: Pad(AtomicU64::new(0)),
            solved: Pad(AtomicU64::new(0)),
            spare: Pad(Mutex::new(Vec::new())),
            bufs: Pad(Mutex::new(Vec::new())),
            counters: Mutex::new([0; NCOUNT]),
            trace: std::env::var_os("CUBE_TRACE").is_some(),
            trace_out: Mutex::new(String::new()),
            trace_bits: std::env::var("CUBE_TRACE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            idle_ns: AtomicU64::new(0),
            started: Instant::now(),
            first_pass_span: FIRST_PASS_SPAN,
            #[cfg(test)]
            pause_after_publish: false,
        }
    }

    /// Work on the pool with `nlanes` interleaved lanes until all cubes are
    /// solved or `stop` is set. Calls `done(index, solution, stats,
    /// seconds)` for the cubes whose last worker is this thread.
    pub fn run(
        &self,
        solver: &Solver,
        nlanes: usize,
        stop: &AtomicBool,
        mut done: impl FnMut(usize, Vec<u8>, Stats, f64),
    ) {
        let id = self.next_thread_id.fetch_add(1, Relaxed);
        if id < WARM_BUFFERS {
            let inner = Inner::touched();
            let mut bufs = Bufs::sized();
            bufs.pooled = true;
            // (Buffers only: a panic of a thread holding them does not
            // matter.)
            self.spare
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(inner);
            self.bufs
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(bufs);
        }
        let mut worker = Worker {
            pool: self,
            solver,
            id,
            done: &mut done,
            lanes: (0..nlanes.max(1)).map(|_| SharedLane::new()).collect(),
            active_lanes: 0,
            locals: Vec::new(),
            scratch: Box::new(Lane::new()),
            bufs: Bufs::own(),
            epoch: 0,
            solved: 0,
            split_wait: 0,
            split_backoff: 0,
            donate_wait: 0,
            donate_backoff: 0,
            counts: [0; NCOUNT],
        };
        worker.run(stop);
        // (Counters only: a panic of another thread while it held them
        // does not matter.)
        let mut counters = self.counters.lock().unwrap_or_else(PoisonError::into_inner);
        for (sum, count) in counters.iter_mut().zip(worker.counts) {
            *sum += count;
        }
    }

    /// Event counters of the finished threads, e.g. for tuning: rounds, the
    /// mean number of active lanes per round, jobs started, ...
    #[expect(clippy::cast_precision_loss, reason = "a ratio of counts")]
    pub fn counters(&self) -> String {
        let counts = *self.counters.lock().unwrap_or_else(PoisonError::into_inner);
        let mut out = format!(
            "active lanes/round {:.2}",
            counts[C_VISITS] as f64 / counts[C_ROUNDS].max(1) as f64
        );
        out.extend(
            COUNT_NAMES
                .iter()
                .zip(counts)
                .map(|(name, count)| format!(" {name} {count}")),
        );
        out
    }

    /// Make sure `ctx` is on the open list (new jobs or build tasks).
    fn publish(&self, ctx: &Arc<CubeCtx>) {
        let mut open = self.open.write();
        // Drop the cubes with nothing left to claim (else nobody may: when
        // no lane is idle).
        if open.len() >= 64 && open.len().is_power_of_two() {
            open.retain(keep_open);
        }
        // (`listed` changes only under the write lock.)
        if !ctx.rarely_written.listed.load(Relaxed) {
            ctx.rarely_written.listed.store(true, Relaxed);
            open.push(ctx.clone());
        }
        self.open_count.store(open.len(), Relaxed);
        drop(open);
        self.epoch.fetch_add(1, Release);
    }

    /// The oldest cube with unclaimed jobs (`task`: build tasks).
    fn find_open(&self, task: bool) -> Option<Arc<CubeCtx>> {
        if self.open_count.load(Relaxed) == 0 {
            return None;
        }
        let open = self.open.read();
        let mut dead = 0;
        let mut found = None;
        for ctx in open.iter() {
            if !ctx.open() {
                dead += 1;
                continue;
            }
            let wanted = if task {
                ctx.task_unclaimed()
            } else {
                ctx.unclaimed() > 0
            };
            if wanted {
                found = Some(ctx.clone());
                break;
            }
        }
        drop(open);
        if dead > 0 {
            // (Not a try: under the read locks of the threads looking for
            // work it would rarely succeed, and the dead cubes would keep
            // them all taking the lock. Part of l6-share-a: -38.8% -c 1
            // latency at 48T.)
            let mut open = self.open.write();
            open.retain(keep_open);
            self.open_count.store(open.len(), Relaxed);
        }
        found
    }

    /// Buffers for a staged build.
    fn take_bufs(&self) -> Bufs {
        let mut bufs = self.bufs.lock().unwrap().pop().unwrap_or_else(Bufs::sized);
        bufs.pooled = true;
        bufs.stats_from_atomics = false;
        bufs
    }

    /// Give back the buffers of a staged build (if any).
    fn put_bufs(&self, bufs: Bufs) {
        if bufs.pooled {
            self.bufs.lock().unwrap().push(bufs);
        }
    }

    /// Few cubes in progress (each to be shared by many threads).
    fn few(&self) -> bool {
        self.active.load(Relaxed) * FEW <= self.nthreads as usize
    }

    fn finished(&self) -> bool {
        self.no_fresh.load(Acquire) && self.active.load(Acquire) == 0
    }
}

impl Drop for Pool<'_> {
    fn drop(&mut self) {
        if self.trace {
            let out = std::mem::take(
                &mut *self
                    .trace_out
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner),
            );
            eprint!("{out}");
            let wall = self.started.elapsed().as_secs_f64();
            #[expect(clippy::cast_precision_loss, reason = "trace output")]
            let idle = self.idle_ns.load(Relaxed) as f64 * 1e-9;
            eprintln!(
                "trace pool wall {wall:.6} s, idle {:.1}% of thread time",
                idle / (wall * f64::from(self.nthreads)) * 100.0
            );
        }
    }
}

/// (trace) Append a line to the pool's trace.
fn tlog(pool: &Pool, args: std::fmt::Arguments) {
    use std::fmt::Write;
    let mut out = pool
        .trace_out
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let _ = out.write_fmt(args);
    out.push('\n');
}

/// `retain` predicate of the open list: drops (and unlists) the cubes with
/// nothing left to claim.
fn keep_open(ctx: &Arc<CubeCtx>) -> bool {
    let keep = ctx.open();
    if !keep {
        ctx.rarely_written.listed.store(false, Relaxed);
    }
    keep
}

/// Solve all cubes of `cubes` (until it returns None) with `threads` threads
/// of `nlanes` lanes, at most `max_active` cubes at a time; `done` is called
/// once per cube (from any thread).
pub fn solve_all(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    threads: usize,
    nlanes: usize,
    max_active: usize,
    done: &(dyn Fn(usize, Vec<u8>, Stats, f64) + Sync),
) {
    let pool = Pool::new(cubes, max_active, threads);
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| pool.run(solver, nlanes, &stop, done));
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
    /// The same per phase of a first pass (`Inner::phases`), counted down
    /// from `CubeCtx::phase_left`.
    phase_pending: [u32; MAX_PHASES],
    phase_claimed: [u64; MAX_PHASES],
    nodes: u64,
    lookups: u64,
    /// (trace) Of the current iteration.
    iteration_nodes: u64,
    iteration_jobs: u64,
}

/// A lane of a worker, and the job it runs.
struct SharedLane {
    lane: Lane,
    /// `Worker::locals` slot of the cube of the current job (`NO_SLOT` if
    /// idle).
    slot: usize,
    /// Index of the current job in the cube's job list, and (trace) its
    /// claim entry.
    job: u32,
    entry: u32,
    /// The root children of a run that records its job's lead (0: none).
    lead_children: u32,
    /// The job's tree is shared with other lanes (split, donated or a
    /// piece): its statistic is added atomically.
    split: bool,
    /// The phase of the job's entry (`Inner::phases`; 0 in other
    /// iterations).
    phase: u8,
    /// Lane counters at the start of the job.
    nodes_at_start: u64,
    lookups_at_start: u64,
    /// (trace) When the job (or piece) started.
    started: Instant,
}

impl SharedLane {
    fn new() -> SharedLane {
        let mut lane = Lane::new();
        lane.shared = true;
        lane.jobs.push(Job {
            moves: [0; 4],
            len: 0,
            normal_bound: 0,
            inverse_bound: 0,
            stat: 0.0,
        });
        SharedLane {
            lane,
            slot: NO_SLOT,
            job: 0,
            entry: 0,
            lead_children: 0,
            split: false,
            phase: 0,
            nodes_at_start: 0,
            lookups_at_start: 0,
            started: Instant::now(),
        }
    }
}

/// A thread of a pool: its lanes, and its share of the cubes it works on.
struct Worker<'w, 'a> {
    pool: &'w Pool<'a>,
    solver: &'w Solver<'w>,
    /// Mailbox index (none if beyond `Pool::nthreads`).
    id: usize,
    done: &'w mut Done<'w>,
    /// The active lanes are `lanes[..active_lanes]`.
    lanes: Vec<SharedLane>,
    active_lanes: usize,
    /// The cubes this thread works on, by slot (`SharedLane::slot`).
    locals: Vec<Option<Local>>,
    /// Builds iterations (its job buffers), and the buffers of serial builds.
    scratch: Box<Lane>,
    bufs: Bufs,
    /// `Pool::epoch` when idle lanes last looked for work.
    epoch: u64,
    /// `Pool::solved` when the lanes were last checked for solved cubes.
    solved: u64,
    /// No split attempt before round `split_wait` (after failures).
    split_wait: u32,
    split_backoff: u32,
    /// No donation attempt before round `donate_wait` (after failures).
    donate_wait: u32,
    donate_backoff: u32,
    /// Event counters (`COUNT_NAMES`), summed into the pool's at the end.
    counts: [u64; NCOUNT],
}

/// Build the next iteration of `ctx` (the first one if `first`) with
/// `scratch`'s extension, and publish it. If threads are waiting for work, a
/// long job list is extended and sorted by all of them instead
/// (`build_task`): returns true if that was started (the caller then helps).
fn build(
    scratch: &mut Lane,
    own_bufs: &mut Bufs,
    solver: &Solver,
    pool: &Pool,
    ctx: &Arc<CubeCtx>,
    first: bool,
) -> bool {
    let now = Instant::now();
    // SAFETY: no job of the cube is outstanding: nobody else accesses
    // `inner` until the iteration (or a build task) is published.
    let inner = unsafe { &mut *ctx.inner.get() };
    if pool.trace && !first {
        trace_iteration(pool, ctx, inner, now);
    }
    inner.build_started = now;
    inner.extended = now;
    if !first {
        inner.target += 1;
    }
    // The iterations up to the first one with jobs of `FIRST_PASS_JOB_LEN`
    // moves (from a target at most `first_pass_span` below) run as one pass
    // on its jobs, without barriers (`Inner::phases`): the ones below it
    // are cheap and nearly never the last (17 of 1000 random states end in
    // them), but their barriers, tails and builds each took ~0.03-0.12 ms of
    // -c 1 at 48 threads. As one pass: -7.4% -c 1, -7.1% -c 0, +2.0% bench
    // share at 48T (8 paired holds); 15/16/17-move scrambles -18/-43/-22%.
    // (A job prefix cannot solve the cube: the target is a lower bound above
    // the prefix length.)
    let pass_target = solver.base + 3 + FIRST_PASS_JOB_LEN;
    inner.phases = 1;
    if inner.target < pass_target
        && inner.target + pool.first_pass_span >= pass_target
        && usize::from(inner.target) > solver.job_len(pass_target)
    {
        inner.phases = u32::from(pass_target - inner.target) + 1;
        inner.target = pass_target;
    }
    inner.sort_by_bounds = first;
    let target = inner.target;
    // (As in `start_iteration`: the stack depth stays below the target.)
    assert!((target as usize) < MAXD);
    // Stream mode's prefixes: 4-move prefixes from iteration 15 on cost +8%
    // -c 1 latency at 48T (l6-share-2).
    let job_len = solver.job_len(target);
    // (With few cubes, threads are about to wait for this one.) Staging the
    // builds of few cubes before anyone waits: part of l6-share-2 (-5.6% -c 1
    // latency at 48T for the series; not measured alone).
    let parallel = pool.few() || pool.hungry.load(Relaxed) != 0;
    let len = inner.jobs[0].len as usize;
    // Sorted by the statistics as they are (not copied to the jobs first)?
    // (The serial copy took ~90 us per build; part of l6-share-a's -38.8% -c 1
    // latency at 48T, not measured alone.)
    let from_atomics =
        !first && len >= job_len && job_len > 0 && parallel && inner.jobs.len() >= SORT_MIN;
    plan_leads(
        inner,
        first,
        job_len == MAX_JOB_LEN && parallel,
        from_atomics,
    );
    if !first && !from_atomics {
        copy_stats(inner);
    }
    if len < job_len {
        // (`scratch` gets the cube's buffer and ours, and returns them.)
        scratch.root = ctx.root;
        scratch.root_inverse = ctx.root_inverse;
        scratch.run_state.sym_filter = ctx.sym_filter;
        std::mem::swap(&mut scratch.jobs, &mut inner.jobs);
        std::mem::swap(&mut scratch.jobs_tmp, &mut own_bufs.out);
        // Moves left to add, in parallel. (Also for a first pass, and with a
        // symmetric-start filter that only `keep`s: the tasks then count the
        // kept children. On x-h48's 7 test positions with only a symmetry:
        // -9.9% -c 1 at 48T against extending them serially, median of 4
        // paired rounds of 70 cubes. Not with a filter that dedups, whose
        // classes span the tasks; such cubes, like the superflip, keep few
        // jobs.)
        let parallel_extension = parallel && !ctx.sym_filter.dedups();
        let moves = loop {
            let (n, moves) = (scratch.jobs.len(), job_len - scratch.jobs[0].len as usize);
            if moves == 0
                || parallel_extension && (moves == 1 && n >= EXT_MIN || moves == 2 && n >= EXT_MIN2)
            {
                break moves;
            }
            scratch.extend_jobs(solver);
        };
        std::mem::swap(&mut scratch.jobs, &mut inner.jobs);
        std::mem::swap(&mut scratch.jobs_tmp, &mut own_bufs.out);
        if moves > 0 {
            inner.b = pool.take_bufs();
            // Parallel extension: task c (a chunk of the parents) writes
            // their children (`moves` moves longer) to
            // `out[child_offsets[c]..child_offsets[c + 1]]`, in the order of
            // serial `extend_jobs`.
            let bufs = &mut inner.b;
            bufs.extension_moves = moves;
            bufs.child_offsets.clear();
            let mut offset = 0u32;
            let chunk = if moves == 1 { EXT_CHUNK } else { 1 };
            for parents in inner.jobs.chunks(chunk) {
                bufs.child_offsets.push(offset);
                for parent in parents {
                    offset += kept_children(parent, moves, &ctx.sym_filter);
                }
            }
            bufs.child_offsets.push(offset);
            bufs.out.clear();
            assert!(offset as usize <= bufs.out.capacity());
            bufs.out_ptr = TaskPtr(bufs.out.as_mut_ptr());
            open_stage(pool, ctx, ST_EXT, bufs.child_offsets.len() - 1);
            return true;
        }
    }
    inner.extended = Instant::now();
    if inner.phases > 1 {
        first_pass_order(inner);
        finish_build(pool, ctx);
        return false;
    }
    if job_len > 0 && inner.jobs.len() >= SORT_MIN && parallel {
        if inner.sort_by_bounds {
            stats_from_bounds(&mut inner.jobs);
        }
        inner.b = pool.take_bufs();
        inner.b.stats_from_atomics = from_atomics;
        start_sort(pool, inner, ctx);
        return true;
    }
    if inner.sort_by_bounds {
        stats_from_bounds(&mut inner.jobs);
    }
    // (Serial builds use this thread's buffers: those of the pool were last
    // written by other threads.)
    inner.feasible = if job_len > 0 {
        sort_serial(&mut inner.jobs, own_bufs, target)
    } else {
        inner.jobs.len()
    };
    finish_build(pool, ctx);
    false
}

/// (trace) The end of `ctx`'s current iteration (`now`): when its threads
/// finished, its run (ms from the cube's start), when its claims ran out
/// (and its nodes, jobs, donations and splits).
#[cold]
#[expect(clippy::cast_precision_loss, reason = "trace output")]
fn trace_iteration(pool: &Pool, ctx: &CubeCtx, inner: &Inner, now: Instant) {
    let rarely_written = &ctx.rarely_written;
    let ran_out_ms = rarely_written.claims_ran_out_ns.swap(0, Relaxed) as f64 * 1e-6;
    let nodes = rarely_written.iteration_nodes.swap(0, Relaxed);
    let seconds = (now - inner.published).as_secs_f64();
    // Threads' finish times in this iteration (ms), sorted.
    let published_ns = (inner.published - ctx.started).as_nanos();
    let mut finish: Vec<f64> = ctx
        .thread_done_ns
        .iter()
        .map(|t| t.swap(0, Relaxed))
        .filter(|&t| u128::from(t) > published_ns)
        .map(|t| t as f64 * 1e-6)
        .collect();
    finish.sort_by(f64::total_cmp);
    let quantile = |percent: usize| {
        finish
            .get(finish.len().saturating_sub(1) * percent / 100)
            .copied()
            .unwrap_or(0.0)
    };
    tlog(
        pool,
        format_args!(
            "trace cube {} T {} finish threads {} q10 {:.3} q50 {:.3} q90 {:.3} max {:.3}",
            ctx.index,
            inner.target,
            finish.len(),
            quantile(10),
            quantile(50),
            quantile(90),
            quantile(100)
        ),
    );
    let counters = if pool.trace_bits & TRACE_COUNTERS == 0 {
        String::new()
    } else {
        format!(
            ", nodes {nodes} jobs {} donated {} splits {} ({:.2} Gnodes/s)",
            rarely_written.iteration_jobs.swap(0, Relaxed),
            rarely_written.iteration_donations.swap(0, Relaxed),
            rarely_written.iteration_splits.swap(0, Relaxed),
            nodes as f64 / seconds * 1e-9
        )
    };
    tlog(
        pool,
        format_args!(
            "trace cube {} T {} ran {:.3} .. {:.3} ms, claims out at {ran_out_ms:.3}{counters}",
            ctx.index,
            inner.target,
            (inner.published - ctx.started).as_secs_f64() * 1e3,
            (now - ctx.started).as_secs_f64() * 1e3,
        ),
    );
}

/// Fewer cubes in progress than threads / `FEW` build in stages, and their
/// threads stay with them.
const FEW: usize = 8;
/// Most cubes a thread stays with (`Worker::keep`).
const KEEP: usize = 2;

/// The statistics of the last iteration to its jobs.
fn copy_stats(inner: &mut Inner) {
    let n = inner.jobs.len();
    for i in 0..n {
        // (Written by other threads: fetched well ahead. Part of
        // l6-share-a.)
        if i % 16 == 0 {
            prefetch(&inner.stats[(i + 512).min(n - 1)], false);
        }
        inner.jobs[i].stat = f32::from_bits(inner.stats[i].load(Relaxed));
    }
}

/// Job length of a cube's first pass (`build`), and the most iterations
/// below its target it stands for: from the targets 12 and 13 of random
/// states (h11), the pass is for the iterations up to 16.
const FIRST_PASS_JOB_LEN: u8 = 3;
const FIRST_PASS_SPAN: u8 = 4;
/// Most phases of a first pass (`first_pass_span` + 1 at most).
const MAX_PHASES: usize = 8;
const _: () = assert!((FIRST_PASS_SPAN as usize) < MAX_PHASES);

/// A first pass claims its jobs as the extension left them, all of them:
/// the ones infeasible in a phase are skipped when taken (`Worker::take`).
/// Sorting them by their bounds was a serial pass over 3240 jobs that other
/// threads had just written, on every cube's first build. Without it:
/// -1.9% -c 1 at 48T (7 paired holds), 15/16-move scrambles -16/-11%,
/// depth-15/16 states -20/-28%; -c 0 and bench share neutral.
fn first_pass_order(inner: &mut Inner) {
    inner.feasible = inner.jobs.len();
}

/// Split iterations: the fraction of jobs whose lead group (`lead_word`)
/// runs first, all those first-part runs by decreasing lead code across
/// jobs, before the rest of each job in the order of the job statistics.
/// (Replay of 273 random states ending at T = 18 through share's claims,
/// c11: the solution 0.70 ms into T = 18 on average instead of 0.80, median
/// 0.28 instead of 0.50; 20/30/50/70% of the jobs: 0.72-0.75 ms; c7's full
/// order, 0.69 ms with 1.6 runs per job.)
const LEAD_FRACTION: f64 = 0.1;
/// A job's lead group: its root children (normal side) whose statistic is
/// at least this share of the best one's (one octave, 4 codes as c7's
/// `MERGE_CODES`), with the X or X' sibling of each (`lead_of`). (Replay:
/// 0, 2, 4 or 8 codes within 1%.)
const LEAD_SHARE: f32 = 0.5;
/// First-part runs are counting-sorted by their code in buckets of
/// `1 << LEAD_SHIFT` codes (replay: as good as single codes).
const LEAD_SHIFT: u32 = 2;
const LEAD_BUCKETS: usize = 256 >> LEAD_SHIFT;

/// A job's lead: a tag of the cube and iteration (6 bits, `lead_tag`), the
/// best code of its root children (8 bits, `lead_of`) and its lead group
/// (18 bits).
fn lead_word(tag: u32, code: u8, group: u32) -> u32 {
    tag << 26 | u32::from(code) << 18 | group
}

/// The tag of the leads that iteration `target` of a cube (`tag_base`)
/// writes: leads in `Inner::leads` from other cubes or iterations are
/// ignored (but for 1 in 64).
fn lead_tag(tag_base: u32, target: u8) -> u32 {
    (tag_base + u32::from(target)) % 64
}

/// The code and the group of a lead of the tag `tag` (None if it is
/// another cube's or iteration's, or none).
#[expect(clippy::cast_possible_truncation, reason = "the code's 8 bits")]
fn lead_fields(lead: u32, tag: u32) -> Option<(u8, u32)> {
    (lead >> 26 == tag && lead != 0).then_some(((lead >> 18) as u8, lead & ALL_MOVES))
}

/// The lead of a job whose run over its root children `children` (normal
/// side) gave `child_stats`: the code (`quantize`) of its best child's
/// statistic, weighted one code step up on faces 1, 3 and 5 (`ODD_FACE`, as
/// `quantize18`), and its children whose weighted statistic is at least
/// `LEAD_SHARE` of the best one's, with the X or X' sibling of each
/// (`with_twins`). 0 (none) if no child has a statistic. On the statistics,
/// not their codes: a few µops per job end.
#[cfg(avx512)]
#[expect(clippy::cast_possible_truncation, reason = "the low 16 moves")]
fn lead_of(child_stats: &[f32; 18], children: u32, tag: u32) -> u32 {
    use std::arch::x86_64::{
        _mm512_loadu_ps, _mm512_mask_cmp_ps_mask, _mm512_maskz_mul_ps, _mm512_reduce_max_ps,
        _mm512_set1_ps, _mm512_setr_ps, _CMP_GE_OQ,
    };
    const ODD: f32 = ODD_FACE;
    let low = children as u16;
    // SAFETY: AVX-512 is a target feature (`cfg(avx512)`); the load reads
    // the first 16 statistics.
    let (weighted, low_best) = unsafe {
        let weights = _mm512_setr_ps(
            1.0, 1.0, 1.0, ODD, ODD, ODD, 1.0, 1.0, 1.0, ODD, ODD, ODD, 1.0, 1.0, 1.0, ODD,
        );
        let weighted = _mm512_maskz_mul_ps(low, _mm512_loadu_ps(child_stats.as_ptr()), weights);
        (weighted, _mm512_reduce_max_ps(weighted))
    };
    // (Moves 16 and 17 are on face 5.)
    let high = |mv: usize| {
        if children >> mv & 1 != 0 {
            child_stats[mv] * ODD
        } else {
            0.0
        }
    };
    let (stat16, stat17) = (high(16), high(17));
    let best = low_best.max(stat16).max(stat17);
    let code = super::quantize(best);
    if code == 0 {
        return 0;
    }
    let floor = best * LEAD_SHARE;
    // SAFETY: as above.
    let low_group =
        unsafe { _mm512_mask_cmp_ps_mask::<_CMP_GE_OQ>(low, weighted, _mm512_set1_ps(floor)) };
    let group =
        u32::from(low_group) | u32::from(stat16 >= floor) << 16 | u32::from(stat17 >= floor) << 17;
    lead_word(tag, code, with_twins(group) & children)
}

/// `lead_of` without AVX-512.
#[cfg(not(avx512))]
fn lead_of(child_stats: &[f32; 18], children: u32, tag: u32) -> u32 {
    let weighted: [f32; 18] = std::array::from_fn(|mv| {
        let weight = if mv / 3 % 2 == 1 { ODD_FACE } else { 1.0 };
        if children >> mv & 1 != 0 {
            child_stats[mv] * weight
        } else {
            0.0
        }
    });
    let best = weighted.iter().copied().fold(0.0, f32::max);
    let code = super::quantize(best);
    if code == 0 {
        return 0;
    }
    let floor = best * LEAD_SHARE;
    let group = (0..18)
        .filter(|&mv| children >> mv & 1 != 0 && weighted[mv] >= floor)
        .fold(0, |group, mv| group | 1 << mv);
    lead_word(tag, code, with_twins(group) & children)
}

/// The weight of a child's statistic on faces 1, 3 and 5 in `lead_of`:
/// 2^(1/4), one step of `quantize`.
const ODD_FACE: f32 = 1.189_207_1;

/// `group` with the X and X' children of each face where it has one of
/// them (an X' child runs without its X sibling only with X's other-side
/// value, which a separate run lacks).
fn with_twins(group: u32) -> u32 {
    let x_moves = 0b001_001_001_001_001_001;
    group | (group & x_moves) << 2 | group >> 2 & x_moves
}

/// Statistics of new jobs (no iteration ran them yet) for the sort of a
/// first build that is not a first pass, from their bounds: the largest
/// bound lowest first (the largest trees), then the smallest.
fn stats_from_bounds(jobs: &mut [Job]) {
    for job in jobs {
        // (Bounds are below 16.)
        let (largest, smallest) = (
            job.normal_bound.max(job.inverse_bound).min(15),
            job.normal_bound.min(job.inverse_bound),
        );
        job.stat = f32::from(256 - (u16::from(largest) << 4 | u16::from(smallest)));
    }
}

/// The number of jobs `moves` (at most 2) moves longer than `job`
/// (`extend_jobs`).
fn nchildren(job: &Job, moves: usize) -> u32 {
    /// By the face of the last move (6: none), and moves added.
    static COUNTS: [[u32; 3]; 7] = {
        let mut counts = [[1u32; 3]; 7];
        let mut face = 0;
        while face < 7 {
            let allowed = if face == 6 {
                ALL_MOVES
            } else {
                ALLOWED_AFTER[face]
            };
            counts[face][1] = allowed.count_ones();
            counts[face][2] = 0;
            let mut m = 0;
            while m < 18 {
                if allowed & (1 << m) != 0 {
                    counts[face][2] += ALLOWED_AFTER[m / 3].count_ones();
                }
                m += 1;
            }
            face += 1;
        }
        counts
    };
    let face = if job.len == 0 {
        6
    } else {
        (job.moves[job.len as usize - 1] / 3) as usize
    };
    COUNTS[face][moves]
}

/// The number of jobs `moves` (at most 2) moves longer than `job` that
/// `filter` keeps (`extend_jobs`), for a filter that does not dedup.
fn kept_children(job: &Job, moves: usize, filter: &SymFilter) -> u32 {
    if filter.is_active() {
        let mut prefix = job.moves;
        count_kept(&mut prefix, usize::from(job.len), moves, filter)
    } else {
        nchildren(job, moves)
    }
}

/// The number of extensions of `prefix[..len]` by `moves` moves that
/// `filter` keeps at every level (cold: cubes with a symmetry).
#[cold]
#[expect(clippy::cast_possible_truncation, reason = "moves are below 18")]
fn count_kept(prefix: &mut [u8; 4], len: usize, moves: usize, filter: &SymFilter) -> u32 {
    if moves == 0 {
        return 1;
    }
    let mut todo = if len == 0 {
        ALL_MOVES
    } else {
        ALLOWED_AFTER[usize::from(prefix[len - 1] / 3)]
    };
    let mut count = 0;
    while todo != 0 {
        prefix[len] = todo.trailing_zeros() as u8;
        todo ^= todo.isolate_lowest_one();
        if filter.keep(&prefix[..=len]) {
            count += count_kept(prefix, len + 1, moves - 1, filter);
        }
    }
    count
}

/// Sort `jobs` for iteration `target` with `bufs` (an LSD radix sort
/// of the keys of `sort_key`, then a gather); returns the number of
/// feasible jobs.
#[expect(clippy::cast_possible_truncation, reason = "11-bit digits")]
fn sort_serial(jobs: &mut Vec<Job>, bufs: &mut Bufs, target: u8) -> usize {
    let n = jobs.len();
    let (keys, keys_tmp) = (&mut bufs.keys, &mut bufs.keys_tmp);
    keys.clear();
    keys.extend((0..n).map(|i| sort_key(jobs, &[], false, target, i)));
    keys_tmp.clear();
    keys_tmp.resize(n, 0);
    // (The indices, in the low bits, are in order.)
    for (shift, bits) in [
        (KEY_SHIFT, 11u32),
        (KEY_SHIFT + 11, 11),
        (KEY_SHIFT + 22, 11),
    ] {
        let mask = (1u64 << bits) - 1;
        let mut counts = [0u32; 2048];
        for &key in keys.iter() {
            counts[((key >> shift) & mask) as usize] += 1;
        }
        if counts.iter().any(|&count| count as usize == n) {
            continue; // all keys share this digit
        }
        let mut sum = 0;
        for count in &mut counts {
            (*count, sum) = (sum, sum + *count);
        }
        for &key in keys.iter() {
            let digit = ((key >> shift) & mask) as usize;
            keys_tmp[counts[digit] as usize] = key;
            counts[digit] += 1;
        }
        std::mem::swap(keys, keys_tmp);
    }
    bufs.out.clear();
    bufs.out
        .extend(keys.iter().map(|&key| jobs[key_index(key)]));
    std::mem::swap(jobs, &mut bufs.out);
    keys.partition_point(|&key| key >> 63 == 0)
}

/// Build stages done in parallel (`build_task`).
const ST_EXT: usize = 1;
const ST_KEYS: usize = 2;
const ST_SCATTER: usize = 3;
const ST_BUCKETS: usize = 4;
/// Parallel sort: jobs per chunk, buckets (between splitters taken from a
/// sample of `SAMPLE` keys per bucket).
const SORT_CHUNK: usize = 2048;
const NBUCKETS: usize = 64;
const SAMPLE: usize = 16;

/// Open build stage `stage` with `tasks` tasks (and list the cube: it may have
/// been dropped from the open list while the last stage's tasks ran).
// Listing the cube at every stage lets all threads run its stages: part of
// l6-share-a (-38.8% -c 1 latency at 48T).
#[expect(clippy::cast_possible_truncation, reason = "fewer than 2^24 tasks")]
fn open_stage(pool: &Pool, ctx: &Arc<CubeCtx>, stage: usize, tasks: usize) {
    ctx.task_left.store(tasks as u32, Relaxed);
    ctx.task
        .store((stage as u64) << 56 | (tasks as u64) << 32, Release);
    pool.publish(ctx);
}

/// Set up the parallel sort of `inner.jobs`, the same order as
/// `sort_jobs`: by `!job_key << 32 | index`. Keys are counted per chunk by
/// their top bits (`ST_KEYS`), scattered to their buckets (`ST_SCATTER`),
/// and runs of buckets are sorted and gathered (`ST_BUCKETS`).
fn start_sort(pool: &Pool, inner: &mut Inner, ctx: &Arc<CubeCtx>) {
    let chunks = sort_setup(inner);
    open_stage(pool, ctx, ST_KEYS, chunks);
}

/// Sort key of job `i` in iteration `target` (with the statistic from `stats` if
/// `from_atomics`), in increasing order: the jobs whose bounds exceed the
/// iteration (they would be skipped) last, then by decreasing `job_key`
/// (as `sort_jobs`), then by index: `infeasible << 63 | !job_key << 31 |
/// i`. The feasible jobs are in the order of `sort_jobs`, and only they
/// are claimed.
#[inline(always)]
fn sort_key(jobs: &[Job], stats: &[AtomicU32], from_atomics: bool, target: u8, i: usize) -> u64 {
    let job = &jobs[i];
    let stat = if from_atomics {
        f32::from_bits(stats[i].load(Relaxed))
    } else {
        job.stat
    };
    let infeasible =
        u32::from(job.normal_bound.max(job.inverse_bound)) + u32::from(job.len) > u32::from(target);
    u64::from(infeasible) << 63
        | u64::from(!(stat * job_weight(job)).to_bits()) << KEY_SHIFT
        | i as u64
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
    let chunks = n.div_ceil(SORT_CHUNK);
    let sampled = |x: usize| x * n / (NBUCKETS * SAMPLE);
    let from_atomics = inner.b.stats_from_atomics;
    for x in 0..NBUCKETS * SAMPLE {
        // (The jobs, or statistics, were just written by other threads.)
        prefetch(&inner.jobs[sampled(x)], false);
        if from_atomics {
            prefetch(&inner.stats[sampled(x)], false);
        }
    }
    if inner.split {
        for x in 0..NBUCKETS * SAMPLE {
            prefetch(&inner.leads[sampled(x)], false);
        }
        inner.lead_threshold = lead_threshold(inner, n);
    }
    let bufs = &mut inner.b;
    let mut sample = std::mem::take(&mut bufs.splitters);
    sample.clear();
    sample.extend((0..NBUCKETS * SAMPLE).map(|x| {
        sort_key(
            &inner.jobs,
            &inner.stats,
            from_atomics,
            inner.target,
            sampled(x),
        )
    }));
    sample.sort_unstable();
    sample.dedup();
    let len = sample.len();
    for i in 1..NBUCKETS {
        sample[i - 1] = sample[i * len / NBUCKETS];
    }
    sample.truncate(NBUCKETS - 1);
    bufs.splitters = sample;
    bufs.bucket_counts.clear();
    bufs.bucket_counts.resize(chunks * NBUCKETS, 0);
    for keys in [&mut bufs.keys, &mut bufs.keys_tmp] {
        keys.clear();
        assert!(n <= keys.capacity());
    }
    bufs.key_buckets.clear();
    bufs.out.clear();
    assert!(n <= bufs.key_buckets.capacity() && n <= bufs.out.capacity());
    bufs.keys_ptr = TaskPtr(bufs.keys.as_mut_ptr());
    bufs.keys_tmp_ptr = TaskPtr(bufs.keys_tmp.as_mut_ptr());
    bufs.key_buckets_ptr = TaskPtr(bufs.key_buckets.as_mut_ptr());
    bufs.bucket_counts_ptr = TaskPtr(bufs.bucket_counts.as_mut_ptr());
    bufs.out_ptr = TaskPtr(bufs.out.as_mut_ptr());
    if inner.split {
        bufs.lead_counts.clear();
        bufs.lead_counts.resize(chunks * LEAD_BUCKETS, 0);
        bufs.rest_children.clear();
        inner.runs.clear();
        assert!(n <= bufs.rest_children.capacity() && 2 * n <= inner.runs.capacity());
        bufs.rest_children_ptr = TaskPtr(bufs.rest_children.as_mut_ptr());
        bufs.lead_counts_ptr = TaskPtr(bufs.lead_counts.as_mut_ptr());
        bufs.runs_ptr = TaskPtr(inner.runs.as_mut_ptr());
    }
    chunks
}

/// The lead code above which a split iteration's `n` jobs have a first-part
/// run: about `LEAD_FRACTION` of those with a lead, from the sample of
/// `sort_setup`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a fraction of a count"
)]
fn lead_threshold(inner: &Inner, n: usize) -> u8 {
    let tag = inner.read_tag;
    let mut histogram = [0u32; 256];
    let mut total = 0u32;
    for x in 0..NBUCKETS * SAMPLE {
        let i = x * n / (NBUCKETS * SAMPLE);
        if let Some((code, _)) = lead_fields(inner.leads[i].load(Relaxed), tag) {
            histogram[usize::from(code)] += 1;
            total += 1;
        }
    }
    let limit = (f64::from(total) * LEAD_FRACTION) as u32;
    let mut count = 0;
    for code in (1..=u8::MAX).rev() {
        count += histogram[usize::from(code)];
        if count > limit {
            return code;
        }
    }
    u8::MAX
}

/// Set up lane `sl`'s run (claim entry `entry`) in an iteration that is
/// split or records leads: its root children `children` (all of them
/// except in a split job), and whether it records its job's lead (whole
/// runs and first-part runs do).
#[inline(never)]
fn lead_run(sl: &mut SharedLane, inner: &Inner, entry: u32, children: u32) {
    if children != ALL_MOVES {
        // (The children are normal-side moves: a lead is recorded on the
        // normal side, and the next iteration has the root there too, its
        // bounds being below the remaining depth. A job's runs cover all
        // its children in any case.)
        sl.lane.stack[0].todo &= children;
        // (A job's runs may end at once: its statistic is added.)
        sl.split = true;
    }
    sl.lead_children = if children == ALL_MOVES || entry < inner.first_runs {
        children
    } else {
        0
    };
}

/// The job (an index of `Inner::jobs`) of a split iteration's run.
#[inline(always)]
#[expect(clippy::cast_possible_truncation, reason = "the low half")]
fn run_job(run: u64) -> u32 {
    run as u32
}

/// The root children of a split iteration's run.
#[inline(always)]
fn run_children(run: u64) -> u32 {
    (run >> 32) as u32
}

/// Whether the new iteration of `inner` (the cube's first if `first`)
/// records its jobs' leads (`record`: its jobs have their full length, and
/// it is built in stages), and whether it is split (`sorted_in_stages`: its
/// jobs are those of the last iteration, which recorded them). With h11: T = 17 records, T = 18 (the
/// last iteration for 95% of the random states that reach it) is split and
/// records, T = 19 is split, and so on.
// Split iterations (c11): -2.9% -c 1 at 48T (7 paired holds, nodes -6.8%;
// -c 0, bench share and the short sets neutral; 1T neutral).
fn plan_leads(inner: &mut Inner, first: bool, record: bool, sorted_in_stages: bool) {
    if first {
        inner.leads_target = 0;
    }
    let has_leads = !inner.leads.is_empty();
    inner.split = sorted_in_stages && has_leads && inner.leads_target + 1 == inner.target;
    inner.read_tag = lead_tag(inner.tag_base, inner.leads_target);
    inner.record_leads = !first && record && has_leads && inner.phases == 1;
    if inner.record_leads {
        inner.leads_target = inner.target;
        inner.write_tag = lead_tag(inner.tag_base, inner.target);
    }
}

/// The first-part bucket of a lead code (decreasing codes first).
#[inline(always)]
fn lead_bucket(code: u8) -> usize {
    usize::from(u8::MAX - code) >> LEAD_SHIFT
}

/// Task `task` of sort stage `stage`.
///
/// # Safety
/// Tasks of a stage run concurrently only with each other, after
/// `sort_setup` and the previous stages.
#[expect(clippy::cast_possible_truncation, reason = "bucket indices < NBUCKETS")]
unsafe fn sort_task(inner: &Inner, stage: usize, task: usize) {
    let jobs = &inner.jobs;
    let bufs = &inner.b;
    match stage {
        ST_KEYS => {
            let lo = task * SORT_CHUNK;
            let hi = (lo + SORT_CHUNK).min(jobs.len());
            let (keys, key_buckets) = (bufs.keys_ptr.0, bufs.key_buckets_ptr.0);
            let mut counts = [0u32; NBUCKETS];
            let mut lead_counts = [0u32; LEAD_BUCKETS];
            for i in lo..hi {
                let mut key =
                    sort_key(jobs, &inner.stats, bufs.stats_from_atomics, inner.target, i);
                if inner.split {
                    // A first-part run for a lead above the threshold; the
                    // job's rest run (none if no child is left).
                    let mut rest = ALL_MOVES;
                    if let Some((code, group)) =
                        lead_fields(inner.leads[i].load(Relaxed), inner.read_tag)
                    {
                        if code > inner.lead_threshold && key >> 63 == 0 {
                            let bucket = lead_bucket(code);
                            lead_counts[bucket] += 1;
                            if moves_after(&jobs[i]) & !group == 0 {
                                key |= 1 << 63;
                            }
                            // (With the bucket, for `ST_SCATTER`.)
                            rest = ALL_MOVES & !group | (bucket as u32) << 24;
                        }
                    }
                    *bufs.rest_children_ptr.0.add(i) = rest;
                }
                let bucket = bufs.splitters.partition_point(|&s| s <= key);
                *keys.add(i) = key;
                *key_buckets.add(i) = bucket as u8;
                counts[bucket] += 1;
            }
            let bucket_counts = bufs.bucket_counts_ptr.0.add(task * NBUCKETS);
            std::ptr::copy_nonoverlapping(counts.as_ptr(), bucket_counts, NBUCKETS);
            if inner.split {
                let lead_out = bufs.lead_counts_ptr.0.add(task * LEAD_BUCKETS);
                std::ptr::copy_nonoverlapping(lead_counts.as_ptr(), lead_out, LEAD_BUCKETS);
            }
        }
        ST_SCATTER => {
            let lo = task * SORT_CHUNK;
            let hi = (lo + SORT_CHUNK).min(jobs.len());
            let (keys, keys_tmp) = (bufs.keys_ptr.0.cast_const(), bufs.keys_tmp_ptr.0);
            let key_buckets = bufs.key_buckets_ptr.0.cast_const();
            // (`ST_KEYS`'s counts, turned into positions by `sort_stage_done`.)
            let positions = std::slice::from_raw_parts_mut(
                bufs.bucket_counts_ptr.0.add(task * NBUCKETS),
                NBUCKETS,
            );
            for i in lo..hi {
                let pos = &mut positions[*key_buckets.add(i) as usize];
                *keys_tmp.add(*pos as usize) = *keys.add(i);
                *pos += 1;
            }
            if inner.split {
                // The first-part runs, at their positions (`sort_stage_done`).
                let lead_positions = std::slice::from_raw_parts_mut(
                    bufs.lead_counts_ptr.0.add(task * LEAD_BUCKETS),
                    LEAD_BUCKETS,
                );
                for i in lo..hi {
                    let rest = *bufs.rest_children_ptr.0.add(i);
                    if rest == ALL_MOVES {
                        continue;
                    }
                    let group = ALL_MOVES & !rest;
                    let pos = &mut lead_positions[(rest >> 24) as usize];
                    *bufs.runs_ptr.0.add(*pos as usize) = i as u64 | u64::from(group) << 32;
                    *pos += 1;
                }
            }
        }
        _ => {
            let lo = bufs.bucket_starts[task] as usize;
            let hi = bufs.bucket_starts[task + 1] as usize;
            let run = std::slice::from_raw_parts_mut(bufs.keys_tmp_ptr.0.add(lo), hi - lo);
            // (Keys are distinct: the index breaks ties, as in a stable
            // sort.)
            run.sort_unstable();
            if inner.split {
                // The rest runs: job indices (the jobs stay in place).
                let runs = bufs.runs_ptr.0.add(bufs.first_runs + lo);
                for (p, &key) in run.iter().enumerate() {
                    let i = key_index(key);
                    let children = *bufs.rest_children_ptr.0.add(i) & ALL_MOVES;
                    *runs.add(p) = i as u64 | u64::from(children) << 32;
                }
                return;
            }
            let out = bufs.out_ptr.0.add(lo);
            for (p, &key) in run.iter().enumerate() {
                if let Some(&ahead) = run.get(p + 16) {
                    prefetch(&jobs[key_index(ahead)], false);
                }
                *out.add(p) = jobs[key_index(key)];
            }
        }
    }
}

/// All tasks of sort stage `stage` are done: the next stage and its number
/// of tasks, or None when the jobs are sorted.
fn sort_stage_done(inner: &mut Inner, stage: usize) -> Option<(usize, usize)> {
    inner.sort_stages_ended[stage - ST_KEYS] = Instant::now();
    let bufs = &mut inner.b;
    match stage {
        ST_KEYS => {
            // Counts to the first position of each (chunk, bucket); a
            // bucket is a task of `ST_BUCKETS`.
            let chunks = bufs.bucket_counts.len() / NBUCKETS;
            bufs.bucket_starts.clear();
            let mut pos = 0u32;
            for bucket in 0..NBUCKETS {
                bufs.bucket_starts.push(pos);
                for chunk in 0..chunks {
                    let count = &mut bufs.bucket_counts[chunk * NBUCKETS + bucket];
                    (*count, pos) = (pos, pos + *count);
                }
            }
            bufs.bucket_starts.push(pos);
            if inner.split {
                // The first-part runs by bucket, then chunk (job order).
                let mut pos = 0u32;
                for bucket in 0..LEAD_BUCKETS {
                    for chunk in 0..chunks {
                        let count = &mut bufs.lead_counts[chunk * LEAD_BUCKETS + bucket];
                        (*count, pos) = (pos, pos + *count);
                    }
                }
                bufs.first_runs = pos as usize;
            }
            Some((ST_SCATTER, chunks))
        }
        ST_SCATTER => Some((ST_BUCKETS, NBUCKETS)),
        _ if inner.split => {
            let n = inner.jobs.len();
            inner.first_runs = u32::try_from(bufs.first_runs).unwrap_or(u32::MAX);
            // SAFETY: the tasks wrote the first-part runs and a rest run per
            // job (and sorted `keys_tmp`).
            unsafe {
                inner.runs.set_len(bufs.first_runs + n);
                inner.feasible = bufs.first_runs
                    + std::slice::from_raw_parts(bufs.keys_tmp_ptr.0.cast_const(), n)
                        .partition_point(|&k| k >> 63 == 0);
            }
            None
        }
        _ => {
            let n = inner.jobs.len();
            // SAFETY: the tasks wrote all of `out` (and sorted `keys_tmp`).
            unsafe {
                bufs.out.set_len(n);
                inner.feasible = std::slice::from_raw_parts(bufs.keys_tmp_ptr.0.cast_const(), n)
                    .partition_point(|&k| k >> 63 == 0);
            }
            std::mem::swap(&mut inner.jobs, &mut bufs.out);
            None
        }
    }
}

/// Do a task of `ctx`'s parallel build, if one is left (false otherwise);
/// the last task of a stage opens the next one or publishes the iteration.
// Builds in stages by all waiting threads: l5 (not measured alone); on all
// threads and without page faults since l6-share-a (-38.8% -c 1 at 48T).
fn build_task(
    scratch: &mut Lane,
    solver: &Solver,
    pool: &Pool,
    ctx: &Arc<CubeCtx>,
    thread_id: usize,
) -> bool {
    let (_, count, next) = task_fields(ctx.task.load(Acquire));
    if next >= count {
        return false;
    }
    let (stage, count, task) = task_fields(ctx.task.fetch_add(1, AcqRel));
    if task >= count {
        return false;
    }
    // The stage cannot end while this task is not done; tasks write
    // disjoint parts of the buffers (through the `*_ptr` copies), which are
    // read only in later stages.
    let inner = ctx.inner();
    let start_ms = if pool.trace_bits & TRACE_TASKS != 0 {
        ctx.started.elapsed().as_secs_f32() * 1e3
    } else {
        0.0
    };
    // SAFETY: see above.
    unsafe {
        match stage {
            ST_EXT => {
                let chunk = if inner.b.extension_moves == 1 {
                    EXT_CHUNK
                } else {
                    1
                };
                let lo = task * chunk;
                let hi = (lo + chunk).min(inner.jobs.len());
                scratch.root = ctx.root;
                scratch.root_inverse = ctx.root_inverse;
                // (Parallel extensions run only with a filter that does not
                // dedup: the children that `kept_children` counted.)
                scratch.run_state.sym_filter = ctx.sym_filter;
                scratch.jobs.clear();
                scratch.jobs.extend_from_slice(&inner.jobs[lo..hi]);
                for _ in 0..inner.b.extension_moves {
                    scratch.extend_jobs(solver);
                }
                let bufs = &inner.b;
                let first = bufs.child_offsets[task] as usize;
                let children = bufs.child_offsets[task + 1] as usize - first;
                assert_eq!(scratch.jobs.len(), children);
                let out = bufs.out_ptr.0.add(first);
                std::ptr::copy_nonoverlapping(scratch.jobs.as_ptr(), out, children);
            }
            _ => sort_task(inner, stage, task),
        }
    }
    if pool.trace_bits & TRACE_TASKS != 0 && task < 256 {
        let end_ms = ctx.started.elapsed().as_secs_f32() * 1e3;
        if let Some(entry) = ctx.task_log.get((stage - 1) * 256 + task) {
            entry[0].store(u32::try_from(thread_id + 1).unwrap_or(u32::MAX), Relaxed);
            entry[1].store(start_ms.to_bits(), Relaxed);
            entry[2].store(end_ms.to_bits(), Relaxed);
        }
    }
    if ctx.task_left.fetch_sub(1, AcqRel) == 1 {
        // SAFETY: all tasks of the stage are done, the next one is not open.
        let inner = unsafe { &mut *ctx.inner.get() };
        match stage {
            ST_EXT => {
                let bufs = &mut inner.b;
                let children = *bufs.child_offsets.last().unwrap() as usize;
                // SAFETY: the tasks wrote all of `out`.
                unsafe { bufs.out.set_len(children) };
                std::mem::swap(&mut inner.jobs, &mut bufs.out);
                inner.extended = Instant::now();
                if inner.phases > 1 {
                    first_pass_order(inner);
                    finish_build(pool, ctx);
                    return true;
                }
                if inner.sort_by_bounds {
                    stats_from_bounds(&mut inner.jobs);
                }
                if inner.jobs.len() >= SORT_MIN {
                    start_sort(pool, inner, ctx);
                } else {
                    inner.feasible = sort_serial(&mut inner.jobs, &mut inner.b, inner.target);
                    finish_build(pool, ctx);
                }
            }
            _ => match sort_stage_done(inner, stage) {
                Some((next_stage, tasks)) => open_stage(pool, ctx, next_stage, tasks),
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
    let sorted = Instant::now();
    let n = inner.jobs.len();
    inner.stats.clear();
    inner.stats.resize_with(n, || AtomicU32::new(0));
    // (The infeasible jobs, at the end, keep the statistic 0.)
    // At least one job is claimed (an infeasible one is skipped): the
    // iteration ends when the claimed jobs end. (Only the feasible jobs are
    // claimed: part of l6-share-a, not measured alone.)
    let mut claimed = inner.feasible.max(1) as u64;
    if inner.phases > 1 {
        // (Each job once per phase.)
        inner.phase_entries = u32::try_from(claimed).unwrap_or(u32::MAX);
        for left in &ctx.phase_left[..inner.phases as usize] {
            left.store(claimed, Relaxed);
        }
        *ctx.pass.lock().unwrap() = PassState::default();
        ctx.rarely_written.pass_limit.store(u8::MAX, Relaxed);
        claimed *= u64::from(inner.phases);
    }
    ctx.outstanding.store(claimed, Relaxed);
    ctx.rarely_written.claims_ran_out_ns.store(0, Relaxed);
    inner.published = Instant::now();
    ctx.claim.store(claimed << 32, Release);
    pool.publish(ctx);
    if pool.trace {
        trace_build(pool, ctx, inner, sorted);
    }
}

/// (trace) The build of `ctx`'s current iteration: its times (ms from the
/// cube's start), and per stage its tasks, threads and times.
#[cold]
fn trace_build(pool: &Pool, ctx: &CubeCtx, inner: &Inner, sorted: Instant) {
    let ms = |t: Instant| (t - ctx.started).as_secs_f64() * 1e3;
    let stages = if inner.sort_stages_ended[2] > inner.extended {
        let [keys, scatter, buckets] = inner.sort_stages_ended.map(ms);
        format!(" (parallel: keys {keys:.3} scatter {scatter:.3} buckets {buckets:.3})")
    } else {
        String::new()
    };
    tlog(
        pool,
        format_args!(
            "trace cube {} T {} phases {} jobs {} feasible {} build {:.3} .. {:.3} ms \
         (extended {:.3} sorted {:.3}){stages}",
            ctx.index,
            inner.target,
            inner.phases,
            inner.jobs.len(),
            inner.feasible,
            ms(inner.build_started),
            ms(Instant::now()),
            ms(inner.extended),
            ms(sorted)
        ),
    );
    for stage in ST_EXT..=ST_BUCKETS {
        // (thread + 1, start, end)
        let tasks: Vec<(u32, f32, f32)> = ctx.task_log[(stage - 1) * 256..stage * 256]
            .iter()
            .filter(|entry| entry[0].load(Relaxed) != 0)
            .map(|entry| {
                (
                    entry[0].swap(0, Relaxed),
                    f32::from_bits(entry[1].load(Relaxed)),
                    f32::from_bits(entry[2].load(Relaxed)),
                )
            })
            .collect();
        if tasks.is_empty() {
            continue;
        }
        let mut threads: Vec<u32> = tasks.iter().map(|task| task.0).collect();
        threads.sort_unstable();
        threads.dedup();
        let first = tasks.iter().map(|task| task.1).fold(f32::MAX, f32::min);
        let last = tasks.iter().map(|task| task.2).fold(0f32, f32::max);
        let sum: f32 = tasks.iter().map(|task| task.2 - task.1).sum();
        let max = tasks
            .iter()
            .map(|task| task.2 - task.1)
            .fold(0f32, f32::max);
        tlog(
            pool,
            format_args!(
                "  stage {stage} tasks {} threads {} first {first:.3} last {last:.3} \
             sum {sum:.3} max {max:.3}",
                tasks.len(),
                threads.len()
            ),
        );
    }
}

/// (trace) Lane `sl` (of a thread with `active_lanes` lanes) found the
/// solution of `ctx`, `seconds` after its start: where in the iteration, the
/// job's rank and start, and its nodes before the solution.
#[cold]
fn trace_found(pool: &Pool, ctx: &CubeCtx, sl: &SharedLane, active_lanes: usize, seconds: f64) {
    // SAFETY: (trace) the iteration is still on.
    let inner = unsafe { &*ctx.inner.get() };
    let (_, next) = claim_fields(ctx.claim.load(Relaxed));
    tlog(
        pool,
        format_args!(
            "trace cube {} found {:.3} ms ({:.3} ms into T {}) job {} claimed {} feasible {} \
             job started {:.3} ms into T, job nodes {} split {} active lanes {active_lanes} depth {}",
            ctx.index,
            seconds * 1e3,
            inner.published.elapsed().as_secs_f64() * 1e3,
            sl.lane.target,
            sl.entry,
            next,
            inner.feasible,
            sl.started.saturating_duration_since(inner.published).as_secs_f64() * 1e3,
            sl.lane.stats.nodes - sl.nodes_at_start,
            sl.split,
            sl.lane.depth
        ),
    );
}

/// The job of claim `entry` of an iteration, its target and its phase (in a
/// first pass, `Inner::phases`; else 0).
#[inline(always)]
#[expect(clippy::cast_possible_truncation, reason = "phases <= MAX_PHASES")]
fn entry_job(inner: &Inner, entry: u32) -> (u32, u8, u8) {
    if inner.phases == 1 {
        return (entry, inner.target, 0);
    }
    let phase = entry / inner.phase_entries;
    let job = entry - phase * inner.phase_entries;
    (job, phase_target(inner, phase as u8), phase as u8)
}

/// The target of phase `phase` of the current iteration (`Inner::phases`).
#[inline(always)]
#[expect(clippy::cast_possible_truncation, reason = "phases <= MAX_PHASES")]
fn phase_target(inner: &Inner, phase: u8) -> u8 {
    inner.target - (inner.phases as u8 - 1) + phase
}

#[inline(always)]
fn prefetch<T>(x: &T, write: bool) {
    let p = std::ptr::from_ref(x);
    if write {
        crate::prefetch::write(p);
    } else {
        crate::prefetch::read(p);
    }
}

/// Round `round` is at or after round `other` (wrapping).
fn after(round: u32, other: u32) -> bool {
    round.wrapping_sub(other) < u32::MAX / 2
}

/// Moves of `todo` to give away: the upper half of its faces, never the face
/// `current_face` of the child in progress (an X' child relies on the
/// other-side value noted by its X sibling) and never all of them.
fn split_moves(todo: u32, current_face: u8) -> Option<u32> {
    let mut faces = 0u32;
    for f in 0..6 {
        if todo & (7 << (3 * f)) != 0 {
            faces |= 1 << f;
        }
    }
    let mut left = faces.count_ones() / 2;
    let mut give = 0;
    for f in (0..6u8).rev() {
        if left == 0 {
            break;
        }
        if faces & (1 << f) != 0 && f != current_face {
            give |= todo & (7 << (3 * f));
            left -= 1;
        }
    }
    (give != 0).then_some(give)
}

// `Worker::rounds`'s section (see `crate::hot`).
crate::hot_section!(".text.hot_share", crate::hot::SHARE_PAD);

impl Worker<'_, '_> {
    fn run(&mut self, stop: &AtomicBool) {
        let mut round = 0u32;
        loop {
            if self.active_lanes == 0 && !self.idle_wait(stop) {
                break;
            }
            if !self.rounds(self.solver, stop, &mut round) {
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
    /// once per round by `maintain`. (Kept apart from the cold code, and
    /// placed at a fixed offset of a 64-byte line: see `crate::hot`.)
    // The solver is a parameter (noalias: the table pointer stays in a
    // register): -3.0% -c 0, -2.6% -c 1, +3.8% bench share at 48T, -2.0% -c 0
    // at 1T (l6-share-2).
    #[inline(never)]
    #[cfg_attr(
        all(target_arch = "x86_64", target_os = "linux"),
        unsafe(link_section = ".text.hot_share")
    )]
    fn rounds(&mut self, solver: &Solver, stop: &AtomicBool, round: &mut u32) -> bool {
        while self.active_lanes > 0 {
            let mut i = 0;
            let mut active = self.active_lanes;
            while i < active {
                // SAFETY: i < active_lanes <= lanes.len().
                let lane = unsafe { &mut self.lanes.get_unchecked_mut(i).lane };
                let value = lane.pending_value(solver);
                let outcome = match lane.phase {
                    Phase::WaitOther => {
                        if lane.resolve_other(value, solver) {
                            i += 1;
                            continue;
                        }
                        lane.advance(solver)
                    }
                    Phase::WaitMoved => lane.resolve_moved(value, solver),
                };
                match outcome {
                    Step::Pending => i += 1,
                    outcome => {
                        if self.on_end(i, matches!(outcome, Step::Found)) {
                            i += 1;
                        }
                        active = self.active_lanes;
                    }
                }
            }
            *round = round.wrapping_add(1);
            self.counts[C_ROUNDS] += 1;
            self.counts[C_VISITS] += active as u64;
            if active < self.lanes.len()
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
            self.found(i);
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
            let found = match self.lanes[i].lane.advance_cold(self.solver) {
                Step::Pending => return true,
                step => matches!(step, Step::Found),
            };
            if found {
                self.found(i);
            }
            prefer = Some(self.end_job(i, !found));
        }
    }

    /// Lane `i` found a solution (its job ends). In a first pass, it is
    /// reported if the phases below its own have all ended (it is optimal),
    /// else kept if it is the pass's shortest so far: the entries as long
    /// are then dropped (`take`, `cancel_done`).
    fn found(&mut self, i: usize) {
        let sl = &self.lanes[i];
        let ctx = &self.locals[sl.slot].as_ref().unwrap().ctx;
        if ctx.inner().phases == 1 {
            self.report(i);
            return;
        }
        let ctx = ctx.clone();
        let solution = sl.lane.reconstruct();
        let phase = u32::from(sl.phase);
        let mut pass = ctx.pass.lock().unwrap();
        if phase <= pass.phases_ended {
            drop(pass);
            self.report_pass(&ctx, solution);
            return;
        }
        if pass.candidate.is_empty() || phase < pass.candidate_phase {
            ctx.rarely_written.pass_limit.store(sl.lane.target, Relaxed);
            pass.candidate = solution;
            pass.candidate_phase = phase;
            drop(pass);
            self.pool.solved.fetch_add(1, Release);
        }
    }

    /// The entries of a phase of `ctx`'s first pass have all ended: report
    /// its shortest solution if the phases below it have all ended now.
    fn phase_done(&mut self, ctx: &CubeCtx) {
        let phases = ctx.inner().phases;
        let mut pass = ctx.pass.lock().unwrap();
        while pass.phases_ended < phases
            && ctx.phase_left[pass.phases_ended as usize].load(Acquire) == 0
        {
            pass.phases_ended += 1;
        }
        if !pass.candidate.is_empty() && pass.candidate_phase <= pass.phases_ended {
            let solution = std::mem::take(&mut pass.candidate);
            drop(pass);
            self.report_pass(ctx, solution);
        }
    }

    /// Report `solution` of `ctx`'s first pass (optimal: no shorter one is
    /// left), unless the cube is solved already.
    fn report_pass(&mut self, ctx: &CubeCtx, solution: Vec<u8>) {
        if ctx.done.swap(true, AcqRel) {
            return;
        }
        let seconds = ctx.started.elapsed().as_secs_f64();
        if self.pool.trace {
            tlog(
                self.pool,
                format_args!(
                    "trace cube {} found {:.3} ms (first pass) length {}",
                    ctx.index,
                    seconds * 1e3,
                    solution.len()
                ),
            );
        }
        self.counts[C_PASS_SOLUTIONS] += 1;
        self.publish_solution(ctx, solution, seconds);
    }

    /// Record lane `i`'s solution if it is the first for its cube.
    fn report(&mut self, i: usize) {
        let sl = &self.lanes[i];
        let ctx = &self.locals[sl.slot].as_ref().unwrap().ctx;
        if ctx.done.swap(true, AcqRel) {
            return;
        }
        let solution = sl.lane.reconstruct();
        let seconds = ctx.started.elapsed().as_secs_f64();
        if self.pool.trace {
            trace_found(self.pool, ctx, sl, self.active_lanes, seconds);
        }
        self.publish_solution(ctx, solution, seconds);
    }

    /// `ctx` is solved (`done` was set by this thread): publish the result.
    fn publish_solution(&self, ctx: &CubeCtx, solution: Vec<u8>, seconds: f64) {
        *ctx.result.lock().unwrap() = (solution, seconds);
        self.pool.active.fetch_sub(1, AcqRel);
        self.pool.epoch.fetch_add(1, Release);
        self.pool.solved.fetch_add(1, Release);
        // The "unsolved" part (this thread still holds one).
        ctx.parts.fetch_sub(1, AcqRel);
    }

    /// The end of a first pass (no job outstanding): report its shortest
    /// solution, if it has one left (it is optimal: all phases ended).
    /// Returns whether the cube is solved.
    fn pass_solution(&mut self, ctx: &CubeCtx) -> bool {
        // SAFETY: no job of the cube is outstanding (only the builder runs).
        let inner = unsafe { &*ctx.inner.get() };
        if inner.phases == 1 {
            return false;
        }
        let solution = std::mem::take(&mut ctx.pass.lock().unwrap().candidate);
        if solution.is_empty() {
            return false;
        }
        self.report_pass(ctx, solution);
        true
    }

    /// Account for the end of lane `i`'s job; returns its slot.
    fn end_job(&mut self, i: usize, stat: bool) -> usize {
        let sl = &mut self.lanes[i];
        let slot = sl.slot;
        sl.slot = NO_SLOT;
        let loc = self.locals[slot].as_mut().unwrap();
        loc.nodes += sl.lane.stats.nodes - sl.nodes_at_start;
        loc.lookups += sl.lane.stats.lookups - sl.lookups_at_start;
        loc.iteration_nodes += sl.lane.stats.nodes - sl.nodes_at_start;
        loc.iteration_jobs += 1;
        // The job's statistic is the sum of its per-child statistics, as stream
        // mode records them for c7's child ordering (part of c7-childsplit: -6..-8%
        // nodes in stream mode; share mode keeps whole-job order, not measured here).
        let inner = loc.ctx.inner();
        let phases = inner.phases;
        if stat && inner.record_leads && sl.lead_children != 0 && sl.lane.stack[0].normal {
            let children = moves_after(&inner.jobs[sl.job as usize]) & sl.lead_children;
            let lead = lead_of(&sl.lane.child_stats, children, inner.write_tag);
            inner.leads[sl.job as usize].store(lead, Relaxed);
        }
        // (In a first pass, only the last phase's entries give statistics:
        // its target is the next iteration's.)
        if stat && u32::from(sl.phase) + 1 == phases {
            let job_stat = &inner.stats[sl.job as usize];
            if sl.split {
                add_f32(job_stat, sl.lane.job_stat);
            } else {
                // (Nobody else writes it; it was 0.)
                job_stat.store(sl.lane.job_stat.to_bits(), Relaxed);
            }
        }
        loc.pending -= 1;
        if phases > 1 {
            let phase = usize::from(sl.phase);
            loc.phase_pending[phase] -= 1;
            self.phase_ended(slot, phase);
        }
        slot
    }

    fn deactivate(&mut self, i: usize) {
        self.active_lanes -= 1;
        self.lanes.swap(i, self.active_lanes);
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
        // (Kept cubes: no need to look for them.)
        if self.pool.few() {
            for slot in 0..self.locals.len() {
                if matches!(&self.locals[slot], Some(l) if l.ctx.unclaimed() > 0)
                    && self.continue_local(li, slot)
                {
                    return true;
                }
            }
        }
        self.counts[C_REFILL] += 1;
        let ok = self.fresh(li) || self.open_cube(li) || self.split(li);
        self.counts[C_REFILL_FAIL] += u64::from(!ok);
        ok
    }

    /// Start a job of the cube in `slot` in lane `li`, claiming more jobs if
    /// needed. Releases the slot if nothing is left to do there (unless the
    /// cube is kept).
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
        let loc = self.locals[slot].as_ref().unwrap();
        if loc.pending == 0 && !self.keep(&loc.ctx) {
            self.release(slot);
        }
        false
    }

    /// Count down the jobs of `slot` if none is pending, and leave the cube
    /// then (unless it is kept).
    fn tidy(&mut self, slot: usize) {
        self.settle(slot);
        let loc = self.locals[slot].as_ref().unwrap();
        if loc.pending == 0 && !self.keep(&loc.ctx) {
            self.release(slot);
        }
    }

    /// With few cubes in progress, a thread stays with (at most `KEEP`)
    /// unsolved cubes it has nothing to do on: its next jobs there need no
    /// search of the open list, nor joining again (atomics on lines shared
    /// by all threads, at every iteration).
    // Part of l6-share-2 (-5.6% -c 1 latency at 48T for the series; screening
    // run without it: -c 1 2.46 vs 2.36-2.42 ms).
    fn keep(&self, ctx: &CubeCtx) -> bool {
        !ctx.done.load(Relaxed)
            && self.pool.few()
            && self.locals.iter().filter(|l| l.is_some()).count() <= KEEP
    }

    /// Leave the kept cubes that were solved (nothing pending).
    fn release_done(&mut self) {
        for slot in 0..self.locals.len() {
            if matches!(&self.locals[slot], Some(l) if l.pending == 0 && l.ctx.done.load(Relaxed)) {
                // (Clears the claimed count.)
                self.settle(slot);
                self.release(slot);
            }
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
        loop {
            let loc = self.locals[slot].as_mut().unwrap();
            if loc.next >= loc.end {
                return false;
            }
            let entry = loc.next;
            loc.next += 1;
            let ctx = &*loc.ctx;
            let inner = ctx.inner();
            let (job, target, phase) = entry_job(inner, entry);
            // (A split iteration's run: a job and its root children.)
            let (job, children) = if inner.split {
                let run = inner.runs[job as usize];
                (run_job(run), run_children(run))
            } else {
                (job, ALL_MOVES)
            };
            let phases = inner.phases;
            let job_entry = inner.jobs[job as usize];
            // (A first pass's entries that are infeasible in their phase are
            // skipped before the job's cubes are made, and those as long as
            // a solution found are of no use.)
            let skip = phases > 1
                && (job_entry.normal_bound.max(job_entry.inverse_bound) + job_entry.len > target
                    || target >= ctx.rarely_written.pass_limit.load(Relaxed));
            let sl = &mut self.lanes[li];
            let lane = &mut sl.lane;
            if !skip {
                lane.root = ctx.root;
                lane.root_inverse = ctx.root_inverse;
                lane.target = target;
                lane.jobs[0] = job_entry;
                lane.job_pos = 0;
                if lane.setup_job() {
                    // (The jobs and statistics were fetched when claimed.)
                    self.counts[C_JOBS] += 1;
                    sl.slot = slot;
                    sl.job = job;
                    sl.split = false;
                    sl.phase = phase;
                    sl.nodes_at_start = lane.stats.nodes;
                    sl.lookups_at_start = lane.stats.lookups;
                    if inner.split || inner.record_leads {
                        lead_run(sl, inner, entry, children);
                    }
                    if self.pool.trace {
                        sl.started = Instant::now();
                        sl.entry = entry;
                    }
                    return true;
                }
            }
            // infeasible (statistic 0, as in stream mode)
            loc.pending -= 1;
            if phases > 1 {
                loc.phase_pending[usize::from(phase)] -= 1;
                self.phase_ended(slot, usize::from(phase));
            }
        }
    }

    /// An entry (or piece) of phase `phase` of `slot`'s first pass ended: if
    /// none of this thread's is left, count them down from the cube's, and
    /// if they were the last, the phase ended (`phase_done`).
    fn phase_ended(&mut self, slot: usize, phase: usize) {
        let loc = self.locals[slot].as_mut().unwrap();
        if loc.phase_pending[phase] != 0 || loc.phase_claimed[phase] == 0 {
            return;
        }
        let ended = std::mem::take(&mut loc.phase_claimed[phase]);
        if loc.ctx.phase_left[phase].fetch_sub(ended, AcqRel) == ended {
            let ctx = loc.ctx.clone();
            self.phase_done(&ctx);
        }
    }

    /// Claim more jobs of the cube in `slot` (none must be unstarted).
    fn claim(&mut self, slot: usize) -> bool {
        let loc = self.locals[slot].as_mut().unwrap();
        debug_assert!(loc.next >= loc.end);
        // (Chunks shrink as the iteration runs out: l5, not measured alone.)
        let chunk = (loc.ctx.unclaimed() / (2 * self.pool.nthreads)).clamp(1, CHUNK_MAX);
        let Some((first, count)) = loc.ctx.claim(chunk) else {
            return false;
        };
        loc.next = first;
        loc.end = first + count;
        loc.pending += count;
        loc.claimed += u64::from(count);
        let inner = loc.ctx.inner();
        let mut jobs = (first, loc.end);
        if inner.phases > 1 {
            // (A first pass's entries: per phase, all the jobs again.)
            let per_phase = inner.phase_entries;
            let mut entry = first;
            while entry < loc.end {
                let phase = entry / per_phase;
                let phase_end = ((phase + 1) * per_phase).min(loc.end);
                loc.phase_pending[phase as usize] += phase_end - entry;
                loc.phase_claimed[phase as usize] += u64::from(phase_end - entry);
                entry = phase_end;
            }
            let base = first / per_phase * per_phase;
            jobs = (first - base, (loc.end - base).min(per_phase));
        }
        // The claimed jobs and their statistics (written at their ends):
        // the lines were written by other threads (the jobs by the build,
        // the statistics by the publication), and lanes take the jobs one
        // after another. (l6-share-2; screening run without it: -c 1 3.06 vs
        // 2.36-2.42 ms, bench share 799 vs 845/s at 48T.)
        let (first, end) = (jobs.0 as usize, jobs.1 as usize);
        if inner.split {
            // (The runs' jobs are scattered.)
            for &run in &inner.runs[first..end] {
                prefetch(&inner.jobs[run_job(run) as usize], false);
                prefetch(&inner.stats[run_job(run) as usize], true);
            }
            return true;
        }
        for i in (first..end).step_by(4) {
            prefetch(&inner.jobs[i], false);
        }
        prefetch(&inner.jobs[end - 1], false);
        for i in (first..end).step_by(16) {
            prefetch(&inner.stats[i], true);
        }
        prefetch(&inner.stats[end - 1], true);
        if inner.record_leads {
            for i in (first..end).step_by(16) {
                prefetch(&inner.leads[i], true);
            }
        }
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
        let claimed = loc.claimed;
        loc.claimed = 0;
        if self.pool.trace {
            if let Some(t) = loc.ctx.thread_done_ns.get(self.id) {
                t.store(
                    u64::try_from(loc.ctx.started.elapsed().as_nanos()).unwrap_or(0),
                    Relaxed,
                );
            }
        }
        if self.pool.trace_bits & TRACE_COUNTERS != 0 {
            loc.ctx
                .rarely_written
                .iteration_nodes
                .fetch_add(std::mem::take(&mut loc.iteration_nodes), Relaxed);
            loc.ctx
                .rarely_written
                .iteration_jobs
                .fetch_add(std::mem::take(&mut loc.iteration_jobs), Relaxed);
        }
        // (Release: the statistics of the jobs; Acquire: everybody else's.)
        if loc.ctx.outstanding.fetch_sub(claimed, AcqRel) == claimed && !loc.ctx.done.load(Acquire)
        {
            let ctx = loc.ctx.clone();
            if self.pass_solution(&ctx) {
                return;
            }
            self.counts[C_BUILDS] += 1;
            if build(
                &mut self.scratch,
                &mut self.bufs,
                self.solver,
                self.pool,
                &ctx,
                false,
            ) {
                while build_task(&mut self.scratch, self.solver, self.pool, &ctx, self.id) {}
            }
        }
    }

    /// Leave the cube in `slot` (nothing pending).
    fn release(&mut self, slot: usize) {
        let loc = self.locals[slot].take().unwrap();
        debug_assert!(loc.pending == 0 && loc.claimed == 0);
        let ctx = loc.ctx;
        ctx.rarely_written.nodes.fetch_add(loc.nodes, Relaxed);
        ctx.rarely_written.lookups.fetch_add(loc.lookups, Relaxed);
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
            let (solution, seconds) = std::mem::take(&mut *ctx.result.lock().unwrap());
            let stats = Stats {
                nodes: ctx.rarely_written.nodes.load(Relaxed),
                lookups: ctx.rarely_written.lookups.load(Relaxed),
            };
            (self.done)(ctx.index, solution, stats, seconds);
        }
    }

    /// The slot of `ctx`, joining it if needed (None if it is finished).
    fn local(&mut self, ctx: &Arc<CubeCtx>) -> Option<usize> {
        let joined = |local: &Option<Local>| matches!(local, Some(l) if Arc::ptr_eq(&l.ctx, ctx));
        if let Some(slot) = self.locals.iter().position(joined) {
            return Some(slot);
        }
        // (Like upgrading a weak reference: never revive a reported cube.)
        let mut parts = ctx.parts.load(Acquire);
        loop {
            if parts == 0 {
                return None;
            }
            match ctx
                .parts
                .compare_exchange_weak(parts, parts + 1, AcqRel, Acquire)
            {
                Ok(_) => break,
                Err(now) => parts = now,
            }
        }
        let loc = Local {
            ctx: ctx.clone(),
            next: 0,
            end: 0,
            pending: 0,
            claimed: 0,
            phase_pending: [0; MAX_PHASES],
            phase_claimed: [0; MAX_PHASES],
            nodes: 0,
            lookups: 0,
            iteration_nodes: 0,
            iteration_jobs: 0,
        };
        if let Some(slot) = self.locals.iter().position(Option::is_none) {
            self.locals[slot] = Some(loc);
            Some(slot)
        } else {
            self.locals.push(Some(loc));
            Some(self.locals.len() - 1)
        }
    }

    /// Start a fresh cube in lane `li`.
    fn fresh(&mut self, li: usize) -> bool {
        let pool = self.pool;
        let solver = self.solver;
        while !pool.no_fresh.load(Relaxed) {
            // (A load first: idle threads call this often.)
            if pool.active.load(Relaxed) >= pool.max_active {
                return false;
            }
            if pool.active.fetch_add(1, AcqRel) >= pool.max_active {
                pool.active.fetch_sub(1, AcqRel);
                return false;
            }
            let index = pool.next.fetch_add(1, Relaxed);
            let Some(cubie) = (pool.cubes)(index) else {
                pool.no_fresh.store(true, Release);
                pool.active.fetch_sub(1, AcqRel);
                pool.epoch.fetch_add(1, Release);
                return false;
            };
            let started = Instant::now();
            let root = Cube::from_cubie(&cubie);
            if root.is_solved() {
                pool.active.fetch_sub(1, AcqRel);
                (self.done)(index, vec![], Stats::default(), 0.0);
                continue;
            }
            let root_inverse = root.inverse();
            let bound = solver.lower_bound(root);
            let bound_inverse = solver.lower_bound(root_inverse);
            let spare = pool.spare.lock().unwrap().pop();
            let mut inner = spare.unwrap_or_else(Inner::touched);
            inner.target = bound.max(bound_inverse).max(1);
            inner.tag_base = u32::try_from(index % 64).unwrap_or(0) * 4;
            let ctx = Arc::new(CubeCtx {
                index,
                started,
                root,
                root_inverse,
                sym_filter: solver.sym_filter(root, root_inverse),
                claim: Pad(AtomicU64::new(0)),
                task: Pad(AtomicU64::new(0)),
                task_left: Pad(AtomicU32::new(0)),
                outstanding: Pad(AtomicU64::new(0)),
                done: AtomicBool::new(false),
                phase_left: Pad(std::array::from_fn(|_| AtomicU64::new(0))),
                trace: pool.trace,
                parts: Pad(AtomicU32::new(1)),
                rarely_written: Pad(RarelyWritten {
                    pass_limit: AtomicU8::new(u8::MAX),
                    ..RarelyWritten::default()
                }),
                result: Mutex::new((vec![], 0.0)),
                pass: Mutex::new(PassState::default()),
                task_log: (0..1024).map(|_| Default::default()).collect(),
                thread_done_ns: (0..pool.nthreads).map(|_| AtomicU64::new(0)).collect(),
                inner: UnsafeCell::new(inner),
            });
            {
                // SAFETY: nobody else knows the cube yet.
                let inner = unsafe { &mut *ctx.inner.get() };
                inner.jobs.clear();
                inner.jobs.push(Job {
                    moves: [0; 4],
                    len: 0,
                    normal_bound: bound,
                    inverse_bound: bound_inverse,
                    stat: 0.0,
                });
                inner.stats.clear();
                inner.build_started = started;
                inner.extended = started;
            }
            // Join before `build` publishes the cube: from then on another
            // thread may solve and report it, and joining would fail.
            let slot = self.local(&ctx).unwrap();
            if build(&mut self.scratch, &mut self.bufs, solver, pool, &ctx, true) {
                while build_task(&mut self.scratch, solver, pool, &ctx, self.id) {}
            }
            #[cfg(test)]
            if pool.pause_after_publish {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            self.counts[C_FRESH] += 1;
            if self.continue_local(li, slot) {
                return true;
            }
        }
        false
    }

    /// Start a job of the oldest cube with unclaimed jobs in lane `li`.
    fn open_cube(&mut self, li: usize) -> bool {
        for _ in 0..4 {
            self.counts[C_SCANS] += 1;
            let Some(ctx) = self.pool.find_open(false) else {
                return false;
            };
            if let Some(slot) = self.local(&ctx) {
                if self.continue_local(li, slot) {
                    self.counts[C_OPEN] += 1;
                    return true;
                }
            }
        }
        false
    }

    /// Start the piece in this thread's mailbox in lane `li`.
    fn piece(&mut self, li: usize) -> bool {
        let pool = self.pool;
        let Some(mailbox) = pool.mail.get(self.id) else {
            return false;
        };
        if mailbox.state.load(Acquire) != FULL {
            return false;
        }
        let (ctx, job, phase) = {
            // SAFETY: `FULL`: the slot is ours until we store `IDLE`.
            let piece = unsafe { &mut *mailbox.piece.get() };
            // (The lane is idle: its stack is free.)
            let lane = &mut self.lanes[li].lane;
            lane.stack[..=piece.depth].copy_from_slice(&piece.nodes[..=piece.depth]);
            lane.depth = piece.depth;
            (piece.ctx.take().unwrap(), piece.job, piece.phase)
        };
        mailbox.state.store(IDLE, Release);
        // (A solved cube's outstanding count no longer matters.)
        if ctx.done.load(Relaxed) {
            return false;
        }
        let Some(slot) = self.local(&ctx) else {
            return false;
        };
        let loc = self.locals[slot].as_mut().unwrap();
        loc.pending += 1;
        loc.claimed += 1;
        let inner = ctx.inner();
        if inner.phases > 1 {
            loc.phase_pending[usize::from(phase)] += 1;
            loc.phase_claimed[usize::from(phase)] += 1;
        }
        let sl = &mut self.lanes[li];
        let lane = &mut sl.lane;
        lane.jobs[0] = inner.jobs[job as usize];
        lane.job_pos = 0;
        lane.target = phase_target(inner, phase);
        lane.root = ctx.root;
        lane.root_inverse = ctx.root_inverse;
        lane.child_stats = [0.0; 18];
        sl.slot = slot;
        sl.job = job;
        sl.entry = job;
        sl.lead_children = 0;
        sl.split = true;
        sl.phase = phase;
        sl.nodes_at_start = lane.stats.nodes;
        sl.lookups_at_start = lane.stats.lookups;
        sl.started = Instant::now();
        self.counts[C_PIECES] += 1;
        true
    }

    /// The lane and node to split for another lane: the active lane (other
    /// than `exclude`) with the shallowest node (remaining depth at least
    /// `min_r`) that has moves of two faces left. (lane, depth, moves to
    /// give away.)
    fn find_split(&self, min_r: u8, exclude: usize) -> Option<(usize, usize, u32)> {
        let mut best = None;
        let mut best_r = min_r - 1;
        for (index, sl) in self.lanes[..self.active_lanes].iter().enumerate() {
            if index == exclude || sl.slot == NO_SLOT {
                continue;
            }
            let lane = &sl.lane;
            for depth in 0..=lane.depth {
                let node = &lane.stack[depth];
                if node.remaining_depth <= best_r {
                    break;
                }
                if node.lower_bound > node.remaining_depth || node.todo == 0 {
                    continue;
                }
                // (An active lane's pending child is `pending_move`.)
                let face = if depth < lane.depth {
                    lane.stack[depth + 1].mv / 3
                } else {
                    lane.pending_move / 3
                };
                if let Some(give) = split_moves(node.todo, face) {
                    best = Some((index, depth, give));
                    best_r = node.remaining_depth;
                    break;
                }
            }
        }
        best
    }

    /// Give lane `li` part of a busy lane's remaining moves. (Splits and
    /// donations: part of l5's share mode, which took one cube at a time
    /// from 306 ms to 3.6 ms at 48T; not measured alone.)
    fn split(&mut self, li: usize) -> bool {
        // In an iteration's tail (nothing left to claim), splitting the
        // thread's own lanes into all its idle ones kept every lane slow (a
        // round visits them all; idle threads take pieces instead) and lost
        // the bounds that the split nodes learn from their children: -3.6% -c 1
        // at 48T on top of the first pass (8 paired holds; -c 0 and bench share
        // neutral).
        if self.active_lanes > TAIL_LANES {
            return false;
        }
        let Some((busy, depth, give)) = self.find_split(SPLIT_R_LOCAL, li) else {
            return false;
        };
        let Ok([from, to]) = self.lanes.get_disjoint_mut([busy, li]) else {
            unreachable!("`find_split` excludes lane `li`");
        };
        to.lane.stack[..=depth].copy_from_slice(&from.lane.stack[..=depth]);
        for node in &mut to.lane.stack[..depth] {
            node.todo = 0;
        }
        to.lane.stack[depth].todo = give;
        from.lane.stack[depth].todo &= !give;
        to.lane.depth = depth;
        to.lane.child_stats = [0.0; 18];
        to.lane.jobs[0] = from.lane.jobs[0];
        to.lane.job_pos = 0;
        to.lane.target = from.lane.target;
        to.lane.root = from.lane.root;
        to.lane.root_inverse = from.lane.root_inverse;
        to.slot = from.slot;
        to.job = from.job;
        to.entry = from.entry;
        to.lead_children = 0;
        from.lead_children = 0;
        to.phase = from.phase;
        to.split = true;
        from.split = true;
        to.nodes_at_start = to.lane.stats.nodes;
        to.lookups_at_start = to.lane.stats.lookups;
        to.started = Instant::now();
        let loc = self.locals[from.slot].as_mut().unwrap();
        loc.pending += 1;
        if loc.ctx.inner().phases > 1 {
            loc.phase_pending[usize::from(from.phase)] += 1;
        }
        if self.pool.trace_bits & TRACE_COUNTERS != 0 {
            loc.ctx
                .rarely_written
                .iteration_splits
                .fetch_add(1, Relaxed);
        }
        self.counts[C_SPLITS] += 1;
        true
    }

    /// Donate part of a busy lane's remaining moves to a hungry thread.
    fn donate(&mut self, round: u32) {
        if !after(round, self.donate_wait) {
            return;
        }
        let Some((busy, depth, give)) = self.find_split(SPLIT_R_REMOTE, usize::MAX) else {
            self.donate_backoff = (self.donate_backoff * 2).clamp(4, 256);
            self.donate_wait = round.wrapping_add(self.donate_backoff);
            return;
        };
        self.donate_backoff = 0;
        // a thread waiting for a piece
        let pool = self.pool;
        let mailboxes = pool.mail.len();
        let next = |k: usize| &pool.mail[(self.id + k) % mailboxes].0;
        let Some(mailbox) = (1..mailboxes).map(next).find(|mailbox| {
            mailbox.state.load(Relaxed) == WANT
                && mailbox
                    .state
                    .compare_exchange(WANT, FILL, AcqRel, Relaxed)
                    .is_ok()
        }) else {
            return;
        };
        pool.hungry.fetch_sub(1, Relaxed);
        let sl = &mut self.lanes[busy];
        sl.split = true;
        let ctx = self.locals[sl.slot].as_ref().unwrap().ctx.clone();
        // (The job is outstanding, so the iteration cannot end before the
        // piece is counted.)
        ctx.outstanding.fetch_add(1, Relaxed);
        if ctx.inner().phases > 1 {
            ctx.phase_left[usize::from(sl.phase)].fetch_add(1, Relaxed);
        }
        if pool.trace_bits & TRACE_COUNTERS != 0 {
            ctx.rarely_written.iteration_donations.fetch_add(1, Relaxed);
        }
        {
            // SAFETY: `FILL` (ours): the owner reads the slot once `FULL`.
            let piece = unsafe { &mut *mailbox.piece.get() };
            piece.nodes[..=depth].copy_from_slice(&sl.lane.stack[..=depth]);
            for node in &mut piece.nodes[..depth] {
                node.todo = 0;
            }
            piece.nodes[depth].todo = give;
            piece.ctx = Some(ctx);
            piece.job = sl.job;
            piece.phase = sl.phase;
            piece.depth = depth;
        }
        sl.lane.stack[depth].todo &= !give;
        mailbox.state.store(FULL, Release);
        self.counts[C_DONATED] += 1;
    }

    /// Once per round if lanes are idle, a cube was solved or other threads
    /// are hungry.
    #[inline(never)]
    fn maintain(&mut self, round: u32) {
        let solved = self.pool.solved.load(Acquire);
        if solved != self.solved {
            self.solved = solved;
            self.cancel_done(round);
            self.release_done();
        }
        if self.active_lanes < self.lanes.len() {
            let epoch = self.pool.epoch.load(Acquire);
            if epoch != self.epoch || after(round, self.split_wait) {
                self.epoch = epoch;
                self.fill_idle(round);
            }
        }
        if self.pool.hungry.load(Relaxed) != 0 {
            self.donate(round);
        }
    }

    /// Give idle lanes work.
    fn fill_idle(&mut self, round: u32) {
        while self.active_lanes < self.lanes.len() {
            if !self.restart(self.active_lanes, None) {
                // back off from splitting (busy lanes may have nothing to
                // split for a while)
                self.split_backoff = (self.split_backoff * 2).clamp(4, 256);
                self.split_wait = round.wrapping_add(self.split_backoff);
                return;
            }
            self.active_lanes += 1;
        }
        self.split_backoff = 0;
    }

    /// Drop the jobs of solved cubes (and a first pass's entries as long as
    /// its shortest solution so far), then give the lanes new work.
    fn cancel_done(&mut self, round: u32) {
        let mut i = self.active_lanes;
        let mut dropped = false;
        while i > 0 {
            i -= 1;
            let sl = &self.lanes[i];
            let ctx = &self.locals[sl.slot].as_ref().unwrap().ctx;
            // (A first pass's entries as long as its shortest solution so
            // far are of no use.)
            if !ctx.done.load(Relaxed)
                && sl.lane.target < ctx.rarely_written.pass_limit.load(Relaxed)
            {
                continue;
            }
            self.end_job(i, false);
            self.deactivate(i);
            dropped = true;
        }
        // Refilled together, up to the first lane without work: when each
        // dropped lane looked for work on its own, all threads took the open
        // list's lock once per lane at once, after each solved cube, and the
        // next cube's first build waited for them (at 48 threads, one thread
        // often ran all 18 tasks of its extension): -5.1% -c 1 at 48T (6 paired
        // holds; 15/16-move scrambles -10/-9%, depth-15 states -12%; -c 0 and
        // bench share neutral).
        if dropped {
            self.release_done();
            self.fill_idle(round);
        }
    }

    /// No lane is active: find work or wait for some. Returns false when
    /// the pool is finished (or `stop` is set).
    #[inline(never)]
    fn idle_wait(&mut self, stop: &AtomicBool) -> bool {
        let pool = self.pool;
        let entered = (pool.trace_bits & TRACE_IDLE != 0).then(Instant::now);
        let result = self.idle_wait_loop(stop);
        if let Some(entered) = entered {
            pool.idle_ns.fetch_add(
                u64::try_from(entered.elapsed().as_nanos()).unwrap_or(0),
                Relaxed,
            );
        }
        result
    }

    fn idle_wait_loop(&mut self, stop: &AtomicBool) -> bool {
        let pool = self.pool;
        loop {
            if stop.load(Relaxed) || pool.finished() {
                return false;
            }
            let epoch = pool.epoch.load(Acquire);
            self.release_done();
            // help extending job lists (of kept cubes first)
            for slot in 0..self.locals.len() {
                if let Some(loc) = &self.locals[slot] {
                    while build_task(&mut self.scratch, self.solver, pool, &loc.ctx, self.id) {}
                }
            }
            while let Some(ctx) = pool.find_open(true) {
                while build_task(&mut self.scratch, self.solver, pool, &ctx, self.id) {}
            }
            self.fill_idle(0);
            if self.active_lanes > 0 {
                self.unwant();
                return true;
            }
            self.want();
            self.counts[C_IDLE] += 1;
            let full = || matches!(self.mailbox(), Some(m) if m.state.load(Relaxed) == FULL);
            // Spin a little, then yield: a thread of ours with work may
            // share the core (until the scheduler moves one of us). Long
            // spinning slows the busy cores (screening, -c 1 at 48T: 2.54 ms
            // with these 1K spins, 3.19 ms with 16K, 4.21 ms with 256K);
            // yielding at once was not better in 5 runs (l6).
            for i in 0..1 << 12 {
                if pool.epoch.load(Relaxed) != epoch || full() {
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
        solve_with_pool_counted(solver, cubes, (threads, nlanes, max_active), |pool| {
            pool.pause_after_publish = pause_after_publish;
        })
        .0
    }

    /// `solve_with_pool` with (threads, lanes, cubes in progress) and a
    /// change to the pool's settings; also returns its event counters.
    fn solve_with_pool_counted(
        solver: &Solver,
        cubes: &[CubieCube],
        (threads, nlanes, max_active): (usize, usize, usize),
        settings: impl FnOnce(&mut Pool),
    ) -> (Vec<Vec<u8>>, [u64; NCOUNT]) {
        let get = |i: usize| cubes.get(i).copied();
        let mut pool = Pool::new(&get, max_active, threads);
        settings(&mut pool);
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
        let solutions = solutions
            .into_iter()
            .enumerate()
            .map(|(i, s)| s.unwrap_or_else(|| panic!("cube {i} not reported")))
            .collect();
        let counts = *pool.counters.lock().unwrap();
        (solutions, counts)
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

    /// A cube's first pass, for the iterations from its lower bound up to the
    /// first with 3-move jobs (8 here), notes the solutions shorter than its
    /// target too, and reports the shortest when it ends: as short as
    /// `Solver::solve`, also when the pass is shared by lanes and threads.
    #[test]
    fn share_first_pass_reports_the_shortest_solution() {
        let corners = Cocsep::truncated(4);
        let table = H48Table::zeroed(0, 2);
        let solver = Solver::new(&corners, &table);
        let mut rng = crate::cubie::Rng(91);
        let count = if cfg!(miri) { 3 } else { 40 };
        let cubes: Vec<CubieCube> = (0..count)
            .map(|i| {
                let moves: Vec<u8> = (0..4 + i % 4)
                    .map(|_| u8::try_from(rng.below(18)).unwrap())
                    .collect();
                CubieCube::SOLVED.apply_moves(&moves)
            })
            .collect();
        let optimal: Vec<usize> = cubes.iter().map(|c| solver.solve(c).0.len()).collect();
        let configs: &[(usize, usize, usize)] = if cfg!(miri) {
            &[(2, 2, 1)]
        } else {
            &[(1, 1, 1), (3, 2, 1), (8, 3, 2), (4, 2, usize::MAX)]
        };
        for &config in configs {
            let (solutions, counts) = solve_with_pool_counted(&solver, &cubes, config, |_| {});
            for (i, (cube, solution)) in cubes.iter().zip(&solutions).enumerate() {
                assert_eq!(cube.apply_moves(solution), CubieCube::SOLVED, "cube {i}");
                assert_eq!(solution.len(), optimal[i], "cube {i}: {config:?}");
            }
            assert!(counts[C_PASS_SOLUTIONS] > 0, "{config:?}: {counts:?}");
            // Without first passes: the same lengths.
            let (solutions, counts) = solve_with_pool_counted(&solver, &cubes, config, |pool| {
                pool.first_pass_span = 0;
            });
            assert_eq!(counts[C_PASS_SOLUTIONS], 0);
            for (i, solution) in solutions.iter().enumerate() {
                assert_eq!(solution.len(), optimal[i], "cube {i}: {config:?}");
            }
        }
    }

    /// `lead_of` as written out per child.
    #[test]
    #[expect(clippy::cast_possible_truncation, reason = "random bits")]
    fn lead_of_matches_scalar_lead() {
        let mut rng = crate::cubie::Rng(5);
        for _ in 0..2000 {
            let children = rng.next_u64() as u32 & ALL_MOVES;
            let mut stats = [0f32; 18];
            for stat in &mut stats {
                let r = rng.next_u64();
                *stat = if r.is_multiple_of(3) {
                    0.0
                } else {
                    f32::from((r >> 8) as u16) * 4.0
                };
            }
            let weighted: Vec<f32> = (0..18)
                .map(|mv| {
                    let weight = if mv / 3 % 2 == 1 { 1.189_207_1 } else { 1.0 };
                    if children >> mv & 1 != 0 {
                        stats[mv] * weight
                    } else {
                        0.0
                    }
                })
                .collect();
            let best = weighted.iter().copied().fold(0f32, f32::max);
            let code = super::super::quantize(best);
            let mut group = 0u32;
            for (mv, &stat) in weighted.iter().enumerate() {
                if children >> mv & 1 != 0 && stat >= best * LEAD_SHARE {
                    group |= 1 << mv;
                }
            }
            for face in 0..6 {
                if group >> (3 * face) & 0b101 != 0 {
                    group |= 0b101 << (3 * face);
                }
            }
            let want = if code == 0 {
                0
            } else {
                lead_word(7, code, group & children)
            };
            assert_eq!(
                lead_of(&stats, children, 7),
                want,
                "{stats:?} {children:#x}"
            );
        }
    }

    /// A split iteration's staged sort: each job's root children run once,
    /// in its first-part run (a lead of the cube above the threshold: its
    /// group) or its rest run; the first-part runs by decreasing lead
    /// bucket, ties in job order; the rest runs in the order of `sort_key`
    /// (a job without children left has none).
    #[test]
    #[expect(clippy::cast_possible_truncation, reason = "jobs from random bits")]
    fn split_runs_cover_each_child_once() {
        let mut rng = crate::cubie::Rng(23);
        let n = if cfg!(miri) { 9000 } else { 30000 };
        let tag = 5;
        let mut inner = Inner::touched();
        inner.jobs = (0..n)
            .map(|i| {
                let r = rng.next_u64();
                Job {
                    moves: [0, 3, 6, (r % 18) as u8],
                    len: 4,
                    normal_bound: (i % 13) as u8,
                    inverse_bound: ((r >> 8) % 12) as u8,
                    stat: 0.0,
                }
            })
            .collect();
        inner.stats.clear();
        inner.stats.resize_with(n, || AtomicU32::new(0));
        for i in 0..n {
            let r = rng.next_u64();
            let allowed = moves_after(&inner.jobs[i]);
            let stat = f32::from((r >> 40) as u16 % 1000);
            inner.stats[i].store(stat.to_bits(), Relaxed);
            let code = 48 + (r >> 8) as u8 % 150;
            let group = (r >> 20) as u32 & allowed | 1 << allowed.trailing_zeros();
            let lead = match r % 10 {
                0 => 0,
                1 => lead_word(tag + 1, code, group),
                // (all children in the group: no rest run)
                2 => lead_word(tag, code, allowed),
                _ => lead_word(tag, code, group),
            };
            inner.leads[i].store(lead, Relaxed);
        }
        inner.target = 16;
        inner.split = true;
        inner.read_tag = tag;
        inner.b = Bufs::sized();
        inner.b.stats_from_atomics = true;
        let mut tasks = sort_setup(&mut inner);
        let mut stage = ST_KEYS;
        loop {
            for task in (0..tasks).rev() {
                // SAFETY: one task at a time.
                unsafe { sort_task(&inner, stage, task) };
            }
            match sort_stage_done(&mut inner, stage) {
                Some((next_stage, next_tasks)) => (stage, tasks) = (next_stage, next_tasks),
                None => break,
            }
        }
        let threshold = inner.lead_threshold;
        assert!(threshold > 48 && threshold < 200, "{threshold}");
        let first = inner.runs.len() - n;
        let mut covered = vec![0u32; n];
        let mut last = None;
        for (e, &run) in inner.runs[..inner.feasible].iter().enumerate() {
            let job = run_job(run) as usize;
            let children = run_children(run) & moves_after(&inner.jobs[job]);
            assert_eq!(covered[job] & children, 0, "job {job} child run twice");
            covered[job] |= children;
            if e < first {
                let (code, group) = lead_fields(inner.leads[job].load(Relaxed), tag).unwrap();
                assert!(code > threshold);
                assert_eq!(run_children(run), group);
                let order = Some((lead_bucket(code), job));
                assert!(order > last);
                last = order;
            }
        }
        for (job, &mask) in covered.iter().enumerate() {
            assert_eq!(mask, moves_after(&inner.jobs[job]), "job {job}");
        }
        // The rest runs: the jobs by `sort_key`, those without children
        // left last (not claimed).
        let mut want: Vec<(bool, u64, usize)> = (0..n)
            .map(|i| {
                let has_first = lead_fields(inner.leads[i].load(Relaxed), tag)
                    .is_some_and(|(code, _)| code > threshold);
                let rest = if has_first {
                    ALL_MOVES & !(inner.leads[i].load(Relaxed) & ALL_MOVES)
                } else {
                    ALL_MOVES
                };
                let empty = moves_after(&inner.jobs[i]) & rest == 0;
                (empty, sort_key(&inner.jobs, &inner.stats, true, 16, i), i)
            })
            .collect();
        want.sort_unstable();
        let got: Vec<usize> = inner.runs[first..]
            .iter()
            .map(|&run| run_job(run) as usize)
            .collect();
        assert!(got.iter().eq(want.iter().map(|w| &w.2)));
        assert_eq!(inner.feasible, first + want.iter().filter(|w| !w.0).count());
    }

    /// The staged sort (its tasks run in any order within a stage) gives
    /// the order of `sort_jobs`.
    #[test]
    #[expect(clippy::cast_possible_truncation, reason = "jobs from random bits")]
    fn staged_sort_matches_sort_jobs() {
        let mut rng = crate::cubie::Rng(11);
        for n in [8192usize, 20000, 43254] {
            let jobs: Vec<Job> = (0..n)
                .map(|i| {
                    let r = rng.next_u64();
                    // many ties and a wide range of keys
                    let stat = match r % 5 {
                        0 => 0.0,
                        1 => f32::from((r >> 8) as u8),
                        _ => f32::from_bits((r >> 16) as u32 % 0x4000_0000 + 0x0080_0000),
                    };
                    Job {
                        moves: [(r >> 40) as u8 % 18, 0, 0, (i % 251) as u8],
                        len: 1,
                        normal_bound: (i / 251) as u8,
                        inverse_bound: 0,
                        stat,
                    }
                })
                .collect();
            let mut want = jobs.clone();
            super::super::sort_jobs(&mut want, &mut Vec::new());
            // all feasible
            let mut serial = jobs.clone();
            assert_eq!(sort_serial(&mut serial, &mut Bufs::sized(), 255), n);
            let id = |j: &Job| (j.moves, j.normal_bound, j.stat.to_bits());
            assert!(serial.iter().map(id).eq(want.iter().map(id)), "n = {n}");
            // the feasible ones in that order, then the others
            let target = 100;
            let feasible = |j: &&Job| j.normal_bound < target;
            let want_t: Vec<Job> = want
                .iter()
                .filter(feasible)
                .chain(want.iter().filter(|j| !feasible(j)))
                .copied()
                .collect();
            let feasible = want.iter().filter(feasible).count();
            let mut serial = jobs.clone();
            assert_eq!(
                sort_serial(&mut serial, &mut Bufs::sized(), target),
                feasible
            );
            assert!(serial.iter().map(id).eq(want_t.iter().map(id)), "n = {n}");
            let mut inner = Inner::new(jobs, target);
            inner.b = Bufs::sized();
            let mut tasks = sort_setup(&mut inner);
            let mut stage = ST_KEYS;
            loop {
                for task in (0..tasks).rev() {
                    // SAFETY: one task at a time.
                    unsafe { sort_task(&inner, stage, task) };
                }
                match sort_stage_done(&mut inner, stage) {
                    Some((next_stage, next_tasks)) => (stage, tasks) = (next_stage, next_tasks),
                    None => break,
                }
            }
            assert!(
                inner.jobs.iter().map(id).eq(want_t.iter().map(id)),
                "n = {n}"
            );
            assert_eq!(inner.feasible, feasible);
        }
    }
}
