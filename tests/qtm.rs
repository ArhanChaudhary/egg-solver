//! The quarter-turn metric (QTM): the solvers' QTM lengths against brute
//! force (a breadth-first search over the 12 quarter turns, met in the
//! middle), positions with known QTM distances, and consistency with HTM on
//! random cubes. Every solution is replayed. The tables are h0 tables made
//! in memory (no table file needed): HTM ones bound the QTM search too.

use egg_solver::coord::{cocsep, N_SLOTS};
use egg_solver::cubie::{format_moves, parse_moves, CubieCube, Move, Rng};
use egg_solver::metric::Metric;
use egg_solver::simd::Cube;
use egg_solver::solver::Solver;
use egg_solver::tables::{
    entry_class, entry_depth, entry_sym_to_rep, h48_bytes, h48_rows, Cocsep, H48Meta, H48Table,
    Memory, ROW_BYTES,
};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The cocsep table, HTM depths (built once).
fn corners() -> &'static Cocsep {
    static CORNERS: OnceLock<Cocsep> = OnceLock::new();
    CORNERS.get_or_init(|| Cocsep::generate().unwrap())
}

/// The cocsep table with QTM depths (built once).
fn corners_qtm() -> &'static Cocsep {
    static CORNERS: OnceLock<Cocsep> = OnceLock::new();
    CORNERS.get_or_init(|| Cocsep::generate_metric(Metric::Qtm).unwrap())
}

/// A QTM h0 table with base 9 (generated once).
fn h0_qtm() -> &'static H48Table {
    static TABLE: OnceLock<H48Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        let threads = egg_solver::thread_count();
        unsafe { egg_solver::generate_metric(corners(), 0, 9, Metric::Qtm, None, &[], threads) }
            .unwrap()
            .0
    })
}

/// An HTM h0 table with base 8 (generated once).
fn h0() -> &'static H48Table {
    static TABLE: OnceLock<H48Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        unsafe { egg_solver::generate(corners(), 0, 8, None, &[], egg_solver::thread_count()) }
            .unwrap()
            .0
    })
}

/// The 12 quarter turns.
fn quarter_turns() -> impl Iterator<Item = Move> {
    Move::ALL.into_iter().filter(|mv| mv.index() % 3 != 1)
}

/// Every position within `radius` quarter turns of `start`, with its QTM
/// distance from `start`.
fn qtm_ball(start: Cube, radius: u8) -> HashMap<[u8; 32], u8> {
    let mut seen = HashMap::new();
    seen.insert(start.to_bytes(), 0u8);
    let mut frontier = vec![start];
    for distance in 1..=radius {
        let mut next = Vec::new();
        for cube in frontier {
            for mv in quarter_turns() {
                let child = cube.apply_move(mv);
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(child.to_bytes()) {
                    e.insert(distance);
                    next.push(child);
                }
            }
        }
        frontier = next;
    }
    seen
}

/// The exact QTM distance of `cube` if it is at most `2 * radius` (else
/// `None`): the least sum of the distances of a position within `radius`
/// of both `cube` and solved (`solved_ball`, of that radius).
fn qtm_distance(cube: Cube, solved_ball: &HashMap<[u8; 32], u8>, radius: u8) -> Option<u8> {
    qtm_ball(cube, radius)
        .iter()
        .filter_map(|(position, &d)| solved_ball.get(position).map(|&e| d + e))
        .min()
}

/// Positions (as bytes) and their QTM distances.
type Ball = HashMap<[u8; 32], u8>;

/// The balls around solved (built once each).
fn solved_ball(radius: u8) -> &'static Ball {
    static BALLS: OnceLock<std::sync::Mutex<HashMap<u8, &'static Ball>>> = OnceLock::new();
    let balls = BALLS.get_or_init(Default::default);
    let mut balls = balls.lock().unwrap();
    balls
        .entry(radius)
        .or_insert_with(|| Box::leak(Box::new(qtm_ball(Cube::solved(), radius))))
}

/// Solve with the recursive solver in `metric`, on all threads.
fn par_solve(cubes: &[CubieCube], solver: &Solver) -> Vec<Vec<u8>> {
    let out = std::sync::Mutex::new(vec![vec![]; cubes.len()]);
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..egg_solver::thread_count() {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= cubes.len() {
                    break;
                }
                let (solution, _) = solver.solve(&cubes[i]);
                out.lock().unwrap()[i] = solution.expect("no solution");
            });
        }
    });
    out.into_inner().unwrap()
}

