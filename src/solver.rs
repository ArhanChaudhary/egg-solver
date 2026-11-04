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
//! so that the last iteration tends to find its solution early.

use crate::coord::{cocsep_x3, index_sym};
use crate::cubie::{inverse_move, CubieCube};
use crate::simd::Cube;
use crate::tables::{packed_class, packed_depth, packed_sym_to_rep, Cocsep, H48Table};

/// All 18 moves (the low 18 bits).
pub const ALL_MOVES: u32 = u32::MAX.unbounded_shr(32 - 18);
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
pub struct Solver<'a> {
    pub corners: &'a Cocsep,
    pub table: &'a H48Table,
    /// `table.ptr()`, cached (saves a dependent load per prefetch/lookup).
    table_ptr: *const u8,
    /// `corners.packed_ptr()`, cached (saves a dependent load per index).
    packed_ptr: *const u8,
    /// `coord::index_sym` constants of the table's `k`.
    row_mul: u64,
    eo_mask: u32,
    /// The table's base: codes 1..=3 mean `base + code`.
    pub(crate) base: u8,
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
    /// Lower bounds of the moved and the other side.
    moved_bound: u8,
    other_bound: u8,
    /// Not pruned (yet).
    live: bool,
}

/// A bound not known yet.
const UNKNOWN: u8 = 0xFF;
/// `search`'s result when it found a solution.
const FOUND: u8 = 0xFE;

