//! IDA* optimal solver using the H48 table: the plain recursive search,
//! one cube at a time. `lanes` runs the same search interleaved over many
//! cubes, which is much faster; this one is the reference.
//!
//! IDA* (iterative deepening A*): for target lengths `T` = 1, 2, ..., a
//! depth-first search for a solution of exactly `T` moves that skips any
//! node whose lower bound exceeds its remaining moves. The first target
//! with a solution gives an optimal one.
//!
//! The search runs on both the cube and its inverse (a solution of the
//! inverse is the reversed, inverted solution of the cube): a node either
//! applies a move to the normal cube (appended to the solution) or to the
//! inverse cube (a "premove", prepended in inverted form). Each node moves
//! the side with fewer allowed moves, and both sides' table bounds prune.
//!
//! The first moves of each iteration are split into "jobs" (prefixes of up
//! to 4 moves), sorted by how promising they were in the last iteration,
//! so that the last iteration tends to find its solution early (-36% nodes,
//! +46% at 48T on a batch when added, job-order).
//!
//! `all_solutions` lists every distinct solution up to a length instead
//! (`crate::solutions`): the same search, which records a solution and goes
//! on, through every target up to the limit.
//!
//! In the quarter-turn metric (`metric`; the search's `QTM` parameter) the
//! search is the same, with a half turn charged two moves of the target:
//! targets of the parity of the cube's corner permutation, in steps of 2,
//! and the bounds learned from a child or sibling are lowered by the moves
//! between them (see `Learned`).

use crate::coord::{cocsep_x3, index_sym};
use crate::cubie::{inverse_move, CubieCube, Move};
use crate::metric::{move_cost, Metric};
use crate::simd::Cube;
use crate::solutions::{Collector, Options, Solutions, Stopped};
use crate::symstart::SymFilter;
use crate::tables::{packed_class, packed_depth, packed_sym_to_rep, Cocsep, H48Table, TableError};
use std::cell::RefCell;

/// All 18 moves (the low 18 bits).
pub const ALL_MOVES: u32 = u32::MAX.unbounded_shr(32 - 18);

/// The largest target (iteration depth) searched: every position is solved
/// in at most 20 moves (God's number), so needing a later iteration means
/// an unsolvable cube or a table whose bounds are too high, and the cube is
/// given up (no solution). It also bounds the search stacks' depth.
pub const MAX_TARGET: u8 = 20;
/// `MAX_TARGET` in the quarter-turn metric: God's number there is 26 (the
/// superflip composed with the four-spot pattern).
pub const MAX_TARGET_QTM: u8 = 26;
/// Quarter turns only (moves with m % 3 != 1).
pub const QUARTER_TURNS: u32 = 0b101_101_101_101_101_101;

/// Moves allowed after a move on `face` (on the same side of the search):
/// not the same face again (two turns of a face are one turn or none), and
/// after the second face of an opposite pair (D, L, B) not the first one
/// (U D = D U: only one order is searched).
const fn allowed_after_table() -> [u32; 6] {
    let mut table = [0u32; 6];
    let mut face = 0;
    while face < 6 {
        let mut moves = ALL_MOVES & !(7 << (3 * face));
        if face % 2 == 1 {
            moves &= !(7 << (3 * (face - 1)));
        }
        table[face] = moves;
        face += 1;
    }
    table
}
/// Compile-time copy of `ALLOWED_AFTER` (for `simd::HOT_TABLES`).
pub(crate) const ALLOWED_AFTER_CONST: [u32; 6] = allowed_after_table();
/// Moves allowed after a move on each face (see `allowed_after_table`).
pub static ALLOWED_AFTER: [u32; 6] = ALLOWED_AFTER_CONST;

/// The tables of a search, and the hot path's lookups.
///
/// ```compile_fail,E0616
/// fn replace<'a>(solver: &mut egg_solver::solver::Solver<'a>, table: &'a egg_solver::tables::H48Table) {
///     solver.table = table;
/// }
/// ```
pub struct Solver<'a> {
    pub(crate) corners: &'a Cocsep,
    pub(crate) table: &'a H48Table,
    /// `table.ptr()`, cached (saves a dependent load per prefetch/lookup;
    /// with `value` and `prefetch`, part of m1-micro's +2.2% at 48T).
    table_ptr: *const u8,
    /// `corners.packed_ptr()`, cached (saves a dependent load per index).
    packed_ptr: *const u8,
    /// `coord::index_sym` constants of the table's `k` (with `packed_ptr`,
    /// part of m6-micro's index patch: +0.7% at 1T, -0.4% at 48T).
    row_mul: u64,
    eo_mask: u32,
    /// The table's base: codes 1..=3 mean `base + code`.
    pub(crate) base: u8,
    /// The largest bound a lookup can give: `base + 3`, in QTM also the
    /// largest corner depth (10 with QTM depths, above `base + 3` for bases
    /// below 7). With more moves left no lookup can prune.
    pub(crate) max_value: u8,
    /// The metric of the search.
    metric: Metric,
    /// Symmetric-start filtering of the job prefixes (`symstart`; here -29%
    /// nodes on the 21 positions of `symmetric_start_keeps_optimal_lengths`,
    /// h0); off only to compare against the unfiltered search.
    pub sym_start: bool,
}
// `table_ptr` and `packed_ptr` point into `table` and `corners`, which are
// shared read-only.
unsafe impl Send for Solver<'_> {}
unsafe impl Sync for Solver<'_> {}

