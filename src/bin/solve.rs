//! Solve scrambles read from stdin (one per line, e.g. `R3 B1 F2 L2 U3`, as
//! in `cube_solver_test`) optimally with the work-sharing lanes
//! (`lanes::share`).
//!
//! ```text
//! Usage: solve <table> [-t threads] [-l lanes] [-c concurrency] [-n count]
//!   -t  threads (default: all cores)
//!   -l  interleaved lanes per thread (default: from `<table>.tune` if
//!       `bench <table> tune` made one, else `tables::file::untuned_lanes`)
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
use cubeopt::lanes::share::Pool;
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::{Cocsep, H48Table};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

// `share::Worker::rounds`'s section starts on a page boundary, `SHARE_LINE`
// bytes before the library's 64-byte pad (a binary's module asm comes before
// the library's), so the round loop starts at page offset 0x310, as on main,
// whatever code or data is added before it (`.text` starts after the
// unwind tables, which grow with every function). Only in `solve`: `bench`
// keeps main's layout (s9).
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
const SHARE_LINE: usize = 0x300;
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
core::arch::global_asm!(
    ".pushsection .text.hot_share,\"ax\",@progbits",
    ".p2align 12",
    ".skip {pad}, 0xcc",
    ".popsection",
    pad = const SHARE_LINE,
);

/// `share::solve_all` with `Pool::run_pinned`: each worker's state and stack
/// frames at fixed offsets within a page (main's offsets, so -c 1 is
/// unchanged; see `Pool::run_pinned`).
fn solve_all_pinned(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    threads: usize,
    lanes: usize,
    max_active: usize,
    done: &(dyn Fn(usize, Vec<u8>, Stats, f64) + Sync),
) {
    let pool = Pool::new(cubes, max_active, threads);
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| pool.run_pinned(solver, lanes, &stop, done));
        }
    });
}

/// Print the usage and exit with status 2.
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
    let mut lanes = None;
    let mut concurrency = 0;
    let mut count = usize::MAX;
    // Options are pairs: `-x value`.
    for option in args[2..].chunks(2) {
        let value = option.get(1).and_then(|x| x.parse::<usize>().ok());
        let Some(value) = value else { usage() };
        match option[0].as_str() {
            "-t" => threads = value.max(1),
            "-l" => lanes = Some(value.max(1)),
            "-c" => concurrency = value,
            "-n" => count = value,
            _ => usage(),
        }
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
        let Some(moves) = parse_moves(&line) else {
            eprintln!("line {}: cannot parse {line:?}", n + 1);
            std::process::exit(2);
        };
        cubes.push(CubieCube::SOLVED.apply_moves(&moves));
    }
    let corners = Cocsep::generate();
    let table = H48Table::open(&args[1]).unwrap_or_else(|e| {
        eprintln!("solve: cannot open table {}: {e}", args[1]);
        std::process::exit(2);
    });
    let default_lanes = cubeopt::tables::file::default_lanes(&args[1], table.meta.k);
    let lanes = lanes.unwrap_or_else(|| default_lanes(threads));
    let solver = Solver::new(&corners, &table);
    let get = |i: usize| cubes.get(i).copied();
    let max_active = if concurrency == 0 {
        usize::MAX
    } else {
        concurrency
    };
    let stdout = Mutex::new(std::io::stdout());
    let nodes = AtomicU64::new(0);
    let bad = AtomicBool::new(false);
    let n_done = AtomicUsize::new(0);
    let done = |i: usize, solution: Vec<u8>, stats: Stats, seconds: f64| {
        let ok = cubes[i].apply_moves(&solution) == CubieCube::SOLVED;
        nodes.fetch_add(stats.nodes, Ordering::Relaxed);
        n_done.fetch_add(1, Ordering::Relaxed);
        let mut out = stdout.lock().unwrap();
        if !ok {
            bad.store(true, Ordering::Relaxed);
            let _ = writeln!(out, "BAD {i}");
        }
        let _ = writeln!(
            out,
            "solved {i} {} {} {seconds:.6} {}",
            solution.len(),
            stats.nodes,
            format_moves(&solution)
        );
        let _ = out.flush();
    };
    let start = Instant::now();
    solve_all_pinned(&solver, &get, threads, lanes, max_active, &done);
    let wall = start.elapsed().as_secs_f64();
    let n = cubes.len();
    assert_eq!(n_done.load(Ordering::Relaxed), n, "cubes not reported");
    println!(
        "total {n} cubes wall {wall:.6} wall/cube {:.6} nodes/cube {:.0} threads {threads} lanes {lanes} concurrency {concurrency}",
        wall / n.max(1) as f64,
        nodes.load(Ordering::Relaxed) as f64 / n.max(1) as f64
    );
    if bad.load(Ordering::Relaxed) {
        std::process::exit(1);
    }
}
