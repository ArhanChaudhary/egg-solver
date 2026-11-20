//! Every distinct solution of a cube up to a length, instead of one optimal
//! one: all optimal solutions, those of at most `K` moves more, or all of at
//! most `N` moves (`Options`).
//!
//! The searches are the solvers' own (`Solver::all_solutions`, the recursive
//! reference; `lanes::share::all_solutions`, all threads on one cube or a
//! batch): the same iterations with increasing target, but a solution is
//! recorded and the search goes on, through every target up to the limit.
//! Nothing that only holds for "at least one optimal solution" is used: no
//! symmetric-start filter, no job order, no end at the first solution.
//! Pruning by lower bounds stays: it only cuts nodes that no sequence of
//! the moves left can solve.
//!
//! # What a distinct solution is
//!
//! * A solution is a move sequence up to the order of adjacent commuting
//!   moves (turns of opposite faces: U D = D U). It is listed once, in the
//!   search's canonical order (`metric::simplify`: U before D, R before L,
//!   F before B).
//! * No two moves of a face follow each other, also after reordering
//!   commuting moves: `U D U'` is the shorter `D`, not a solution of 3
//!   moves. (In QTM a half turn is two quarter turns of its face: `U U` is
//!   written `U2`, the same solution of the same length.)
//! * No detour: a sequence with a part that does nothing (a contiguous run
//!   of moves equal to the identity, in any order of its commuting moves;
//!   for example a sequence that solves the cube before its end) is a
//!   shorter solution with a loop added, and is left out. The shortest such
//!   loops have 8 moves in HTM (`U2 D2 R2 L2 U2 D2 R2 L2`), so this only
//!   matters 8 moves above the optimum.
//! * Everything else of at most the limit is listed, non-optimal solutions
//!   included.
//!
//! # How the searches' output becomes that list
//!
//! A search finds a solution as the moves of its normal side followed by
//! those of its inverse side, inverted and reversed. The two sides order
//! commuting moves oppositely (U D in the prefix, D U in the suffix), and
//! moves of one face can meet where they join (`U` `D` `U'`): the strings
//! found are not all canonical, and some are not solutions of their length
//! at all. Every string found goes through `canonical`, which rejects the
//! latter two kinds, into a set (`Collector`). Conversely every distinct
//! solution is found: take its sequence with U before D wherever the search
//! consumes a commuting pair on one side. Each of its moves is allowed where
//! the search needs it (a move after its opposite face's is refused only in
//! the order D U on the normal side, U D on the inverse side), and bounds
//! never cut a node that leads to a solution. The tests check this against
//! a brute-force enumeration (`tests/all_solutions.rs`).
//!
//! The in-search rule "a node that is solved before the target is not
//! searched" stays, as a pruning: every sequence through such a node has a
//! detour (the moves after it do nothing), so `canonical` would reject all
//! of them. The list is decided by `canonical` alone, whatever the search
//! skips and wherever its two sides meet.

use crate::cubie::{CubieCube, Move};
use crate::metric::{simplify, Metric};
use std::collections::BTreeSet;

/// `Options::max_solutions` unless asked otherwise. (Counts grow about 13
/// times per extra move: a random position has a handful of optimal
/// solutions, some hundreds one move above, thousands two above.)
pub const DEFAULT_MAX_SOLUTIONS: usize = 100_000;

/// Which solutions to list: those of at most `plus` moves more than an
/// optimal one and at most `max_length` moves (each limit if given; neither:
/// nothing beyond the metric's God's number limits the length), and at
/// most `max_solutions` of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Moves above the optimal length (0: the optimal solutions only).
    pub plus: Option<u8>,
    /// The largest length listed (in the search's metric). Searches go up to
    /// God's number of the metric at most (20 in HTM, 26 in QTM).
    pub max_length: Option<u8>,
    /// The most solutions listed (at least 1; 0 is taken as 1). With
    /// more, the search stops and the list is marked truncated
    /// (`Solutions::truncated`).
    pub max_solutions: usize,
}

impl Options {
    /// Every optimal solution.
    #[must_use]
    pub const fn all_optimal() -> Options {
        Options::plus(0)
    }

    /// Every solution of at most `plus` moves more than an optimal one.
    #[must_use]
    pub const fn plus(plus: u8) -> Options {
        Options {
            plus: Some(plus),
            max_length: None,
            max_solutions: DEFAULT_MAX_SOLUTIONS,
        }
    }

