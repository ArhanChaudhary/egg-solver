//! End-to-end correctness: the table generator against brute force, and
//! the solvers (recursive, stream lanes, work-sharing lanes) against known
//! optimal lengths and an independent plain IDA*. The tables here are h0
//! tables generated in memory (no table file needed).

use cubeopt::coord::cocsep;
use cubeopt::cubie::{conjugate, parse_moves, symmetries, CubieCube, Rng};
use cubeopt::simd::Cube;
use cubeopt::solver::Solver;
use cubeopt::tables::{entry_depth, Cocsep, H48Table};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The cocsep table (built once).
fn corners() -> &'static Cocsep {
    static CORNERS: OnceLock<Cocsep> = OnceLock::new();
    CORNERS.get_or_init(Cocsep::generate)
}

/// An h0 table with base 8 (generated once; ~0.5 GB).
fn h0() -> &'static H48Table {
    static TABLE: OnceLock<H48Table> = OnceLock::new();
    TABLE.get_or_init(|| cubeopt::generate(corners(), 0, 8, None, &[], cubeopt::thread_count()).0)
}

/// The scrambles of a file in `testdata`.
fn dataset(name: &str) -> Vec<CubieCube> {
    let path = format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| CubieCube::SOLVED.apply_moves(&parse_moves(l).unwrap()))
        .collect()
}

/// Solve with the recursive solver, all threads.
fn par_solve(cubes: &[CubieCube], table: &H48Table) -> Vec<Vec<u8>> {
    let solver = Solver::new(corners(), table);
    let out = std::sync::Mutex::new(vec![vec![]; cubes.len()]);
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..cubeopt::thread_count() {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= cubes.len() {
                    break;
                }
                let (solution, _) = solver.solve(&cubes[i]);
                out.lock().unwrap()[i] = solution;
            });
        }
    });
    out.into_inner().unwrap()
}

/// All positions within `depth` moves, with their distance.
fn ball(depth: u8) -> HashMap<[u8; 32], (Cube, u8)> {
    let mut seen: HashMap<[u8; 32], (Cube, u8)> = HashMap::new();
    let mut frontier = vec![Cube::solved()];
    seen.insert(Cube::solved().to_bytes(), (Cube::solved(), 0));
    for distance in 1..=depth {
        let mut next = Vec::new();
        for cube in frontier {
            for mv in 0..18 {
                let child = cube.apply_move(mv);
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(child.to_bytes()) {
                    e.insert((child, distance));
                    next.push(child);
                }
            }
        }
        frontier = next;
    }
    seen
}

/// Generate h0 with a small base and check every entry of depth <= 5 against
/// brute force (this exercises the BFS and the self-symmetry handling).
#[test]
fn h0_table_matches_brute_force() {
    let corners = corners();
    let base = 3u8;
    let (table, _) = cubeopt::generate(corners, 0, base, None, &[], cubeopt::thread_count());
    // The distance of each table entry: the least over the cubes it covers.
    let mut best: HashMap<u64, u8> = HashMap::new();
    for &(cube, distance) in ball(5).values() {
        let index = corners.index(cube, 0);
        let entry = best.entry(index).or_insert(distance);
        *entry = (*entry).min(distance);
        // admissible
        let (code, group_min) = table.get(index);
        let value = if code == 0 { group_min } else { base + code };
        assert!(
            value <= distance,
            "inadmissible: index {index} value {value} > distance {distance}"
        );
    }
    let mut want = [0u64; 3];
    for (&index, &distance) in &best {
        let (code, _) = table.get(index);
        let expect = distance.saturating_sub(base).min(3);
        assert_eq!(code, expect, "index {index} depth {distance}");
        if distance <= base + 2 {
            want[usize::from(expect)] += 1;
        }
    }
    let counts = table.distribution();
    assert_eq!(&counts[..3], &want[..], "entries with depth <= base+2");
}

#[test]
fn solves_known_depth_15() {
    let cubes: Vec<CubieCube> = dataset("depth15.txt").into_iter().take(48).collect();
    let solutions = par_solve(&cubes, h0());
    for (c, s) in cubes.iter().zip(solutions.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), 15);
    }
}

/// Independent reference: plain IDA* with only the corner (cocsep) bound.
fn reference_length(cube: &CubieCube, corners: &Cocsep) -> usize {
    /// A solution of exactly `depth` moves, not starting on `last_face`?
    fn search(cube: Cube, corners: &Cocsep, depth: u8, last_face: usize) -> bool {
        if depth == 0 {
            return cube.is_solved();
        }
        if entry_depth(corners.entries[cocsep(cube)]) > depth {
            return false;
        }
        for mv in 0..18 {
            if mv / 3 == last_face {
                continue;
            }
            if search(cube.apply_move(mv), corners, depth - 1, mv / 3) {
                return true;
            }
        }
        false
    }
    let cube = Cube::from_cubie(cube);
    (0..=20)
        .find(|&depth| search(cube, corners, depth, 6))
        .map(usize::from)
        .unwrap()
}

