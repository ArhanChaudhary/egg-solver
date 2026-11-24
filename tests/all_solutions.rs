//! Every distinct solution up to a length (`egg_solver::solutions`):
//! the recursive solver's and the share mode's lists against brute force.
//!
//! Brute force is a depth-first search over all canonical move sequences
//! on the reference cube model (`CubieCube`), which keeps the sequences
//! that solve the cube and have no detour, with the rules of what a
//! distinct solution is written out here, apart from the library's. Its
//! only shortcut: with few moves left (`radius`), a cube that is not
//! within that many moves of solved (a table of all such positions) is
//! not searched further. The tables are h0 tables made in memory.

use egg_solver::cubie::{format_moves, parse_moves, CubieCube, Move, Rng};
use egg_solver::lanes::share;
use egg_solver::metric::Metric;
use egg_solver::solutions::{Options, Solutions};
use egg_solver::solver::Solver;
use egg_solver::tables::{Cocsep, H48Table};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Mutex, OnceLock};

/// The cocsep table, HTM depths (built once).
fn corners() -> &'static Cocsep {
    static CORNERS: OnceLock<Cocsep> = OnceLock::new();
    CORNERS.get_or_init(|| Cocsep::generate().unwrap())
}

/// An HTM h0 table of base `base` (generated once each): base 8 as the
/// binaries make it (bounds up to 11: one job per iteration below 12
/// moves), base 2 with bounds up to 5, so that the iterations of 6 to 9
/// moves and more run on jobs of 1 to 4 moves, as those of 18 and more do
/// on big tables. (HTM tables bound QTM searches too.)
fn h0(base: u8) -> &'static H48Table {
    static TABLES: OnceLock<Mutex<HashMap<u8, &'static H48Table>>> = OnceLock::new();
    let mut tables = TABLES.get_or_init(Default::default).lock().unwrap();
    tables.entry(base).or_insert_with(|| {
        let threads = egg_solver::thread_count();
        let (table, _) =
            unsafe { egg_solver::generate(corners(), 0, base, None, &[], threads) }.unwrap();
        Box::leak(Box::new(table))
    })
}

fn solver(base: u8, metric: Metric) -> Solver<'static> {
    Solver::with_metric(corners(), h0(base), metric).unwrap()
}

fn cube_of(moves: &str) -> CubieCube {
    CubieCube::SOLVED
        .apply_moves(&parse_moves(moves).unwrap())
        .unwrap()
}

// ---------------------------------------------------------------------
// Brute force
// ---------------------------------------------------------------------

/// Brute force stops at cubes that are not within the moves left of
/// solved, for at most this many moves left: 5 moves (621,649 positions),
/// 6 quarter turns in QTM (983,926).
fn radius(metric: Metric) -> usize {
    match metric {
        Metric::Htm => 5,
        Metric::Qtm => 6,
    }
}

/// What a move costs: 1, in QTM 2 for a half turn.
fn cost(metric: Metric, mv: u8) -> usize {
    if metric == Metric::Qtm && mv % 3 == 1 {
        2
    } else {
        1
    }
}

fn length(metric: Metric, moves: &[u8]) -> usize {
    moves.iter().map(|&mv| cost(metric, mv)).sum()
}

fn turn(cube: &CubieCube, mv: u8) -> CubieCube {
    cube.apply_move(Move::ALL[usize::from(mv)])
}

/// Every position within `radius` moves (of `metric`) of solved, with its
/// distance: level by level, each level from the ones one and (QTM half
/// turns) two below.
fn ball(metric: Metric) -> &'static HashMap<CubieCube, u8> {
    static BALLS: OnceLock<Mutex<HashMap<Metric, &'static HashMap<CubieCube, u8>>>> =
        OnceLock::new();
    let mut balls = BALLS.get_or_init(Default::default).lock().unwrap();
    balls.entry(metric).or_insert_with(|| {
        let mut distance = HashMap::new();
        distance.insert(CubieCube::SOLVED, 0u8);
        let mut levels = vec![vec![CubieCube::SOLVED]];
        for d in 1..=radius(metric) {
            let mut level = Vec::new();
            for mv in 0..18u8 {
                let Some(from) = d.checked_sub(cost(metric, mv)) else {
                    continue;
                };
                for cube in &levels[from] {
                    let child = turn(cube, mv);
                    if let std::collections::hash_map::Entry::Vacant(entry) = distance.entry(child)
                    {
                        entry.insert(u8::try_from(d).unwrap());
                        level.push(child);
                    }
                }
            }
            levels.push(level);
        }
        Box::leak(Box::new(distance))
    })
}

/// Whether `mv` may follow `last` in a canonical sequence: not the same
/// face, and of two opposite faces U before D, R before L, F before B.
fn may_follow(last: Option<u8>, mv: u8) -> bool {
    let Some(last) = last else {
        return true;
    };
    let (last_face, face) = (last / 3, mv / 3);
    let opposite = last_face / 2 == face / 2;
    last_face != face && !(opposite && last_face > face)
}

fn is_canonical(moves: &[u8]) -> bool {
    moves.windows(2).all(|w| may_follow(Some(w[0]), w[1]))
}

/// All canonical sequences of length at most `left` more (in `metric`)
/// that take `cube` to solved, each appended to `prefix`.
fn search(
    cube: &CubieCube,
    prefix: &mut Vec<u8>,
    left: usize,
    metric: Metric,
    found: &mut Vec<Vec<u8>>,
) {
    if *cube == CubieCube::SOLVED {
        found.push(prefix.clone());
    }
    if left <= radius(metric) {
        let near = ball(metric).get(cube);
        if near.is_none_or(|&d| usize::from(d) > left) {
            return;
        }
    }
    for mv in 0..18u8 {
        if cost(metric, mv) <= left && may_follow(prefix.last().copied(), mv) {
            prefix.push(mv);
            search(
                &turn(cube, mv),
                prefix,
                left - cost(metric, mv),
                metric,
                found,
            );
            prefix.pop();
        }
    }
}

