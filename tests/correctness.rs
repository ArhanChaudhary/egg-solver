//! End-to-end correctness: the table generator against brute force, and
//! the solvers (recursive, stream lanes, work-sharing lanes) against known
//! optimal lengths and an independent plain IDA*. The tables here are h0
//! tables generated in memory (no table file needed).

use egg_solver::coord::cocsep;
use egg_solver::coord::N_SLOTS;
use egg_solver::cubie::{conjugate, parse_moves, symmetries, CubieCube, Move, Rng};
use egg_solver::simd::Cube;
use egg_solver::solver::Solver;
use egg_solver::tables::{
    entry_depth, h48_bytes, h48_rows, Cocsep, H48Meta, H48Table, Memory, ROW_BYTES,
};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The cocsep table (built once).
fn corners() -> &'static Cocsep {
    static CORNERS: OnceLock<Cocsep> = OnceLock::new();
    CORNERS.get_or_init(|| Cocsep::generate().unwrap())
}

/// An h0 table with base 8 (generated once; ~0.5 GB).
fn h0() -> &'static H48Table {
    static TABLE: OnceLock<H48Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        unsafe { egg_solver::generate(corners(), 0, 8, None, &[], egg_solver::thread_count()) }
            .unwrap()
            .0
    })
}

/// The scrambles of a file in `testdata`.
fn dataset(name: &str) -> Vec<CubieCube> {
    let path = format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            CubieCube::SOLVED
                .apply_moves(&parse_moves(l).unwrap())
                .unwrap()
        })
        .collect()
}