#[test]
fn short_scrambles_are_optimal() {
    let mut rng = Rng(42);
    let corners = corners();
    let mut cubes = Vec::new();
    for len in 0..=8 {
        for _ in 0..12 {
            let ms: Vec<u8> = (0..len).map(|_| rng.below_u8(18)).collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&ms));
        }
    }
    let solutions = par_solve(&cubes, h0());
    for (c, s) in cubes.iter().zip(solutions.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), reference_length(c, corners), "scramble {c:?}");
    }
}

#[test]
fn symmetric_positions() {
    // Positions with lots of symmetry stress the symmetry handling.
    let cases = [
        ("U2 D2 R2 L2 F2 B2", 6),
        ("R2 L2 U2 D2", 4),
        ("U D' R L' F B' U D'", 8),
    ];
    let cubes: Vec<CubieCube> = cases
        .iter()
        .map(|(s, _)| CubieCube::SOLVED.apply_moves(&parse_moves(s).unwrap()))
        .collect();
    let solutions = par_solve(&cubes, h0());
    for ((c, s), (_, len)) in cubes.iter().zip(solutions.iter()).zip(cases.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), *len);
    }
}

#[test]
fn table_value_is_symmetry_invariant() {
    let corners = corners();
    let table = h0();
    let mut rng = Rng(99);
    for _ in 0..500 {
        let cube = Cube::from_cubie(&CubieCube::random(&mut rng));
        let code = table.get(corners.index(cube, 0)).0;
        for sym in 0..48 {
            assert_eq!(table.get(corners.index(cube.transform(sym), 0)).0, code);
        }
    }
}

fn lanes_solve(cubes: &[CubieCube], table: &H48Table, nlanes: usize) -> Vec<Vec<u8>> {
    let solver = Solver::new(corners(), table);
    let out = std::sync::Mutex::new(vec![vec![]; cubes.len()]);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let get = |i: usize| cubes.get(i).copied();
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                cubeopt::lanes::run_lanes(&solver, &get, &next, &stop, nlanes, |i, sol, _, _| {
                    out.lock().unwrap()[i] = sol;
                });
            });
        }
    });
    out.into_inner().unwrap()
}

#[test]
fn lanes_solve_known_depth_15() {
    let cubes: Vec<CubieCube> = dataset("depth15.txt").into_iter().take(40).collect();
    for nlanes in [1, 7] {
        let solutions = lanes_solve(&cubes, h0(), nlanes);
        for (c, s) in cubes.iter().zip(solutions.iter()) {
            assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
            assert_eq!(s.len(), 15);
        }
    }
}

#[test]
fn lanes_short_scrambles_are_optimal() {
    let mut rng = Rng(4242);
    let corners = corners();
    let mut cubes = Vec::new();
    for len in 0..=8 {
        for _ in 0..10 {
            let ms: Vec<u8> = (0..len).map(|_| rng.below_u8(18)).collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&ms));
        }
    }
    let solutions = lanes_solve(&cubes, h0(), 5);
    for (c, s) in cubes.iter().zip(solutions.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), reference_length(c, corners), "scramble {c:?}");
    }
}

/// Solve with the work-sharing lanes (`lanes::share`): `threads` threads of
/// `nlanes` lanes, at most `max_active` cubes at once. Returns the solutions
/// and the pool's event counters.
fn share_solve(
    cubes: &[CubieCube],
    table: &H48Table,
    threads: usize,
    nlanes: usize,
    max_active: usize,
) -> (Vec<Vec<u8>>, String) {
    let solver = Solver::new(corners(), table);
    let out = std::sync::Mutex::new(vec![None; cubes.len()]);
    let get = |i: usize| cubes.get(i).copied();
    let pool = cubeopt::lanes::share::Pool::new(&get, max_active, threads);
    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                // (A panic stops the other threads: they would wait for its
                // work forever.)
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    pool.run(&solver, nlanes, &stop, |i, sol, _, _| {
                        let mut o = out.lock().unwrap();
                        assert!(o[i].is_none(), "cube {i} reported twice");
                        o[i] = Some(sol);
                    });
                }));
                if let Err(e) = r {
                    stop.store(true, std::sync::atomic::Ordering::Relaxed);
                    std::panic::resume_unwind(e);
                }
            });
        }
    });
    let solutions = out.into_inner().unwrap();
    let solutions = solutions
        .into_iter()
        .enumerate()
        .map(|(i, s)| s.unwrap_or_else(|| panic!("cube {i} not reported")))
        .collect();
    (solutions, pool.counters())
}

#[test]
fn share_solve_known_depth_15() {
    let cubes: Vec<CubieCube> = dataset("depth15.txt").into_iter().take(40).collect();
    // batch, one cube at a time, a few at a time; one lane, several lanes
    for (threads, nlanes, max_active) in [(4, 7, usize::MAX), (4, 7, 1), (3, 1, 2), (1, 5, 3)] {
        let (solutions, counters) = share_solve(&cubes, h0(), threads, nlanes, max_active);
        for (c, s) in cubes.iter().zip(solutions.iter()) {
            assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
            assert_eq!(
                s.len(),
                15,
                "{threads} threads {nlanes} lanes {max_active}: {counters}"
            );
        }
    }
}