/// The definition of a detour, taken literally: in some order of the
/// adjacent commuting moves of `moves` (canonical: such pairs are apart),
/// some contiguous part is the identity. In QTM in terms of quarter
/// turns, a half turn being two of either direction.
fn has_detour(moves: &[u8], metric: Metric) -> bool {
    let pairs: Vec<usize> = (1..moves.len())
        .filter(|&i| moves[i - 1] / 6 == moves[i] / 6)
        .collect();
    for order in 0..1u32 << pairs.len() {
        let mut ordered = moves.to_vec();
        for (bit, &i) in pairs.iter().enumerate() {
            if order >> bit & 1 != 0 {
                ordered.swap(i - 1, i);
            }
        }
        let halves: Vec<usize> = (0..ordered.len())
            .filter(|&i| metric == Metric::Qtm && ordered[i] % 3 == 1)
            .collect();
        for directions in 0..1u32 << halves.len() {
            let mut turns = Vec::new();
            for (i, &mv) in ordered.iter().enumerate() {
                match halves.iter().position(|&h| h == i) {
                    Some(bit) => {
                        let quarter = if directions >> bit & 1 != 0 {
                            mv + 1
                        } else {
                            mv - 1
                        };
                        turns.extend([quarter, quarter]);
                    }
                    None => turns.push(mv),
                }
            }
            for start in 0..turns.len() {
                let mut cube = CubieCube::SOLVED;
                for &mv in &turns[start..] {
                    cube = turn(&cube, mv);
                    if cube == CubieCube::SOLVED {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Brute force's answer for one cube: its distinct solutions of at most
/// `max_length` moves, and the number of sequences left out for a detour
/// (and, of those, for one that begins or ends within a half turn only: no
/// part of whole moves is the identity; QTM).
struct Truth {
    solutions: BTreeSet<Vec<u8>>,
    detours: usize,
    detours_within_half_turns: usize,
}

impl Truth {
    fn of(cube: &CubieCube, metric: Metric, max_length: usize) -> Truth {
        let mut found = Vec::new();
        search(cube, &mut Vec::new(), max_length, metric, &mut found);
        let all = found.len();
        let (detours, solutions): (Vec<_>, Vec<_>) = found
            .into_iter()
            .partition(|moves| has_detour(moves, metric));
        assert_eq!(all, detours.len() + solutions.len());
        Truth {
            detours: detours.len(),
            detours_within_half_turns: (detours.iter())
                .filter(|moves| !has_detour(moves, Metric::Htm))
                .count(),
            solutions: solutions.into_iter().collect(),
        }
    }

    /// The optimal length (if there is a solution).
    fn optimal(&self, metric: Metric) -> Option<usize> {
        self.solutions.iter().map(|s| length(metric, s)).min()
    }

    /// The solutions of at most `max_length` moves.
    fn up_to(&self, metric: Metric, max_length: usize) -> BTreeSet<Vec<u8>> {
        let short = |s: &&Vec<u8>| length(metric, s) <= max_length;
        self.solutions.iter().filter(short).cloned().collect()
    }
}

/// `Truth::of` for each cube, with the solutions up to `extra` moves above
/// its optimum (found by longer and longer searches), on all threads.
fn truths(cubes: &[CubieCube], metric: Metric, extra: usize) -> Vec<Truth> {
    let out: Vec<Mutex<Option<Truth>>> = cubes.iter().map(|_| Mutex::new(None)).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    // (Built before the threads need it.)
    ball(metric);
    std::thread::scope(|scope| {
        for _ in 0..egg_solver::thread_count().min(16) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(cube) = cubes.get(i) else {
                    break;
                };
                let optimal = (0..=usize::from(metric.max_target()))
                    .find(|&n| !Truth::of(cube, metric, n).solutions.is_empty())
                    .unwrap();
                *out[i].lock().unwrap() = Some(Truth::of(cube, metric, optimal + extra));
            });
        }
    });
    out.into_iter()
        .map(|truth| truth.into_inner().unwrap().unwrap())
        .collect()
}

// ---------------------------------------------------------------------
// The engines
// ---------------------------------------------------------------------

/// How to run a search: the recursive solver, or the share mode with
/// (threads, lanes, cubes in progress).
#[derive(Clone, Copy, Debug)]
enum Engine {
    Recursive,
    Share(usize, usize, usize),
}

const ENGINES: [Engine; 7] = [
    Engine::Recursive,
    Engine::Share(1, 1, 1),
    Engine::Share(1, 6, usize::MAX),
    Engine::Share(2, 3, 1),
    Engine::Share(3, 5, usize::MAX),
    Engine::Share(8, 4, 1),
    Engine::Share(16, 8, 2),
];

/// The lists of `cubes`, in their order.
fn lists(
    engine: Engine,
    solver: &Solver,
    cubes: &[CubieCube],
    options: &Options,
) -> Vec<Solutions> {
    match engine {
        Engine::Recursive => cubes
            .iter()
            .map(|cube| solver.all_solutions(cube, options).0)
            .collect(),
        Engine::Share(threads, lanes, max_active) => {
            let get = |i: usize| cubes.get(i).copied();
            let out = Mutex::new(vec![None; cubes.len()]);
            let done = |i: usize, list: Solutions, _, _| {
                let mut out = out.lock().unwrap();
                assert!(out[i].is_none(), "cube {i} reported twice");
                out[i] = Some(list);
            };
            share::all_solutions(solver, &get, threads, lanes, max_active, options, &done).unwrap();
            let out = out.into_inner().unwrap();
            out.into_iter()
                .map(|list| list.expect("not reported"))
                .collect()
        }
    }
}

/// Check a list against brute force's `truth` (which reaches at least the
/// list's limit): the same set, each solution once, replayed, canonical,
/// within the limit, in order; and the list's limits as `options` say.
fn check(
    what: &str,
    cube: &CubieCube,
    metric: Metric,
    options: &Options,
    list: &Solutions,
    truth: &Truth,
) {
    let optimal = truth.optimal(metric);
    let god = usize::from(metric.max_target());
    let limit =
        god.min(options.max_length.map_or(god, usize::from))
            .min(match (optimal, options.plus) {
                (Some(optimal), Some(plus)) => optimal + usize::from(plus),
                _ => god,
            });
    let within = optimal.filter(|&optimal| optimal <= limit);
    assert_eq!(list.optimal.map(usize::from), within, "{what}: optimal");
    assert_eq!(usize::from(list.max_length), limit, "{what}: max_length");
    assert!(!list.truncated, "{what}: truncated");
    assert_eq!(list.complete_length, list.max_length, "{what}");
    for solution in &list.solutions {
        let text = format_moves(solution);
        assert_eq!(
            cube.apply_moves(solution),
            Some(CubieCube::SOLVED),
            "{what}: {text} does not solve the cube"
        );
        assert!(is_canonical(solution), "{what}: {text} is not canonical");
        assert!(length(metric, solution) <= limit, "{what}: {text} too long");
    }
    let key = |s: &Vec<u8>| (length(metric, s), s.clone());
    assert!(
        list.solutions.windows(2).all(|w| key(&w[0]) < key(&w[1])),
        "{what}: not in order, or a solution twice"
    );
    let want = truth.up_to(metric, limit);
    let got: BTreeSet<Vec<u8>> = list.solutions.iter().cloned().collect();
    let show = |set: &BTreeSet<Vec<u8>>, other: &BTreeSet<Vec<u8>>| {
        let only: Vec<String> = set
            .difference(other)
            .take(5)
            .map(|s| format_moves(s))
            .collect();
        format!("{} ({})", set.difference(other).count(), only.join(" / "))
    };
    assert!(
        got == want,
        "{what}: {} solutions, brute force {}; missing {}, not in brute force {}",
        got.len(),
        want.len(),
        show(&want, &got),
        show(&got, &want)
    );
    let counts: usize = list.counts(metric).iter().map(|&(_, count)| count).sum();
    assert_eq!(counts, list.solutions.len(), "{what}: counts");
}

/// Check every engine's lists of `cubes` for each of `options` against
/// brute force (`truths`, which reach `extra` moves above each optimum:
/// the limits of `options` stay within that).
fn check_engines(
    name: &str,
    solver: &Solver,
    cubes: &[CubieCube],
    truths: &[Truth],
    options: &[Options],
    engines: &[Engine],
) {
    let metric = solver.metric();
    for options in options {
        for &engine in engines {
            let lists = lists(engine, solver, cubes, options);
            assert_eq!(lists.len(), cubes.len());
            for (i, ((cube, list), truth)) in cubes.iter().zip(&lists).zip(truths).enumerate() {
                let what = format!("{name}, {engine:?}, {options:?}, cube {i}");
                check(&what, cube, metric, options, list, truth);
            }
        }
    }
}

/// Scrambles of 1 to `max_moves` random moves, `per_length` of each.
fn scrambles(seed: u64, max_moves: usize, per_length: usize) -> Vec<CubieCube> {
    let mut rng = Rng(seed);
    let mut cubes = Vec::new();
    for moves in 1..=max_moves {
        for _ in 0..per_length {
            let sequence: Vec<u8> = (0..moves).map(|_| rng.below_u8(18)).collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&sequence).unwrap());
        }
    }
    cubes
}

/// The options whose limits are the optimum plus 0, 1 and 2 for every
/// cube: by `plus` (and all optimal).
const PLUS: [Options; 3] = [Options::all_optimal(), Options::plus(1), Options::plus(2)];

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

/// Brute force itself, on cases known by hand.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn brute_force_knows_small_cases() {
    let htm = Metric::Htm;
    // One move away: one solution; no other up to 7 moves (the shortest
    // sequences that do nothing have 8).
    let truth = Truth::of(&cube_of("R"), htm, 7);
    assert_eq!(
        truth.solutions,
        BTreeSet::from([parse_moves("R'").unwrap()])
    );
    assert_eq!(truth.detours, 0);
    // U D: one solution (U' D' and D' U' are the same).
    let truth = Truth::of(&cube_of("U D"), htm, 4);
    assert_eq!(
        truth.solutions,
        BTreeSet::from([parse_moves("U' D'").unwrap()])
    );
    // The solved cube: the empty solution; the 8-move loops are detours,
    // and there are no shorter ones.
    let truth = Truth::of(&CubieCube::SOLVED, htm, 8);
    assert_eq!(truth.solutions, BTreeSet::from([vec![]]));
    assert!(truth.detours > 0);
    assert_eq!(Truth::of(&CubieCube::SOLVED, htm, 7).detours, 0);
    // (Its 8-move loops: U2 D2 R2 L2 U2 D2 R2 L2 and the like.)
    assert!(has_detour(
        &parse_moves("U2 D2 R2 L2 U2 D2 R2 L2").unwrap(),
        htm
    ));
    assert!(!has_detour(
        &parse_moves("U2 D2 R2 L2 U2 D2 R2").unwrap(),
        htm
    ));
    // QTM: U2 is U U, one solution of 2; R U R' U' has one of 4.
    let qtm = Metric::Qtm;
    let truth = Truth::of(&cube_of("U2"), qtm, 3);
    assert_eq!(
        truth.solutions,
        BTreeSet::from([parse_moves("U2").unwrap()])
    );
    assert_eq!(truth.optimal(qtm), Some(2));
}