/// Solve with the recursive solver, all threads.
fn par_solve(cubes: &[CubieCube], table: &H48Table) -> Vec<Vec<u8>> {
    let solver = Solver::new(corners(), table);
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
                out.lock().unwrap()[i] = solution.unwrap();
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
            for mv in Move::ALL {
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
    let (table, _) =
        unsafe { egg_solver::generate(corners, 0, base, None, &[], egg_solver::thread_count()) }
            .unwrap();
    // The distance of each table entry: the least over the cubes it covers.
    let mut best: HashMap<u64, u8> = HashMap::new();
    for &(cube, distance) in ball(5).values() {
        let index = corners.index(cube, 0);
        let entry = best.entry(index).or_insert(distance);
        *entry = (*entry).min(distance);
        // admissible
        let (code, group_min) = table.get(index).unwrap();
        let value = if code == 0 { group_min } else { base + code };
        assert!(
            value <= distance,
            "inadmissible: index {index} value {value} > distance {distance}"
        );
    }
    let mut want = [0u64; 3];
    for (&index, &distance) in &best {
        let (code, _) = table.get(index).unwrap();
        let expect = distance.saturating_sub(base).min(3);
        assert_eq!(code, expect, "index {index} depth {distance}");
        if distance <= base + 2 {
            want[usize::from(expect)] += 1;
        }
    }
    let counts = table.distribution().unwrap();
    assert_eq!(&counts[..3], &want[..], "entries with depth <= base+2");
}

#[test]
fn solves_known_depth_15() {
    let cubes: Vec<CubieCube> = dataset("depth15.txt").into_iter().take(48).collect();
    let solutions = par_solve(&cubes, h0());
    for (c, s) in cubes.iter().zip(solutions.iter()) {
        assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
        if entry_depth(corners.entries()[cocsep(cube)]) > depth {
            return false;
        }
        for mv in Move::ALL {
            if mv.index() / 3 == last_face {
                continue;
            }
            if search(cube.apply_move(mv), corners, depth - 1, mv.index() / 3) {
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
            cubes.push(CubieCube::SOLVED.apply_moves(&ms).unwrap());
        }
    }
    let solutions = par_solve(&cubes, h0());
    for (c, s) in cubes.iter().zip(solutions.iter()) {
        assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
        .map(|(s, _)| {
            CubieCube::SOLVED
                .apply_moves(&parse_moves(s).unwrap())
                .unwrap()
        })
        .collect();
    let solutions = par_solve(&cubes, h0());
    for ((c, s), (_, len)) in cubes.iter().zip(solutions.iter()).zip(cases.iter()) {
        assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
        let code = table.get(corners.index(cube, 0)).unwrap().0;
        for sym in 0..48 {
            assert_eq!(
                table.get(corners.index(cube.transform(sym), 0)).unwrap().0,
                code
            );
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
                egg_solver::lanes::run_lanes(
                    &solver,
                    &get,
                    &next,
                    &stop,
                    nlanes,
                    |i, sol, _, _| {
                        out.lock().unwrap()[i] = sol.unwrap();
                    },
                );
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
            assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
            cubes.push(CubieCube::SOLVED.apply_moves(&ms).unwrap());
        }
    }
    let solutions = lanes_solve(&cubes, h0(), 5);
    for (c, s) in cubes.iter().zip(solutions.iter()) {
        assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
    let pool = egg_solver::lanes::share::Pool::new(&get, max_active, threads);
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
        .map(|(i, s)| {
            s.flatten()
                .unwrap_or_else(|| panic!("cube {i} not reported or solved"))
        })
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
            assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
        assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
            assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
            cubes.push(CubieCube::SOLVED.apply_moves(&ms).unwrap());
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
            assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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
            cubes.push(CubieCube::SOLVED.apply_moves(&ms).unwrap());
        }
    }
    for max_active in [usize::MAX, 1] {
        let (solutions, _) = share_solve(&cubes, h0(), 4, 5, max_active);
        for (c, s) in cubes.iter().zip(solutions.iter()) {
            assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
            assert_eq!(s.len(), reference_length(c, corners), "scramble {c:?}");
        }
    }
}

/// Scrambles of 1 to 14 moves in share mode, whose first pass (h0: one
/// pass for the iterations from 10 up to 14, without barriers) reports a
/// solution once the iterations below its length ended: as short as the
/// recursive solver's, in batch and one cube at a time, with splits and
/// donations.
#[test]
fn share_first_pass_finds_the_shortest_solutions() {
    let mut rng = Rng(4245);
    let mut cubes = Vec::new();
    for len in 1..=14 {
        for _ in 0..3 {
            let ms: Vec<u8> = (0..len).map(|_| rng.below_u8(18)).collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&ms).unwrap());
        }
    }
    let want: Vec<usize> = par_solve(&cubes, h0()).iter().map(Vec::len).collect();
    for (threads, nlanes, max_active) in [(4, 5, usize::MAX), (8, 3, 1), (3, 1, 2)] {
        let (solutions, counters) = share_solve(&cubes, h0(), threads, nlanes, max_active);
        for ((c, s), &n) in cubes.iter().zip(&solutions).zip(&want) {
            assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
            assert_eq!(
                s.len(),
                n,
                "{threads} threads {nlanes} lanes {max_active}: {counters}"
            );
        }
        let count = |name: &str| -> u64 {
            let fields: Vec<&str> = counters.split_whitespace().collect();
            let i = fields.iter().position(|&x| x == name).unwrap();
            fields[i + 1].parse().unwrap()
        };
        assert!(count("pass_solutions") > 0, "{counters}");
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
        let cube = CubieCube::SOLVED.apply_moves(&moves).unwrap();
        for base in [cube, cube.inverse()] {
            cubes.extend(symmetries().iter().map(|s| conjugate(&base, s)));
        }
    }
    let lengths = |solutions: &[Vec<u8>]| -> Vec<usize> {
        for (c, s) in cubes.iter().zip(solutions) {
            assert_eq!(c.apply_moves(s), Some(CubieCube::SOLVED));
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

/// hk generated depth-first (`gen::dfs`) with base 3 and a BFS prefix of
/// `prefix_depth`, in `parts` row ranges (each in its own memory).
fn dfs_table_in_parts(k: u32, prefix_depth: u8, parts: u64) -> H48Table {
    let settings = egg_solver::gen::dfs::Settings {
        k,
        base: 3,
        threads: egg_solver::thread_count(),
        prefix_depth,
        verbose: false,
    };
    let generator = egg_solver::gen::dfs::Generator::new(corners(), settings).unwrap();
    let memory = Memory::anonymous(usize::try_from(h48_bytes(k)).unwrap()).unwrap();
    let rows = h48_rows(k);
    let per = rows.div_ceil(parts);
    for start in (0..rows).step_by(usize::try_from(per).unwrap()) {
        let range = start..(start + per).min(rows);
        let part_mem =
            Memory::anonymous(usize::try_from((range.end - range.start) * ROW_BYTES).unwrap())
                .unwrap();
        let part = egg_solver::gen::dfs::Part {
            // SAFETY: fresh writable memory, copied only after workers finish.
            words: unsafe { part_mem.words().unwrap() },
            first_slot: range.start * N_SLOTS as u64,
            end_slot: range.end * N_SLOTS as u64,
        };
        generator.generate(&range, &part).unwrap();
        let offset = usize::try_from(range.start * ROW_BYTES).unwrap();
        // SAFETY: the part's rows of the table's own memory.
        unsafe {
            std::ptr::copy_nonoverlapping(
                part_mem.ptr(),
                memory.ptr().add(offset),
                part_mem.bytes().unwrap().len(),
            );
        }
    }
    H48Table::from_memory(memory, H48Meta { k, base: 3 }).unwrap()
}

fn dfs_table(k: u32, prefix_depth: u8) -> H48Table {
    dfs_table_in_parts(k, prefix_depth, 1)
}

/// The depth-first generator gives the BFS's h0 byte for byte (for any
/// prefix depth), an h2 that matches brute force (codes, group minima, no
/// entry too low), and hk that collapse to h(k-1) (`gen::collapse`).
#[test]
fn dfs_generator_matches_bfs_and_brute_force() {
    let cs = corners();
    let base = 3u8;
    let len = usize::try_from(h48_bytes(0)).unwrap();
    let bfs = unsafe { egg_solver::generate(cs, 0, base, None, &[], egg_solver::thread_count()) }
        .unwrap()
        .0;
    for prefix_depth in [1, 2, 5] {
        let t = dfs_table(0, prefix_depth);
        assert!(
            t.memory().bytes().unwrap()[..len] == bfs.memory().bytes().unwrap()[..len],
            "h0 with prefix depth {prefix_depth}"
        );
    }
    // h2 against all positions within 5 moves (base + 2: every code but 3
    // is decided by them).
    let k = 2;
    let t = dfs_table(k, 2);
    let mut best: HashMap<u64, u8> = HashMap::new();
    for &(c, d) in ball(5).values() {
        let e = best.entry(cs.index(c, k)).or_insert(d);
        *e = (*e).min(d);
    }
    let mut group_min: HashMap<u64, u8> = HashMap::new();
    for (&idx, &d) in &best {
        let (code, _) = t.get(idx).unwrap();
        assert_eq!(code, d.saturating_sub(base).min(3), "idx {idx} depth {d}");
        let g = group_min.entry(idx >> 6).or_insert(d);
        *g = (*g).min(d);
    }
    let dist = t.distribution().unwrap();
    assert_eq!(
        dist[0] + dist[1] + dist[2],
        best.len() as u64,
        "no entry too low"
    );
    for (&g, &d) in &group_min {
        assert_eq!(t.get(g << 6 | 2).unwrap().1, d, "group {g}");
    }
    // Collapse: h3 -> h2, h3 -> h0.
    let t3 = dfs_table(3, 2);
    // In parts (rows of several classes and parts of a class): the same.
    let n3 = usize::try_from(h48_bytes(3)).unwrap();
    for parts in [3, 7] {
        let t = dfs_table_in_parts(3, 2, parts);
        assert!(
            t.memory().bytes().unwrap()[..n3] == t3.memory().bytes().unwrap()[..n3],
            "h3 in {parts} parts"
        );
    }
    let threads = egg_solver::thread_count();
    for (small, want) in [(&t, 2), (&bfs, 0)] {
        let derived = egg_solver::gen::collapse::derive(&t3, want, threads).unwrap();
        let n = usize::try_from(h48_bytes(want)).unwrap();
        assert!(
            derived.memory().bytes().unwrap()[..n] == small.memory().bytes().unwrap()[..n],
            "h3 -> h{want}"
        );
        let diff = egg_solver::gen::collapse::compare(small, &t3, threads).unwrap();
        assert_eq!((diff.codes, diff.group_minima), (0, 0), "{diff:?}");
    }
}

/// Positions with a symmetry (the first 13: random words in moves and move
/// pairs fixed by a symmetry `t`) or an antisymmetry (the last 8:
/// `A · t(A)⁻¹` for an involution `t`), with their optimal lengths (main's
/// solver, h11). Their jobs have 1 to 4 moves with h0 base 8.
const SYMMETRIC_CASES: [(&str, usize); 21] = [
    (
        "U2 D2 L' R U' D' R2 L' L' R' F' B' R L' L' R2 L' R' L' R2 L' R2",
        12,
    ),
    (
        "F' B' D' U2 L' R' U' L' R' D2 U2 L2 R2 U2 U D L2 R2 U D D2 D2 U2",
        12,
    ),
    (
        "L2 R2 R' L' D2 U2 F2 B' F' B' F' D U F B2 B F L R B2 F2 B F'",
        12,
    ),
    (
        "D' U' R' L L' R2 R R' L D2 U2 F B L' R2 R' L' U D L' L' R",
        13,
    ),
    (
        "R2 L' L' R' L R' U' D' R B2 F2 L2 R F' B' D' U' L' R L2 R2 L2 L2 R2 R2 L'",
        13,
    ),
    (
        "R2 L2 B2 F2 U U2 D2 D2 U' D' U R L D' U F2 B2 U2 D2 U' F2 B2 R L D U",
        13,
    ),
    ("L' F2 B2 U D F B L' R2 L R2 L B F U' D' B' F' L R' R L", 14),
    (
        "R2 L2 U D F L' R' F2 B2 F2 B F' B2 F F B' L2 R2 B2 F' R L",
        14,
    ),
    (
        "D2 U2 D' D U2 U' D2 U' D' D' U U' D2 U D' F' B' R' L' U D2 L' R' D2 U F B",
        14,
    ),
    (
        "L' R' L' R R' D' U' R' L' L' R2 D' U' R B' F' R2 L U' D' L R",
        15,
    ),
    (
        "R' F' B' R R L R F' B' L' R L' L2 F' B' D U L L2 L R F' B'",
        15,
    ),
    (
        "L2 R2 F2 B' B F B B F F2 B' U D L' R' B2 F2 U2 D2 B2 F' L' R' F' B D' U'",
        15,
    ),
    (
        "R2 L2 D L R D2 D' U2 B2 F2 D2 U' B F D' U R L D2 U2 L2 R2 U D' U2 D2 D U L2 R2 U' D",
        15,
    ),
    ("R' B' L' U B B2 R U' L' U L D' F2 F' L' U F D", 14),
    ("D L' L R' B2 F' R' U U R R U' F' B2 U' D D' L", 12),
    ("D2 B' B' R' D2 R D D' B F' R R' D' R2 D F F R2", 12),
    ("D D2 L' R' D D' D' F' D2 U2 F' U' U' U R' L' U2 U", 12),
    ("L' R D F' F2 R L2 B F' F' B U2 D F2 F' R D U'", 14),
    ("F R' F2 U2 D2 R' F2 F2 D U' B2 B2 R U2 D2 B2 R B'", 14),
    ("D2 D B F2 U' F' F2 B' F2 D2 U D2 D B D2 U' F' F2", 12),
    ("R2 B B L' D D' D2 R' B' F D R2 R R' U F' F' D2", 12),
];

/// (length, nodes) of each of `cubes` in stream lanes (3 threads of 5).
fn lanes_lengths_and_nodes(solver: &Solver, cubes: &[CubieCube]) -> Vec<(usize, u64)> {
    let get = |i: usize| cubes.get(i).copied();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let out = std::sync::Mutex::new(vec![(0, 0); cubes.len()]);
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                egg_solver::lanes::run_lanes(solver, &get, &next, &stop, 5, |i, sol, stats, _| {
                    let sol = sol.unwrap();
                    assert_eq!(cubes[i].apply_moves(&sol), Some(CubieCube::SOLVED));
                    out.lock().unwrap()[i] = (sol.len(), stats.nodes);
                });
            });
        }
    });
    out.into_inner().unwrap()
}