/// Search statistics.
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    /// Nodes generated (cubes reached by a move).
    pub nodes: u64,
    /// Table lookups (each costs a likely cache miss).
    pub lookups: u64,
}

/// A canonical move prefix searched as a unit; jobs are re-ordered between
/// iterations by `stat`, an estimate of how close their subtree got to
/// solved (sum of 2^(12-h) over evaluated nodes), so that in the last
/// iteration the solution tends to be found early.
#[derive(Clone, Copy)]
struct Job {
    moves: [u8; 4],
    len: u8,
    /// The moves' length in the search's metric (`len` in HTM).
    cost: u8,
    /// The cube after the prefix, and its inverse.
    cube: Cube,
    inverse: Cube,
    stat: u64,
    /// Lower bounds of the job root (normal, inverse); computed once.
    normal_bound: u8,
    inverse_bound: u8,
}

/// Face weight compensating for D/L/B prefixes having fewer continuations.
#[inline(always)]
fn job_weight(j: &Job) -> u64 {
    if (j.moves[j.len as usize - 1] / 3).is_multiple_of(2) {
        47525
    } else {
        58206
    }
}

/// The path of the current search and its counters.
struct SearchState {
    /// Moves applied to the normal cube, from the root.
    moves: [u8; 32],
    n_moves: usize,
    /// Moves applied to the inverse cube.
    premoves: [u8; 32],
    n_premoves: usize,
    stats: Stats,
    /// The current job's statistic.
    stat: u64,
}

/// A child of a search node, while its bounds are looked up.
#[derive(Clone, Copy)]
struct Child {
    /// The moved cube (of the node's chosen side) and the other side's cube.
    moved: Cube,
    other: Cube,
    /// Pending table index.
    index: u64,
    mv: u8,
    /// Moves left to the target after this child's move (in QTM 2 fewer
    /// than at the node for a half turn, else 1).
    remaining: u8,
    /// Lower bounds of the moved and the other side.
    moved_bound: u8,
    other_bound: u8,
    /// Not pruned (yet).
    live: bool,
}

/// A bound not known yet.
const UNKNOWN: u8 = 0xFF;
/// `search`'s result when it found a solution (in a search for all
/// solutions: when it is to stop, having found more than asked for).
const FOUND: u8 = 0xFE;

/// What a search does with a solution: `One` ends with it (the path in
/// `SearchState` is the solution); a search for all solutions
/// (`Solver::all_solutions`) records it in its `Collector` and goes on.
/// (A type parameter of the search, passed by value: `One` is no data, so
/// the search for one solution has the code and the state it has without
/// the other.)
trait Found: Copy {
    /// Whether solutions are recorded.
    const ALL: bool;
    /// Record the solution of the path in `state`. Returns whether the
    /// search goes on (false: there are more solutions than asked for).
    fn record(self, state: &SearchState) -> bool;
}

/// See `Found`.
#[derive(Clone, Copy)]
struct One;

impl Found for One {
    const ALL: bool = false;
    #[inline(always)]
    fn record(self, _: &SearchState) -> bool {
        false
    }
}

impl Found for &RefCell<Collector> {
    const ALL: bool = true;
    fn record(self, state: &SearchState) -> bool {
        let mut solution = state.moves[..state.n_moves].to_vec();
        let premoves = state.premoves[..state.n_premoves].iter().rev();
        solution.extend(premoves.map(|&mv| inverse_move(mv)));
        // (Borrowed only here: never twice.)
        self.try_borrow_mut()
            .is_ok_and(|mut collector| collector.offer(&solution))
    }
}

/// Half turns are left out on one side when the other side's bound is
/// within this many moves of the moves left (`bound + slack >= remaining`):
/// a half turn keeps the other side's coset, hence its bound, and costs
/// one move (HTM: slack 0, the bound must be below `remaining`) or two
/// (QTM: slack 1, at most `remaining - 2`).
#[inline(always)]
pub(crate) const fn half_turn_slack(qtm: bool) -> u8 {
    qtm as u8
}

/// Lower bounds learned for the children of a node, for sibling pruning
/// and the node's own bound. HTM keeps the largest bound per face
/// (`bound[0..6]`): the true distance changes by at most one per move, and
/// same-face siblings are one move apart. QTM keeps one per move: X and
/// X2, and X2 and X', are one quarter turn apart, X and X' two, and a
/// child's bound `v` bounds the node by `v - cost`.
struct Learned<const QTM: bool> {
    bound: [u8; 18],
}

impl<const QTM: bool> Learned<QTM> {
    #[inline(always)]
    fn new() -> Self {
        Learned { bound: [0; 18] }
    }

    /// Note a lower bound `value` for the child of move `mv`.
    #[inline(always)]
    fn note(&mut self, mv: u8, value: u8) {
        let slot = if QTM {
            usize::from(mv)
        } else {
            usize::from(mv / 3)
        };
        self.bound[slot] = self.bound[slot].max(value);
    }

    /// Whether the child of move `mv`, `remaining` moves short of the
    /// target after it, is pruned by a bound noted for it or a same-face
    /// sibling.
    #[inline(always)]
    fn pruned(&self, mv: u8, remaining: u8) -> bool {
        if QTM {
            // Sibling `s` bounds this child by `bound[s] - distance(s, mv)`.
            let first = usize::from(3 * (mv / 3));
            let kind = mv % 3;
            (0..3u8).any(|s| {
                let distance = s.abs_diff(kind);
                self.bound[first + usize::from(s)] > remaining + distance
            })
        } else {
            self.bound[usize::from(mv / 3)] > remaining + 1
        }
    }