/// Random scrambles of 1 to 8 moves on the usual h0: all optimal solutions
/// and those 1 and 2 moves above, by every engine.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn all_solutions_match_brute_force() {
    let cubes = scrambles(2026, 8, 6);
    let truths = truths(&cubes, Metric::Htm, 2);
    let solver = solver(8, Metric::Htm);
    check_engines("h0 base 8", &solver, &cubes, &truths, &PLUS, &ENGINES);
    let solutions: usize = truths.iter().map(|t| t.solutions.len()).sum();
    eprintln!(
        "{} cubes, {solutions} solutions up to 2 above optimal",
        cubes.len()
    );
}

/// The same on a table with low bounds, where iterations of 6 moves and
/// more run on jobs of 1 to 4 moves (claimed by several threads, extended
/// and sorted in stages, split between lanes and threads), with positions
/// with symmetries among them (the symmetric-start filter would drop jobs
/// that hold solutions).
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn all_solutions_match_brute_force_on_jobs() {
    let mut cubes = scrambles(7, 8, 3);
    for scramble in [
        "U2 D2 R2 L2 F2 B2",
        "R2 L2 U2 D2",
        "U D' R L' F B' U D'",
        "U R L' F' B U' D R L'",
        "F2 B2 U D' R2 L2 U D'",
        "R U R' U' R U R' U'",
        "U D R L F B",
    ] {
        cubes.push(cube_of(scramble));
    }
    let truths = truths(&cubes, Metric::Htm, 2);
    let solver = solver(2, Metric::Htm);
    check_engines("h0 base 2", &solver, &cubes, &truths, &PLUS, &ENGINES);
    // (Symmetric positions have many optimal solutions: the checkerboard
    // of 6 half turns has one per order of its three axes' pairs.)
    let checkerboard = Truth::of(&cube_of("U2 D2 R2 L2 F2 B2"), Metric::Htm, 6);
    assert_eq!(checkerboard.solutions.len(), 6);
    let solutions: usize = truths.iter().map(|t| t.solutions.len()).sum();
    eprintln!(
        "{} cubes, {solutions} solutions up to 2 above optimal",
        cubes.len()
    );
}

/// Scrambles of `moves` random half turns: positions with many solutions
/// of nearly the same length (half turns of opposite faces commute with
/// those of the other axes' pairs).
fn half_turn_scrambles(seed: u64, moves: usize, count: usize) -> Vec<CubieCube> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|_| {
            let sequence: Vec<u8> = (0..moves).map(|_| 3 * rng.below_u8(6) + 1).collect();
            CubieCube::SOLVED.apply_moves(&sequence).unwrap()
        })
        .collect()
}

/// Positions with many solutions (half-turn scrambles, and scrambles of
/// slice-like pairs): all engines list every one of them.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn many_solutions_match_brute_force() {
    let mut cubes = half_turn_scrambles(5, 5, 6);
    cubes.extend(half_turn_scrambles(6, 7, 6));
    for scramble in [
        "U D' R L' F B'",
        "U D R2 L2 U' D'",
        "R L U2 D2 R' L' U2",
        "F B' U2 D2 F' B R2",
    ] {
        cubes.push(cube_of(scramble));
    }
    let truths = truths(&cubes, Metric::Htm, 2);
    let solutions: usize = truths.iter().map(|t| t.solutions.len()).sum();
    eprintln!(
        "{} cubes, {solutions} solutions up to 2 above optimal",
        cubes.len()
    );
    assert!(solutions > 20 * cubes.len(), "{solutions}: not many");
    for base in [8, 2] {
        let solver = solver(base, Metric::Htm);
        let name = format!("h0 base {base}");
        check_engines(&name, &solver, &cubes, &truths, &PLUS, &ENGINES);
    }
}

