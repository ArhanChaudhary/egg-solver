#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]
use cubeopt::coord::*;
use cubeopt::cubie::*;
use cubeopt::simd::Cube;
use cubeopt::solver::Solver;
use cubeopt::tables::*;
use std::collections::HashMap;
use std::sync::OnceLock;

fn cs_data() -> &'static Cocsep {
    static C: OnceLock<Cocsep> = OnceLock::new();
    C.get_or_init(Cocsep::generate)
}

fn h0() -> &'static H48Table {
    static T: OnceLock<H48Table> = OnceLock::new();
    T.get_or_init(|| cubeopt::generate(cs_data(), 0, 8, None, &[], cubeopt::nthreads()).0)
}

fn dataset(name: &str) -> Vec<CubieCube> {
    let path = format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| CubieCube::SOLVED.apply_moves(&parse_moves(l).unwrap()))
        .collect()
}

fn par_solve(cubes: &[CubieCube], table: &H48Table) -> Vec<Vec<u8>> {
    let solver = Solver::new(cs_data(), table);
    let out = std::sync::Mutex::new(vec![vec![]; cubes.len()]);
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..cubeopt::nthreads() {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= cubes.len() {
                    break;
                }
                let (sol, _) = solver.solve(&cubes[i]);
                out.lock().unwrap()[i] = sol;
            });
        }
    });
    out.into_inner().unwrap()
}

/// All positions within `depth` moves, with their distance.
fn ball(depth: usize) -> HashMap<[u8; 32], (Cube, u8)> {
    let mut seen: HashMap<[u8; 32], (Cube, u8)> = HashMap::new();
    let mut frontier = vec![Cube::solved()];
    seen.insert(Cube::solved().to_bytes(), (Cube::solved(), 0));
    for d in 1..=depth {
        let mut next = Vec::new();
        for c in frontier {
            for m in 0..18 {
                let n = c.apply_move(m);
                let k = n.to_bytes();
                if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(k) {
                    e.insert((n, d as u8));
                    next.push(n);
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
    let cs = cs_data();
    let base = 3u8;
    let (t, _) = cubeopt::generate(cs, 0, base, None, &[], cubeopt::nthreads());
    let b = ball(5);
    let mut best: HashMap<u64, u8> = HashMap::new();
    for &(c, d) in b.values() {
        let idx = cs.index(c, 0);
        let e = best.entry(idx).or_insert(d);
        *e = (*e).min(d);
        // admissible
        let (code, min) = t.get(idx);
        let v = if code == 0 { min } else { base + code };
        assert!(v <= d, "inadmissible: idx {idx} value {v} > distance {d}");
    }
    let mut want = [0u64; 3];
    for (&idx, &d) in best.iter() {
        let (code, _) = t.get(idx);
        let expect = d.saturating_sub(base).min(3);
        assert_eq!(code, expect, "idx {idx} depth {d}");
        if d <= base + 2 {
            want[expect as usize] += 1;
        }
    }
    let dist = t.distribution();
    assert_eq!(&dist[..3], &want[..], "entries with depth <= base+2");
}

#[test]
fn solves_known_depth_15() {
    let cubes: Vec<CubieCube> = dataset("depth15.txt").into_iter().take(48).collect();
    let sols = par_solve(&cubes, h0());
    for (c, s) in cubes.iter().zip(sols.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), 15);
    }
}

/// Independent reference: plain IDA* with only the corner (cocsep) bound.
fn reference_length(c: &CubieCube, cs: &Cocsep) -> usize {
    fn dfs(c: Cube, cs: &Cocsep, depth: u8, last: usize) -> bool {
        if depth == 0 {
            return c.is_solved();
        }
        if cd_depth(cs.data[cocsep(c)]) > depth {
            return false;
        }
        for m in 0..18 {
            if m / 3 == last {
                continue;
            }
            if dfs(c.apply_move(m), cs, depth - 1, m / 3) {
                return true;
            }
        }
        false
    }
    let c = Cube::from_cubie(c);
    (0..=20).find(|&d| dfs(c, cs, d as u8, 99)).unwrap()
}

#[test]
fn short_scrambles_are_optimal() {
    let mut rng = Rng(42);
    let cs = cs_data();
    let mut cubes = Vec::new();
    for len in 0..=8 {
        for _ in 0..12 {
            let ms: Vec<u8> = (0..len).map(|_| rng.below(18) as u8).collect();
            cubes.push(CubieCube::SOLVED.apply_moves(&ms));
        }
    }
    let sols = par_solve(&cubes, h0());
    for (c, s) in cubes.iter().zip(sols.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), reference_length(c, cs), "scramble {:?}", c);
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
    let sols = par_solve(&cubes, h0());
    for ((c, s), (_, len)) in cubes.iter().zip(sols.iter()).zip(cases.iter()) {
        assert_eq!(c.apply_moves(s), CubieCube::SOLVED);
        assert_eq!(s.len(), *len);
    }
}

#[test]
fn table_value_is_symmetry_invariant() {
    let cs = cs_data();
    let t = h0();
    let mut rng = Rng(99);
    for _ in 0..500 {
        let c = Cube::from_cubie(&CubieCube::random(&mut rng));
        let v0 = t.get(cs.index(c, 0)).0;
        for s in 0..48 {
            assert_eq!(t.get(cs.index(c.transform(s), 0)).0, v0);
        }
    }
}
