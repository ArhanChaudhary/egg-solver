//! Every distinct solution of a cube up to a length, instead of one optimal
//! one: all optimal solutions, those of at most `K` moves more, or all of at
//! most `N` moves (`Options`).
//!
//! The searches are the solvers' own (`Solver::all_solutions`, the recursive
//! reference; `lanes::share::all_solutions`, all threads on one cube or a
//! batch): the same iterations with increasing target, but a solution is
//! recorded and the search goes on, through every target up to the limit.
//! Nothing that only holds for "at least one optimal solution" is used: no
//! job order, no end at the first solution, and of the symmetric-start
//! filter only the part that stays exact (below). Pruning by lower bounds
//! stays: it only cuts nodes that no sequence of the moves left can solve.
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
//!
//! # Cubes with symmetries
//!
//! If a symmetry `t` leaves the cube `X` as it is (`t X t⁻¹ = X`), the
//! image `t(s)` of a solution `s` (each move `m` replaced by `t m t⁻¹`) is a
//! solution too: `X·t(s) = t (X·s) t⁻¹`. Images keep the length, the
//! commuting pairs and the detours, so the solutions of `X` come in orbits
//! under its symmetries, and the list is the union of whole orbits.
//!
//! The search then runs only the jobs that `SymFilter::keep` keeps: a job
//! (a canonical prefix) is dropped if a symmetry of the cube maps it to a
//! smaller prefix, or one of its own prefixes to a smaller one. Every
//! solution found is recorded with its images (`Collector::add`). That
//! gives every solution: take, in an orbit, the member `c` that is smallest
//! (as a sequence of move numbers, in canonical form). Each of its prefixes
//! is kept. For if `t` mapped the prefix of `j` moves to a smaller one `q`,
//! the canonical form of `t(c)`, also in the orbit, would start with `q`,
//! or with `q` and its last move swapped for the earlier opposite-face
//! move that follows it: in both cases smaller than `c`. So the job of `c`
//! is searched and `c` found (as above), and the images of `c` are its
//! orbit.
//!
//! Only `keep` is used, not `SymFilter::dedup`, which also drops jobs that
//! reach the same position as another by accident: their solutions are
//! different sequences, and no symmetry gives them back. A symmetry that
//! takes the cube to its inverse (an antisymmetry) also maps solutions to
//! solutions (`t(s⁻¹)`), but relates a solution's first moves to another's
//! last ones, which are not jobs: it is not used to search less.
//!
//! # Stopping early
//!
//! A list can be stopped before its limit: by `Options::max_solutions`
//! (`Solutions::truncated`), by `Options::time_limit` or by
//! `Options::cancel` (`Solutions::stopped`). It then holds what was found:
//! every solution of at most `Solutions::complete_length` moves, and some
//! of the longer ones. The searches look at the clock and the flag between
//! jobs (the share mode at the flag once per round), never in their inner
//! loops.

use crate::cubie::{CubieCube, Move};
use crate::metric::{simplify, Metric};
use crate::symstart::SymFilter;
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// `Options::max_solutions` unless asked otherwise. (Counts grow about 13
/// times per extra move: a random position has a handful of optimal
/// solutions, some hundreds one move above, thousands two above.)
pub const DEFAULT_MAX_SOLUTIONS: usize = 100_000;

/// Which solutions to list: those of at most `plus` moves more than an
/// optimal one and at most `max_length` moves (each limit if given; neither:
/// nothing beyond the metric's God's number limits the length), and at
/// most `max_solutions` of them; and when to give up (`time_limit`,
/// `cancel`).
#[derive(Clone, Copy, Debug)]
pub struct Options<'a> {
    /// Moves above the optimal length (0: the optimal solutions only).
    pub plus: Option<u8>,
    /// The largest length listed (in the search's metric). Searches go up to
    /// God's number of the metric at most (20 in HTM, 26 in QTM).
    pub max_length: Option<u8>,
    /// The most solutions listed (at least 1; 0 is taken as 1). With
    /// more, the search stops and the list is marked truncated
    /// (`Solutions::truncated`).
    pub max_solutions: usize,
    /// The longest a cube is searched, from its start. After that its
    /// search stops (when a job of its ends), and its list is what was
    /// found (`Stopped::Time`).
    pub time_limit: Option<Duration>,
    /// A flag that stops the searches once it is true (set from another
    /// thread or a signal handler): the lists are what was found
    /// (`Stopped::Cancelled`), and cubes not started are not listed.
    pub cancel: Option<&'a AtomicBool>,
}