/// Far above the optimum, where sequences with detours exist (a shorter
/// solution with an 8-move loop: left out) next to solutions that are
/// longer for other reasons (listed): 1- and 2-move scrambles up to 10
/// moves, and the solved cube (the empty solution only).
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn detours_are_left_out() {
    let htm = Metric::Htm;
    let cubes: Vec<CubieCube> = ["R", "U2", "R U", "U D", "R2 L2", ""].map(cube_of).to_vec();
    ball(htm);
    let truths: Vec<Truth> = std::thread::scope(|scope| {
        let searches: Vec<_> = (cubes.iter())
            .map(|cube| scope.spawn(|| Truth::of(cube, htm, 10)))
            .collect();
        (searches.into_iter())
            .map(|search| search.join().unwrap())
            .collect()
    });
    for (cube, truth) in cubes.iter().zip(&truths) {
        assert!(truth.detours > 0, "no detour to leave out");
        if *cube != CubieCube::SOLVED {
            assert!(truth.solutions.len() > 1, "no longer solution");
        }
    }
    // (The solved cube: the empty solution, whatever the limit.)
    assert_eq!(truths[5].solutions, BTreeSet::from([vec![]]));
    let options = [Options::plus(8), Options::max_length(10)];
    let engines = [ENGINES[0], ENGINES[2], ENGINES[5]];
    for base in [8, 2] {
        let solver = solver(base, htm);
        let name = format!("h0 base {base}");
        check_engines(&name, &solver, &cubes, &truths, &options, &engines);
    }
    let (solutions, detours): (usize, usize) = truths
        .iter()
        .fold((0, 0), |(s, d), t| (s + t.solutions.len(), d + t.detours));
    eprintln!("{solutions} solutions up to 10 moves, {detours} sequences with detours left out");
}

/// The options: `plus 0` is all optimal is `max_length` the optimum; both
/// limits together give the smaller; a limit below the optimum gives an
/// empty list without an optimum; and the solver's one solution is among
/// the optimal ones.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn options_agree() {
    let htm = Metric::Htm;
    let mut cubes = scrambles(31, 7, 2);
    cubes.extend(half_turn_scrambles(32, 6, 4));
    let truths = truths(&cubes, htm, 2);
    let solver = solver(8, htm);
    let engines = [ENGINES[0], ENGINES[3], ENGINES[4]];
    for (i, (cube, truth)) in cubes.iter().zip(&truths).enumerate() {
        let optimal = u8::try_from(truth.optimal(htm).unwrap()).unwrap();
        let cube = std::slice::from_ref(cube);
        for engine in engines {
            let list = |options: Options| {
                let list = lists(engine, &solver, cube, &options).remove(0);
                check(
                    &format!("{engine:?}, {options:?}, cube {i}"),
                    &cube[0],
                    htm,
                    &options,
                    &list,
                    truth,
                );
                list
            };
            let all_optimal = list(Options::all_optimal());
            assert_eq!(list(Options::plus(0)), all_optimal);
            assert_eq!(
                list(Options::max_length(optimal)).solutions,
                all_optimal.solutions
            );
            let both = Options {
                max_length: Some(optimal),
                ..Options::plus(2)
            };
            assert_eq!(list(both), all_optimal);
            let both = Options {
                max_length: Some(optimal + 1),
                ..Options::plus(2)
            };
            assert_eq!(list(both).solutions, list(Options::plus(1)).solutions);
            let both = Options {
                max_length: Some(optimal + 2),
                ..Options::plus(1)
            };
            assert_eq!(list(both), list(Options::plus(1)));
            // Below the optimum: nothing, and no optimum.
            if optimal > 0 {
                let below = list(Options::max_length(optimal - 1));
                assert!(below.solutions.is_empty() && below.optimal.is_none());
                assert_eq!(below.max_length, optimal - 1);
            }
            // The one solution of `solve`, in canonical order.
            let (one, _) = solver.solve(&cube[0]);
            let one = egg_solver::metric::simplify(&one.unwrap());
            assert!(
                all_optimal.solutions.contains(&one),
                "cube {i}: {}",
                format_moves(&one)
            );
        }
    }
}

/// More solutions than `max_solutions`: the list is marked truncated, has
/// that many solutions, all of them right, all those up to
/// `complete_length` among them, and the cap is first passed one length
/// (in QTM two) above that. With room for all of them the list is whole.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn truncated_lists_say_so() {
    for metric in Metric::ALL {
        // (Cubes with several solutions; in QTM, where brute force reaches
        // less far, the optimal ones only.)
        let (mut cubes, plus, step) = match metric {
            Metric::Htm => (half_turn_scrambles(41, 6, 4), 2, 1),
            Metric::Qtm => (vec![cube_of("U2 D2 R2 L2")], 0, 2),
        };
        cubes.push(cube_of("U2 D2 R2 L2 F2 B2"));
        cubes.push(cube_of("R U R' U'"));
        let truths = truths(&cubes, metric, usize::from(plus));
        let solver = solver(8, metric);
        let mut truncated = 0;
        for engine in [ENGINES[0], ENGINES[1], ENGINES[4], ENGINES[6]] {
            for cap in [1, 2, 3, 7, 20, 1000] {
                let options = Options::plus(plus).with_max_solutions(cap);
                let lists = lists(engine, &solver, &cubes, &options);
                for (i, ((cube, list), truth)) in cubes.iter().zip(&lists).zip(&truths).enumerate()
                {
                    let what = format!("{metric}, {engine:?}, cap {cap}, cube {i}");
                    let limit = truth.optimal(metric).unwrap() + usize::from(plus);
                    let all = truth.up_to(metric, limit);
                    if all.len() <= cap {
                        check(&what, cube, metric, &options, list, truth);
                        continue;
                    }
                    truncated += 1;
                    assert!(list.truncated, "{what}: not truncated");
                    assert_eq!(list.solutions.len(), cap, "{what}");
                    assert_eq!(
                        list.optimal.map(usize::from),
                        truth.optimal(metric),
                        "{what}"
                    );
                    assert_eq!(usize::from(list.max_length), limit, "{what}");
                    let listed: BTreeSet<Vec<u8>> = list.solutions.iter().cloned().collect();
                    assert_eq!(listed.len(), cap, "{what}: a solution twice");
                    assert!(listed.is_subset(&all), "{what}: a wrong solution");
                    let complete = usize::from(list.complete_length);
                    assert!(complete < limit, "{what}: complete {complete}");
                    let surely = truth.up_to(metric, complete);
                    assert!(surely.is_subset(&listed), "{what}: complete {complete}");
                    assert!(surely.len() <= cap, "{what}");
                    let next = truth.up_to(metric, complete + step);
                    assert!(next.len() > cap, "{what}: stopped early");
                }
            }
        }
        assert!(truncated >= 8, "{metric}: {truncated} truncated lists");
    }
}

