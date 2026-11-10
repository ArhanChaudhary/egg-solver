//! Symmetric-start filtering: for a cube with a symmetry, search one job of
//! each class of equivalent job prefixes (nissy's task filter, nxopt's `-s`,
//! Rokicki's "drop symmetry duplicates in the first levels").
//!
//! Gains (x-h48's 14 symmetric and antisymmetric test positions, `solve -c
//! 1`, on main d4125fd): 22.8x at 1T (9.75 -> 0.43 s) and 7.9x at 48T
//! (267 -> 34 ms), nearly all of it the superflip's (71x at 1T, 21x at
//! 48T); the 7 positions with only a symmetry 1.87x at 1T (-47% nodes) and
//! 1.30x at 48T. Stream mode (8 threads, on 387034c): 28.9x solves/s,
//! -96.6% nodes.
//!
//! A job is a canonical move prefix `p` of the solution, searched from
//! `X·p`. A job is dropped if a lexicographically smaller job `q` (move
//! indices; same length) reaches an equivalent position: `X·q` is a
//! conjugate of `X·p` or of its inverse (one of 96), so both are equally far
//! from solved. Two rules find such a `q`:
//!
//! * `keep`: for each symmetry `t` of `X` (`t X t⁻¹ = X`), `q = canon(t(p))`
//!   (`X·t(p) = t (X·p) t⁻¹`; `canon` puts adjacent opposite-face moves in
//!   the canonical order, U before D). Cheap: no cube is touched.
//! * `dedup`: exact classes (the least of the 96 conjugates as a key) of a
//!   whole level of jobs; keeps the smallest job of each class. This finds
//!   the equivalences that come from an antisymmetry (`t X t⁻¹ = X⁻¹`) of a
//!   cube that also has symmetries (the superflip, which commutes with every
//!   move: `X·p` and `X·p⁻¹`), and accidental ones.
//!
//! Which cubes: `keep` runs for every cube with a symmetry, `dedup` only if
//! it also has an antisymmetry. On x-h48's symmetric timing set, `dedup` of
//! the 7 cubes with only a symmetry dropped nothing beyond `keep` (1656 of
//! 3240 3-move jobs kept either way) and took ~0.4 ms per cube at 1T; for
//! the 6 with only an antisymmetry neither rule drops anything (it relates
//! `X·p` to positions with `p` on the inverse side, which are not jobs), so
//! they are not filtered (filtered by `dedup`: ~0.75 ms per cube at 1T, and
//! 1.5-3.2x their time at 48T `-c 1` in one screening run). For the
//! superflip, `dedup` takes the 3-move jobs from 60 to 44 and the 4-move
//! ones from 553 to 380.
//!
//! Why one optimal solution is still found: let `g` be the smallest job of
//! its length that is the prefix of a canonical optimal solution. If `g` was
//! dropped for `q < g`, then `q` followed by an optimal solution of `X·q` is
//! optimal, and its canonical form starts with `q` or with `q`'s last move
//! swapped for an earlier opposite-face one: a smaller prefix of an optimal
//! solution, a contradiction. Each prefix of `g` is the smallest of its own
//! length (a smaller one would extend to a smaller `g`), so filtering every
//! extension level never drops it either. The search inside a job (both
//! sides, premoves) is unchanged.
//!
//! Random states never have a symmetry: none of 7M positions (2000 random
//! states and their prefixes up to 3 moves deep). The check at a cube's start
//! costs 95 conjugations (~0.3 µs), and nothing else happens unless the cube
//! has a symmetry.

use crate::simd::Cube;
use std::collections::HashMap;

/// Exact classes are computed for levels of at most this many jobs (after
/// `keep`): ~0.2 µs per job at 1T (3240 jobs in ~0.7 ms).
const DEDUP_MAX_JOBS: usize = 8192;

/// The jobs to keep for one cube (see the module docs).
#[derive(Clone, Copy)]
pub struct SymFilter {
    /// The cube's symmetries other than the identity: the first
    /// `n_symmetries` of `symmetries` (none: every job is kept and nothing
    /// else is read).
    n_symmetries: u8,
    symmetries: [u8; 47],
    /// The cube also has an antisymmetry: `dedup` runs.
    antisymmetric: bool,
}

impl SymFilter {
    /// Keeps every job.
    pub const NONE: SymFilter = SymFilter {
        n_symmetries: 0,
        symmetries: [0; 47],
        antisymmetric: false,
    };