impl<'a> Options<'a> {
    /// Every optimal solution.
    #[must_use]
    pub const fn all_optimal() -> Options<'a> {
        Options::plus(0)
    }

    /// Every solution of at most `plus` moves more than an optimal one.
    #[must_use]
    pub const fn plus(plus: u8) -> Options<'a> {
        Options {
            plus: Some(plus),
            max_length: None,
            max_solutions: DEFAULT_MAX_SOLUTIONS,
            time_limit: None,
            cancel: None,
        }
    }

    /// Every solution of at most `max_length` moves.
    #[must_use]
    pub const fn max_length(max_length: u8) -> Options<'a> {
        Options {
            plus: None,
            max_length: Some(max_length),
            max_solutions: DEFAULT_MAX_SOLUTIONS,
            time_limit: None,
            cancel: None,
        }
    }

    /// The same with at most `max_solutions` solutions listed.
    #[must_use]
    pub const fn with_max_solutions(mut self, max_solutions: usize) -> Options<'a> {
        self.max_solutions = max_solutions;
        self
    }

    /// The same with each cube's search stopped after `time_limit`.
    #[must_use]
    pub const fn with_time_limit(mut self, time_limit: Duration) -> Options<'a> {
        self.time_limit = Some(time_limit);
        self
    }

    /// The same with the searches stopped once `cancel` is true.
    #[must_use]
    pub const fn with_cancel(mut self, cancel: &'a AtomicBool) -> Options<'a> {
        self.cancel = Some(cancel);
        self
    }

    /// Whether `cancel` is set.
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.cancel
            .is_some_and(|cancel| cancel.load(std::sync::atomic::Ordering::Relaxed))
    }
}

/// Why a list ended before its limit (`Solutions::stopped`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stopped {
    /// `Options::time_limit` was reached.
    Time,
    /// `Options::cancel` was set.
    Cancelled,
}

