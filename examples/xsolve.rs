//! Cross-check driver: solves a cube set in one mode and prints every cube's
//! solution length, so that modes (and builds) can be compared cube by cube.
//!
//! ```text
//! xsolve <table> <set> <mode> [threads] [lanes]
//!   set:  random:N:SEED (as `bench`) | moves:N:LEN:SEED (random move
//!         sequences of LEN moves) | conj:N:SEED (N random cubes, each with
//!         its 48 conjugates and those of its inverse) | <file of scrambles>
//!   mode: stream | rec | share:C (at most C cubes in progress, 0 = all) |
//!         ref (an independent plain IDA*, see `Reference`)
//! ```
//!
//! Output: `<index> <length> <solution> | <scramble>` per cube in index
//! order, then a `lengths ...` summary. Every solution is checked: the exit
//! status is 1 if one is wrong, 2 if a cube is not reported (or twice).
use cubeopt::coord::cocsep;
use cubeopt::cubie::{
    conjugate, format_moves, inverse_move, parse_moves, symmetries, CubieCube, Rng,
};
use cubeopt::simd::Cube;
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::{entry_depth, h48_slots, Cocsep, H48Table};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;

/// Independent reference solver: plain IDA* over canonical sequences (no
/// face twice in a row, opposite faces in increasing order), pruned by the
/// larger table bound of the cube and of its inverse. It shares only the
/// cube operations and the table with the real solvers: not their index path
/// (`Cocsep::index` instead of `Solver::index`) nor any of their pruning.
struct Reference<'a> {
    corners: &'a Cocsep,
    table: &'a H48Table,
}