    /// The filter of `cube` (with inverse `inverse`).
    #[must_use]
    pub fn new(cube: Cube, inverse: Cube) -> SymFilter {
        let mut filter = SymFilter::NONE;
        let mut antisymmetric = false;
        for sym in 0..48u8 {
            let conjugate = cube.transform(usize::from(sym));
            if sym > 0 && conjugate.equals(cube) {
                filter.symmetries[usize::from(filter.n_symmetries)] = sym;
                filter.n_symmetries += 1;
            }
            antisymmetric |= conjugate.equals(inverse);
        }
        filter.antisymmetric = antisymmetric;
        filter
    }

    /// Whether the filter drops anything (the cube has a symmetry).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.n_symmetries > 0
    }

    /// Whether `dedup` runs (the cube also has an antisymmetry). Without it,
    /// whether a job is kept depends on its moves alone (`keep`).
    #[must_use]
    pub fn dedups(&self) -> bool {
        self.is_active() && self.antisymmetric
    }

    /// Whether to keep the job with move prefix `moves` (nonempty,
    /// canonical, at most 4 moves): false if a symmetry of the cube maps it
    /// to a smaller one.
    #[must_use]
    pub fn keep(&self, moves: &[u8]) -> bool {
        if !self.is_active() {
            return true;
        }
        let conjugates = move_conjugates();
        for &sym in &self.symmetries[..usize::from(self.n_symmetries)] {
            let mut image = [0u8; 4];
            for (image_move, &mv) in image.iter_mut().zip(moves) {
                *image_move = conjugates[usize::from(sym)][usize::from(mv)];
            }
            let image = &mut image[..moves.len()];
            // Canonical order of opposite-face pairs (no three consecutive
            // moves share an axis in a canonical sequence, so pairs are
            // disjoint: one pass).
            for i in 1..image.len() {
                let (first_face, second_face) = (image[i - 1] / 3, image[i] / 3);
                if first_face / 2 == second_face / 2 && first_face > second_face {
                    image.swap(i - 1, i);
                }
            }
            if *image < *moves {
                return false;
            }
        }
        true
    }

    /// Keep only the smallest job (by `moves`) of each class of equivalent
    /// positions (`position`) among `jobs`, a whole level (same length), in
    /// order. Does nothing unless the cube has a symmetry and an
    /// antisymmetry (`dedups`), or above `DEDUP_MAX_JOBS` jobs.
    pub fn dedup<T>(
        &self,
        jobs: &mut Vec<T>,
        moves: impl Fn(&T) -> &[u8],
        position: impl Fn(&T) -> Cube,
    ) {
        if !self.dedups() || jobs.len() > DEDUP_MAX_JOBS {
            return;
        }
        let keys: Vec<(u64, u128)> = jobs.iter().map(|job| class_key(position(job))).collect();
        // The smallest job of each class.
        let mut smallest: HashMap<(u64, u128), usize> = HashMap::with_capacity(jobs.len());
        for (i, key) in keys.iter().enumerate() {
            smallest
                .entry(*key)
                .and_modify(|best| {
                    if moves(&jobs[i]) < moves(&jobs[*best]) {
                        *best = i;
                    }
                })
                .or_insert(i);
        }
        let mut i = 0;
        jobs.retain(|_| {
            let keep = smallest[&keys[i]] == i;
            i += 1;
            keep
        });
    }
}

/// `conjugates[t][m]`: the move `t m t⁻¹` (`Cube::transform` by `t` of
/// move `m`): each symmetry's move map, from build.rs.
fn move_conjugates() -> &'static [[u8; 18]; 48] {
    &crate::simd::SYM_MOVE_MAP
}

/// The corner and edge bytes of a cube.
fn position_key(cube: Cube) -> (u64, u128) {
    let bytes = cube.to_bytes();
    let mut corners = [0u8; 8];
    for (x, &byte) in corners.iter_mut().zip(&bytes) {
        *x = byte;
    }
    let mut edges = [0u8; 16];
    for (x, &byte) in edges.iter_mut().zip(bytes.iter().skip(16).take(12)) {
        *x = byte;
    }
    (u64::from_le_bytes(corners), u128::from_le_bytes(edges))
}