impl<'a> Solver<'a> {
    #[must_use]
    pub fn new(corners: &'a Cocsep, table: &'a H48Table) -> Solver<'a> {
        let k = table.meta.k;
        Solver {
            corners,
            table,
            table_ptr: table.ptr(),
            packed_ptr: corners.packed_ptr(),
            row_mul: 1 << (k + 9),
            eo_mask: u32::MAX.unbounded_shr(32 - k) << (28 - k),
            base: table.meta.base,
        }
    }

    /// (table index, corner lower bound) of a cube.
    #[inline(always)]
    pub(crate) fn index(&self, cube: Cube) -> (u64, u8) {
        // `self.corners.packed(cocsep(cube))`
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
        unsafe {
            let line = self.table_ptr.add((index >> 2) as usize);
            // NTA: each line is read once; less L2 pollution (cocsep data).
            std::arch::x86_64::_mm_prefetch(line.cast::<i8>(), std::arch::x86_64::_MM_HINT_NTA);
        }
    }

    /// Lower bound for a single cube (no pipelining).
    #[must_use]
    pub fn lower_bound(&self, cube: Cube) -> u8 {
        let (index, corner_bound) = self.index(cube);
        self.value(index, corner_bound)
    }

    /// Length of the job prefixes for an iteration with target `target`:
    /// longer jobs for deeper targets (more of them to sort), up to 4.
    pub(crate) fn job_len(&self, target: u8) -> usize {
        let top = self.base + 3;
        if target > top {
            ((target - top) as usize).min(4)
        } else {
            0
        }
    }

    /// Extend jobs by one move; children inherit the parent's stat.
    #[expect(clippy::cast_possible_truncation, reason = "moves are below 18")]
    fn extend_jobs(jobs: &[Job]) -> Vec<Job> {
        let mut out = Vec::with_capacity(jobs.len() * 15);
        for j in jobs {
            let allowed = if j.len == 0 {
                ALL_MOVES
            } else {
                ALLOWED_AFTER[(j.moves[j.len as usize - 1] / 3) as usize]
            };
            let mut todo = allowed;
            while todo != 0 {
                let mv = todo.trailing_zeros() as usize;
                todo ^= todo.isolate_lowest_one();
                let mut child = *j;
                child.moves[child.len as usize] = mv as u8;
                child.len += 1;
                child.cube = j.cube.apply_move(mv);
                child.inverse = j.inverse.premove_inverse(mv);
                out.push(child);
            }
        }
        out
    }

    /// Compute the root bounds of jobs, prefetching in batches.
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

    /// Find one optimal solution.
    #[must_use]
    pub fn solve(&self, cube: &CubieCube) -> (Vec<u8>, Stats) {
        let cube = Cube::from_cubie(cube);
        if cube.is_solved() {
            return (vec![], Stats::default());
        }
        let inverse = cube.inverse();
        let normal_bound = self.lower_bound(cube);
        let inverse_bound = self.lower_bound(inverse);
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
            cube,
            inverse,
            stat: 0,
            normal_bound,
            inverse_bound,
        }];
        let mut target = normal_bound.max(inverse_bound).max(1);
        loop {
            let job_len = self.job_len(target);
            while (jobs[0].len as usize) < job_len {
                jobs = Self::extend_jobs(&jobs);
                self.job_bounds(&mut jobs);
            }
            if job_len > 0 {
                // stable sort keeps the canonical order among equal keys
                jobs.sort_by_key(|j| std::cmp::Reverse(j.stat * job_weight(j)));
            }
            for j in &mut jobs {
                let remaining_depth = target - j.len;
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
                if inverse_bound == remaining_depth {
                    normal_moves &= QUARTER_TURNS;
                }
                if normal_bound == remaining_depth {
                    inverse_moves &= QUARTER_TURNS;
                }
                state.moves[..j.len as usize].copy_from_slice(&j.moves[..j.len as usize]);
                state.n_moves = j.len as usize;
                state.n_premoves = 0;
                let found = self.search(
                    &mut state,
                    j.cube,
                    j.inverse,
                    normal_bound,
                    inverse_bound,
                    normal_moves,
                    inverse_moves,
                    remaining_depth,
                ) == FOUND;
                j.stat = state.stat;
                if found {
                    let mut solution: Vec<u8> = state.moves[..state.n_moves].to_vec();
                    for k in (0..state.n_premoves).rev() {
                        solution.push(inverse_move(state.premoves[k]));
                    }
                    return (solution, state.stats);
                }
            }
            target += 1;
        }
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
    /// `v - 1` as well.
    #[allow(clippy::too_many_arguments)]
    fn search(
        &self,
        state: &mut SearchState,
        cube: Cube,
        inverse: Cube,
        normal_bound: u8,
        inverse_bound: u8,
        normal_moves: u32,
        inverse_moves: u32,
        remaining_depth: u8,
    ) -> u8 {
        if remaining_depth == 1 {
            return Self::last_move(state, cube, normal_moves & inverse_moves);
        }
        let child_remaining = remaining_depth - 1;
        // Move the side with fewer moves to try (ties: the normal cube).
        let normal = normal_moves.count_ones() <= inverse_moves.count_ones();
        let (moved, other, allowed, other_side_bound) = if normal {
            (cube, inverse, normal_moves, inverse_bound)
        } else {
            (inverse, cube, inverse_moves, normal_bound)
        };
        // Best known lower bound per face over that face's children.
        let mut face_bound = [0u8; 6];
        let (mut children, n_children) = Self::generate_children(
            state,
            (moved, other),
            allowed,
            other_side_bound,
            child_remaining,
            &mut face_bound,
        );
        let children = &mut children[..n_children];
        self.bound_children(state, children, &mut face_bound, child_remaining);
        // Lower bound for this node from all children.
        let mut lower_bound = face_bound
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .saturating_sub(1);
        if lower_bound > remaining_depth {
            return lower_bound;
        }
        for child in &*children {
            let face = (child.mv / 3) as usize;
            // Pruned directly, or by a same-face sibling that is >= 2 over.
            if !child.live || face_bound[face] > child_remaining + 1 {
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
            if child_inverse_bound == child_remaining {
                child_normal_moves &= QUARTER_TURNS;
            }
            if child_normal_bound == child_remaining {
                child_inverse_moves &= QUARTER_TURNS;
            }
            let result = self.search(
                state,
                child_cube,
                child_inverse,
                child_normal_bound,
                child_inverse_bound,
                child_normal_moves,
                child_inverse_moves,
                child_remaining,
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
            face_bound[face] = face_bound[face].max(result);
            lower_bound = lower_bound.max(result.saturating_sub(1));
            if lower_bound > remaining_depth {
                return lower_bound;
            }
        }
        lower_bound
    }

    /// The last move of a search: is one of `moves` (allowed on both sides)
    /// a solution? (Nodes with one move left always move the normal cube.)
    #[expect(clippy::cast_possible_truncation, reason = "moves are below 18")]
    fn last_move(state: &mut SearchState, cube: Cube, moves: u32) -> u8 {
        let mut todo = moves;
        while todo != 0 {
            let mv = todo.trailing_zeros() as usize;
            todo ^= todo.isolate_lowest_one();
            state.stats.nodes += 1;
            if cube.apply_move(mv).is_solved() {
                state.moves[state.n_moves] = mv as u8;
                state.n_moves += 1;
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
    #[expect(clippy::cast_possible_truncation, reason = "moves are below 18")]
    #[inline(always)]
    fn generate_children(
        state: &mut SearchState,
        (moved, other): (Cube, Cube),
        allowed: u32,
        other_side_bound: u8,
        child_remaining: u8,
        face_bound: &mut [u8; 6],
    ) -> ([Child; 18], usize) {
        let mut children = [Child {
            moved,
            other,
            index: 0,
            mv: 0,
            moved_bound: 0,
            other_bound: 0,
            live: false,
        }; 18];
        let mut n_children = 0;
        let mut todo = allowed;
        while todo != 0 {
            let mv = todo.trailing_zeros() as usize;
            todo ^= todo.isolate_lowest_one();
            let other_bound = if mv % 3 == 1 {
                other_side_bound
            } else {
                UNKNOWN
            };
            if other_bound != UNKNOWN && other_bound > child_remaining {
                face_bound[mv / 3] = face_bound[mv / 3].max(other_bound);
                continue;
            }
            state.stats.nodes += 1;
            children[n_children] = Child {
                moved: moved.apply_move(mv),
                other: other.premove_inverse(mv),
                index: 0,
                mv: mv as u8,
                moved_bound: 0,
                other_bound,
                live: true,
            };
            n_children += 1;
        }
        (children, n_children)
    }

    /// Look up the bounds of the children, pruning the ones that exceed
    /// `child_remaining` and noting each face's best bound. The lookups
    /// are pipelined: all indices are computed and prefetched before the
    /// first value is read (other side first, then the moved side).
    #[inline(always)]
    fn bound_children(
        &self,
        state: &mut SearchState,
        children: &mut [Child],
        face_bound: &mut [u8; 6],
        child_remaining: u8,
    ) {
        let max_value = self.base + 3;
        if child_remaining > max_value {
            // No table value can prune: skip the lookups.
            for child in children {
                child.moved_bound = 0;
                if child.other_bound == UNKNOWN {
                    child.other_bound = 0;
                }
            }
            return;
        }
        // Stage 1: other-side index + prefetch.
        for child in &mut *children {
            if child.other_bound == UNKNOWN {
                let (index, corner_bound) = self.index(child.other);
                if corner_bound > child_remaining {
                    child.live = false;
                    let face = (child.mv / 3) as usize;
                    face_bound[face] = face_bound[face].max(corner_bound);
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
            if !child.live {
                continue;
            }
            let face = (child.mv / 3) as usize;
            if child.moved_bound == UNKNOWN {
                state.stats.lookups += 1;
                child.other_bound = self.value(child.index, child.other_bound);
                if child.other_bound > child_remaining {
                    child.live = false;
                    face_bound[face] = face_bound[face].max(child.other_bound);
                    continue;
                }
            }
            let (index, corner_bound) = self.index(child.moved);
            if corner_bound > child_remaining {
                child.live = false;
                face_bound[face] = face_bound[face].max(corner_bound);
                continue;
            }
            child.index = index;
            child.moved_bound = corner_bound;
            self.prefetch(index);
        }
        // Stage 3: moved-side value.
        for child in &mut *children {
            if !child.live {
                continue;
            }
            state.stats.lookups += 1;
            child.moved_bound = self.value(child.index, child.moved_bound);
            state.stat +=
                (4096u64 >> child.other_bound.min(63)) + (4096u64 >> child.moved_bound.min(63));
            let face = (child.mv / 3) as usize;
            face_bound[face] = face_bound[face]
                .max(child.moved_bound)
                .max(child.other_bound);
            if child.moved_bound > child_remaining {
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
    use crate::tables::entry_depth;

    /// The hot-path index (`Solver::index`: `cocsep_x3`, packed entries,
    /// `index_sym`) equals `Cocsep::index` for every `k`, symmetric cubes too.
    #[test]
    #[cfg_attr(miri, ignore = "Cocsep::generate is too slow under Miri")]
    fn hot_index_matches_reference_index() {
        let corners = Cocsep::generate();
        let mut rng = Rng(31);
        let mut cubes = vec![CubieCube::SOLVED];
        for scramble in [
            "U2 D2 R2 L2 F2 B2",
            "U D' R L' F B' U D'",
            "U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2",
        ] {
            cubes.push(CubieCube::SOLVED.apply_moves(&parse_moves(scramble).unwrap()));
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
            let table = H48Table::zeroed(k, 10);
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
