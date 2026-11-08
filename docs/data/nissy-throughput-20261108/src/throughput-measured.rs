//! Finite-batch throughput and lane-occupancy probe; uses the existing search APIs.
use egg_solver::{
    cubie::{format_moves, parse_moves, CubieCube},
    lanes::{self, share::Pool},
    solver::{Solver, Stats},
    tables::{
        file::{LoadMode, LoadOptions},
        Cocsep, H48Table,
    },
};
use std::{
    error::Error,
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Mutex, PoisonError,
    },
    time::Instant,
};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |i| {
        args.get(i)
            .map(String::as_str)
            .ok_or("usage: throughput TABLE INPUT THREADS LANES stream|CONCURRENCY")
    };
    let threads: usize = arg(3)?.parse()?;
    let lanes: usize = arg(4)?.parse()?;
    if threads == 0 || threads > 48 || lanes == 0 || lanes > 256 {
        return Err("invalid thread/lane configuration".into());
    }
    let mode = arg(5)?;
    let concurrency = if mode == "stream" {
        None
    } else {
        Some(mode.parse::<usize>()?)
    };
    let text = std::fs::read_to_string(arg(2)?)?;
    let cubes: Vec<CubieCube> = text
        .lines()
        .map(|s| {
            parse_moves(s)
                .and_then(|m| CubieCube::SOLVED.apply_moves(&m))
                .ok_or("invalid scramble")
        })
        .collect::<Result<_, _>>()?;
    if cubes.is_empty() {
        return Err("empty input".into());
    }
    // SAFETY: benchmark tables are retained, immutable files for this process lifetime.
    let table = unsafe {
        H48Table::open_with(
            arg(1)?,
            &LoadOptions {
                mode: LoadMode::Map,
                prefault: true,
                ..LoadOptions::default()
            },
        )
    }?;
    let corners = Cocsep::generate()?;
    let solver = Solver::new(&corners, &table);
    let get = |i| cubes.get(i).copied();
    let pool = concurrency.map(|c| Pool::new(&get, if c == 0 { usize::MAX } else { c }, threads));
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let failures = AtomicUsize::new(0);
    let solved = AtomicUsize::new(0);
    let nodes = AtomicU64::new(0);
    let lookups = AtomicU64::new(0);
    let stdout = Mutex::new(io::stdout());
    let done = |i: usize, solution: Option<Vec<u8>>, stats: Stats, seconds: f64| {
        let valid = solution.as_deref().is_some_and(|m| {
            cubes.get(i).and_then(|c| c.apply_moves(m)) == Some(CubieCube::SOLVED)
        });
        let length = solution.as_ref().map(Vec::len);
        let moves = solution.as_deref().map(format_moves);
        if !valid {
            failures.fetch_add(1, Ordering::Relaxed);
        }
        solved.fetch_add(1, Ordering::Relaxed);
        nodes.fetch_add(stats.nodes, Ordering::Relaxed);
        lookups.fetch_add(stats.lookups, Ordering::Relaxed);
        let mut out = stdout.lock().unwrap_or_else(PoisonError::into_inner);
        let result = serde_json::to_writer(
            &mut *out,
            &serde_json::json!({"type":"solved","index":i,"length":length,"moves":moves,"nodes":stats.nodes,"lookups":stats.lookups,"seconds":seconds,"ok":valid}),
        );
        if result.is_err() || writeln!(out).is_err() {
            failures.fetch_add(1, Ordering::Relaxed);
            stop.store(true, Ordering::Relaxed);
        }
    };
    let start = Instant::now();
    std::thread::scope(|scope| -> io::Result<()> {
        let mut handles = Vec::new();
        let mut error = None;
        for _ in 0..threads {
            match std::thread::Builder::new().spawn_scoped(scope, || {
                if let Some(p) = &pool {
                    p.run(&solver, lanes, &stop, &done)
                } else {
                    lanes::run_lanes(&solver, &get, &next, &stop, lanes, &done)
                }
            }) {
                Ok(h) => handles.push(h),
                Err(e) => {
                    stop.store(true, Ordering::Relaxed);
                    error = Some(e);
                    break;
                }
            }
        }
        for h in handles {
            if h.join().is_err() {
                error = Some(io::Error::other("worker panicked"));
            }
        }
        error.map_or(Ok(()), Err)
    })?;
    let wall = start.elapsed().as_secs_f64();
    let count = solved.load(Ordering::Relaxed);
    let mut out = stdout.lock().unwrap_or_else(PoisonError::into_inner);
    serde_json::to_writer(
        &mut *out,
        &serde_json::json!({"type":"total","cubes":count,"wall":wall,"wall_per_cube":wall/cubes.len() as f64,"nodes_per_cube":nodes.load(Ordering::Relaxed) as f64/cubes.len() as f64,"lookups_per_cube":lookups.load(Ordering::Relaxed) as f64/cubes.len() as f64,"threads":threads,"lanes":lanes,"mode":mode,"failures":failures.load(Ordering::Relaxed),"counters":pool.as_ref().map(Pool::counters)}),
    )?;
    writeln!(out)?;
    if count != cubes.len() || failures.load(Ordering::Relaxed) > 0 {
        return Err("incomplete or invalid solutions".into());
    }
    Ok(())
}