/// The same key for all cubes equivalent to `cube`: the least over its 48
/// conjugates and those of its inverse.
fn class_key(cube: Cube) -> (u64, u128) {
    let inverse = cube.inverse();
    (0..48)
        .flat_map(|sym| {
            [
                position_key(cube.transform(sym)),
                position_key(inverse.transform(sym)),
            ]
        })
        .fold((u64::MAX, u128::MAX), std::cmp::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::{parse_moves, CubieCube, Move, Rng};

    fn cube(moves: &str) -> Cube {
        Cube::from_cubie(
            &CubieCube::SOLVED
                .apply_moves(&parse_moves(moves).unwrap())
                .unwrap(),
        )
    }

    /// Conjugation is a homomorphism:
    /// `t (X·m) t⁻¹ = (t X t⁻¹)·conjugates[t][m]`.
    #[test]
    fn move_conjugates_compose() {
        let conjugates = move_conjugates();
        let mut rng = Rng(5);
        for _ in 0..if cfg!(miri) { 2 } else { 50 } {
            let x = Cube::from_cubie(&CubieCube::random(&mut rng));
            for (sym, row) in conjugates.iter().enumerate() {
                for (mv, &image_move) in row.iter().enumerate() {
                    let lhs = x.apply_move(Move::ALL[mv]).transform(sym);
                    let rhs = x
                        .transform(sym)
                        .apply_move(Move::ALL[usize::from(image_move)]);
                    assert!(lhs.equals(rhs), "sym {sym} move {mv}");
                }
            }
        }
    }

    #[test]
    fn random_cubes_are_not_filtered() {
        let mut rng = Rng(9);
        for _ in 0..if cfg!(miri) { 2 } else { 200 } {
            let cube = Cube::from_cubie(&CubieCube::random(&mut rng));
            assert!(!SymFilter::new(cube, cube.inverse()).is_active());
        }
    }

    /// Jobs kept per length (1..=4) for cube `x`, extending level by level
    /// as the solvers do.
    fn kept_per_level(x: Cube) -> [usize; 5] {
        let filter = SymFilter::new(x, x.inverse());
        let mut kept = [0usize; 5];
        let mut level: Vec<Vec<u8>> = vec![vec![]];
        for kept_of_length in kept.iter_mut().skip(1) {
            let mut next = Vec::new();
            for prefix in &level {
                for mv in 0..18u8 {
                    if let Some(&last) = prefix.last() {
                        let (last_face, face) = (last / 3, mv / 3);
                        if last_face == face || (last_face % 2 == 1 && face == last_face - 1) {
                            continue;
                        }
                    }
                    let mut child = prefix.clone();
                    child.push(mv);
                    if filter.keep(&child) {
                        next.push(child);
                    }
                }
            }
            filter.dedup(
                &mut next,
                |prefix| prefix,
                |prefix| {
                    prefix
                        .iter()
                        .fold(x, |cube, &mv| cube.apply_move(Move::ALL[usize::from(mv)]))
                },
            );
            *kept_of_length = next.len();
            level = next;
        }
        kept
    }

    /// The superflip has all 48 symmetries and is its own inverse (and
    /// commutes with every move): one quarter turn and one half turn are
    /// left as first moves, and far fewer than nissy's 1038 of the 43254
    /// 4-move prefixes.
    #[test]
    fn superflip_jobs() {
        let x = cube("U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2");
        let filter = SymFilter::new(x, x.inverse());
        assert!(filter.dedups() && filter.n_symmetries == 47);
        let kept = kept_per_level(x);
        assert_eq!(kept[1], 2, "{kept:?}");
        assert!(kept[4] <= 1038, "{kept:?}");
    }

    /// Only an antisymmetry: no filter (no symmetry for `keep`, and `dedup`
    /// would find nothing: see the module docs).
    #[test]
    fn antisymmetric_cube() {
        // A · t(A)⁻¹ is antisymmetric for an involution t (here y2).
        let a = cube("R U2 F' L D B2 R'");
        let sym = (0..48)
            .find(|&sym| {
                let u = Cube::solved().apply_move(Move::U);
                u.transform(sym).equals(u)
                    && Cube::solved()
                        .apply_move(Move::R)
                        .transform(sym)
                        .equals(Cube::solved().apply_move(Move::L))
            })
            .unwrap();
        let x = a.compose(a.transform(sym).inverse());
        assert!(x.transform(sym).equals(x.inverse()));
        let filter = SymFilter::new(x, x.inverse());
        assert!(filter.antisymmetric && filter.n_symmetries == 0);
        assert!(!filter.is_active() && !filter.dedups());
        assert!(filter.keep(&[3, 6, 0]));
        assert_eq!(kept_per_level(x), [0, 18, 243, 3240, 43254]);
    }
}
