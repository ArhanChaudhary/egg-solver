//! Solve scrambles read from stdin (one per line, e.g. `R3 B1 F2 L2 U3`, as
//! in `cube_solver_test`) optimally with the work-sharing lanes
//! (`lanes::share`).
//!
//! ```text
//! Usage: solve [OPTIONS] <TABLE> < scrambles
//!   -t  threads (default: all cores)
//!   -l  interleaved lanes per thread (default: from `<table>.tune` if
//!       `bench <table> tune` made one, else `tables::file::untuned_lanes`)
//!   -c  cubes in progress at once: 1 solves one cube at a time with all
//!       threads (latency); 0 (default) solves the whole input as a batch
//!   -n  solve only the first `count` scrambles
//! ```
//! (`solve --help` has all options.)
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
//! `BAD <index>` and makes the exit status 1. With `--json`, the same as
//! JSON lines (`Line`).
use anyhow::Context as _;
use clap::Parser;
use cubeopt::cli::{self, BadInput, LoadArgs, Verbosity};
use cubeopt::cubie::{format_moves, parse_moves, CubieCube};
use cubeopt::lanes::share::Pool;
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::Cocsep;
use std::io::{BufRead, Write};
use std::process::ExitCode;
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

/// Solve Rubik's cube scrambles from stdin optimally (fewest moves, any
/// face turn is one move).
#[derive(Parser, Debug)]
#[command(name = "solve", version, after_long_help = OUTPUT_HELP)]
struct Args {
    /// The table file (made by `gen`, e.g. `gen 8 auto h8`).
    #[arg(env = "CUBEOPT_TABLE", required_unless_present = "completions")]
    table: Option<String>,
    /// Threads.
    #[arg(
        short,
        long,
        env = "CUBEOPT_THREADS",
        default_value_t = cubeopt::thread_count(),
        value_parser = cli::positive(),
    )]
    threads: usize,
    /// Interleaved lanes per thread [default: from `<TABLE>.tune` if `bench
    /// tune` made one, else a default for the table's size].
    #[arg(short, long, value_parser = cli::positive())]
    lanes: Option<usize>,
    /// Cubes in progress at once: 1 solves one cube at a time with all
    /// threads (lowest latency); 0 the whole input as a batch (highest
    /// throughput).
    #[arg(short, long, default_value_t = 0)]
    concurrency: usize,
    /// Solve only the first COUNT scrambles.
    #[arg(short = 'n', long)]
    count: Option<usize>,
    /// Print JSON lines: one object per cube, then the total.
    #[arg(long)]
    json: bool,
    #[command(flatten)]
    load: LoadArgs,
    #[command(flatten)]
    verbosity: Verbosity,
    /// Print a shell completion script.
    #[arg(long, value_name = "SHELL", exclusive = true, hide = true)]
    completions: Option<clap_complete::Shell>,
}

const OUTPUT_HELP: &str = "\
Output, one line per cube in completion order, then a summary:
  solved <index> <length> <nodes> <seconds> <solution>
  total <cubes> cubes wall <s> wall/cube <s> nodes/cube <n> threads <t> lanes <l> concurrency <c>
<seconds> is the cube's time from its start to its solution (with -c 1, its
latency). Every solution is verified: a wrong one is reported as BAD <index>
and makes the exit status 1. With --json, the same as JSON lines:
  {\"type\":\"solved\",\"index\":..,\"moves\":..,\"length\":..,\"nodes\":..,\"lookups\":..,\"seconds\":..,\"ok\":true}
  {\"type\":\"total\",\"cubes\":..,\"wall\":..,\"wall_per_cube\":..,\"nodes_per_cube\":..,\"threads\":..,\"lanes\":..,\"concurrency\":..}";

/// A line of `--json` output.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Line {
    /// A solved cube (`index`: its line among the scrambles, from 0).
    Solved {
        index: usize,
        moves: String,
        length: usize,
        nodes: u64,
        lookups: u64,
        seconds: f64,
        /// Whether the solution solves the cube (always, unless a bug).
        ok: bool,
    },
    /// The summary, after all cubes.
    Total {
        cubes: usize,
        wall: f64,
        wall_per_cube: f64,
        nodes_per_cube: f64,
        threads: usize,
        lanes: usize,
        concurrency: usize,
    },
}

/// Print `value` as one line of JSON (stdout flushes at the newline).
fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    if let Some(shell) = args.completions {
        cli::print_completions::<Args>(shell);
        return ExitCode::SUCCESS;
    }
    args.verbosity.init_logging();
    cli::exit_status("solve", run(&args))
}

/// The cubes of the scrambles on stdin, one per line (blank lines
/// skipped), at most `count`.
fn read_scrambles(count: usize) -> anyhow::Result<Vec<CubieCube>> {
    let mut cubes = Vec::new();
    for (n, line) in std::io::stdin().lock().lines().enumerate() {
        let line = line.context("cannot read stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        if cubes.len() >= count {
            break;
        }
        let moves = parse_moves(&line)
            .ok_or_else(|| BadInput(format!("line {}: cannot parse {line:?}", n + 1)))?;
        cubes.push(CubieCube::SOLVED.apply_moves(&moves));
    }
    Ok(cubes)
}

#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn run(args: &Args) -> anyhow::Result<ExitCode> {
    let path = args.table.as_deref().context("no table")?;
    let cubes = read_scrambles(args.count.unwrap_or(usize::MAX))?;
    let corners = Cocsep::generate();
    let table = args.load.open(path)?;
    let threads = args.threads;
    let lanes = cli::lanes(args.lanes, path, table.meta.k, threads);
    let concurrency = args.concurrency;
    log::info!(
        "{} cubes, at most {concurrency} at once (0: all)",
        cubes.len()
    );
    let solver = Solver::new(&corners, &table);
    let get = |i: usize| cubes.get(i).copied();
    let max_active = if concurrency == 0 {
        usize::MAX
    } else {
        concurrency
    };
    let json = args.json;
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
        }
        if json {
            let line = Line::Solved {
                index: i,
                moves: format_moves(&solution),
                length: solution.len(),
                nodes: stats.nodes,
                lookups: stats.lookups,
                seconds,
                ok,
            };
            let _ = serde_json::to_writer(&mut *out, &line);
            let _ = writeln!(out);
        } else {
            if !ok {
                let _ = writeln!(out, "BAD {i}");
            }
            let _ = writeln!(
                out,
                "solved {i} {} {} {seconds:.6} {}",
                solution.len(),
                stats.nodes,
                format_moves(&solution)
            );
        }
        let _ = out.flush();
    };
    let start = Instant::now();
    solve_all_pinned(&solver, &get, threads, lanes, max_active, &done);
    let wall = start.elapsed().as_secs_f64();
    let n = cubes.len();
    let reported = n_done.load(Ordering::Relaxed);
    anyhow::ensure!(reported == n, "{reported} of {n} cubes reported");
    let wall_per_cube = wall / n.max(1) as f64;
    let nodes_per_cube = nodes.load(Ordering::Relaxed) as f64 / n.max(1) as f64;
    if json {
        print_json(&Line::Total {
            cubes: n,
            wall,
            wall_per_cube,
            nodes_per_cube,
            threads,
            lanes,
            concurrency,
        })?;
    } else {
        println!(
            "total {n} cubes wall {wall:.6} wall/cube {wall_per_cube:.6} nodes/cube {nodes_per_cube:.0} threads {threads} lanes {lanes} concurrency {concurrency}"
        );
    }
    Ok(if bad.load(Ordering::Relaxed) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