fn qtm_solver() -> Solver<'static> {
    let solver = Solver::with_metric(corners(), h0(), Metric::Qtm).unwrap();
    assert_eq!(solver.metric(), Metric::Qtm);
    solver
}

fn cube_of(moves: &str) -> CubieCube {
    CubieCube::SOLVED
        .apply_moves(&parse_moves(moves).unwrap())
        .unwrap()
}

/// Random scrambles of 0..=`max_quarter_turns` quarter turns (as random
/// moves of all 18, kept while their QTM length fits), `per_length` each.
fn short_scrambles(seed: u64, max_quarter_turns: usize, per_length: usize) -> Vec<CubieCube> {
    let mut rng = Rng(seed);
    let mut cubes = Vec::new();
    for len in 0..=max_quarter_turns {
        for _ in 0..per_length {
            let mut moves = Vec::new();
            while Metric::Qtm.length(&moves) < len {
                let mv = rng.below_u8(18);
                if Metric::Qtm.length(&moves) + usize::from(Metric::Qtm.move_cost(mv)) <= len {
                    moves.push(mv);
                }
            }
            cubes.push(CubieCube::SOLVED.apply_moves(&moves).unwrap());
        }
    }
    cubes
}

/// Check a QTM solution: it solves the cube, is canonical and has the
/// expected QTM length.
fn check(cube: &CubieCube, solution: &[u8], length: usize) {
    assert_eq!(
        cube.apply_moves(solution),
        Some(CubieCube::SOLVED),
        "{} does not solve the cube",
        format_moves(solution)
    );
    assert_eq!(
        Metric::Qtm.length(solution),
        length,
        "{} is not optimal",
        format_moves(solution)
    );
    assert_eq!(
        egg_solver::metric::simplify(solution),
        solution,
        "not canonical"
    );
}

/// The recursive solver's QTM lengths equal brute force on random cubes up
/// to 10 quarter turns from solved.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_lengths_match_brute_force() {
    let radius = 5;
    let ball = solved_ball(radius);
    let cubes = short_scrambles(2026, 10, 24);
    let solutions = par_solve(&cubes, &qtm_solver());
    for (cube, solution) in cubes.iter().zip(&solutions) {
        let distance = qtm_distance(Cube::from_cubie(cube), ball, radius).unwrap();
        check(cube, solution, usize::from(distance));
    }
}

/// Known QTM distances: the checkerboard (6 half turns) is 12q (checked by
/// brute force here too), U2 alone 2q, a quarter turn 1q; the superflip
/// (20 in HTM) is 24q.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_known_positions() {
    let solver = qtm_solver();
    let checkerboard = cube_of("U2 D2 F2 B2 L2 R2");
    let ball = solved_ball(6);
    assert_eq!(
        qtm_distance(Cube::from_cubie(&checkerboard), ball, 6),
        Some(12)
    );
    for (moves, length) in [
        ("U2", 2),
        ("R'", 1),
        ("U2 D2 F2 B2 L2 R2", 12),
        ("R U R' U'", 4),
    ] {
        let cube = cube_of(moves);
        let (solution, _) = solver.solve(&cube);
        check(&cube, &solution.unwrap(), length);
    }
    let (solution, _) = solver.solve(&CubieCube::SOLVED);
    assert_eq!(solution, Some(vec![]));
}

/// Scrambles of `moves` random moves (all 18), `count` of them.
fn random_scrambles(seed: u64, moves: usize, count: usize) -> Vec<CubieCube> {
    let mut rng = Rng(seed);
    (0..count)
        .map(|_| {
            let seq: Vec<u8> = (0..moves).map(|_| rng.below_u8(18)).collect();
            CubieCube::SOLVED.apply_moves(&seq).unwrap()
        })
        .collect()
}

