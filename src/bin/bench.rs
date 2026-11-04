#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]
//! Optimal-solve throughput benchmark.
//!
//! Usage:
//!   bench <table> stream <warmup s> <window s> [threads] [lanes] [seed]
//!       Endless deterministic stream of uniformly random states; reports
//!       solves completed per second during the window (steady state).
//!   bench <table> <scramble file | random:N:SEED> [count] [threads] [lanes]
//!       Solves a fixed batch and verifies every solution.
//! lanes = 0 uses the recursive single-cube solver; default
//! `lanes::DEFAULT_LANES` (24).
//! A last argument `share[:C]` uses the work-sharing mode (`lanes::share`)
//! with at most C cubes in progress at once (default: threads * lanes; 1
//! solves one cube at a time with all threads).
use cubeopt::cubie::*;
use cubeopt::lanes::share::Pool;
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::*;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

struct Rec {
    t: f64,
    dt: f64,
    len: usize,
    st: Stats,
}

fn run(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    threads: usize,
    nlanes: usize,
    duration: Option<f64>,
    share: Option<usize>,
) -> (Vec<Rec>, f64) {
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let pool = share.map(|c| {
        let c = if c == 0 { threads * nlanes } else { c };
        cubeopt::lanes::share::Pool::new(cubes, c, threads)
    });
    let recs = Mutex::new(Vec::new());
    let bad = AtomicU64::new(0);
    let t0 = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let record = |i: usize, sol: Vec<u8>, st: Stats, dt: f64| {
                    let c = cubes(i).unwrap();
                    if c.apply_moves(&sol) != CubieCube::SOLVED {
                        bad.fetch_add(1, Ordering::Relaxed);
                    }
                    recs.lock().unwrap().push(Rec {
                        t: t0.elapsed().as_secs_f64(),
                        dt,
                        len: sol.len(),
                        st,
                    });
                };
                if let Some(pool) = &pool {
                    share_thread(pool, solver, nlanes, &stop, record);
                } else if nlanes > 0 {
                    stream_thread(solver, cubes, &next, &stop, nlanes, record);
                } else {
                    while !stop.load(Ordering::Relaxed) {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(c) = cubes(i) else { break };
                        let ts = Instant::now();
                        let (sol, st) = solver.solve(&c);
                        record(i, sol, st, ts.elapsed().as_secs_f64());
                    }
                }
            });
        }
        if let Some(d) = duration {
            std::thread::sleep(std::time::Duration::from_secs_f64(d));
            stop.store(true, Ordering::Relaxed);
        }
    });
    assert_eq!(bad.load(Ordering::Relaxed), 0, "bad solutions");
    if let Some(pool) = &pool {
        eprintln!("share: {}", pool.counters());
    }
    (recs.into_inner().unwrap(), t0.elapsed().as_secs_f64())
}

/// A thread of a share-mode run. (This and `stream_thread` are not inlined
/// into `run`'s thread closure: each mode is compiled on its own, so changes
/// to one cannot change the other's code, only where it is placed.)
#[inline(never)]
fn share_thread(
    pool: &Pool,
    solver: &Solver,
    nlanes: usize,
    stop: &AtomicBool,
    record: impl FnMut(usize, Vec<u8>, Stats, f64),
) {
    pool.run(solver, nlanes, stop, record);
}

/// A thread of a stream-mode run (`run_lanes`).
#[inline(never)]
fn stream_thread(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    next: &AtomicUsize,
    stop: &AtomicBool,
    nlanes: usize,
    record: impl FnMut(usize, Vec<u8>, Stats, f64),
) {
    cubeopt::lanes::run_lanes(solver, cubes, next, stop, nlanes, record);
}