/// Scrambles of 0 to `max_quarter_turns` quarter turns, `per_length` each.
fn quarter_turn_scrambles(
    seed: u64,
    max_quarter_turns: usize,
    per_length: usize,
) -> Vec<CubieCube> {
    let mut rng = Rng(seed);
    let mut cubes = Vec::new();
    for turns in 0..=max_quarter_turns {
        for _ in 0..per_length {
            let sequence: Vec<u8> = (0..turns)
                .map(|_| 3 * rng.below_u8(6) + 2 * rng.below_u8(2))
                .collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&sequence).unwrap());
        }
    }
    cubes
}

/// The quarter-turn metric: all optimal solutions and those 2 and 4
/// quarter turns above (lengths keep their parity), by every engine, on
/// both tables; half turns in the solutions are written as such.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_solutions_match_brute_force() {
    let qtm = Metric::Qtm;
    let mut cubes = quarter_turn_scrambles(77, 7, 4);
    for scramble in [
        "U2",
        "U2 D2",
        "R2 L2 U2 D2",
        "F2 R2 F2",
        "U D' R2",
        "R U2 R'",
    ] {
        cubes.push(cube_of(scramble));
    }
    let truths = truths(&cubes, qtm, 4);
    let solutions: usize = truths.iter().map(|t| t.solutions.len()).sum();
    eprintln!(
        "QTM: {} cubes, {solutions} solutions up to 4 above optimal",
        cubes.len()
    );
    // (`plus 1` and `plus 3` list what `plus 0` and `plus 2` do.)
    let options = [
        Options::all_optimal(),
        Options::plus(1),
        Options::plus(2),
        Options::plus(3),
        Options::plus(4),
    ];
    for base in [8, 2] {
        let solver = solver(base, qtm);
        let name = format!("QTM, h0 base {base}");
        check_engines(&name, &solver, &cubes, &truths, &options, &ENGINES);
    }
    assert!(truths
        .iter()
        .any(|t| t.solutions.iter().any(|s| s.iter().any(|mv| mv % 3 == 1))));
}

/// QTM detours: the shortest loops have 12 quarter turns, and a detour
/// can begin or end within a half turn (`F2 w` with `F w` a loop is `F`
/// and a loop): a quarter turn up to 12 above optimal.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_detours_are_left_out() {
    let qtm = Metric::Qtm;
    let cubes: Vec<CubieCube> = ["F", ""].map(cube_of).to_vec();
    let truths = truths(&cubes, qtm, 12);
    for truth in &truths {
        assert!(truth.detours > 0, "no detour to leave out");
    }
    // (No loop of fewer than 12 quarter turns.)
    assert_eq!(Truth::of(&CubieCube::SOLVED, qtm, 11).detours, 0);
    assert!(truths[0].solutions.len() > 1, "no longer solution");
    assert!(
        truths[0].detours_within_half_turns > 0,
        "no detour within a half turn"
    );
    let options = [Options::plus(12), Options::max_length(13)];
    let solver = solver(8, qtm);
    let engines = [ENGINES[0], ENGINES[4]];
    check_engines(
        "QTM, h0 base 8",
        &solver,
        &cubes,
        &truths,
        &options,
        &engines,
    );
    let (solutions, detours): (usize, usize) = truths
        .iter()
        .fold((0, 0), |(s, d), t| (s + t.solutions.len(), d + t.detours));
    eprintln!(
        "QTM: {solutions} solutions up to 12 above optimal, {detours} with detours left out \
         ({} within half turns)",
        truths[0].detours_within_half_turns
    );
}

/// Deeper cubes (11 and 12 random moves; in QTM 9 and 10), where the
/// usual h0 searches on jobs and brute force is out of reach: the share
/// mode lists what the recursive solver lists, and both replay.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn engines_agree_on_deeper_cubes() {
    for (metric, moves, plus) in [(Metric::Htm, 11, 1), (Metric::Qtm, 9, 2)] {
        let mut rng = Rng(2027);
        let cubes: Vec<CubieCube> = (0..4)
            .map(|i| {
                // (Canonical sequences: few are shorter than they look.)
                let mut sequence: Vec<u8> = Vec::new();
                while sequence.len() < moves + i % 2 {
                    let mv = rng.below_u8(18);
                    if may_follow(sequence.last().copied(), mv) {
                        sequence.push(mv);
                    }
                }
                CubieCube::SOLVED.apply_moves(&sequence).unwrap()
            })
            .collect();
        let solver = solver(8, metric);
        let options = Options::plus(plus);
        let want = lists(Engine::Recursive, &solver, &cubes, &options);
        for (cube, list) in cubes.iter().zip(&want) {
            assert!(!list.solutions.is_empty() && !list.truncated);
            for solution in &list.solutions {
                assert_eq!(cube.apply_moves(solution), Some(CubieCube::SOLVED));
                assert!(is_canonical(solution));
            }
        }
        for engine in [ENGINES[2], ENGINES[5], ENGINES[6]] {
            assert_eq!(
                lists(engine, &solver, &cubes, &options),
                want,
                "{metric} {engine:?}"
            );
        }
        let lengths: Vec<_> = want.iter().map(|list| list.optimal.unwrap_or(0)).collect();
        eprintln!("{metric}: optimal lengths {lengths:?}");
    }
}

// ---------------------------------------------------------------------
// Symmetric positions
// ---------------------------------------------------------------------

/// The kind of a position's symmetry: the number of its symmetries (the
/// identity included) and whether it has an antisymmetry.
fn symmetry_kind(cube: &CubieCube) -> (usize, bool) {
    let cube = egg_solver::simd::Cube::from_cubie(cube);
    let filter = egg_solver::symstart::SymFilter::new(cube, cube.inverse());
    (filter.group_order(), filter.antisymmetric())
}

/// Positions with symmetries of many kinds: of each kind (`symmetry_kind`)
/// with a symmetry, the first `per_kind` positions among the canonical
/// sequences of at most 5 moves. (Their kinds: 2, 4 or 8 symmetries with
/// and without an antisymmetry, and 16 with one.)
fn short_symmetric_positions(per_kind: usize) -> Vec<CubieCube> {
    fn walk(
        cube: &CubieCube,
        sequence: &mut Vec<u8>,
        kinds: &mut std::collections::BTreeMap<(usize, bool), Vec<CubieCube>>,
        per_kind: usize,
    ) {
        if !sequence.is_empty() {
            let kind = symmetry_kind(cube);
            let found = kinds.entry(kind).or_default();
            if kind.0 > 1 && found.len() < per_kind && !found.contains(cube) {
                found.push(*cube);
            }
        }
        if sequence.len() == 5 {
            return;
        }
        for mv in 0..18u8 {
            if may_follow(sequence.last().copied(), mv) {
                sequence.push(mv);
                walk(&turn(cube, mv), sequence, kinds, per_kind);
                sequence.pop();
            }
        }
    }
    let mut kinds = std::collections::BTreeMap::new();
    walk(&CubieCube::SOLVED, &mut Vec::new(), &mut kinds, per_kind);
    kinds.into_values().flatten().collect()
}