/// QTM and HTM on the same scrambles (12 random moves): the QTM length is at
/// least the HTM length, at most the HTM solution's QTM length, and has the
/// parity of the corner permutation; both solutions replay.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_at_least_htm() {
    let cubes = random_scrambles(15, 12, 16);
    let htm = par_solve(&cubes, &Solver::new(corners(), h0()));
    let qtm = par_solve(
        &cubes,
        &Solver::with_metric(corners_qtm(), h0_qtm(), Metric::Qtm).unwrap(),
    );
    for ((cube, h), q) in cubes.iter().zip(&htm).zip(&qtm) {
        assert_eq!(cube.apply_moves(h), Some(CubieCube::SOLVED));
        assert_eq!(cube.apply_moves(q), Some(CubieCube::SOLVED));
        let q_len = Metric::Qtm.length(q);
        assert!(q_len >= h.len(), "QTM {q_len} below HTM {}", h.len());
        // (The HTM solution's QTM length bounds the optimum from above.)
        assert!(q_len <= Metric::Qtm.length(h));
        assert_eq!(q_len % 2, cube.corner_parity() as usize);
    }
}

/// An HTM search refuses QTM data, and `Solver::new` picks HTM on HTM
/// tables (the default: unchanged).
#[test]
fn metric_selection() {
    let solver = Solver::new(corners(), h0());
    assert_eq!(solver.metric(), Metric::Htm);
    assert!(Solver::with_metric(corners(), h0(), Metric::Htm).is_ok());
}

/// The QTM corner table: the HTM one's classes and symmetries, depths of a
/// quarter-turn BFS (at least the HTM depths, at most 15 for the packed
/// entries, invariant under the 48 symmetries), and the solvers' metric
/// rules: an HTM search refuses QTM data.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_corner_table() {
    let (htm, qtm) = (corners(), corners_qtm());
    assert_eq!((htm.metric(), qtm.metric()), (Metric::Htm, Metric::Qtm));
    for (&h, &q) in htm.entries().iter().zip(qtm.entries()) {
        assert_eq!(entry_class(h), entry_class(q));
        assert_eq!(entry_sym_to_rep(h), entry_sym_to_rep(q));
        assert!(entry_depth(q) >= entry_depth(h));
    }
    assert_eq!(qtm.distribution().iter().sum::<u64>(), 153_090);
    // (As an independent BFS over Kociemba's corner model gives.)
    assert_eq!(
        &qtm.distribution()[..12],
        &[1, 6, 51, 324, 1430, 5232, 17796, 47394, 61380, 19284, 192, 0]
    );
    assert!(qtm.max_depth() <= 15 && qtm.max_depth() > htm.max_depth());
    // Exact on short scrambles: the QTM depth is the corners' QTM distance.
    let ball = solved_ball(5);
    let mut corner_distance: HashMap<usize, u8> = HashMap::new();
    for (bytes, &d) in ball {
        let cube = Cube::from_bytes(*bytes).unwrap();
        let e = corner_distance.entry(cocsep(cube)).or_insert(d);
        *e = (*e).min(d);
    }
    for (&coordinate, &d) in &corner_distance {
        let depth = entry_depth(qtm.entries()[coordinate]);
        assert_eq!(depth, d, "coordinate {coordinate}");
    }
    let mut rng = Rng(77);
    for _ in 0..200 {
        let cube = Cube::from_cubie(&CubieCube::random(&mut rng));
        let d = entry_depth(qtm.entries()[cocsep(cube)]);
        for sym in 0..48 {
            assert_eq!(entry_depth(qtm.entries()[cocsep(cube.transform(sym))]), d);
        }
    }
    eprintln!("QTM cocsep depths: {:?}", qtm.distribution());
    // Metric rules.
    assert!(Solver::with_metric(qtm, h0(), Metric::Htm).is_err());
    assert!(Solver::with_metric(corners(), h0_qtm(), Metric::Htm).is_err());
    assert_eq!(Solver::new(qtm, h0()).metric(), Metric::Qtm);
    assert_eq!(Solver::new(corners(), h0_qtm()).metric(), Metric::Qtm);
    assert_eq!(h0_qtm().meta().metric, Metric::Qtm);
}