fn summary(recs: &[Rec]) {
    let n = recs.len().max(1) as f64;
    let nodes: u64 = recs.iter().map(|r| r.st.nodes).sum();
    let lookups: u64 = recs.iter().map(|r| r.st.lookups).sum();
    let mut lens = [0usize; 32];
    for r in recs {
        lens[r.len] += 1;
    }
    let dist: Vec<String> = lens
        .iter()
        .enumerate()
        .filter(|(_, &c)| c > 0)
        .map(|(l, c)| format!("{l}:{c}"))
        .collect();
    println!(
        "nodes/cube {:.0} lookups/cube {:.0}",
        nodes as f64 / n,
        lookups as f64 / n
    );
    println!("lengths {}", dist.join(" "));
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let cs = Cocsep::generate();
    let tl = Instant::now();
    let table = H48Table::open(&a[1]).expect("open table");
    eprintln!(
        "table h{} base {} loaded in {:.1}s",
        table.meta.k,
        table.meta.base,
        tl.elapsed().as_secs_f64()
    );
    let solver = Solver::new(&cs, &table);
    // trailing `share[:C]`
    let mut a = a;
    let share = match a.last() {
        Some(x) if x.starts_with("share") => {
            let c = x.strip_prefix("share:").map_or(0, |c| c.parse().unwrap());
            a.pop();
            Some(c)
        }
        _ => None,
    };
    let arg = |i: usize, d: f64| a.get(i).map(|x| x.parse().unwrap()).unwrap_or(d);
    if a[2] == "stream" {
        let warm = arg(3, 3.0);
        let window = arg(4, 15.0);
        let threads = arg(5, cubeopt::nthreads() as f64) as usize;
        let nlanes: usize = a
            .get(6)
            .map_or(cubeopt::lanes::DEFAULT_LANES, |x| x.parse().unwrap());
        let seed = arg(7, 2026.0) as u64;
        let cubes = move |i: usize| {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64));
            rng.next_u64();
            Some(CubieCube::random(&mut rng))
        };
        let (recs, _) = run(&solver, &cubes, threads, nlanes, Some(warm + window), share);
        let w: Vec<Rec> = recs
            .into_iter()
            .filter(|r| r.t >= warm && r.t < warm + window)
            .collect();
        let mode = share.map_or(String::new(), |c| format!(" share {c}"));
        println!("threads {threads} lanes {nlanes} window {window}s{mode}");
        summary(&w);
        println!("throughput {:.3}", w.len() as f64 / window);
        return;
    }
    let file = &a[2];
    let mut cubes: Vec<CubieCube> = if let Some(spec) = file.strip_prefix("random:") {
        let v: Vec<u64> = spec.split(':').map(|x| x.parse().unwrap()).collect();
        let mut rng = Rng(v[1]);
        (0..v[0]).map(|_| CubieCube::random(&mut rng)).collect()
    } else {
        std::fs::read_to_string(file)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| CubieCube::SOLVED.apply_moves(&parse_moves(l).unwrap()))
            .collect()
    };
    if let Some(n) = a.get(3) {
        cubes.truncate(n.parse().unwrap());
    }
    let threads = arg(4, cubeopt::nthreads() as f64) as usize;
    let nlanes: usize = a
        .get(5)
        .map_or(cubeopt::lanes::DEFAULT_LANES, |x| x.parse().unwrap());
    let get = |i: usize| cubes.get(i).copied();
    let (recs, el) = run(&solver, &get, threads, nlanes, None, share);
    assert_eq!(recs.len(), cubes.len(), "cubes not reported");
    let n = cubes.len() as f64;
    let mode = share.map_or(String::new(), |c| format!(" share {c}"));
    println!(
        "cubes {} threads {threads} lanes {nlanes} time {el:.3}s{mode}",
        cubes.len()
    );
    println!("solves/sec {:.3}", n / el);
    summary(&recs);
    let mut t: Vec<f64> = recs.iter().map(|r| r.dt).collect();
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "solve time max {:.3}s p50 {:.4}s",
        t[t.len() - 1],
        t[t.len() / 2]
    );
}
