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
use cubeopt::cubie::{parse_moves, CubieCube, Rng};
use cubeopt::lanes::share::Pool;
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::{Cocsep, H48Table};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// One solved cube.
struct Record {
    /// When it was solved (seconds since the start).
    finished: f64,
    /// Its solve time (seconds from its start).
    seconds: f64,
    length: usize,
    stats: Stats,
}

/// Solve `cubes(i)` for i = 0, 1, ... with `threads` threads (in `lanes`
/// lanes each; 0: the recursive solver; with `share`: the work-sharing mode
/// with that many cubes at once) until `cubes` returns `None` or `duration`
/// seconds have passed. Every solution is checked. Returns the records and
/// the elapsed seconds.
fn run(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    threads: usize,
    lanes: usize,
    duration: Option<f64>,
    share: Option<usize>,
) -> (Vec<Record>, f64) {
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let pool = share.map(|max_active| {
        let max_active = if max_active == 0 {
            threads * lanes
        } else {
            max_active
        };
        cubeopt::lanes::share::Pool::new(cubes, max_active, threads)
    });
    let records = Mutex::new(Vec::new());
    let bad = AtomicU64::new(0);
    let t0 = Instant::now();
    // (The stream mode's hot loop is the spawned closure, with `run_lanes`
    // and `Pool::run` inlined: see the commits' asm checks.)
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let record = |i: usize, solution: Vec<u8>, stats: Stats, seconds: f64| {
                    let cube = cubes(i).unwrap();
                    if cube.apply_moves(&solution) != CubieCube::SOLVED {
                        bad.fetch_add(1, Ordering::Relaxed);
                    }
                    records.lock().unwrap().push(Record {
                        finished: t0.elapsed().as_secs_f64(),
                        seconds,
                        length: solution.len(),
                        stats,
                    });
                };
                if let Some(pool) = &pool {
                    share_thread(pool, solver, lanes, &stop, record);
                } else if lanes > 0 {
                    stream_thread(solver, cubes, &next, &stop, lanes, record);
                } else {
                    while !stop.load(Ordering::Relaxed) {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(cube) = cubes(i) else { break };
                        let start = Instant::now();
                        let (solution, stats) = solver.solve(&cube);
                        record(i, solution, stats, start.elapsed().as_secs_f64());
                    }
                }
            });
        }
        if let Some(duration) = duration {
            std::thread::sleep(std::time::Duration::from_secs_f64(duration));
            stop.store(true, Ordering::Relaxed);
        }
    });
    assert_eq!(bad.load(Ordering::Relaxed), 0, "bad solutions");
    if let Some(pool) = &pool {
        eprintln!("share: {}", pool.counters());
    }
    (records.into_inner().unwrap(), t0.elapsed().as_secs_f64())
}

/// A thread of a share-mode run. (This and `stream_thread` are not inlined
/// into `run`'s thread closure: each mode is compiled on its own, so changes
/// to one cannot change the other's code, only where it is placed.)
#[inline(never)]
fn share_thread(
    pool: &Pool,
    solver: &Solver,
    lanes: usize,
    stop: &AtomicBool,
    record: impl FnMut(usize, Vec<u8>, Stats, f64),
) {
    pool.run(solver, lanes, stop, record);
}

/// A thread of a stream-mode run (`run_lanes`).
#[inline(never)]
fn stream_thread(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    next: &AtomicUsize,
    stop: &AtomicBool,
    lanes: usize,
    record: impl FnMut(usize, Vec<u8>, Stats, f64),
) {
    cubeopt::lanes::run_lanes(solver, cubes, next, stop, lanes, record);
}