    /// Every solution of at most `max_length` moves.
    #[must_use]
    pub const fn max_length(max_length: u8) -> Options {
        Options {
            plus: None,
            max_length: Some(max_length),
            max_solutions: DEFAULT_MAX_SOLUTIONS,
        }
    }

    /// The same with at most `max_solutions` solutions listed.
    #[must_use]
    pub const fn with_max_solutions(mut self, max_solutions: usize) -> Options {
        self.max_solutions = max_solutions;
        self
    }
}

/// The distinct solutions of a cube (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solutions {
    /// The solutions, each in canonical form, by increasing length (in the
    /// search's metric), then by their moves' numbers.
    pub solutions: Vec<Vec<u8>>,
    /// The optimal length: that of the shortest solution. `None` if the
    /// cube has no solution within `max_length`.
    pub optimal: Option<u8>,
    /// The length limit of the list: the smallest of the optimal length
    /// plus `Options::plus`, `Options::max_length` and God's number of the
    /// metric.
    pub max_length: u8,
    /// There are more solutions within `max_length` than
    /// `Options::max_solutions`: the search stopped, and the list holds
    /// that many of them.
    pub truncated: bool,
    /// Every solution of at most this length is listed: `max_length`, or if
    /// `truncated` the last length searched completely (of the length
    /// above, the list has only a part).
    pub complete_length: u8,
}

impl Solutions {
    /// The number of solutions of each length present, by increasing length
    /// (in `metric`, that of the search).
    #[must_use]
    pub fn counts(&self, metric: Metric) -> Vec<(usize, usize)> {
        let mut counts: Vec<(usize, usize)> = Vec::new();
        for solution in &self.solutions {
            let length = metric.length(solution);
            match counts.last_mut() {
                Some((last, count)) if *last == length => *count += 1,
                _ => counts.push((length, 1)),
            }
        }
        counts
    }
}

/// The canonical form of a sequence that a search found (`raw`, which
/// solves the cube), if it is a solution of its own length: `None` if its
/// moves merge or cancel once commuting moves are reordered (`U D U'`: a
/// shorter solution spelled longer), or if it has a detour (`has_detour`).
/// Lengths are in `metric`: in QTM `U U` is `U2`, of the same length.
#[must_use]
pub fn canonical(metric: Metric, raw: &[u8]) -> Option<Vec<u8>> {
    let canonical = simplify(raw);
    (metric.length(&canonical) == metric.length(raw) && !has_detour(metric, &canonical))
        .then_some(canonical)
}

/// The amounts of move `mv` that a contiguous part of a sequence can end
/// with: nothing, or the move; in QTM a half turn is two quarter turns of
/// either direction, so also each quarter turn of its face.
fn partial_turns(metric: Metric, mv: u8) -> ([Option<u8>; 4], usize) {
    if metric.is_qtm() && mv % 3 == 1 {
        ([None, Some(mv - 1), Some(mv + 1), Some(mv)], 4)
    } else {
        ([None, Some(mv), None, None], 2)
    }
}

/// Whether the canonical sequence `moves` has a detour: a contiguous part,
/// in some order of its adjacent commuting moves, that is the identity (in
/// QTM in terms of quarter turns: a part can begin or end within a half
/// turn). The sequence is then a shorter one with that loop inserted.
///
/// A part is what lies between two cuts, and it is the identity if the
/// moves up to the one cut and up to the other give the same cube. The
/// cuts: before and after every move; within a pair of adjacent commuting
/// moves after either one alone (the pair can be in either order); in QTM
/// also within a half turn. So: the products of the moves before a maximal
/// run of moves of one axis (one or two moves, in a canonical sequence),
/// times every combination of amounts of the run's moves. Two of these
/// cubes are equal only at different cuts of one sequence order (within a
/// run they all differ), so a detour is a repeated cube.
#[must_use]
pub fn has_detour(metric: Metric, moves: &[u8]) -> bool {
    let turn = |cube: CubieCube, mv: Option<u8>| match mv.and_then(|mv| Move::from_index(mv.into()))
    {
        Some(mv) => cube.apply_move(mv),
        None => cube,
    };
    // (A few dozen cubes at most: compared one by one.)
    let mut seen: Vec<CubieCube> = Vec::with_capacity(4 * moves.len() + 1);
    seen.push(CubieCube::SOLVED);
    let mut before = CubieCube::SOLVED;
    let mut rest = moves;
    while let [first, tail @ ..] = rest {
        // The run of this axis: one move, or two of opposite faces.
        let second = tail.first().copied().filter(|next| next / 6 == first / 6);
        let (first_turns, n_first) = partial_turns(metric, *first);
        let (second_turns, n_second) = match second {
            Some(second) => partial_turns(metric, second),
            None => ([None; 4], 1),
        };
        for (i, &a) in first_turns.iter().take(n_first).enumerate() {
            for (j, &b) in second_turns.iter().take(n_second).enumerate() {
                // (Nothing of either: the cut before the run, seen already.)
                if (i, j) == (0, 0) {
                    continue;
                }
                let cube = turn(turn(before, a), b);
                if seen.contains(&cube) {
                    return true;
                }
                seen.push(cube);
            }
        }
        before = turn(turn(before, Some(*first)), second);
        rest = &tail[usize::from(second.is_some())..];
    }
    false
}