impl Reference<'_> {
    /// Table lower bound of one cube.
    fn table_bound(&self, cube: Cube) -> u8 {
        let corner_bound = entry_depth(self.corners.entries[cocsep(cube)]);
        let index = self.corners.index(cube, self.table.meta.k);
        assert!(index < h48_slots(self.table.meta.k));
        let (code, group_min) = self.table.get(index);
        if code == 0 {
            group_min.max(corner_bound)
        } else {
            self.table.meta.base + code
        }
    }

    /// Lower bound of a cube from both sides (`d(c) = d(c^-1)`).
    fn bound(&self, cube: Cube) -> u8 {
        self.table_bound(cube).max(self.table_bound(cube.inverse()))
    }

    /// Search for a solution of exactly `remaining` more moves.
    fn search(&self, cube: Cube, remaining: u8, last_face: u8, path: &mut Vec<u8>) -> bool {
        if remaining == 0 {
            return cube.is_solved();
        }
        // The bounds of all children first, so that their table loads overlap.
        let mut children = [(Cube::solved(), 0u8, 0u8); 18];
        let mut count = 0;
        for m in 0..18u8 {
            let face = m / 3;
            if last_face < 6 && (face == last_face || (face == (last_face ^ 1) && face < last_face))
            {
                continue;
            }
            let child = cube.apply_move(usize::from(m));
            children[count] = (child, m, self.bound(child));
            count += 1;
        }
        for &(child, m, bound) in &children[..count] {
            if bound < remaining {
                path.push(m);
                if self.search(child, remaining - 1, m / 3, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }

    fn solve(&self, cube: &CubieCube) -> Vec<u8> {
        let cube = Cube::from_cubie(cube);
        let mut path = Vec::new();
        if cube.is_solved() {
            return path;
        }
        let mut depth = self.bound(cube).max(1);
        while !self.search(cube, depth, 6, &mut path) {
            depth += 1;
            assert!(depth <= 20, "no solution within 20 moves");
        }
        path
    }
}

/// The cube set of a `set` argument (see the module doc).
fn load_set(spec: &str) -> Vec<CubieCube> {
    let numbers = |s: &str| -> Vec<u64> { s.split(':').map(|x| x.parse().unwrap()).collect() };
    let random_move = |rng: &mut Rng| u8::try_from(rng.below(18)).unwrap();
    if let Some(s) = spec.strip_prefix("random:") {
        let v = numbers(s);
        let mut rng = Rng(v[1]);
        return (0..v[0]).map(|_| CubieCube::random(&mut rng)).collect();
    }
    if let Some(s) = spec.strip_prefix("moves:") {
        let v = numbers(s);
        let mut rng = Rng(v[2]);
        return (0..v[0])
            .map(|_| {
                let moves: Vec<u8> = (0..v[1]).map(|_| random_move(&mut rng)).collect();
                CubieCube::SOLVED.apply_moves(&moves)
            })
            .collect();
    }
    if let Some(s) = spec.strip_prefix("conj:") {
        let v = numbers(s);
        let mut rng = Rng(v[1]);
        let mut cubes = Vec::new();
        for _ in 0..v[0] {
            let cube = CubieCube::random(&mut rng);
            for base in [cube, cube.inverse()] {
                cubes.extend(symmetries().iter().map(|s| conjugate(&base, s)));
            }
        }
        return cubes;
    }
    std::fs::read_to_string(spec)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| CubieCube::SOLVED.apply_moves(&parse_moves(l).expect("parse")))
        .collect()
}

/// Solve `cubes` with `threads` threads in `mode`; calls `record` per cube.
fn solve_all(
    solver: &Solver,
    reference: &Reference,
    cubes: &[CubieCube],
    mode: &str,
    threads: usize,
    lanes: usize,
    record: &(dyn Fn(usize, Vec<u8>, Stats, f64) + Sync),
) {
    let get = |i: usize| cubes.get(i).copied();
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    if let Some(c) = mode.strip_prefix("share:") {
        let max_active = match c.parse().unwrap() {
            0 => usize::MAX,
            c => c,
        };
        let pool = cubeopt::lanes::share::Pool::new(&get, max_active, threads);
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| pool.run(solver, lanes, &stop, record));
            }
        });
        eprintln!("share: {}", pool.counters());
        return;
    }
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| match mode {
                "stream" => cubeopt::lanes::run_lanes(solver, &get, &next, &stop, lanes, record),
                "rec" | "ref" => loop {
                    let i = next.fetch_add(1, Relaxed);
                    let Some(cube) = get(i) else { break };
                    let (solution, stats) = if mode == "rec" {
                        solver.solve(&cube)
                    } else {
                        (reference.solve(&cube), Stats::default())
                    };
                    record(i, solution, stats, 0.0);
                },
                _ => panic!("unknown mode {mode}"),
            });
        }
    });
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: xsolve <table> <set> <stream|rec|share:C|ref> [threads] [lanes]");
        std::process::exit(3);
    }
    let cubes = load_set(&args[2]);
    let mode = args[3].as_str();
    let threads: usize = args.get(4).map_or(8, |x| x.parse().unwrap());
    let lanes: usize = args.get(5).map_or(20, |x| x.parse().unwrap());
    let corners = Cocsep::generate();
    let table = H48Table::open(&args[1]).expect("open table");
    let solver = Solver::new(&corners, &table);
    let reference = Reference {
        corners: &corners,
        table: &table,
    };
    let solutions: Mutex<Vec<Option<Vec<u8>>>> = Mutex::new(vec![None; cubes.len()]);
    let reported_twice = AtomicBool::new(false);
    let nodes = AtomicU64::new(0);
    let record = |i: usize, solution: Vec<u8>, stats: Stats, _seconds: f64| {
        nodes.fetch_add(stats.nodes, Relaxed);
        let mut all = solutions.lock().unwrap();
        if all[i].is_some() {
            reported_twice.store(true, Relaxed);
        }
        all[i] = Some(solution);
    };
    let start = std::time::Instant::now();
    solve_all(&solver, &reference, &cubes, mode, threads, lanes, &record);
    let seconds = start.elapsed().as_secs_f64();
    let mut bad = false;
    let mut missing = reported_twice.load(Relaxed);
    let mut lengths = [0usize; 32];
    for (i, solution) in solutions.into_inner().unwrap().iter().enumerate() {
        let Some(solution) = solution else {
            println!("{i} MISSING");
            missing = true;
            continue;
        };
        let ok = cubes[i].apply_moves(solution) == CubieCube::SOLVED;
        bad |= !ok;
        lengths[solution.len()] += 1;
        // (The inverse of the solution is a scramble of the cube: input for
        // other solvers.)
        let scramble: Vec<u8> = solution.iter().rev().map(|&m| inverse_move(m)).collect();
        println!(
            "{i} {}{} {} | {}",
            solution.len(),
            if ok { "" } else { " BAD" },
            format_moves(solution),
            format_moves(&scramble)
        );
    }
    let histogram: Vec<String> = lengths
        .iter()
        .enumerate()
        .filter(|&(_, &count)| count > 0)
        .map(|(length, count)| format!("{length}:{count}"))
        .collect();
    println!(
        "lengths {} mode {mode} threads {threads} lanes {lanes} time {seconds:.2}s nodes/cube {}",
        histogram.join(" "),
        nodes.load(Relaxed) / u64::try_from(cubes.len().max(1)).unwrap()
    );
    if bad {
        std::process::exit(1);
    }
    if missing {
        std::process::exit(2);
    }
}