/// The kinds of symmetry among `cubes`, for the test log.
fn kinds_of(cubes: &[CubieCube]) -> String {
    let kinds: BTreeSet<(usize, bool)> = cubes.iter().map(symmetry_kind).collect();
    let name = |&(order, anti): &(usize, bool)| format!("{order}{}", if anti { "a" } else { "" });
    kinds.iter().map(name).collect::<Vec<_>>().join(" ")
}

/// Cubes with symmetries, where the searches run only the jobs that the
/// symmetric-start filter keeps and add each solution's images: the lists
/// are brute force's, for positions with 2 to 16 symmetries, with and
/// without an antisymmetry, on a table whose low bounds make iterations of
/// 5 to 9 moves run on jobs of 1 to 4 moves (and on the usual one). And
/// the filter did drop jobs: fewer nodes than without it, the same lists.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn symmetric_positions_match_brute_force() {
    let htm = Metric::Htm;
    let cubes = short_symmetric_positions(2);
    let kinds: BTreeSet<(usize, bool)> = cubes.iter().map(symmetry_kind).collect();
    eprintln!(
        "{} symmetric positions of at most 5 moves, kinds (symmetries, a: antisymmetric): {}",
        cubes.len(),
        kinds_of(&cubes)
    );
    // (Small and larger groups, with and without an antisymmetry.)
    let want = [
        (2, false),
        (2, true),
        (4, false),
        (4, true),
        (8, false),
        (8, true),
        (16, true),
    ];
    assert_eq!(kinds, BTreeSet::from(want));
    let truths = truths(&cubes, htm, 4);
    let options = [Options::all_optimal(), Options::plus(2), Options::plus(4)];
    for base in [8, 1] {
        let solver = solver(base, htm);
        let name = format!("symmetric, h0 base {base}");
        check_engines(&name, &solver, &cubes, &truths, &options, &ENGINES);
    }
    // Without the filter (`sym_start` off): the same lists, more nodes.
    let filtered = solver(1, htm);
    let mut plain = solver(1, htm);
    plain.sym_start = false;
    let (mut nodes, mut plain_nodes) = (0, 0);
    for cube in &cubes {
        let (list, stats) = filtered.all_solutions(cube, &Options::plus(4));
        let (plain_list, plain_stats) = plain.all_solutions(cube, &Options::plus(4));
        assert_eq!(list, plain_list);
        assert!(stats.nodes <= plain_stats.nodes);
        nodes += stats.nodes;
        plain_nodes += plain_stats.nodes;
    }
    eprintln!("nodes with the filter {nodes}, without {plain_nodes}");
    assert!(
        2 * nodes < plain_nodes,
        "the filter dropped little: {nodes} of {plain_nodes}"
    );
    // The share mode's jobs are filtered too.
    let share_nodes = |solver: &Solver| {
        let get = |i: usize| cubes.get(i).copied();
        let nodes = std::sync::atomic::AtomicU64::new(0);
        let done = |_, _, stats: egg_solver::solver::Stats, _| {
            nodes.fetch_add(stats.nodes, std::sync::atomic::Ordering::Relaxed);
        };
        share::all_solutions(solver, &get, 2, 4, 1, &Options::plus(4), &done).unwrap();
        nodes.into_inner()
    };
    let (nodes, plain_nodes) = (share_nodes(&filtered), share_nodes(&plain));
    eprintln!("share mode: nodes with the filter {nodes}, without {plain_nodes}");
    assert!(
        2 * nodes < plain_nodes,
        "the filter dropped little: {nodes} of {plain_nodes}"
    );
    let solutions: usize = truths.iter().map(|t| t.solutions.len()).sum();
    eprintln!("{solutions} solutions up to 4 above optimal");
}

/// Longer symmetric positions (6 to 8 moves: the checkerboard with all 48
/// symmetries, slice patterns, positions that are their own inverse), up
/// to 2 above optimal, on the usual h0 and on one of base 2 (jobs of 1 to 4
/// moves in the iterations of 6 to 10).
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn longer_symmetric_positions_match_brute_force() {
    let htm = Metric::Htm;
    let cubes: Vec<CubieCube> = [
        "U2 D2 R2 L2 F2 B2",
        "U D' R L' F B' U D'",
        "U R L' F' B U' D R L'",
        "F2 B2 U D' R2 L2 U D'",
        "U D R L F B",
        "U D' R L' F B'",
        "R2 L2 U D' F2 B2 U' D",
        "U R U' R' U' F' U F",
        "R U2 R' L' U2 L",
    ]
    .map(cube_of)
    .to_vec();
    eprintln!("longer symmetric positions, kinds: {}", kinds_of(&cubes));
    let kinds: BTreeSet<(usize, bool)> = cubes.iter().map(symmetry_kind).collect();
    // (Among them a group of order 6, and all 48 symmetries.)
    assert!(
        kinds.contains(&(6, true)) && kinds.contains(&(48, true)),
        "{kinds:?}"
    );
    let truths = truths(&cubes, htm, 2);
    for base in [8, 2] {
        let solver = solver(base, htm);
        let name = format!("longer symmetric, h0 base {base}");
        check_engines(&name, &solver, &cubes, &truths, &PLUS, &ENGINES);
    }
    let solutions: usize = truths.iter().map(|t| t.solutions.len()).sum();
    eprintln!("{solutions} solutions up to 2 above optimal");
}

/// Cubes with 3 and 24 symmetries, which no position of few moves has
/// (those above have 2, 4, 6, 8, 16 or 48), and `TWELVE_SYMMETRIES` with
/// 12: their symmetries include a turn about a corner. They were built
/// from twists, flips and cycles of pieces on the orbits of such a group.
/// `(scramble, symmetries, optimal length, optimal solutions, solutions up
/// to one move more)`.
///
/// Brute force does not reach these lengths. The numbers are those of a
/// build from before the symmetric-start filter, which searched every job
/// (commit 17be652, on an h8 table, 2026-11-23): the build with the filter
/// listed the same solutions, one for one, for 39 such cubes, these among
/// them.
const SYMMETRIC_CUBES: [(&str, usize, usize, usize, usize); 6] = [
    ("L F' D2 B D B' D2 F D' L'", 3, 10, 6, 30),
    ("U B R' F2 R B' R' F2 R U'", 3, 10, 12, 36),
    ("U' B R2 L2 F' L' F L2 R2 B' L U", 3, 12, 12, 174),
    ("U F' L2 B2 R B' L B' L B' R' B F U", 3, 14, 24, 132),
    (
        "L2 U' B2 L R F2 U' R2 D2 R' F2 D U B2 R' U2",
        24,
        16,
        2448,
        8304,
    ),
    (
        "D R2 L2 B2 F2 U' L2 B F2 L2 R2 D2 U2 F' R2 U2",
        24,
        16,
        2448,
        8304,
    ),
];

/// A cube with 12 symmetries (six flipped edges around a corner-to-corner
/// axis), of 17 moves with 24 optimal solutions. Every such cube found is
/// of 17 or 18 moves, which an h0 table makes slow.
const TWELVE_SYMMETRIES: (&str, usize, usize, usize) =
    ("U' F' R D F' L' B2 F2 D R' F U R' B' L2 R2 U2", 12, 17, 24);