    /// The node's bound from the children's: the largest `bound - cost`.
    #[inline(always)]
    fn node_bound(&self) -> u8 {
        if QTM {
            (0..18u8)
                .map(|mv| self.bound[usize::from(mv)].saturating_sub(move_cost(true, mv)))
                .max()
                .unwrap_or(0)
        } else {
            self.bound[..6]
                .iter()
                .copied()
                .max()
                .unwrap_or(0)
                .saturating_sub(1)
        }
    }
}

impl<'a> Solver<'a> {
    /// Corner table borrowed for the lifetime of the cached pointer.
    #[must_use]
    pub fn corners(&self) -> &'a Cocsep {
        self.corners
    }

    /// Pruning table borrowed for the lifetime of the cached pointer.
    #[must_use]
    pub fn table(&self) -> &'a H48Table {
        self.table
    }

    /// A solver with the tables' metric: QTM if the table or the corner
    /// table holds QTM distances (which are not lower bounds in HTM), else
    /// HTM. (`with_metric` chooses it.)
    #[must_use]
    pub fn new(corners: &'a Cocsep, table: &'a H48Table) -> Solver<'a> {
        let metric = if table.meta.metric.is_qtm() || corners.metric().is_qtm() {
            Metric::Qtm
        } else {
            Metric::Htm
        };
        Solver::build(corners, table, metric)
    }

    /// A solver of `metric`: QTM on HTM or QTM tables, HTM on HTM tables.
    ///
    /// # Errors
    /// `TableError::WrongMetric` for an HTM search on a QTM table or corner
    /// table (whose values are not lower bounds on HTM distances).
    pub fn with_metric(
        corners: &'a Cocsep,
        table: &'a H48Table,
        metric: Metric,
    ) -> Result<Solver<'a>, TableError> {
        for of in [table.meta.metric, corners.metric()] {
            if !metric.admits(of) {
                return Err(TableError::WrongMetric {
                    search: metric,
                    table: of,
                });
            }
        }
        Ok(Solver::build(corners, table, metric))
    }

    fn build(corners: &'a Cocsep, table: &'a H48Table, metric: Metric) -> Solver<'a> {
        let k = table.meta.k;
        let base = table.meta.base;
        Solver {
            corners,
            table,
            table_ptr: table.ptr(),
            packed_ptr: corners.packed_ptr(),
            row_mul: 1 << (k + 9),
            eo_mask: u32::MAX.unbounded_shr(32 - k) << (28 - k),
            base,
            max_value: match metric {
                Metric::Htm => base + 3,
                Metric::Qtm => (base + 3).max(corners.max_depth()),
            },
            metric,
            sym_start: true,
        }
    }

    /// The metric of the search.
    #[must_use]
    pub fn metric(&self) -> Metric {
        self.metric
    }

    /// The job filter of a cube (`symstart`), or none if `sym_start` is off.
    /// (In QTM without `dedup`: jobs of one length can differ in cost there,
    /// and `symstart`'s argument for it assumes they do not.)
    #[must_use]
    pub fn sym_filter(&self, cube: Cube, inverse: Cube) -> SymFilter {
        if !self.sym_start {
            SymFilter::NONE
        } else if self.metric.is_qtm() {
            SymFilter::new(cube, inverse).without_dedup()
        } else {
            SymFilter::new(cube, inverse)
        }
    }

    /// The first target of a search: at least the root's bounds, at least
    /// 1 (the cube is not solved), and in QTM of the parity of its corner
    /// permutation (`metric`).
    #[must_use]
    pub(crate) fn first_target(&self, cube: Cube, normal_bound: u8, inverse_bound: u8) -> u8 {
        let target = normal_bound.max(inverse_bound).max(1);
        if self.metric.is_qtm() {
            #[expect(clippy::cast_possible_truncation, reason = "a parity, 0 or 1")]
            let parity = cube.to_cubie().corner_parity() as u8;
            target + ((target ^ parity) & 1)
        } else {
            target
        }
    }

    /// The step between targets: 1, in QTM 2 (`first_target`).
    #[must_use]
    pub(crate) fn target_step(&self) -> u8 {
        if self.metric.is_qtm() {
            2
        } else {
            1
        }
    }

    /// The last target tried (God's number of the metric).
    #[must_use]
    pub(crate) fn max_target(&self) -> u8 {
        self.metric.max_target()
    }

    /// (table index, corner lower bound) of a cube.
    #[inline(always)]
    pub(crate) fn index(&self, cube: Cube) -> (u64, u8) {
        // `self.corners.packed(cocsep(cube))`
        // SAFETY: `cocsep_x3` is 3 * a coordinate below `N_COCSEP`, and the
        // packed table has 3 * N_COCSEP + 1 bytes (4-byte reads).
        let packed = unsafe {
            self.packed_ptr
                .add(cocsep_x3(cube))
                .cast::<u32>()
                .read_unaligned()
        };
        let index = index_sym(
            cube,
            packed_sym_to_rep(packed),
            packed_class(packed),
            self.row_mul,
            self.eo_mask,
        );
        (index, packed_depth(packed))
    }

    /// The lower bound of table entry `index`: `base + code`, or for code 0
    /// (at most `base`) the larger of the group minimum and the corner bound.
    #[inline(always)]
    #[expect(clippy::cast_ptr_alignment, reason = "tables are page aligned")]
    pub(crate) fn value(&self, index: u64, corner_bound: u8) -> u8 {
        // `H48Table::get` through the cached pointer.
        // SAFETY: indexes are of the table (`index`), whose words are all
        // in its memory.
        let (code, group_min) = unsafe {
            let words = self.table_ptr.cast::<u64>();
            let w = (index >> 5) as usize;
            let word = *words.add(w);
            let group_min = *words.add(w & !1) & 15;
            (((word >> ((index & 31) * 2)) & 3) as u8, group_min as u8)
        };
        if code == 0 {
            group_min.max(corner_bound)
        } else {
            self.base + code
        }
    }

    /// Prefetch table entry `index` (`H48Table::prefetch`).
    #[inline(always)]
    pub(crate) fn prefetch(&self, index: u64) {
        // NTA: each line is read once; less L2 pollution (cocsep data).
        // (+2.6% at 48T when added, prefetch-nta.)
        // SAFETY: an entry of the table (indices are below its size).
        unsafe { crate::prefetch::read_once(self.table_ptr.wrapping_add((index >> 2) as usize)) };
    }

    /// Lower bound for a single cube (no pipelining).
    #[must_use]
    pub fn lower_bound(&self, cube: Cube) -> u8 {
        let (index, corner_bound) = self.index(cube);
        self.value(index, corner_bound)
    }

    /// Length of the job prefixes for an iteration with target `target`:
    /// longer jobs for deeper targets (more of them to sort), up to 4. (In
    /// QTM also below `target / 2`: a job of half turns then leaves moves
    /// to search.)
    pub(crate) fn job_len(&self, target: u8) -> usize {
        let top = self.base + 3;
        let len = if target > top {
            ((target - top) as usize).min(4)
        } else {
            0
        };
        if self.metric.is_qtm() {
            len.min(usize::from(target.saturating_sub(1) / 2))
        } else {
            len
        }
    }

    /// Extend jobs by one move; children inherit the parent's stat. Jobs
    /// that `filter` drops (symmetric start) are left out.
    fn extend_jobs<const QTM: bool>(jobs: &[Job], filter: &SymFilter) -> Vec<Job> {
        let mut out = Vec::with_capacity(jobs.len() * 15);
        for j in jobs {
            let allowed = if j.len == 0 {
                ALL_MOVES
            } else {
                ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
            };
            let mut todo = allowed;
            while let Some(mv) = Move::lowest(todo) {
                todo ^= todo.isolate_lowest_one();
                let mut child = *j;
                child.moves[child.len as usize] = mv as u8;
                child.len += 1;
                child.cost += move_cost(QTM, mv as u8);
                if !filter.keep(&child.moves[..usize::from(child.len)]) {
                    continue;
                }
                child.cube = j.cube.apply_move(mv);
                child.inverse = j.inverse.premove_inverse(mv);
                out.push(child);
            }
        }
        filter.dedup(
            &mut out,
            |job| &job.moves[..usize::from(job.len)],
            |job| job.cube,
        );
        out
    }

    /// Compute the root bounds of jobs, prefetching in batches (+2.4%
    /// throughput, +1.8% on a 48T batch when added, job-bounds).
    fn job_bounds(&self, jobs: &mut [Job]) {
        for chunk in jobs.chunks_mut(32) {
            let mut lookups = [(0u64, 0u8); 64];
            for (n, j) in chunk.iter().enumerate() {
                lookups[2 * n] = self.index(j.cube);
                lookups[2 * n + 1] = self.index(j.inverse);
                self.prefetch(lookups[2 * n].0);
                self.prefetch(lookups[2 * n + 1].0);
            }
            for (n, j) in chunk.iter_mut().enumerate() {
                j.normal_bound = self.value(lookups[2 * n].0, lookups[2 * n].1);
                j.inverse_bound = self.value(lookups[2 * n + 1].0, lookups[2 * n + 1].1);
            }
        }
    }

    /// Find one optimal solution: `None` if there is none within
    /// `MAX_TARGET` moves (`MAX_TARGET_QTM` in QTM; an unsolvable cube, or
    /// a table whose bounds are too high). A QTM solution is in canonical
    /// form (`metric::simplify`).
    #[must_use]
    pub fn solve(&self, cube: &CubieCube) -> (Option<Vec<u8>>, Stats) {
        if self.metric.is_qtm() {
            let (solution, stats) = self.solve_metric::<true>(cube);
            (solution.map(|moves| crate::metric::simplify(&moves)), stats)
        } else {
            self.solve_metric::<false>(cube)
        }
    }

    /// `solve` in QTM if `QTM`, else in HTM.
    fn solve_metric<const QTM: bool>(&self, cube: &CubieCube) -> (Option<Vec<u8>>, Stats) {
        let cube = Cube::from_cubie(cube);
        if cube.is_solved() {
            return (Some(vec![]), Stats::default());
        }
        let inverse = cube.inverse();
        let normal_bound = self.lower_bound(cube);
        let inverse_bound = self.lower_bound(inverse);
        let filter = self.sym_filter(cube, inverse);
        let mut state = SearchState {
            moves: [0; 32],
            n_moves: 0,
            premoves: [0; 32],
            n_premoves: 0,
            stats: Stats::default(),
            stat: 0,
        };
        let mut jobs = vec![Job {
            moves: [0; 4],
            len: 0,
            cost: 0,
            cube,
            inverse,
            stat: 0,
            normal_bound,
            inverse_bound,
        }];
        let mut target = self.first_target(cube, normal_bound, inverse_bound);
        let slack = half_turn_slack(QTM);
        loop {
            // (Also keeps the moves within `SearchState::moves`.)
            if target > self.max_target() {
                return (None, state.stats);
            }
            let job_len = self.job_len(target);
            while jobs.first().is_some_and(|j| usize::from(j.len) < job_len) {
                jobs = Self::extend_jobs::<QTM>(&jobs, &filter);
                self.job_bounds(&mut jobs);
            }
            if job_len > 0 {
                // stable sort keeps the canonical order among equal keys
                jobs.sort_by_key(|j| std::cmp::Reverse(j.stat * job_weight(j)));
            }
            for j in &mut jobs {
                // (At least 1: `job_len`.)
                let remaining_depth = target - j.cost;
                let (normal_bound, inverse_bound) = (j.normal_bound, j.inverse_bound);
                state.stat = 0;
                if normal_bound > remaining_depth
                    || inverse_bound > remaining_depth
                    || (j.len > 0 && j.cube.is_solved())
                {
                    j.stat = 0;
                    continue;
                }
                let mut normal_moves = if j.len == 0 {
                    ALL_MOVES
                } else {
                    ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
                };
                let mut inverse_moves = ALL_MOVES;
                // A tight bound on one side: a half turn on the other side
                // keeps it (see `search`), so only quarter turns can help.
                if inverse_bound + slack >= remaining_depth {
                    normal_moves &= QUARTER_TURNS;
                }
                if normal_bound + slack >= remaining_depth {
                    inverse_moves &= QUARTER_TURNS;
                }
                for (to, &mv) in state
                    .moves
                    .iter_mut()
                    .zip(&j.moves)
                    .take(usize::from(j.len))
                {
                    *to = mv;
                }
                state.n_moves = j.len as usize;
                state.n_premoves = 0;
                let found = self.search::<QTM, One>(
                    &mut state,
                    j.cube,
                    j.inverse,
                    normal_bound,
                    inverse_bound,
                    normal_moves,
                    inverse_moves,
                    remaining_depth,
                    One,
                ) == FOUND;
                j.stat = state.stat;
                if found {
                    let mut solution: Vec<u8> = state.moves[..state.n_moves].to_vec();
                    for k in (0..state.n_premoves).rev() {
                        solution.push(inverse_move(state.premoves[k]));
                    }
                    return (Some(solution), state.stats);
                }
            }
            target += self.target_step();
        }
    }

    /// Every distinct solution of `cube` within the limits of `options`
    /// (`crate::solutions`: all optimal ones, or up to some moves more),
    /// in the solver's metric, each in canonical form. An unsolvable cube
    /// (or a table whose bounds are too high) has none.
    ///
    /// The search of `solve`, but a solution is recorded and the search
    /// goes on, through every target up to the limit, on the jobs in any
    /// order. Of the symmetric-start filter only `keep` is used, and each
    /// solution is recorded with its images under the cube's symmetries
    /// (exact: `crate::solutions`). Pruning by bounds is unchanged.
    /// `Options::time_limit` and `Options::cancel` are looked at between
    /// jobs.
    #[must_use]
    pub fn all_solutions(&self, cube: &CubieCube, options: &Options) -> (Solutions, Stats) {
        if self.metric.is_qtm() {
            self.all_metric::<true>(cube, options)
        } else {
            self.all_metric::<false>(cube, options)
        }
    }

    /// `all_solutions` in QTM if `QTM`, else in HTM.
    fn all_metric<const QTM: bool>(
        &self,
        cube: &CubieCube,
        options: &Options,
    ) -> (Solutions, Stats) {
        let cube = Cube::from_cubie(cube);
        if cube.is_solved() {
            return (Collector::solved(self.metric, options), Stats::default());
        }
        let inverse = cube.inverse();
        let normal_bound = self.lower_bound(cube);
        let inverse_bound = self.lower_bound(inverse);
        let started = std::time::Instant::now();
        // (Only `keep`: `dedup` drops jobs whose solutions no symmetry
        // gives back.)
        let filter = self.sym_filter(cube, inverse).without_dedup();
        let collector = RefCell::new(Collector::new(self.metric, options, filter));
        let limit = || {
            collector
                .try_borrow()
                .map_or(0, |collector| collector.limit())
        };
        let mut state = SearchState {
            moves: [0; 32],
            n_moves: 0,
            premoves: [0; 32],
            n_premoves: 0,
            stats: Stats::default(),
            stat: 0,
        };
        let mut jobs = vec![Job {
            moves: [0; 4],
            len: 0,
            cost: 0,
            cube,
            inverse,
            stat: 0,
            normal_bound,
            inverse_bound,
        }];
        let mut target = self.first_target(cube, normal_bound, inverse_bound);
        if let Ok(mut collector) = collector.try_borrow_mut() {
            collector.start(target);
        }
        // Whether to give up: the cancel flag or the clock (the list is
        // then marked stopped).
        let give_up = || {
            let reason = if options.cancelled() {
                Stopped::Cancelled
            } else if options
                .time_limit
                .is_some_and(|limit| started.elapsed() >= limit)
            {
                Stopped::Time
            } else {
                return false;
            };
            collector
                .try_borrow_mut()
                .is_ok_and(|mut collector| collector.stop(reason))
        };
        // (The limit is at most `max_target`, which keeps the moves within
        // `SearchState::moves`.)
        while target <= limit() {
            let job_len = self.job_len(target);
            while jobs.first().is_some_and(|j| usize::from(j.len) < job_len) {
                jobs = self.extend_all_jobs::<QTM>(&jobs, &filter);
            }
            let mut stopped = false;
            for j in &jobs {
                if give_up() || self.search_job_all::<QTM>(&mut state, j, target, &collector) {
                    stopped = true;
                    break;
                }
            }
            // (A stopped iteration is not complete.)
            let go_on = !stopped
                && collector.try_borrow_mut().is_ok_and(|mut collector| {
                    collector.iteration_done(target, self.target_step())
                });
            if !go_on {
                break;
            }
            target += self.target_step();
        }
        (collector.into_inner().finish(), state.stats)
    }

    /// (All solutions.) Search job `j` for the solutions of `target` moves,
    /// recording them in `collector`: the job's root as `solve_metric` sets
    /// it up. Returns whether the search is to stop (more solutions than
    /// asked for).
    fn search_job_all<const QTM: bool>(
        &self,
        state: &mut SearchState,
        j: &Job,
        target: u8,
        collector: &RefCell<Collector>,
    ) -> bool {
        let slack = half_turn_slack(QTM);
        let remaining_depth = target - j.cost;
        let (normal_bound, inverse_bound) = (j.normal_bound, j.inverse_bound);
        // (A job that solves the cube: every sequence that starts with it
        // has a detour, see `crate::solutions`.)
        if normal_bound > remaining_depth
            || inverse_bound > remaining_depth
            || (j.len > 0 && j.cube.is_solved())
        {
            return false;
        }
        let mut normal_moves = if j.len == 0 {
            ALL_MOVES
        } else {
            ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
        };
        let mut inverse_moves = ALL_MOVES;
        if inverse_bound + slack >= remaining_depth {
            normal_moves &= QUARTER_TURNS;
        }
        if normal_bound + slack >= remaining_depth {
            inverse_moves &= QUARTER_TURNS;
        }
        for (to, &mv) in state
            .moves
            .iter_mut()
            .zip(&j.moves)
            .take(usize::from(j.len))
        {
            *to = mv;
        }
        state.n_moves = j.len as usize;
        state.n_premoves = 0;
        self.search::<QTM, &RefCell<Collector>>(
            state,
            j.cube,
            j.inverse,
            normal_bound,
            inverse_bound,
            normal_moves,
            inverse_moves,
            remaining_depth,
            collector,
        ) == FOUND
    }

    /// (All solutions.) Extend jobs by one move, every child that `filter`
    /// keeps (`SymFilter::keep`) with its bounds: `extend_jobs` and
    /// `job_bounds` without `dedup`. (Apart from them: they stay the
    /// one-solution search's own, compiled as before.)
    fn extend_all_jobs<const QTM: bool>(&self, jobs: &[Job], filter: &SymFilter) -> Vec<Job> {
        let mut out = Vec::with_capacity(jobs.len() * 15);
        for j in jobs {
            let mut todo = if j.len == 0 {
                ALL_MOVES
            } else {
                ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
            };
            while let Some(mv) = Move::lowest(todo) {
                todo ^= todo.isolate_lowest_one();
                let mut child = *j;
                child.moves[child.len as usize] = mv as u8;
                child.len += 1;
                child.cost += move_cost(QTM, mv as u8);
                if !filter.keep(&child.moves[..usize::from(child.len)]) {
                    continue;
                }
                child.cube = j.cube.apply_move(mv);
                child.inverse = j.inverse.premove_inverse(mv);
                child.normal_bound = self.lower_bound(child.cube);
                child.inverse_bound = self.lower_bound(child.inverse);
                out.push(child);
            }
        }
        out
    }

    /// Search for a solution of exactly `remaining_depth` (> 0) more moves.
    /// The bounds `normal_bound`, `inverse_bound` are at most
    /// `remaining_depth`; `normal_moves`, `inverse_moves` are the moves
    /// allowed on each side.
    ///
    /// Returns `FOUND`, or a lower bound on the distance of this node learned
    /// from its children (0 if nothing was learned). Every child value is a
    /// lower bound on the child's true distance, and the true distance
    /// changes by at most one per move, so (a) a child value `v` bounds this
    /// node by `v - 1` and (b) bounds same-face siblings (one move apart) by
    /// `v - 1` as well. (Both: -17.7% nodes, +7.5% throughput when added,
    /// sibling-prune.) In QTM a half turn is two moves (`Learned`).
    ///
    /// In a search for all solutions (`found`, see `Found`) a solution is
    /// recorded and the search goes on; `FOUND` then means that it is to
    /// stop. A node that is solved with moves left is not searched in
    /// either case: there because every sequence through it has a detour
    /// (see `crate::solutions`).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn search<const QTM: bool, F: Found>(
        &self,
        state: &mut SearchState,
        cube: Cube,
        inverse: Cube,
        normal_bound: u8,
        inverse_bound: u8,
        normal_moves: u32,
        inverse_moves: u32,
        remaining_depth: u8,
        found: F,
    ) -> u8 {
        if remaining_depth == 1 {
            // (In QTM only quarter turns are left: half turns are masked
            // with one move left, `half_turn_slack`.)
            return Self::last_move(state, cube, normal_moves & inverse_moves, found);
        }
        let slack = half_turn_slack(QTM);
        // Move the side with fewer moves to try (ties: the normal cube; as
        // the other search techniques of the first version, not measured
        // separately).
        let normal = normal_moves.count_ones() <= inverse_moves.count_ones();
        let (moved, other, allowed, other_side_bound) = if normal {
            (cube, inverse, normal_moves, inverse_bound)
        } else {
            (inverse, cube, inverse_moves, normal_bound)
        };
        // Best known lower bounds of the children, per face (HTM) or move.
        let mut learned = Learned::<QTM>::new();
        let (mut children, n_children) = Self::generate_children::<QTM>(
            state,
            (moved, other),
            allowed,
            other_side_bound,
            remaining_depth,
            &mut learned,
        );
        let children = &mut children[..n_children];
        self.bound_children(state, children, &mut learned);
        // Lower bound for this node from all children.
        let mut lower_bound = learned.node_bound();
        if lower_bound > remaining_depth {
            return lower_bound;
        }
        for child in &*children {
            let face = (child.mv / 3) as usize;
            let child_remaining = child.remaining;
            // Pruned directly, or by a same-face sibling that is >= 2 over.
            if !child.live || learned.pruned(child.mv, child_remaining) {
                continue;
            }
            if child_remaining == 0 {
                // (QTM: a half turn with two moves left.) Solved: found.
                if child.moved.is_solved() {
                    if normal {
                        state.moves[state.n_moves] = child.mv;
                        state.n_moves += 1;
                    } else {
                        state.premoves[state.n_premoves] = child.mv;
                        state.n_premoves += 1;
                    }
                    if F::ALL {
                        let go_on = found.record(state);
                        if normal {
                            state.n_moves -= 1;
                        } else {
                            state.n_premoves -= 1;
                        }
                        if go_on {
                            continue;
                        }
                    }
                    return FOUND;
                }
                continue;
            }
            // Solved before the target depth: a shorter solution exists.
            if child.moved.is_solved() {
                continue;
            }
            // The child's two sides, back in normal/inverse terms.
            let (mut child_normal_moves, mut child_inverse_moves, child_cube, child_inverse);
            let (child_normal_bound, child_inverse_bound);
            if normal {
                child_normal_moves = ALLOWED_AFTER[face];
                child_inverse_moves = inverse_moves;
                child_cube = child.moved;
                child_inverse = child.other;
                child_normal_bound = child.moved_bound;
                child_inverse_bound = child.other_bound;
                state.moves[state.n_moves] = child.mv;
                state.n_moves += 1;
            } else {
                child_normal_moves = normal_moves;
                child_inverse_moves = ALLOWED_AFTER[face];
                child_cube = child.other;
                child_inverse = child.moved;
                child_normal_bound = child.other_bound;
                child_inverse_bound = child.moved_bound;
                state.premoves[state.n_premoves] = child.mv;
                state.n_premoves += 1;
            }
            // A tight bound on one side: a half turn on the other side keeps
            // that side's coset, hence its bound, so it cannot finish in time.
            if child_inverse_bound + slack >= child_remaining {
                child_normal_moves &= QUARTER_TURNS;
            }
            if child_normal_bound + slack >= child_remaining {
                child_inverse_moves &= QUARTER_TURNS;
            }
            let result = self.search::<QTM, F>(
                state,
                child_cube,
                child_inverse,
                child_normal_bound,
                child_inverse_bound,
                child_normal_moves,
                child_inverse_moves,
                child_remaining,
                found,
            );
            if result == FOUND {
                return FOUND;
            }
            if normal {
                state.n_moves -= 1;
            } else {
                state.n_premoves -= 1;
            }
            // Bound learned for the child: prunes its same-face siblings and
            // possibly this whole node.
            learned.note(child.mv, result);
            lower_bound = lower_bound.max(result.saturating_sub(move_cost(QTM, child.mv)));
            if lower_bound > remaining_depth {
                return lower_bound;
            }
        }
        lower_bound
    }

    /// The last move of a search: is one of `moves` (allowed on both sides)
    /// a solution? (Nodes with one move left always move the normal cube.)
    /// (All solutions: each one that is, recorded; see `search`.)
    fn last_move<F: Found>(state: &mut SearchState, cube: Cube, moves: u32, found: F) -> u8 {
        let mut todo = moves;
        while let Some(mv) = Move::lowest(todo) {
            todo ^= todo.isolate_lowest_one();
            state.stats.nodes += 1;
            if cube.apply_move(mv).is_solved() {
                state.moves[state.n_moves] = mv as u8;
                state.n_moves += 1;
                if F::ALL {
                    let go_on = found.record(state);
                    state.n_moves -= 1;
                    if go_on {
                        continue;
                    }
                }
                return FOUND;
            }
        }
        0
    }

    /// Generate the children of a node: the `allowed` moves applied to
    /// `moved` (the node's cube of the chosen side), premoved on `other`.
    /// Returns them and their number.
    ///
    /// A half turn on this side does not change the other side's coset
    /// (every half turn lies in the subgroup whose cosets the table
    /// indexes), so its child keeps the node's other-side bound: children
    /// whose bound is already too high are not generated.
    #[inline(always)]
    fn generate_children<const QTM: bool>(
        state: &mut SearchState,
        (moved, other): (Cube, Cube),
        allowed: u32,
        other_side_bound: u8,
        remaining_depth: u8,
        learned: &mut Learned<QTM>,
    ) -> ([Child; 18], usize) {
        let mut children = [Child {
            moved,
            other,
            index: 0,
            mv: 0,
            remaining: 0,
            moved_bound: 0,
            other_bound: 0,
            live: false,
        }; 18];
        let mut n_children = 0;
        let mut todo = allowed;
        while let Some(mv) = Move::lowest(todo) {
            todo ^= todo.isolate_lowest_one();
            let child_remaining = remaining_depth - move_cost(QTM, mv as u8);
            let other_bound = if mv.index() % 3 == 1 {
                other_side_bound
            } else {
                UNKNOWN
            };
            if other_bound != UNKNOWN && other_bound > child_remaining {
                learned.note(mv as u8, other_bound);
                continue;
            }
            state.stats.nodes += 1;
            children[n_children] = Child {
                moved: moved.apply_move(mv),
                other: other.premove_inverse(mv),
                index: 0,
                mv: mv as u8,
                remaining: child_remaining,
                moved_bound: 0,
                other_bound,
                live: true,
            };
            n_children += 1;
        }
        (children, n_children)
    }

    /// Look up the bounds of the children, pruning the ones that exceed
    /// their moves left (`Child::remaining`) and noting each one's bound.
    /// The lookups are pipelined: all indices are computed and prefetched
    /// before the first value is read (other side first, then the moved
    /// side). Children with more moves left than any value (`max_value`)
    /// or none left are not looked up (bounds 0, or the other side's bound
    /// of a half turn).
    #[inline(always)]
    fn bound_children<const QTM: bool>(
        &self,
        state: &mut SearchState,
        children: &mut [Child],
        learned: &mut Learned<QTM>,
    ) {
        let max_value = self.max_value;
        // Stage 1: other-side index + prefetch.
        for child in &mut *children {
            let child_remaining = child.remaining;
            if child_remaining > max_value || child_remaining == 0 {
                // No table value can prune (or a QTM half turn's leaf):
                // skip the lookups.
                child.moved_bound = 0;
                if child.other_bound == UNKNOWN {
                    child.other_bound = 0;
                }
                child.index = u64::MAX;
                continue;
            }
            if child.other_bound == UNKNOWN {
                let (index, corner_bound) = self.index(child.other);
                if corner_bound > child_remaining {
                    child.live = false;
                    learned.note(child.mv, corner_bound);
                    continue;
                }
                child.index = index;
                child.other_bound = corner_bound; // temporarily the corner bound
                self.prefetch(index);
                child.moved_bound = UNKNOWN; // marks "other_bound pending"
            }
        }
        // Stage 2: other-side value, moved-side index + prefetch.
        for child in &mut *children {
            if !child.live || child.index == u64::MAX {
                continue;
            }
            let child_remaining = child.remaining;
            if child.moved_bound == UNKNOWN {
                state.stats.lookups += 1;
                child.other_bound = self.value(child.index, child.other_bound);
                if child.other_bound > child_remaining {
                    child.live = false;
                    learned.note(child.mv, child.other_bound);
                    continue;
                }
            }
            let (index, corner_bound) = self.index(child.moved);
            if corner_bound > child_remaining {
                child.live = false;
                learned.note(child.mv, corner_bound);
                continue;
            }
            child.index = index;
            child.moved_bound = corner_bound;
            self.prefetch(index);
        }
        // Stage 3: moved-side value.
        for child in &mut *children {
            if !child.live || child.index == u64::MAX {
                continue;
            }
            state.stats.lookups += 1;
            child.moved_bound = self.value(child.index, child.moved_bound);
            state.stat +=
                (4096u64 >> child.other_bound.min(63)) + (4096u64 >> child.moved_bound.min(63));
            learned.note(child.mv, child.moved_bound.max(child.other_bound));
            if child.moved_bound > child.remaining {
                child.live = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::cocsep;
    use crate::cubie::{conjugate, parse_moves, symmetries, Rng};
    use crate::metric::Metric;
    use crate::tables::{entry_depth, H48Meta, Memory};

    /// The hot-path index (`Solver::index`: `cocsep_x3`, packed entries,
    /// `index_sym`) equals `Cocsep::index` for every `k`, symmetric cubes too.
    #[test]
    #[cfg_attr(miri, ignore = "Cocsep::generate is too slow under Miri")]
    fn hot_index_matches_reference_index() {
        let corners = Cocsep::generate().unwrap();
        let mut rng = Rng(31);
        let mut cubes = vec![CubieCube::SOLVED];
        for scramble in [
            "U2 D2 R2 L2 F2 B2",
            "U D' R L' F B' U D'",
            "U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2",
        ] {
            cubes.push(
                CubieCube::SOLVED
                    .apply_moves(&parse_moves(scramble).unwrap())
                    .unwrap(),
            );
        }
        for _ in 0..40 {
            cubes.push(CubieCube::random(&mut rng));
        }
        // All conjugates and inverse conjugates (the index picks a symmetry).
        let variants: Vec<CubieCube> = cubes
            .iter()
            .flat_map(|c| [*c, c.inverse()])
            .flat_map(|c| symmetries().iter().map(move |s| conjugate(&c, s)))
            .collect();
        for k in [0, 7, 8, 11] {
            // (`index` does not read the table: a page stands in for it, so
            // that this also runs where h11 cannot be allocated, e.g. wasm32.)
            let table = H48Table {
                memory: Memory::anonymous(4096).unwrap(),
                meta: H48Meta {
                    k,
                    base: 10,
                    metric: Metric::Htm,
                },
            };
            let solver = Solver::new(&corners, &table);
            for c in &variants {
                let cube = Cube::from_cubie(c);
                let (index, corner_bound) = solver.index(cube);
                assert_eq!(index, corners.index(cube, k), "k {k}: {c:?}");
                assert_eq!(corner_bound, entry_depth(corners.entries[cocsep(cube)]));
            }
        }
    }
}