/// QTM h0 by BFS with a small base against brute force (every entry within
/// 6 quarter turns: codes and group minima), and by depth-first passes
/// (any prefix depth) the same table byte for byte; a depth-first QTM h2
/// matches brute force too.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_tables_match_brute_force() {
    let base = 3u8;
    let threads = egg_solver::thread_count();
    let (bfs, _) =
        unsafe { egg_solver::generate_metric(corners(), 0, base, Metric::Qtm, None, &[], threads) }
            .unwrap();
    assert_eq!(bfs.meta().metric, Metric::Qtm);
    let ball = solved_ball(6);
    for k in [0u32, 2] {
        let table = if k == 0 {
            &bfs
        } else {
            &dfs_qtm_table(k, base, 2)
        };
        let mut best: HashMap<u64, u8> = HashMap::new();
        for (bytes, &d) in ball {
            let cube = Cube::from_bytes(*bytes).unwrap();
            let e = best.entry(corners().index(cube, k)).or_insert(d);
            *e = (*e).min(d);
        }
        let mut group_min: HashMap<u64, u8> = HashMap::new();
        for (&index, &d) in &best {
            let (code, _) = table.get(index).unwrap();
            // (Depths of 6 decide codes up to base + 2 = 5 and "at least
            // base + 3".)
            assert_eq!(
                code,
                d.saturating_sub(base).min(3),
                "h{k} index {index} depth {d}"
            );
            let g = group_min.entry(index >> 6).or_insert(d);
            *g = (*g).min(d);
        }
        let dist = table.distribution().unwrap();
        let within = best.values().filter(|&&d| d <= base + 2).count() as u64;
        assert_eq!(
            dist[0] + dist[1] + dist[2],
            within,
            "h{k}: no entry too low"
        );
        for (&g, &d) in &group_min {
            if d <= base + 2 {
                assert_eq!(table.get(g << 6 | 2).unwrap().1, d, "h{k} group {g}");
            }
        }
    }
    let len = usize::try_from(h48_bytes(0)).unwrap();
    for prefix_depth in [1, 2, 4] {
        let t = dfs_qtm_table(0, base, prefix_depth);
        assert!(
            t.memory().bytes().unwrap()[..len] == bfs.memory().bytes().unwrap()[..len],
            "QTM h0 with prefix depth {prefix_depth}"
        );
    }
}

/// A QTM hk of base `base` made by depth-first passes (`gen::dfs`) with a
/// BFS prefix of `prefix_depth`.
fn dfs_qtm_table(k: u32, base: u8, prefix_depth: u8) -> H48Table {
    let settings = egg_solver::gen::dfs::Settings {
        k,
        base,
        threads: egg_solver::thread_count(),
        prefix_depth,
        verbose: false,
        metric: Metric::Qtm,
    };
    let generator = egg_solver::gen::dfs::Generator::new(corners(), settings).unwrap();
    let memory = Memory::anonymous(usize::try_from(h48_bytes(k)).unwrap()).unwrap();
    let rows = h48_rows(k);
    let part = egg_solver::gen::dfs::Part {
        // SAFETY: fresh writable memory, read only after the workers finish.
        words: unsafe { memory.words().unwrap() },
        first_slot: 0,
        end_slot: rows * N_SLOTS as u64,
    };
    generator.generate(&(0..rows), &part).unwrap();
    assert_eq!(memory.len() as u64, rows * ROW_BYTES);
    H48Table::from_memory(
        memory,
        H48Meta {
            k,
            base,
            metric: Metric::Qtm,
        },
    )
    .unwrap()
}

/// Solve with the stream lanes: `threads` threads of `lanes` lanes.
fn lanes_solve(solver: &Solver, cubes: &[CubieCube], lanes: usize) -> Vec<Vec<u8>> {
    let get = |i: usize| cubes.get(i).copied();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let out = std::sync::Mutex::new(vec![None; cubes.len()]);
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                egg_solver::lanes::run_lanes(solver, &get, &next, &stop, lanes, |i, sol, _, _| {
                    out.lock().unwrap()[i] = Some(sol.expect("no solution"));
                });
            });
        }
    });
    out.into_inner()
        .unwrap()
        .into_iter()
        .map(|s| s.expect("not reported"))
        .collect()
}