/// A list of a `SYMMETRIC_CUBES` entry on its own terms: `count` solutions
/// of `length` moves at most, the first of `optimal` moves; each solves
/// the cube, is canonical and comes once, in order.
fn check_symmetric_list(
    what: &str,
    cube: &CubieCube,
    list: &Solutions,
    (optimal, length, count): (usize, usize, usize),
) {
    let htm = Metric::Htm;
    assert_eq!(list.optimal.map(usize::from), Some(optimal), "{what}");
    assert_eq!(usize::from(list.max_length), length, "{what}");
    assert_eq!(list.complete_length, list.max_length, "{what}");
    assert!(!list.truncated && list.stopped.is_none(), "{what}");
    assert_eq!(list.solutions.len(), count, "{what}: solutions");
    for solution in &list.solutions {
        let text = format_moves(solution);
        assert_eq!(
            cube.apply_moves(solution),
            Some(CubieCube::SOLVED),
            "{what}: {text} does not solve the cube"
        );
        assert!(is_canonical(solution), "{what}: {text} is not canonical");
        assert!(
            (optimal..=length).contains(&solution.len()),
            "{what}: {text}"
        );
    }
    let key = |s: &Vec<u8>| (s.len(), s.clone());
    assert!(
        list.solutions.windows(2).all(|w| key(&w[0]) < key(&w[1])),
        "{what}: not in order, or a solution twice"
    );
    let counts: usize = list.counts(htm).iter().map(|&(_, count)| count).sum();
    assert_eq!(counts, count, "{what}: counts");
}

/// The lists of cubes with 3 and 24 symmetries (`SYMMETRIC_CUBES`): the
/// numbers of optimal solutions for all of them; for the cubes of at most
/// 14 moves also the solutions up to one move above optimal, and the same
/// lists without the filter, which take more nodes wherever the search
/// runs on jobs. That is from 12 moves on the usual h0 (shorter iterations
/// are one job, with nothing for the filter to drop), so the 10-move cubes
/// are also listed on the h0 of base 2 (jobs from 6 moves). The share mode
/// with one cube at a time on up to 16 threads, and for the short cubes
/// the recursive solver.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn cubes_with_3_and_24_symmetries_have_the_known_lists() {
    let htm = Metric::Htm;
    let share = Engine::Share(egg_solver::thread_count().min(16), 8, 1);
    for (scramble, symmetries, optimal, count, count_plus_1) in SYMMETRIC_CUBES {
        let cube = cube_of(scramble);
        assert_eq!(symmetry_kind(&cube).0, symmetries, "{scramble}");
        let short = optimal <= 14;
        let bases: &[u8] = if optimal <= 10 { &[8, 2] } else { &[8] };
        for &base in bases {
            let filtered = solver(base, htm);
            let mut plain = solver(base, htm);
            plain.sym_start = false;
            let mut cases = vec![(Options::all_optimal(), (optimal, optimal, count))];
            if short {
                cases.push((Options::plus(1), (optimal, optimal + 1, count_plus_1)));
            }
            for (options, expected) in cases {
                let name = format!("{scramble}, h0 base {base}, {options:?}");
                let list = lists(share, &filtered, &[cube], &options).remove(0);
                check_symmetric_list(&format!("{name}, {share:?}"), &cube, &list, expected);
                if !short {
                    continue;
                }
                let (recursive, stats) = filtered.all_solutions(&cube, &options);
                assert_eq!(recursive, list, "{name}: the recursive solver's list");
                // Without the filter: the same list, and more nodes where
                // the last iteration runs on jobs.
                let (plain_list, plain_stats) = plain.all_solutions(&cube, &options);
                assert_eq!(plain_list, list, "{name}: without the filter");
                let on_jobs = base == 2 || expected.1 >= 12;
                assert!(
                    stats.nodes <= plain_stats.nodes
                        && (stats.nodes < plain_stats.nodes || !on_jobs),
                    "{name}: {} nodes, {} without the filter",
                    stats.nodes,
                    plain_stats.nodes
                );
            }
        }
    }
}

/// The list of the cube with 12 symmetries (`TWELVE_SYMMETRIES`): its 24
/// optimal solutions. Not run by default: on h0 it takes a minute and a
/// half with 16 threads (`cargo test --test all_solutions -- --ignored`).
#[test]
#[ignore = "about 90 s on 16 threads"]
fn cube_with_12_symmetries_has_the_known_list() {
    let (scramble, symmetries, optimal, count) = TWELVE_SYMMETRIES;
    let cube = cube_of(scramble);
    assert_eq!(symmetry_kind(&cube).0, symmetries, "{scramble}");
    let share = Engine::Share(egg_solver::thread_count().min(16), 8, 1);
    let solver = solver(8, Metric::Htm);
    let list = lists(share, &solver, &[cube], &Options::all_optimal()).remove(0);
    check_symmetric_list(scramble, &cube, &list, (optimal, optimal, count));
}

/// Symmetric positions in the quarter-turn metric (those of at most 6
/// quarter turns), up to 4 above optimal, on both tables.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_symmetric_positions_match_brute_force() {
    let qtm = Metric::Qtm;
    let mut cubes = short_symmetric_positions(2);
    cubes.push(cube_of("U2 D2 R2"));
    cubes.push(cube_of("U D' R L'"));
    ball(qtm);
    cubes.retain(|cube| ball(qtm).get(cube).is_some_and(|&d| d <= 6));
    let kinds: BTreeSet<(usize, bool)> = cubes.iter().map(symmetry_kind).collect();
    eprintln!(
        "QTM: {} symmetric positions, kinds: {}",
        cubes.len(),
        kinds_of(&cubes)
    );
    assert!(kinds.len() >= 6, "{kinds:?}");
    let truths = truths(&cubes, qtm, 4);
    let options = [Options::all_optimal(), Options::plus(2), Options::plus(4)];
    for base in [8, 1] {
        let solver = solver(base, qtm);
        let name = format!("QTM symmetric, h0 base {base}");
        check_engines(&name, &solver, &cubes, &truths, &options, &ENGINES);
    }
}

// ---------------------------------------------------------------------
// Stopping a list: the time limit and the cancel flag
// ---------------------------------------------------------------------

/// A scramble whose list up to `LONG` moves takes long (seconds with the
/// usual h0; its solutions up to `SHORT` moves take milliseconds).
const LONG_SCRAMBLE: &str = "R U F2 D L' B";
const LONG: u8 = 15;
const SHORT: u8 = 12;