impl Stopped {
    /// `time` or `cancelled` (as the command line and JSON name it).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Stopped::Time => "time",
            Stopped::Cancelled => "cancelled",
        }
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
    /// metric. (Of a stopped list without a solution: as far as known,
    /// without the first.)
    pub max_length: u8,
    /// There are more solutions within `max_length` than
    /// `Options::max_solutions`: the search stopped, and the list holds
    /// that many of them.
    pub truncated: bool,
    /// The search was stopped before its limit, by the clock or the cancel
    /// flag: the list holds what was found until then.
    pub stopped: Option<Stopped>,
    /// Every solution of at most this length is listed: `max_length`, or if
    /// `truncated` or `stopped` the last length searched completely (of
    /// the lengths above, the list has only a part, or nothing).
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
    stopped: Option<Stopped>,
    /// The search reached its limit (then it is not stopped any more).
    finished: bool,
    /// The last target searched completely.
    complete: u8,
    /// The cube's symmetries, if the search runs only the jobs that the
    /// filter keeps: every solution is recorded with its images.
    symmetries: SymFilter,
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
    /// For a search in `metric` on the jobs that `symmetries` keeps (of
    /// the cube; `SymFilter::NONE`: on all jobs).
    pub(crate) fn new(metric: Metric, options: &Options, symmetries: SymFilter) -> Collector {
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
            stopped: None,
            finished: false,
            complete: 0,
            symmetries,
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

    /// The search starts at target `first_target`: the cube's bounds show
    /// that it has no shorter solution.
    pub(crate) fn start(&mut self, first_target: u8) {
        self.complete = first_target.saturating_sub(1);
    }

    /// `offer`, given `canonical` of the sequence found.
    ///
    /// The first solution kept gives the optimal length: iterations run in
    /// order of their targets, and at the optimal target every sequence
    /// found is a solution of its length (one that is not would be a
    /// shorter solution).
    ///
    /// The solution is recorded with its images under the cube's
    /// symmetries (the module docs, "Cubes with symmetries"): solutions of
    /// its length that the filtered search may not find itself.
    pub(crate) fn add(&mut self, solution: Option<Vec<u8>>) -> bool {
        #[cfg(test)]
        {
            self.offered += 1;
            self.rejected += u64::from(solution.is_none());
        }
        let Some(solution) = solution else {
            return self.searching();
        };
        if !self.searching() {
            return false;
        }
        if self.optimal.is_none() {
            let length = u8::try_from(self.metric.length(&solution)).unwrap_or(u8::MAX);
            self.optimal = Some(length);
            self.limit = self.limit.min(length.saturating_add(self.plus));
        }
        // (An image is a solution as the sequence is; `canonical` puts it
        // in canonical order.)
        for image in self.symmetries.images(&solution) {
            if let Some(image) = canonical(self.metric, &image) {
                self.found.insert(image);
            }
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

    /// Whether the search goes on: not truncated, stopped or finished.
    fn searching(&self) -> bool {
        !self.truncated && self.stopped.is_none() && !self.finished
    }

    /// Stop the search for `reason` (the clock or the cancel flag), unless
    /// it ended already. Returns whether it is stopped (for any reason:
    /// not if it had reached its limit).
    pub(crate) fn stop(&mut self, reason: Stopped) -> bool {
        if self.searching() {
            self.stopped = Some(reason);
        }
        !self.finished
    }

    /// The iteration with target `target` ended, searched completely.
    /// Returns whether the next one (`step` moves longer) is to be searched.
    /// (Not if the search was stopped: that iteration is not complete.)
    pub(crate) fn iteration_done(&mut self, target: u8, step: u8) -> bool {
        if !self.searching() {
            return false;
        }
        self.complete = self.complete.max(target);
        self.finished = target.saturating_add(step) > self.limit;
        !self.finished
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
            stopped: self.stopped,
            complete_length: if self.truncated || self.stopped.is_some() {
                // (The search stopped within the iteration after
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
        let mut collector = Collector::new(metric, options, SymFilter::NONE);
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

    /// A solution of a cube with symmetries is recorded with its images:
    /// the solutions that a search of the kept jobs alone does not find.
    #[test]
    fn collector_records_the_images() {
        use crate::cubie::CubieCube;
        use crate::simd::Cube;
        let htm = Metric::Htm;
        // U2 D2 R2 L2: its two optimal solutions are images of each other
        // (a quarter turn of the whole cube about the F axis).
        let cube = CubieCube::SOLVED
            .apply_moves(&moves("U2 D2 R2 L2"))
            .unwrap();
        let cube = Cube::from_cubie(&cube);
        let filter = SymFilter::new(cube, cube.inverse()).without_dedup();
        assert_eq!(filter.group_order(), 16);
        let mut collector = Collector::new(htm, &Options::all_optimal(), filter);
        assert!(collector.offer(&moves("U2 D2 R2 L2")));
        assert!(!collector.iteration_done(4, 1));
        let list = collector.finish();
        assert_eq!(list.solutions, [moves("U2 D2 R2 L2"), moves("R2 L2 U2 D2")]);
        // The cap counts the images: one solution is asked for, two exist.
        let options = Options::all_optimal().with_max_solutions(1);
        let mut collector = Collector::new(htm, &options, filter);
        assert!(!collector.offer(&moves("U2 D2 R2 L2")));
        let list = collector.finish();
        assert_eq!((list.truncated, list.solutions.len()), (true, 1));
        // Stopped: the reason is kept, later ones and solutions are not.
        let mut collector = Collector::new(htm, &Options::plus(3), SymFilter::NONE);
        collector.start(4);
        assert!(collector.offer(&moves("R U F D")));
        collector.stop(Stopped::Time);
        collector.stop(Stopped::Cancelled);
        assert!(!collector.offer(&moves("R U F D' B")));
        assert!(!collector.iteration_done(4, 1));
        let list = collector.finish();
        assert_eq!(list.stopped, Some(Stopped::Time));
        assert_eq!((list.solutions.len(), list.complete_length), (1, 3));
        // Before any solution: complete below the first target.
        let mut collector = Collector::new(htm, &Options::all_optimal(), SymFilter::NONE);
        collector.start(9);
        collector.stop(Stopped::Cancelled);
        let list = collector.finish();
        assert_eq!((list.optimal, list.complete_length), (None, 8));
        assert_eq!(
            (list.stopped, list.max_length),
            (Some(Stopped::Cancelled), 20)
        );
        // A list that reached its limit is not stopped afterwards.
        let mut collector = Collector::new(htm, &Options::max_length(4), SymFilter::NONE);
        assert!(!collector.iteration_done(4, 1));
        collector.stop(Stopped::Cancelled);
        assert_eq!(collector.finish().stopped, None);
    }

    #[test]
    fn collector_limits_and_truncation() {
        let htm = Metric::Htm;
        // The first solution sets the optimal length and the limit.
        let mut collector = Collector::new(htm, &Options::plus(1), SymFilter::NONE);
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
                stopped: None,
                complete_length: 4,
            }
        );
        assert_eq!(list.counts(htm), [(3, 2), (4, 1)]);
        // `max_length` below `plus`, and above God's number.
        let options = Options {
            max_length: Some(3),
            ..Options::plus(5)
        };
        assert_eq!(Collector::new(htm, &options, SymFilter::NONE).limit(), 3);
        assert_eq!(
            Collector::new(htm, &Options::max_length(99), SymFilter::NONE).limit(),
            20
        );
        assert_eq!(
            Collector::new(Metric::Qtm, &Options::max_length(99), SymFilter::NONE).limit(),
            26
        );
        // More solutions than the cap: stop; complete below.
        let mut collector = Collector::new(
            htm,
            &Options::max_length(9).with_max_solutions(2),
            SymFilter::NONE,
        );
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
        let mut collector = Collector::new(
            htm,
            &Options::all_optimal().with_max_solutions(1),
            SymFilter::NONE,
        );
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