/// (length, nodes) of each of `cubes` in share mode (3 threads of 4 lanes,
/// at most `max_active` cubes at once), and the pool's event counters.
fn share_lengths_and_nodes(
    solver: &Solver,
    cubes: &[CubieCube],
    max_active: usize,
) -> (Vec<(usize, u64)>, String) {
    let get = |i: usize| cubes.get(i).copied();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let pool = egg_solver::lanes::share::Pool::new(&get, max_active, 3);
    let out = std::sync::Mutex::new(vec![(0, 0); cubes.len()]);
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                pool.run(solver, 4, &stop, |i, sol, stats, _| {
                    let sol = sol.unwrap();
                    assert_eq!(cubes[i].apply_moves(&sol), Some(CubieCube::SOLVED));
                    out.lock().unwrap()[i] = (sol.len(), stats.nodes);
                });
            });
        }
    });
    (out.into_inner().unwrap(), pool.counters())
}

/// Symmetric start (`symstart`) keeps the optimal lengths in every mode
/// (share mode: in its first pass, h0's iterations up to 14, and after it)
/// and searches fewer nodes; random states are not filtered (same nodes).
#[test]
fn symmetric_start_keeps_optimal_lengths() {
    use egg_solver::symstart::SymFilter;
    let cubes: Vec<CubieCube> = SYMMETRIC_CASES
        .iter()
        .map(|(scramble, _)| {
            CubieCube::SOLVED
                .apply_moves(&parse_moves(scramble).unwrap())
                .unwrap()
        })
        .collect();
    let known: Vec<usize> = SYMMETRIC_CASES.iter().map(|case| case.1).collect();
    // (Only the cubes with a symmetry are filtered.)
    for (i, cubie) in cubes.iter().enumerate() {
        let cube = Cube::from_cubie(cubie);
        assert_eq!(SymFilter::new(cube, cube.inverse()).is_active(), i < 13);
    }
    let filtered = Solver::new(corners(), h0());
    let mut plain = Solver::new(corners(), h0());
    plain.sym_start = false;
    // Recursive solver: lengths, and the nodes of each cube.
    let solve_all = |solver: &Solver, cubes: &[CubieCube]| -> Vec<(usize, u64)> {
        cubes
            .iter()
            .map(|cube| {
                let (solution, stats) = solver.solve(cube);
                let solution = solution.unwrap();
                assert_eq!(cube.apply_moves(&solution), Some(CubieCube::SOLVED));
                (solution.len(), stats.nodes)
            })
            .collect()
    };
    let lengths = |results: &[(usize, u64)]| results.iter().map(|r| r.0).collect::<Vec<_>>();
    let nodes = |results: &[(usize, u64)]| results.iter().map(|r| r.1).sum::<u64>();
    let want = solve_all(&plain, &cubes);
    let got = solve_all(&filtered, &cubes);
    assert_eq!(lengths(&want), known);
    assert_eq!(lengths(&got), known);
    assert!(
        nodes(&got) < nodes(&want),
        "{} vs {}",
        nodes(&got),
        nodes(&want)
    );
    // Stream lanes, with and without the filter.
    let lanes_filtered = lanes_lengths_and_nodes(&filtered, &cubes);
    let lanes_plain = lanes_lengths_and_nodes(&plain, &cubes);
    assert_eq!(lengths(&lanes_filtered), known, "lanes");
    assert_eq!(lengths(&lanes_plain), known, "lanes, no filter");
    assert!(nodes(&lanes_filtered) < nodes(&lanes_plain), "lanes");
    // Share mode, batch and one cube at a time.
    let counter = |counters: &str, name: &str| -> u64 {
        let fields: Vec<&str> = counters.split_whitespace().collect();
        let i = fields.iter().position(|&field| field == name).unwrap();
        fields[i + 1].parse().unwrap()
    };
    for max_active in [usize::MAX, 1] {
        let (share_filtered, counters) = share_lengths_and_nodes(&filtered, &cubes, max_active);
        assert_eq!(
            lengths(&share_filtered),
            known,
            "share {max_active}: {counters}"
        );
        // (The first pass found the cubes of lengths 12 to 14.)
        assert!(counter(&counters, "pass_solutions") > 0, "{counters}");
        let (share_plain, _) = share_lengths_and_nodes(&plain, &cubes, max_active);
        assert_eq!(
            lengths(&share_plain),
            known,
            "share {max_active}, no filter"
        );
        // (Share mode's counts vary from run to run, by a few percent here; the
        // filter takes about 30% off.)
        assert!(
            nodes(&share_filtered) < nodes(&share_plain),
            "share {max_active}"
        );
    }
    // Random states: never filtered, identical search.
    let randoms: Vec<CubieCube> = dataset("depth15.txt").into_iter().take(10).collect();
    assert_eq!(solve_all(&filtered, &randoms), solve_all(&plain, &randoms));
}
