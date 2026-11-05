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
//!
//! `solve --fast [SCRAMBLE]...` solves one scramble as fast as this machine
//! can (what fast.sh did): the largest table found, all threads on the one
//! cube (`-c 1`), a warm-up cube first; see `run_fast`.
//!
//! `solve --shm-load TABLE` loads a table into a shared memory segment that
//! later runs attach at once (`--shm`; README.md "Shared memory"), `solve
//! --shm-remove TABLE` removes it.
use anyhow::Context as _;
use clap::Parser;
use cubeopt::cli::{self, BadInput, LoadArgs, Verbosity};
use cubeopt::cubie::{format_moves, parse_moves, CubieCube, Rng};
use cubeopt::lanes::share::Pool;
use cubeopt::solver::{Solver, Stats};
use cubeopt::tables::Cocsep;
use std::hash::{Hash as _, Hasher as _};
use std::io::{BufRead, Write};
use std::path::Path;
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
    /// The table file (made by `gen`, e.g. `gen 8 auto h8`) [with --fast:
    /// the largest found].
    #[arg(required_unless_present_any = [
        "completions",
        "fast",
        "table_flag",
        "shm_load",
        "shm_remove",
    ])]
    table: Option<String>,
    /// The table file (as TABLE).
    #[arg(
        long = "table",
        id = "table_flag",
        value_name = "TABLE",
        env = "CUBEOPT_TABLE"
    )]
    table_flag: Option<String>,
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
    /// Print JSON lines: one object per cube, then the total (with --fast:
    /// one object).
    #[arg(long)]
    json: bool,
    /// Solve one scramble (the arguments, or a random 25-move one) as fast
    /// as this machine can and show the time: the largest table found (hK,
    /// K from 11 down, in /mnt/huge, /data2/tables, /data/tables, ./tables),
    /// all threads on the one cube (-c 1), after a warm-up cube; the time
    /// is the cube's own (table load excluded). Don't run it beside other
    /// heavy jobs: they share the cores and the time.
    #[arg(long, conflicts_with_all = ["concurrency", "count"])]
    fast: bool,
    /// With --fast: the scramble (moves as arguments, or one quoted
    /// string) [default: a random 25-move scramble].
    #[arg(value_name = "SCRAMBLE", requires = "fast")]
    scramble: Vec<String>,
    /// With --fast: the cube solved first, to warm up threads, caches and
    /// TLBs ("" for none) [default: a fixed 25-move scramble].
    #[arg(long, value_name = "SCRAMBLE", requires = "fast")]
    warmup: Option<String>,
    #[command(flatten)]
    load: LoadArgs,
    /// Load TABLE into its shared memory segment (as `--shm` does, if it is
    /// not there yet), print the segment and exit: later runs on TABLE
    /// attach it at once.
    #[arg(
        long,
        value_name = "TABLE",
        conflicts_with_all = ["fast", "shm_remove"],
        display_order = 95
    )]
    shm_load: Option<String>,
    /// Remove TABLE's shared memory segment (as `ipcrm -M KEY`) and exit: its
    /// memory is freed once no process has it attached.
    #[arg(
        long,
        value_name = "TABLE",
        conflicts_with = "fast",
        display_order = 96
    )]
    shm_remove: Option<String>,
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
  {\"type\":\"total\",\"cubes\":..,\"wall\":..,\"wall_per_cube\":..,\"nodes_per_cube\":..,\"threads\":..,\"lanes\":..,\"concurrency\":..}

With --fast:
  table    <path>
  threads  <t> (all on this one cube)
  scramble <moves>
  solution <moves>
  length   <n> moves (optimal, verified)
  time     <ms> ms   (<n> M nodes, <rate> M nodes/s)
(with --json one object: table, threads, lanes, scramble, solution, length,
nodes, lookups, seconds, ms, nodes_per_second).";

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
    let result = if let Some(path) = &args.shm_load {
        args.load.shm_load(path)
    } else if let Some(path) = &args.shm_remove {
        args.load.shm_remove(path)
    } else if args.fast {
        run_fast(&args)
    } else {
        run(&args)
    };
    cli::exit_status("solve", result)
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
        let cube = parse_moves(&line)
            .and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
            .ok_or_else(|| BadInput(format!("line {}: cannot parse {line:?}", n + 1)))?;
        cubes.push(cube);
    }
    Ok(cubes)
}

#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn run(args: &Args) -> anyhow::Result<ExitCode> {
    let path = (args.table.as_deref())
        .or(args.table_flag.as_deref())
        .context("no table")?;
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
        let ok = cubes[i].apply_moves(&solution) == Some(CubieCube::SOLVED);
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

/// `--fast`'s warm-up cube (fast.sh's).
const WARMUP: &str = "R U2 F' L D2 B R' U F2 D' L2 B' U R2 F D L' B2 U' R F' D2 L B U2";

/// Where `--fast` looks for tables (for each K, from 11 down).
const TABLE_DIRS: [&str; 4] = ["/mnt/huge", "/data2/tables", "/data/tables", "tables"];

/// The largest table: hK of the largest K in `TABLE_DIRS`.
fn largest_table() -> Option<String> {
    (0..=11).rev().find_map(|k| {
        TABLE_DIRS
            .iter()
            .map(|dir| format!("{dir}/h{k}"))
            .find(|path| Path::new(path).is_file())
    })
}

