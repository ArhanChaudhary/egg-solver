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