/// The solutions found for one cube so far, and how far to search.
pub(crate) struct Collector {
    metric: Metric,
    /// `Options::plus` (`u8::MAX`: none).
    plus: u8,
    /// The last target to search: `Solutions::max_length` as far as known.
    limit: u8,
    max_solutions: usize,
    optimal: Option<u8>,
    /// (A B-tree, not a hash table: those share code with the hash tables
    /// of the search for one solution, whose machine code then changes.)
    found: BTreeSet<Vec<u8>>,
    truncated: bool,
    /// The last target searched completely.
    complete: u8,
    /// (Tests) Sequences offered, and those that `canonical` rejected or
    /// that were found before.
    #[cfg(test)]
    pub(crate) offered: u64,
    #[cfg(test)]
    pub(crate) rejected: u64,
    #[cfg(test)]
    pub(crate) repeated: u64,
}

impl Collector {
    /// For a search in `metric`.
    pub(crate) fn new(metric: Metric, options: &Options) -> Collector {
        let limit = options
            .max_length
            .unwrap_or(u8::MAX)
            .min(metric.max_target());
        Collector {
            metric,
            plus: options.plus.unwrap_or(u8::MAX),
            limit,
            max_solutions: options.max_solutions.max(1),
            optimal: None,
            found: BTreeSet::new(),
            truncated: false,
            complete: 0,
            #[cfg(test)]
            offered: 0,
            #[cfg(test)]
            rejected: 0,
            #[cfg(test)]
            repeated: 0,
        }
    }

    /// The last target to search (it only shrinks: to the optimal length
    /// plus `Options::plus` once a solution is found).
    pub(crate) fn limit(&self) -> u8 {
        self.limit
    }

    /// A sequence that a search found (it solves the cube). Returns false
    /// if there are now more solutions than `Options::max_solutions`: the
    /// search is to stop.
    pub(crate) fn offer(&mut self, raw: &[u8]) -> bool {
        self.add(canonical(self.metric, raw))
    }

    /// `offer`, given `canonical` of the sequence found.
    ///
    /// The first solution kept gives the optimal length: iterations run in
    /// order of their targets, and at the optimal target every sequence
    /// found is a solution of its length (one that is not would be a
    /// shorter solution).
    pub(crate) fn add(&mut self, solution: Option<Vec<u8>>) -> bool {
        #[cfg(test)]
        {
            self.offered += 1;
            self.rejected += u64::from(solution.is_none());
        }
        let Some(solution) = solution else {
            return !self.truncated;
        };
        if self.truncated {
            return false;
        }
        if self.optimal.is_none() {
            let length = u8::try_from(self.metric.length(&solution)).unwrap_or(u8::MAX);
            self.optimal = Some(length);
            self.limit = self.limit.min(length.saturating_add(self.plus));
        }
        let new = self.found.insert(solution);
        #[cfg(test)]
        {
            self.repeated += u64::from(!new);
        }
        #[cfg(not(test))]
        let _ = new;
        self.truncated = self.found.len() > self.max_solutions;
        !self.truncated
    }

    /// The iteration with target `target` ended, searched completely.
    /// Returns whether the next one (`step` moves longer) is to be searched.
    /// (Not if the search was stopped: that iteration is not complete.)
    pub(crate) fn iteration_done(&mut self, target: u8, step: u8) -> bool {
        if self.truncated {
            return false;
        }
        self.complete = self.complete.max(target);
        target.saturating_add(step) <= self.limit
    }