/// Solve with the work-sharing lanes: 3 threads of `lanes` lanes, at most
/// `max_active` cubes at once.
fn share_solve(
    solver: &Solver,
    cubes: &[CubieCube],
    lanes: usize,
    max_active: usize,
) -> Vec<Vec<u8>> {
    let get = |i: usize| cubes.get(i).copied();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let pool = egg_solver::lanes::share::Pool::new(&get, max_active, 3);
    let out = std::sync::Mutex::new(vec![None; cubes.len()]);
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                pool.run(solver, lanes, &stop, |i, sol, _, _| {
                    let mut out = out.lock().unwrap();
                    assert!(out[i].is_none(), "cube {i} reported twice");
                    out[i] = Some(sol.expect("no solution"));
                });
            });
        }
    });
    out.into_inner()
        .unwrap()
        .into_iter()
        .map(|s| s.expect("not reported"))
        .collect()
}

/// Every engine (recursive, stream lanes, share mode in batch and one cube
/// at a time), on HTM and QTM tables and corner tables, gives brute force's
/// QTM lengths.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_engines_match_brute_force() {
    let radius = 5;
    let ball = solved_ball(radius);
    let cubes = short_scrambles(99, 10, 10);
    let want: Vec<usize> = cubes
        .iter()
        .map(|c| usize::from(qtm_distance(Cube::from_cubie(c), ball, radius).unwrap()))
        .collect();
    let setups: [(&Cocsep, &H48Table, &str); 3] = [
        (corners(), h0(), "HTM h0, HTM corners"),
        (corners_qtm(), h0(), "HTM h0, QTM corners"),
        (corners_qtm(), h0_qtm(), "QTM h0, QTM corners"),
    ];
    for (corner_table, table, name) in setups {
        let solver = Solver::with_metric(corner_table, table, Metric::Qtm).unwrap();
        let runs = [
            ("recursive", par_solve(&cubes, &solver)),
            ("lanes 1", lanes_solve(&solver, &cubes, 1)),
            ("lanes 7", lanes_solve(&solver, &cubes, 7)),
            ("share batch", share_solve(&solver, &cubes, 5, usize::MAX)),
            ("share -c 1", share_solve(&solver, &cubes, 5, 1)),
        ];
        for (engine, solutions) in runs {
            for ((cube, solution), &length) in cubes.iter().zip(&solutions).zip(&want) {
                assert_eq!(
                    cube.apply_moves(solution),
                    Some(CubieCube::SOLVED),
                    "{name} {engine}"
                );
                assert_eq!(
                    Metric::Qtm.length(solution),
                    length,
                    "{name} {engine}: {}",
                    format_moves(solution)
                );
            }
        }
    }
}

/// Deeper cubes (13 random moves, QTM 14-17): all engines and tables agree
/// on the QTM length (the recursive solver without symmetric-start
/// filtering as the reference), positions with symmetry (filtered jobs)
/// included.
#[test]
#[cfg_attr(miri, ignore = "too slow under Miri")]
fn qtm_engines_agree_on_deeper_cubes() {
    let mut cubes = random_scrambles(16, 13, 8);
    // Symmetric positions (symmetric-start filtering without `dedup`).
    for scramble in [
        "U2 D2 R2 L2 F2 B2",
        "U D' R L' F B' U D'",
        "R2 L2 U2 D2",
        "U R L' F' B U' D R L'",
        "F2 B2 U D' R2 L2 U D'",
    ] {
        cubes.push(cube_of(scramble));
    }
    let mut plain = Solver::with_metric(corners_qtm(), h0_qtm(), Metric::Qtm).unwrap();
    plain.sym_start = false;
    let want: Vec<usize> = par_solve(&cubes, &plain)
        .iter()
        .map(|s| Metric::Qtm.length(s))
        .collect();
    for (corner_table, table) in [(corners_qtm(), h0()), (corners_qtm(), h0_qtm())] {
        let solver = Solver::with_metric(corner_table, table, Metric::Qtm).unwrap();
        for solutions in [
            par_solve(&cubes, &solver),
            lanes_solve(&solver, &cubes, 6),
            share_solve(&solver, &cubes, 6, usize::MAX),
            share_solve(&solver, &cubes, 6, 1),
        ] {
            for ((cube, solution), &length) in cubes.iter().zip(&solutions).zip(&want) {
                assert_eq!(cube.apply_moves(solution), Some(CubieCube::SOLVED));
                assert_eq!(
                    Metric::Qtm.length(solution),
                    length,
                    "{}",
                    format_moves(solution)
                );
            }
        }
    }
}
