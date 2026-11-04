//! Solve scrambles read from stdin (one per line, e.g. `R3 B1 F2 L2 U3`, as
//! in `cube_solver_test`) optimally with the work-sharing lanes
//! (`lanes::share`).
//!
//! ```text
//! Usage: solve <table> [-t threads] [-l lanes] [-c concurrency] [-n count]
//!   -t  threads (default: all cores)
//!   -l  interleaved lanes per thread (default `lanes::DEFAULT_LANES`, 24)
//!   -c  cubes in progress at once: 1 solves one cube at a time with all
//!       threads (latency); 0 (default) solves the whole input as a batch
//!   -n  solve only the first `count` scrambles
//! ```
//!
//! Output, one line per cube in completion order, then a summary:
//!
//! ```text
//! solved <index> <length> <nodes> <seconds> <solution>
//! total <cubes> cubes wall <seconds> wall/cube <seconds> nodes/cube <n> ...
//! ```
//!
//! `seconds` is the cube's time from its start to its solution (with -c 1,
//! its latency). Every solution is verified; a wrong one is reported as
//! `BAD <index>` and makes the exit status 1.
use cubeopt::cubie::{format_moves, parse_moves, CubieCube};
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::{Cocsep, H48Table};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

fn usage() -> ! {
    eprintln!(
        "usage: solve <table> [-t threads] [-l lanes] [-c concurrency] [-n count] < scrambles"
    );
    std::process::exit(2);
}

#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let mut threads = cubeopt::thread_count();
    let mut lanes = cubeopt::lanes::DEFAULT_LANES;
    let mut conc = 0;
    let mut count = usize::MAX;
    let mut i = 2;
    while i < args.len() {
        let v = args.get(i + 1).and_then(|x| x.parse::<usize>().ok());
        let Some(v) = v else { usage() };
        match args[i].as_str() {
            "-t" => threads = v.max(1),
            "-l" => lanes = v.max(1),
            "-c" => conc = v,
            "-n" => count = v,
            _ => usage(),
        }
        i += 2;
    }
    let mut cubes = Vec::new();
    for (n, line) in std::io::stdin().lock().lines().enumerate() {
        let line = line.expect("read stdin");
        if line.trim().is_empty() {
            continue;
        }
        if cubes.len() >= count {
            break;
        }
        let Some(ms) = parse_moves(&line) else {
            eprintln!("line {}: cannot parse {line:?}", n + 1);
            std::process::exit(2);
        };
        cubes.push(CubieCube::SOLVED.apply_moves(&ms));
    }
    let cs = Cocsep::generate();
    let table = H48Table::open(&args[1]).expect("open table");
    let solver = Solver::new(&cs, &table);
    let get = |i: usize| cubes.get(i).copied();
    let max_active = if conc == 0 { usize::MAX } else { conc };
    let out = Mutex::new(std::io::stdout());
    let nodes = AtomicU64::new(0);
    let bad = AtomicBool::new(false);
    let ndone = AtomicUsize::new(0);
    let done = |i: usize, sol: Vec<u8>, st: Stats, dt: f64| {
        let ok = cubes[i].apply_moves(&sol) == CubieCube::SOLVED;
        nodes.fetch_add(st.nodes, Ordering::Relaxed);
        ndone.fetch_add(1, Ordering::Relaxed);
        let mut o = out.lock().unwrap();
        if !ok {
            bad.store(true, Ordering::Relaxed);
            let _ = writeln!(o, "BAD {i}");
        }
        let _ = writeln!(
            o,
            "solved {i} {} {} {dt:.6} {}",
            sol.len(),
            st.nodes,
            format_moves(&sol)
        );
        let _ = o.flush();
    };
    let t0 = Instant::now();
    cubeopt::lanes::share::solve_all(&solver, &get, threads, lanes, max_active, &done);
    let wall = t0.elapsed().as_secs_f64();
    let n = cubes.len();
    assert_eq!(ndone.load(Ordering::Relaxed), n, "cubes not reported");
    println!(
        "total {n} cubes wall {wall:.6} wall/cube {:.6} nodes/cube {:.0} threads {threads} lanes {lanes} concurrency {conc}",
        wall / n.max(1) as f64,
        nodes.load(Ordering::Relaxed) as f64 / n.max(1) as f64
    );
    if bad.load(Ordering::Relaxed) {
        std::process::exit(1);
    }
}