/// Check a list that was (or may have been) stopped early: every solution
/// replays, is canonical and has no detour (so it is one of the full
/// list's), each once and in order; all those of at most `complete_length`
/// moves are there (`full`: a list that reaches at least that far); and a
/// list that was not stopped is the whole one.
fn check_stopped(what: &str, cube: &CubieCube, list: &Solutions, full: &Solutions) {
    let htm = Metric::Htm;
    for solution in &list.solutions {
        let text = format_moves(solution);
        assert_eq!(
            cube.apply_moves(solution),
            Some(CubieCube::SOLVED),
            "{what}: {text}"
        );
        assert!(is_canonical(solution), "{what}: {text} is not canonical");
        assert!(!has_detour(solution, htm), "{what}: {text} has a detour");
        assert!(
            solution.len() <= usize::from(list.max_length),
            "{what}: {text}"
        );
    }
    let key = |s: &Vec<u8>| (s.len(), s.clone());
    assert!(
        list.solutions.windows(2).all(|w| key(&w[0]) < key(&w[1])),
        "{what}: not in order, or a solution twice"
    );
    assert!(!list.truncated, "{what}");
    let complete = usize::from(list.complete_length);
    assert!(
        complete <= usize::from(full.max_length),
        "{what}: complete {complete}"
    );
    let up_to = |list: &Solutions| -> Vec<Vec<u8>> {
        let short = |s: &&Vec<u8>| s.len() <= complete;
        list.solutions.iter().filter(short).cloned().collect()
    };
    assert_eq!(up_to(list), up_to(full), "{what}: complete {complete}");
    if list.stopped.is_none() {
        assert_eq!(list.complete_length, list.max_length, "{what}");
    }
}

/// A tiny time limit on a long list: the search returns at once, the list
/// says that it was stopped by the clock, and holds what was found, with
/// every solution up to `complete_length`; in a batch each cube has its
/// own clock. A limit that is not reached changes nothing.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn time_limit_stops_a_list() {
    use egg_solver::solutions::Stopped;
    use std::time::{Duration, Instant};
    let htm = Metric::Htm;
    let solver = solver(8, htm);
    let cube = cube_of(LONG_SCRAMBLE);
    let full = solver.all_solutions(&cube, &Options::max_length(SHORT)).0;
    assert!(full.stopped.is_none() && full.solutions.len() > 10);
    let tiny = Options::max_length(LONG).with_time_limit(Duration::from_micros(300));
    for engine in ENGINES {
        let start = Instant::now();
        let list = lists(engine, &solver, std::slice::from_ref(&cube), &tiny).remove(0);
        let seconds = start.elapsed().as_secs_f64();
        let what = format!("{engine:?}");
        assert_eq!(list.stopped, Some(Stopped::Time), "{what}");
        assert!(seconds < 20.0, "{what}: stopped after {seconds} s");
        assert!(list.complete_length < LONG, "{what}");
        assert_eq!(list.max_length, LONG, "{what}");
        // (Should it have got beyond `SHORT`: compared up to there.)
        let mut list = list;
        list.complete_length = list.complete_length.min(SHORT);
        check_stopped(&what, &cube, &list, &full);
    }
    // A batch: every cube is stopped by its own clock, the solved cube
    // (listed without a search) is not.
    let cubes = [
        cube,
        cube_of("F2 D L2 B' U R"),
        CubieCube::SOLVED,
        cube_of("U R2 F B R B2"),
    ];
    let batch = lists(Engine::Share(4, 4, usize::MAX), &solver, &cubes, &tiny);
    let stopped: Vec<bool> = batch.iter().map(|list| list.stopped.is_some()).collect();
    assert_eq!(stopped, [true, true, false, true]);
    assert_eq!(batch[2].solutions, [Vec::<u8>::new()]);
    // A limit that is not reached: the list without it.
    let roomy = Options::max_length(SHORT).with_time_limit(Duration::from_secs(3600));
    for engine in [ENGINES[0], ENGINES[4]] {
        let list = lists(engine, &solver, std::slice::from_ref(&cube), &roomy).remove(0);
        assert_eq!(list, full, "{engine:?}");
    }
}

/// The cancel flag, set by another thread during a long list: the search
/// returns, the list says that it was cancelled and holds what was found.
/// A flag that is set from the start: the recursive solver returns at
/// once; the share mode starts no cube. A flag that is never set changes
/// nothing.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn cancel_stops_a_list() {
    use egg_solver::solutions::Stopped;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};
    let htm = Metric::Htm;
    let solver = solver(8, htm);
    let cube = cube_of(LONG_SCRAMBLE);
    let full = solver.all_solutions(&cube, &Options::max_length(SHORT)).0;
    for engine in ENGINES {
        let cancel = AtomicBool::new(false);
        // (Set when the engine takes the cube: the flag is to be set while
        // its list is searched, not before it starts.)
        let taken = AtomicBool::new(false);
        let options = Options::max_length(LONG).with_cancel(&cancel);
        let start = Instant::now();
        let list = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !taken.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(1));
                }
                std::thread::sleep(Duration::from_millis(30));
                cancel.store(true, Ordering::Relaxed);
            });
            match engine {
                Engine::Recursive => {
                    taken.store(true, Ordering::Relaxed);
                    solver.all_solutions(&cube, &options).0
                }
                Engine::Share(threads, lanes, max_active) => {
                    let get = |i: usize| {
                        taken.store(true, Ordering::Relaxed);
                        (i == 0).then_some(cube)
                    };
                    let out = Mutex::new(None);
                    let done = |_, list: Solutions, _, _| {
                        let previous = out.lock().unwrap().replace(list);
                        assert!(previous.is_none(), "reported twice");
                    };
                    share::all_solutions(
                        &solver, &get, threads, lanes, max_active, &options, &done,
                    )
                    .unwrap();
                    out.into_inner().unwrap().expect("not reported")
                }
            }
        });
        let seconds = start.elapsed().as_secs_f64();
        let what = format!("{engine:?}");
        assert_eq!(list.stopped, Some(Stopped::Cancelled), "{what}");
        assert!(seconds < 20.0, "{what}: returned after {seconds} s");
        assert!(list.complete_length < LONG, "{what}");
        // (Its complete lengths may go beyond `SHORT`: compared up to there.)
        let mut list = list;
        list.complete_length = list.complete_length.min(SHORT);
        check_stopped(&what, &cube, &list, &full);
    }
    // Set from the start.
    let cancel = AtomicBool::new(true);
    let options = Options::max_length(LONG).with_cancel(&cancel);
    let (list, stats) = solver.all_solutions(&cube, &options);
    assert_eq!(list.stopped, Some(Stopped::Cancelled));
    assert!(list.solutions.is_empty() && list.optimal.is_none() && stats.nodes == 0);
    assert!(
        list.complete_length < 6,
        "below the optimum: {}",
        list.complete_length
    );
    let reported = std::sync::atomic::AtomicUsize::new(0);
    let get = |i: usize| (i < 3).then_some(cube);
    let done = |_, _, _, _| {
        reported.fetch_add(1, Ordering::Relaxed);
    };
    share::all_solutions(&solver, &get, 3, 4, 1, &options, &done).unwrap();
    assert_eq!(reported.into_inner(), 0, "a cube was started");
    // Never set: the list without the flag.
    let cancel = AtomicBool::new(false);
    let options = Options::max_length(SHORT).with_cancel(&cancel);
    for engine in [ENGINES[0], ENGINES[5]] {
        let list = lists(engine, &solver, std::slice::from_ref(&cube), &options).remove(0);
        assert_eq!(list, full, "{engine:?}");
    }
}