/// Print nodes and lookups per cube and the distribution of lengths.
#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn summary(records: &[Record]) {
    let n = records.len().max(1) as f64;
    let nodes: u64 = records.iter().map(|r| r.stats.nodes).sum();
    let lookups: u64 = records.iter().map(|r| r.stats.lookups).sum();
    let mut by_length = [0usize; 32];
    for r in records {
        by_length[r.length] += 1;
    }
    let histogram: Vec<String> = by_length
        .iter()
        .enumerate()
        .filter(|(_, &count)| count > 0)
        .map(|(length, count)| format!("{length}:{count}"))
        .collect();
    println!(
        "nodes/cube {:.0} lookups/cube {:.0}",
        nodes as f64 / n,
        lookups as f64 / n
    );
    println!("lengths {}", histogram.join(" "));
}

/// Argument `i`, or `default` if it is not given.
fn arg<T: std::str::FromStr>(args: &[String], i: usize, default: T) -> T
where
    T::Err: std::fmt::Debug,
{
    args.get(i).map_or(default, |x| x.parse().unwrap())
}

#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let corners = Cocsep::generate();
    let load_start = Instant::now();
    let table = H48Table::open(&args[1]).expect("open table");
    eprintln!(
        "table h{} base {} loaded in {:.1}s",
        table.meta.k,
        table.meta.base,
        load_start.elapsed().as_secs_f64()
    );
    let solver = Solver::new(&corners, &table);
    // trailing `share[:C]`
    let share = match args.last() {
        Some(x) if x.starts_with("share") => {
            let max_active = x.strip_prefix("share:").map_or(0, |c| c.parse().unwrap());
            args.pop();
            Some(max_active)
        }
        _ => None,
    };
    let mode = share.map_or(String::new(), |c| format!(" share {c}"));
    if args[2] == "stream" {
        let warmup = arg(&args, 3, 3.0);
        let window = arg(&args, 4, 15.0);
        let threads = arg(&args, 5, cubeopt::thread_count());
        let lanes = arg(&args, 6, cubeopt::lanes::DEFAULT_LANES);
        let seed: u64 = arg(&args, 7, 2026);
        // Cube i of the stream: from its own seed, so that it does not
        // depend on the order in which the threads take the cubes.
        let cubes = move |i: usize| {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64));
            rng.next_u64();
            Some(CubieCube::random(&mut rng))
        };
        let (records, _) = run(
            &solver,
            &cubes,
            threads,
            lanes,
            Some(warmup + window),
            share,
        );
        let in_window: Vec<Record> = records
            .into_iter()
            .filter(|r| r.finished >= warmup && r.finished < warmup + window)
            .collect();
        println!("threads {threads} lanes {lanes} window {window}s{mode}");
        summary(&in_window);
        println!("throughput {:.3}", in_window.len() as f64 / window);
        return;
    }
    let set = &args[2];
    let mut cubes: Vec<CubieCube> = if let Some(spec) = set.strip_prefix("random:") {
        let numbers: Vec<u64> = spec.split(':').map(|x| x.parse().unwrap()).collect();
        let mut rng = Rng(numbers[1]);
        (0..numbers[0])
            .map(|_| CubieCube::random(&mut rng))
            .collect()
    } else {
        std::fs::read_to_string(set)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| CubieCube::SOLVED.apply_moves(&parse_moves(l).unwrap()))
            .collect()
    };
    if let Some(count) = args.get(3) {
        cubes.truncate(count.parse().unwrap());
    }
    let threads = arg(&args, 4, cubeopt::thread_count());
    let lanes = arg(&args, 5, cubeopt::lanes::DEFAULT_LANES);
    let get = |i: usize| cubes.get(i).copied();
    let (records, elapsed) = run(&solver, &get, threads, lanes, None, share);
    assert_eq!(records.len(), cubes.len(), "cubes not reported");
    let n = cubes.len() as f64;
    println!(
        "cubes {} threads {threads} lanes {lanes} time {elapsed:.3}s{mode}",
        cubes.len()
    );
    println!("solves/sec {:.3}", n / elapsed);
    summary(&records);
    let mut times: Vec<f64> = records.iter().map(|r| r.seconds).collect();
    times.sort_by(f64::total_cmp);
    println!(
        "solve time max {:.3}s p50 {:.4}s",
        times[times.len() - 1],
        times[times.len() / 2]
    );
}