    /// The list.
    pub(crate) fn finish(self) -> Solutions {
        let metric = self.metric;
        let mut solutions: Vec<Vec<u8>> = self.found.into_iter().collect();
        solutions.sort_unstable_by(|a, b| {
            (metric.length(a), a.as_slice()).cmp(&(metric.length(b), b.as_slice()))
        });
        solutions.truncate(self.max_solutions);
        Solutions {
            solutions,
            optimal: self.optimal,
            max_length: self.limit,
            truncated: self.truncated,
            complete_length: if self.truncated {
                // (A truncated search stopped within the iteration after
                // `complete`, or within the first one with solutions: every
                // length below the optimal one is complete, with none.)
                self.complete
                    .max(self.optimal.map_or(0, |optimal| optimal.saturating_sub(1)))
                    .min(self.limit)
            } else {
                self.limit
            },
        }
    }

    /// The list of the solved cube: the empty solution (every other
    /// sequence that does nothing is a detour).
    pub(crate) fn solved(metric: Metric, options: &Options) -> Solutions {
        let mut collector = Collector::new(metric, options);
        let _ = collector.offer(&[]);
        collector.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::parse_moves;

    fn moves(text: &str) -> Vec<u8> {
        parse_moves(text).unwrap()
    }

    #[test]
    fn canonical_orders_and_rejects() {
        let htm = Metric::Htm;
        let qtm = Metric::Qtm;
        // Commuting moves in the search's order.
        assert_eq!(
            canonical(htm, &moves("D U R L' B F")),
            Some(moves("U D R L' F B"))
        );
        assert_eq!(canonical(htm, &moves("R U F")), Some(moves("R U F")));
        assert_eq!(canonical(htm, &[]), Some(vec![]));
        // Moves of one face meet: a shorter sequence.
        for raw in ["U D U'", "R U D U F", "R L R", "F B2 F' U", "U U"] {
            assert_eq!(canonical(htm, &moves(raw)), None, "{raw}");
        }
        // QTM: `U U` is `U2` (2 = 2), other merges are shorter.
        assert_eq!(canonical(qtm, &moves("U U")), Some(moves("U2")));
        assert_eq!(canonical(qtm, &moves("R U D U F")), Some(moves("R U2 D F")));
        for raw in ["U U2", "U D U'", "U2 D U2", "R L R'"] {
            assert_eq!(canonical(qtm, &moves(raw)), None, "{raw}");
        }
    }

    /// `count` repetitions of `unit`.
    fn repeat(unit: &str, count: usize) -> String {
        vec![unit; count].join(" ")
    }

    #[test]
    fn detours() {
        let htm = Metric::Htm;
        let qtm = Metric::Qtm;
        // The shortest loops have 8 moves.
        let loop8 = "U2 D2 R2 L2 U2 D2 R2 L2";
        assert!(has_detour(htm, &moves(loop8)));
        assert!(!has_detour(htm, &moves("U2 D2 R2 L2 U2 D2 R2")));
        assert!(!has_detour(htm, &moves("R U R' U' F2 D")));
        assert!(!has_detour(htm, &[]));
        // A loop after, before and inside a sequence.
        for sequence in [
            format!("F {loop8}"),
            format!("{loop8} F"),
            format!("F R {loop8} B' U"),
        ] {
            let sequence = moves(&sequence);
            assert_eq!(simplify(&sequence), sequence);
            assert!(has_detour(htm, &sequence));
            assert!(has_detour(qtm, &sequence));
        }
        // A loop that begins within a commuting pair: `D` then a loop
        // that starts with `U`. In canonical order the loop's `U` comes
        // before the `D`, and the loop is no longer contiguous as written.
        let begins = simplify(&moves(&format!("F D {} B", repeat("U R U' R'", 6))));
        assert_eq!(begins[..4], moves("F U D R")[..]);
        assert_eq!(begins.len(), 27);
        assert!(has_detour(htm, &begins));
        // ... and one that ends within a pair: a loop that ends with `D'`,
        // then `U`.
        let ends = simplify(&moves(&format!("F {} U B", repeat("R D R' D'", 6))));
        assert_eq!(ends[23..], moves("R' U D' B")[..]);
        assert!(has_detour(htm, &ends));
        // Without the loop's last move there is no detour.
        let open = format!("F D {} U R U' B", repeat("U R U' R'", 5));
        assert!(!has_detour(htm, &simplify(&moves(&open))));
        // QTM: a loop that begins within a half turn. `F2 w` with `F w`
        // a loop is `F` followed by that loop in quarter turns; in HTM no
        // part of it is the identity.
        let within = moves(&format!("F2 U F' U' {}", repeat("F U F' U'", 5)));
        assert_eq!(simplify(&within), within);
        assert!(has_detour(qtm, &within));
        assert!(!has_detour(htm, &within));
        // ... in either direction (`F2` = `F' F'`), and one that ends
        // within a half turn.
        let within = moves(&format!("F2 U' F U {}", repeat("F' U' F U", 5)));
        assert!(has_detour(qtm, &within) && !has_detour(htm, &within));
        let within = moves(&format!("{} U F U' F2", repeat("U F U' F'", 5)));
        assert!(has_detour(qtm, &within) && !has_detour(htm, &within));
    }

    #[test]
    fn collector_limits_and_truncation() {
        let htm = Metric::Htm;
        // The first solution sets the optimal length and the limit.
        let mut collector = Collector::new(htm, &Options::plus(1));
        assert_eq!(collector.limit(), 20);
        assert!(collector.iteration_done(2, 1));
        assert!(collector.offer(&moves("R U F")));
        assert_eq!(collector.limit(), 4);
        assert!(collector.offer(&moves("F D B")));
        assert!(collector.offer(&moves("F D B")), "found again");
        assert!(collector.offer(&moves("U D U'")), "rejected");
        assert!(collector.iteration_done(3, 1));
        assert!(collector.offer(&moves("L U D F")));
        assert!(collector.offer(&moves("L D U F")), "the same solution");
        assert!(!collector.iteration_done(4, 1));
        assert_eq!(
            (collector.offered, collector.rejected, collector.repeated),
            (6, 1, 2)
        );
        let list = collector.finish();
        assert_eq!(
            list,
            Solutions {
                solutions: vec![moves("R U F"), moves("F D B"), moves("L U D F")],
                optimal: Some(3),
                max_length: 4,
                truncated: false,
                complete_length: 4,
            }
        );
        assert_eq!(list.counts(htm), [(3, 2), (4, 1)]);
        // `max_length` below `plus`, and above God's number.
        let options = Options {
            max_length: Some(3),
            ..Options::plus(5)
        };
        assert_eq!(Collector::new(htm, &options).limit(), 3);
        assert_eq!(Collector::new(htm, &Options::max_length(99)).limit(), 20);
        assert_eq!(
            Collector::new(Metric::Qtm, &Options::max_length(99)).limit(),
            26
        );
        // More solutions than the cap: stop; complete below.
        let mut collector = Collector::new(htm, &Options::max_length(9).with_max_solutions(2));
        assert!(collector.iteration_done(3, 1));
        assert!(collector.offer(&moves("R U F D")));
        assert!(collector.iteration_done(4, 1));
        assert!(collector.offer(&moves("R U F D L")));
        assert!(!collector.offer(&moves("R U F D B")));
        assert!(!collector.iteration_done(5, 1));
        let list = collector.finish();
        assert!(list.truncated);
        assert_eq!(list.solutions, [moves("R U F D"), moves("R U F D L")]);
        assert_eq!(
            (list.optimal, list.max_length, list.complete_length),
            (Some(4), 9, 4)
        );
        // Truncated within the first iteration with solutions.
        let mut collector = Collector::new(htm, &Options::all_optimal().with_max_solutions(1));
        assert!(collector.iteration_done(5, 1));
        assert!(collector.offer(&moves("R U F D L B")));
        assert!(!collector.offer(&moves("R U F D L B'")));
        let list = collector.finish();
        assert_eq!(
            (list.truncated, list.complete_length, list.solutions.len()),
            (true, 5, 1)
        );
        // The solved cube.
        let list = Collector::solved(htm, &Options::max_length(12));
        assert_eq!(list.solutions, [Vec::<u8>::new()]);
        assert_eq!(
            (list.optimal, list.max_length, list.truncated),
            (Some(0), 12, false)
        );
        assert_eq!(
            Collector::solved(htm, &Options::all_optimal()).max_length,
            0
        );
    }
}