/// A random scramble of `length` moves: no face twice in a row, and no X Y X
/// with Y opposite X (e.g. U D U, which is U2 D).
fn random_scramble(rng: &mut Rng, length: usize) -> String {
    // (Opposite faces have the same index mod 3: U D, R L, F B.)
    const FACES: [char; 6] = ['U', 'R', 'F', 'D', 'L', 'B'];
    const SUFFIXES: [&str; 3] = ["", "2", "'"];
    let below = |rng: &mut Rng, n: u64| usize::try_from(rng.below(n)).expect("below 6");
    let (mut previous, mut before) = (None, None);
    let mut moves = Vec::with_capacity(length);
    for _ in 0..length {
        let face = loop {
            let face = below(rng, 6);
            let opposite_return =
                previous.is_some_and(|p: usize| p % 3 == face % 3) && before == Some(face);
            if previous != Some(face) && !opposite_return {
                break face;
            }
        };
        moves.push(format!("{}{}", FACES[face], SUFFIXES[below(rng, 3)]));
        (before, previous) = (previous, Some(face));
    }
    moves.join(" ")
}

/// A seed from the clock and the process id. (The monotonic clock: reading
/// `SystemTime` would change the code of the worker threads' `Instant::now`
/// calls, which then pass the clock id instead of having it folded in.)
fn time_seed() -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    std::time::Instant::now().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

/// `--fast`'s JSON object.
#[derive(serde::Serialize)]
struct FastReport<'a> {
    table: &'a str,
    threads: usize,
    lanes: usize,
    scramble: &'a str,
    solution: String,
    length: usize,
    nodes: u64,
    lookups: u64,
    seconds: f64,
    ms: f64,
    nodes_per_second: f64,
}

/// `solve --fast`: one scramble as fast as this machine can (fast.sh, as a
/// flag): the largest table, all threads on the one cube (`-c 1`), a
/// warm-up cube first in the same process (threads, page tables and TLBs
/// are hot when the scramble starts). Prints fast.sh's lines; the time is
/// the scramble's own solve time.
#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn run_fast(args: &Args) -> anyhow::Result<ExitCode> {
    // (A first argument that is a move, not a file, starts the scramble.)
    let mut words = args.scramble.clone();
    let mut table_arg = args.table.clone();
    if let Some(first) =
        table_arg.take_if(|first| !Path::new(first).is_file() && parse_moves(first).is_some())
    {
        words.insert(0, first);
    }
    let path = (table_arg.or_else(|| args.table_flag.clone()))
        .or_else(largest_table)
        .context("no table found; generate one, e.g.: target/release/gen 8 auto tables/h8")?;
    let scramble = if words.is_empty() {
        random_scramble(&mut Rng(time_seed()), 25)
    } else {
        words.join(" ")
    };
    let parse = |text: &str| {
        parse_moves(text)
            .and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
            .ok_or_else(|| BadInput(format!("cannot parse {text:?}")))
    };
    let mut cubes = Vec::new();
    let warmup = args.warmup.as_deref().unwrap_or(WARMUP);
    if !warmup.trim().is_empty() {
        cubes.push(parse(warmup)?);
    }
    cubes.push(parse(&scramble)?);
    let threads = args.threads;
    if !args.json {
        println!("table    {path}");
        println!("threads  {threads} (all on this one cube)");
        println!("scramble {scramble}");
    }
    let corners = Cocsep::generate();
    let table = args.load.open(&path)?;
    let lanes = cli::lanes(args.lanes, &path, table.meta.k, threads);
    let solver = Solver::new(&corners, &table);
    let get = |i: usize| cubes.get(i).copied();
    let last = cubes.len() - 1;
    let result = Mutex::new(None);
    let done = |i: usize, solution: Vec<u8>, stats: Stats, seconds: f64| {
        if i == last {
            *result.lock().unwrap() = Some((solution, stats, seconds));
        }
    };
    solve_all_pinned(&solver, &get, threads, lanes, 1, &done);
    let (solution, stats, seconds) = (result.into_inner().unwrap()).context("not solved")?;
    anyhow::ensure!(
        cubes[last].apply_moves(&solution) == Some(CubieCube::SOLVED),
        "BAD: {} does not solve the scramble",
        format_moves(&solution)
    );
    // (The seconds of solve's line, as fast.sh read them.)
    let seconds: f64 = format!("{seconds:.6}").parse()?;
    let rate = if seconds > 0.0 {
        stats.nodes as f64 / seconds
    } else {
        0.0
    };
    if args.json {
        print_json(&FastReport {
            table: &path,
            threads,
            lanes,
            scramble: &scramble,
            solution: format_moves(&solution),
            length: solution.len(),
            nodes: stats.nodes,
            lookups: stats.lookups,
            seconds,
            ms: seconds * 1000.0,
            nodes_per_second: rate,
        })?;
    } else {
        println!("solution {}", format_moves(&solution));
        println!("length   {} moves (optimal, verified)", solution.len());
        println!(
            "time     {:.3} ms   ({:.1} M nodes, {:.0} M nodes/s)",
            seconds * 1000.0,
            stats.nodes as f64 / 1e6,
            rate / 1e6
        );
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::{random_scramble, Rng};
    use cubeopt::cubie::parse_moves;

    /// 25 moves, no face twice in a row, no X Y X on opposite faces.
    #[test]
    fn random_scrambles() {
        let mut rng = Rng(7);
        for _ in 0..1000 {
            let scramble = random_scramble(&mut rng, 25);
            let moves = parse_moves(&scramble).unwrap();
            assert_eq!(moves.len(), 25, "{scramble}");
            let faces: Vec<u8> = moves.iter().map(|m| m / 3).collect();
            for w in faces.windows(2) {
                assert_ne!(w[0], w[1], "{scramble}");
            }
            for w in faces.windows(3) {
                let opposite = w[0] / 2 == w[1] / 2;
                assert!(!(opposite && w[0] == w[2]), "{scramble}");
            }
        }
    }
}