/// One cube at a time with idle lanes and threads: lanes split each other's
/// jobs and threads donate pieces.
#[test]
fn share_splits_and_donates() {
    let cubes: Vec<CubieCube> = dataset("depth15.txt")
        .into_iter()
        .skip(40)
        .take(12)
        .collect();
    let (solutions, counters) = share_solve(&cubes, h0(), 4, 12, 1);
    for (c, s) in cubes.iter().zip(solutions.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), 15);
    }
    let count = |name: &str| -> u64 {
        let f: Vec<&str> = counters.split_whitespace().collect();
        let i = f.iter().position(|&x| x == name).unwrap();
        f[i + 1].parse().unwrap()
    };
    assert!(count("splits") > 0, "{counters}");
    assert!(count("donated") > 0, "{counters}");
    assert!(count("pieces") > 0, "{counters}");
}

/// Few cubes in progress for the threads (at most threads / 8): longer
/// prefixes early (extended by two moves in parallel), staged builds, and
/// threads staying with their cubes.
#[test]
fn share_few_cubes() {
    let cubes: Vec<CubieCube> = dataset("depth15.txt").into_iter().take(24).collect();
    for (threads, nlanes, max_active) in [(8, 3, 1), (9, 2, 1), (16, 1, 2)] {
        let (solutions, counters) = share_solve(&cubes, h0(), threads, nlanes, max_active);
        for (c, s) in cubes.iter().zip(solutions.iter()) {
            assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
            assert_eq!(
                s.len(),
                15,
                "{threads} threads {nlanes} lanes {max_active}: {counters}"
            );
        }
    }
}

/// Many short runs with small thread and lane counts (races of the
/// mailboxes, the open list, builds and cube hand-over): every solution
/// optimal, every cube reported once.
#[test]
fn share_stress() {
    let mut rng = Rng(4244);
    let corners = corners();
    let mut cubes = Vec::new();
    for len in 0..=8 {
        for _ in 0..6 {
            let ms: Vec<u8> = (0..len).map(|_| rng.below_u8(18)).collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&ms));
        }
    }
    let mut want: Vec<usize> = cubes.iter().map(|c| reference_length(c, corners)).collect();
    let deep = dataset("depth15.txt");
    cubes.extend(deep.into_iter().skip(60).take(6));
    want.resize(cubes.len(), 15);
    for run in 0..40 {
        let threads = [2, 3, 8, 9, 16][run % 5];
        let nlanes = [1, 2, 5][run % 3];
        let max_active = [1, 2, usize::MAX, 3][run % 4];
        let (solutions, counters) = share_solve(&cubes, h0(), threads, nlanes, max_active);
        for ((c, s), &n) in cubes.iter().zip(solutions.iter()).zip(want.iter()) {
            assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
            assert_eq!(
                s.len(),
                n,
                "run {run}: {threads} threads {nlanes} lanes {max_active}: {counters}"
            );
        }
    }
}

#[test]
fn share_short_scrambles_are_optimal() {
    let mut rng = Rng(4243);
    let corners = corners();
    let mut cubes = Vec::new();
    for len in 0..=8 {
        for _ in 0..10 {
            let ms: Vec<u8> = (0..len).map(|_| rng.below_u8(18)).collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&ms));
        }
    }
    for max_active in [usize::MAX, 1] {
        let (solutions, _) = share_solve(&cubes, h0(), 4, 5, max_active);
        for (c, s) in cubes.iter().zip(solutions.iter()) {
            assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
            assert_eq!(s.len(), reference_length(c, corners), "scramble {c:?}");
        }
    }
}

/// All conjugates of a cube and of its inverse (by the 48 symmetries) have
/// the same optimal length: recursive, stream (odd lane counts) and share.
#[test]
fn conjugates_have_equal_lengths_in_all_modes() {
    let mut rng = Rng(2027);
    let mut cubes = Vec::new();
    for _ in 0..3 {
        let moves: Vec<u8> = (0..9)
            .map(|_| u8::try_from(rng.below(18)).unwrap())
            .collect();
        let cube = CubieCube::SOLVED.apply_moves(&moves);
        for base in [cube, cube.inverse()] {
            cubes.extend(symmetries().iter().map(|s| conjugate(&base, s)));
        }
    }
    let lengths = |solutions: &[Vec<u8>]| -> Vec<usize> {
        for (c, s) in cubes.iter().zip(solutions) {
            assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        }
        solutions.iter().map(Vec::len).collect()
    };
    let want = lengths(&par_solve(&cubes, h0()));
    for group in want.chunks(96) {
        assert!(group.iter().all(|&n| n == group[0]), "{group:?}");
    }
    for nlanes in [1, 3, 21] {
        assert_eq!(
            lengths(&lanes_solve(&cubes, h0(), nlanes)),
            want,
            "{nlanes} lanes"
        );
    }
    for (threads, nlanes, max_active) in [(3, 2, usize::MAX), (2, 5, 1)] {
        let (solutions, counters) = share_solve(&cubes, h0(), threads, nlanes, max_active);
        assert_eq!(lengths(&solutions), want, "{counters}");
    }
}
