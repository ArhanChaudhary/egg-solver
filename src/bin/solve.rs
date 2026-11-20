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
//! `--metric qtm` solves in the quarter-turn metric (a half turn is two
//! moves; `metric`): lengths are QTM lengths, and each solution is followed
//! by its length marked as such, e.g. `(22q)`.
//!
//! `solve --fast --table TABLE [SCRAMBLE]...` uses all selected threads on
//! one cube. Without arguments it reads one scramble from stdin. Warm-up
//! solves are optional; see `run_fast`.
//!
//! `--max-length N` lists every distinct solution of each cube of at most
//! N moves instead of one optimal solution (`egg_solver::solutions`;
//! `list_batch`, `list_fast`), at most `--max-solutions` of them.
//!
//! `solve --shm-load TABLE` loads a table into a shared memory segment that
//! later runs attach at once (`--shm`; `docs/reference/USER_GUIDE.md` "Shared memory"), `solve
//! --shm-remove TABLE` removes it.

// No intentional panics in shipped code (user request; asserts, which no
// lint covers, are checked by tests/no_runtime_panics.rs).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

use anyhow::Context as _;
use clap::{parser::ValueSource, CommandFactory as _, FromArgMatches as _, Parser};
use egg_solver::cli::{self, BadInput, LoadArgs, MetricArgs, Verbosity};
use egg_solver::cubie::{format_moves, parse_moves, CubieCube};
use egg_solver::lanes::share::Pool;
use egg_solver::metric::Metric;
use egg_solver::solutions::{Options, Solutions, DEFAULT_MAX_SOLUTIONS};
use egg_solver::solver::{Solver, Stats};
use egg_solver::tables::Cocsep;
use std::io::{BufRead, IsTerminal as _, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
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
    done: &(dyn Fn(usize, Option<Vec<u8>>, Stats, f64) + Sync),
) -> anyhow::Result<()> {
    let pool = Pool::new(cubes, max_active, threads);
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let mut spawned = Vec::with_capacity(threads.max(1));
        let mut result = Ok(());
        for _ in 0..threads.max(1) {
            let thread = std::thread::Builder::new()
                .spawn_scoped(scope, || pool.run_pinned(solver, lanes, &stop, done));
            match thread {
                Ok(thread) => spawned.push(thread),
                Err(error) => {
                    stop.store(true, Ordering::Relaxed);
                    result = Err(anyhow::Error::from(error).context("cannot start a thread"));
                    break;
                }
            }
        }
        let mut all_ok = true;
        for thread in spawned {
            if thread.join().is_err() {
                stop.store(true, Ordering::Relaxed);
                all_ok = false;
            }
        }
        result?;
        anyhow::ensure!(all_ok, "a thread panicked");
        Ok(())
    })
}

/// Solve Rubik's cube scrambles from stdin optimally (fewest moves: any
/// face turn is one move, or with --metric qtm a quarter turn is one and a
/// half turn two).
#[derive(Parser, Debug)]
#[command(name = "solve", version, after_long_help = OUTPUT_HELP)]
struct Args {
    /// The table file (made by `gen`, e.g. `gen 8 auto h8`). With --fast,
    /// this position starts the scramble instead; supply --table explicitly.
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
        env = egg_solver::env::name("EGG_SOLVER_TABLE")
    )]
    table_flag: Option<String>,
    /// Threads.
    #[arg(
        short,
        long,
        env = egg_solver::env::name("EGG_SOLVER_THREADS"),
        default_value_t = egg_solver::thread_count(),
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
    /// Solve one scramble from arguments or stdin with all selected threads.
    /// Requires an explicit --table. Time excludes table loading and any
    /// optional --warmup solve.
    #[arg(long, requires = "table_flag", conflicts_with_all = ["concurrency", "count"])]
    fast: bool,
    /// With --fast: the scramble (moves as arguments, or one quoted
    /// string). If omitted, read exactly one nonblank line from stdin.
    #[arg(value_name = "SCRAMBLE", requires = "fast")]
    scramble: Vec<String>,
    /// With --fast: the cube solved first, to warm up threads, caches and
    /// TLBs. No warm-up solve by default.
    #[arg(long, value_name = "SCRAMBLE", requires = "fast")]
    warmup: Option<String>,
    /// List every distinct solution of at most N moves of each cube, not
    /// just one optimal solution. At most 20 (with --metric qtm: 26).
    #[arg(long, value_name = "N", display_order = 72)]
    max_length: Option<u8>,
    /// With --max-length: list at most M solutions per cube. If a cube has
    /// more, its search stops and its list is marked truncated.
    #[arg(
        long,
        value_name = "M",
        requires = "max_length",
        default_value_t = DEFAULT_MAX_SOLUTIONS,
        value_parser = cli::positive(),
        display_order = 73
    )]
    max_solutions: usize,
    #[command(flatten)]
    metric: MetricArgs,
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
and makes the exit status 1. With --metric qtm, <length> is the QTM length,
each solution is followed by it as (<length>q), and the summary ends with
metric qtm. With --json, the same as JSON lines (\"length\" in the metric):
  {\"type\":\"solved\",\"index\":..,\"moves\":..,\"length\":..,\"metric\":\"htm\",\"nodes\":..,\"lookups\":..,\"seconds\":..,\"ok\":true}
  {\"type\":\"total\",\"cubes\":..,\"wall\":..,\"wall_per_cube\":..,\"nodes_per_cube\":..,\"threads\":..,\"lanes\":..,\"concurrency\":..,\"metric\":\"htm\"}

With --fast:
  table    <path>
  threads  <t> (all on this one cube)
  scramble <moves>
  solution <moves>
  length   <n> moves (optimal, verified)
  time     <ms> ms   (<n> M nodes, <rate> M nodes/s)
(with --metric qtm: length <n>q; with --json one object: table, threads,
lanes, scramble, solution, length, metric, nodes, lookups, seconds, ms,
nodes_per_second).

With --max-length N, every distinct solution of each cube of at most N
moves (commuting moves in one order, U D not D U; no sequence that only adds
a loop to a shorter solution), by length:
  solutions <index> <count> <optimal> <max> <nodes> <seconds>
  solution <index> <length> <solution>
  total <cubes> cubes wall <s> ... (as above)
<count> solutions follow; <max> is the length limit N and <optimal> the
optimal length (- if the cube has no solution of <max> moves).
A cube with more than --max-solutions M solutions gets, after its first line,
  truncated <index> <M> <complete>
and M solutions: all of at most <complete> moves, some of the longer ones.
With --json: {\"type\":\"solutions\",\"index\":..,\"count\":..,\"optimal\":..,
\"max_length\":..,\"truncated\":false,\"complete_length\":..,\"metric\":\"htm\",
\"by_length\":[{\"length\":..,\"count\":..}],\"solutions\":[{\"moves\":..,\"length\":..}],
\"nodes\":..,\"lookups\":..,\"seconds\":..,\"ok\":true}. With --fast: optimal, listed
and (if so) truncated lines, then the solutions under a line per length.";

/// A line of `--json` output.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Line {
    /// A solved cube (`index`: its line among the scrambles, from 0).
    Solved {
        index: usize,
        moves: String,
        /// In the metric of the search.
        length: usize,
        metric: Metric,
        nodes: u64,
        lookups: u64,
        seconds: f64,
        /// Whether the solution solves the cube (always, unless a bug).
        ok: bool,
    },
    /// Every distinct solution of a cube up to a length (`--max-length`).
    Solutions {
        index: usize,
        #[serde(flatten)]
        listing: Listing,
        nodes: u64,
        lookups: u64,
        seconds: f64,
        /// Whether every solution solves the cube (always, unless a bug).
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
        metric: Metric,
    },
}

/// Print `value` as one line of JSON (stdout flushes at the newline).
fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn main() -> ExitCode {
    let matches = Args::command().get_matches();
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    if let Some(shell) = args.completions {
        cli::print_completions::<Args>(shell);
        return ExitCode::SUCCESS;
    }
    if args.fast && matches.value_source("table_flag") != Some(ValueSource::CommandLine) {
        return cli::exit_status(
            "solve",
            Err(BadInput("--fast requires an explicit --table PATH".into()).into()),
        );
    }
    args.verbosity.init_logging();
    let result = if let Some(path) = &args.shm_load {
        // SAFETY: Mapped/shared CLI inputs are required to remain unchanged while loaded (README contract).
        unsafe { args.load.shm_load(path) }
    } else if let Some(path) = &args.shm_remove {
        // SAFETY: the table segment follows the documented storage contract.
        unsafe { args.load.shm_remove(path) }
    } else if let Some(options) = args.listing() {
        if args.fast {
            list_fast(&args, &options)
        } else {
            list_batch(&args, &options)
        }
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
    if std::io::stdin().is_terminal() {
        return Err(BadInput("provide scrambles on stdin, one per line".into()).into());
    }
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
    if cubes.is_empty() {
        return Err(BadInput("no scrambles on stdin".into()).into());
    }
    let metric = args.metric.metric;
    let corners = Cocsep::generate_metric(metric)?;
    // SAFETY: Mapped/shared CLI inputs are required to remain unchanged while loaded (README contract).
    let table = unsafe { args.load.open(path) }?;
    let threads = args.threads;
    let lanes = cli::lanes(args.lanes, path, table.meta().k, threads);
    let concurrency = args.concurrency;
    log::info!(
        "{} cubes, at most {concurrency} at once (0: all), {metric}",
        cubes.len()
    );
    let solver = solver_of(&corners, &table, metric, path)?;
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
    let done = |i: usize, solution: Option<Vec<u8>>, stats: Stats, seconds: f64| {
        // (No solution, or one that does not solve the cube: BAD.)
        let solves_cube = |moves: &[u8]| {
            cubes.get(i).and_then(|cube| cube.apply_moves(moves)) == Some(CubieCube::SOLVED)
        };
        let ok = solution.as_deref().is_some_and(solves_cube);
        let solution = solution.unwrap_or_default();
        nodes.fetch_add(stats.nodes, Ordering::Relaxed);
        n_done.fetch_add(1, Ordering::Relaxed);
        let mut out = stdout.lock().unwrap_or_else(PoisonError::into_inner);
        if !ok {
            bad.store(true, Ordering::Relaxed);
        }
        if json {
            let line = Line::Solved {
                index: i,
                moves: format_moves(&solution),
                length: metric.length(&solution),
                metric,
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
                "solved {i} {} {} {seconds:.6} {}{}",
                metric.length(&solution),
                stats.nodes,
                format_moves(&solution),
                length_mark(metric, &solution)
            );
        }
        let _ = out.flush();
    };
    let start = Instant::now();
    solve_all_pinned(&solver, &get, threads, lanes, max_active, &done)?;
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
            metric,
        })?;
    } else {
        let metric_note = if metric.is_qtm() { " metric qtm" } else { "" };
        println!(
            "total {n} cubes wall {wall:.6} wall/cube {wall_per_cube:.6} nodes/cube {nodes_per_cube:.0} threads {threads} lanes {lanes} concurrency {concurrency}{metric_note}"
        );
    }
    Ok(if bad.load(Ordering::Relaxed) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// Read one explicit scramble, rejecting empty or multi-cube input before
/// loading a table. An explicitly supplied empty argument represents the
/// solved cube; a blank stdin line is ignored, as in batch mode.
fn fast_scramble(args: &Args) -> anyhow::Result<String> {
    if args.table.is_some() || !args.scramble.is_empty() {
        return Ok(args
            .table
            .iter()
            .chain(&args.scramble)
            .cloned()
            .collect::<Vec<_>>()
            .join(" "));
    }
    if std::io::stdin().is_terminal() {
        return Err(BadInput("provide a scramble as arguments or on stdin".into()).into());
    }
    let mut scramble = None;
    for line in std::io::stdin().lock().lines() {
        let line = line.context("cannot read stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        if scramble.is_some() {
            return Err(BadInput(
                "--fast accepts exactly one scramble; use batch mode for multiple lines".into(),
            )
            .into());
        }
        scramble = Some(line);
    }
    scramble.ok_or_else(|| {
        BadInput("no scramble provided; pass one as arguments or on stdin".into()).into()
    })
}

/// What follows a solution in the text output: nothing in HTM (the output
/// before QTM), its length marked as QTM in QTM (` (22q)`).
fn length_mark(metric: Metric, solution: &[u8]) -> String {
    if metric.is_qtm() {
        format!(" ({})", cli::length_label(metric, solution))
    } else {
        String::new()
    }
}

/// The solver of `metric` on `table` (at `path`), with `corners` (of that
/// metric): an error for an HTM search on a QTM table.
fn solver_of<'a>(
    corners: &'a Cocsep,
    table: &'a egg_solver::tables::H48Table,
    metric: Metric,
    path: &str,
) -> anyhow::Result<Solver<'a>> {
    Solver::with_metric(corners, table, metric).map_err(|error| {
        BadInput(format!(
            "{path}: {error}; use --metric {}",
            table.meta().metric
        ))
        .into()
    })
}

/// `--fast`'s JSON object.
#[derive(serde::Serialize)]
struct FastReport<'a> {
    table: &'a str,
    threads: usize,
    lanes: usize,
    scramble: &'a str,
    solution: String,
    /// In the metric of the search.
    length: usize,
    metric: Metric,
    nodes: u64,
    lookups: u64,
    seconds: f64,
    ms: f64,
    nodes_per_second: f64,
}

/// Solve the supplied scramble with all selected threads. Table loading
/// and an explicitly requested warm-up are excluded from the reported time.
#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn run_fast(args: &Args) -> anyhow::Result<ExitCode> {
    let path = args
        .table_flag
        .as_deref()
        .context("--fast requires --table PATH")?;
    let scramble = fast_scramble(args)?;
    let parse = |text: &str| {
        parse_moves(text)
            .and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
            .ok_or_else(|| BadInput(format!("cannot parse {text:?}")))
    };
    let mut cubes = Vec::new();
    let warmup = args.warmup.as_deref().unwrap_or("");
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
    let metric = args.metric.metric;
    let corners = Cocsep::generate_metric(metric)?;
    // SAFETY: Mapped/shared CLI inputs are required to remain unchanged while loaded (README contract).
    let table = unsafe { args.load.open(path) }?;
    let lanes = cli::lanes(args.lanes, path, table.meta().k, threads);
    let solver = solver_of(&corners, &table, metric, path)?;
    let get = |i: usize| cubes.get(i).copied();
    // (The scramble, after the warm-up cube if any.)
    let (last, &cube) = cubes
        .iter()
        .enumerate()
        .next_back()
        .context("no scramble")?;
    let result = Mutex::new(None);
    let done = |i: usize, solution: Option<Vec<u8>>, stats: Stats, seconds: f64| {
        if i == last {
            *result.lock().unwrap_or_else(PoisonError::into_inner) =
                Some((solution, stats, seconds));
        }
    };
    solve_all_pinned(&solver, &get, threads, lanes, 1, &done)?;
    let result = result.into_inner().unwrap_or_else(PoisonError::into_inner);
    let (solution, stats, seconds) = result.context("not solved")?;
    let solution = solution.with_context(|| {
        format!(
            "no solution within {} moves ({metric}): is the table damaged?",
            metric.max_target()
        )
    })?;
    anyhow::ensure!(
        cube.apply_moves(&solution) == Some(CubieCube::SOLVED),
        "BAD: {} does not solve the scramble",
        format_moves(&solution)
    );
    // Match the precision of the plain-text solve output.
    let seconds: f64 = format!("{seconds:.6}").parse()?;
    let rate = if seconds > 0.0 {
        stats.nodes as f64 / seconds
    } else {
        0.0
    };
    if args.json {
        print_json(&FastReport {
            table: path,
            threads,
            lanes,
            scramble: &scramble,
            solution: format_moves(&solution),
            length: metric.length(&solution),
            metric,
            nodes: stats.nodes,
            lookups: stats.lookups,
            seconds,
            ms: seconds * 1000.0,
            nodes_per_second: rate,
        })?;
    } else {
        println!("solution {}", format_moves(&solution));
        if metric.is_qtm() {
            println!(
                "length   {} (quarter-turn metric; optimal, verified)",
                cli::length_label(metric, &solution)
            );
        } else {
            println!("length   {} moves (optimal, verified)", solution.len());
        }
        println!(
            "time     {:.3} ms   ({:.1} M nodes, {:.0} M nodes/s)",
            seconds * 1000.0,
            stats.nodes as f64 / 1e6,
            rate / 1e6
        );
    }
    Ok(ExitCode::SUCCESS)
}

impl Args {
    /// What to list, if all solutions up to a length are asked for
    /// (`--max-length`): `None` for one optimal solution per cube.
    fn listing(&self) -> Option<Options> {
        let max_length = self.max_length?;
        Some(Options::max_length(max_length).with_max_solutions(self.max_solutions))
    }
}

/// A solution of a list, in JSON.
#[derive(serde::Serialize)]
struct Listed {
    moves: String,
    /// In the metric of the search.
    length: usize,
}

/// The number of solutions of a length, in JSON.
#[derive(serde::Serialize)]
struct LengthCount {
    length: usize,
    count: usize,
}

/// A cube's solutions up to a length, in JSON (`Line::Solutions`,
/// `FastListing`); lengths in the metric of the search.
#[derive(serde::Serialize)]
struct Listing {
    count: usize,
    /// The optimal length (null: no solution of at most `max_length`).
    optimal: Option<u8>,
    /// The length limit, N.
    max_length: u8,
    /// More solutions than `--max-solutions`: `count` of them are listed,
    /// all of at most `complete_length` moves and some longer ones.
    truncated: bool,
    complete_length: u8,
    metric: Metric,
    by_length: Vec<LengthCount>,
    solutions: Vec<Listed>,
}

impl Listing {
    fn new(list: &Solutions, metric: Metric) -> Listing {
        let listed = |moves: &Vec<u8>| Listed {
            moves: format_moves(moves),
            length: metric.length(moves),
        };
        Listing {
            count: list.solutions.len(),
            optimal: list.optimal,
            max_length: list.max_length,
            truncated: list.truncated,
            complete_length: list.complete_length,
            metric,
            by_length: (list.counts(metric).into_iter())
                .map(|(length, count)| LengthCount { length, count })
                .collect(),
            solutions: list.solutions.iter().map(listed).collect(),
        }
    }
}

/// The limits of a listing checked against the metric: `--max-length`
/// above God's number is refused (no search goes further).
fn check_listing(options: &Options, metric: Metric) -> anyhow::Result<()> {
    let longest = metric.max_target();
    if options.max_length.is_some_and(|n| n > longest) {
        return Err(BadInput(format!(
            "--max-length is at most {longest} ({metric}: no position needs more moves, and \
             searches go no further)"
        ))
        .into());
    }
    Ok(())
}

/// Whether a list is right as far as can be checked here: every solution
/// solves `cube`, and a cube searched up to God's number has a solution.
fn list_ok(cube: Option<&CubieCube>, list: &Solutions, metric: Metric) -> bool {
    let solves =
        |moves: &Vec<u8>| cube.and_then(|cube| cube.apply_moves(moves)) == Some(CubieCube::SOLVED);
    let none_possible = list.max_length < metric.max_target();
    list.solutions.iter().all(solves) && (list.optimal.is_some() || none_possible)
}

/// A length as `--fast`'s text names it: `18 moves`, in QTM `22q`.
fn length_text(metric: Metric, length: usize) -> String {
    if metric.is_qtm() {
        format!("{length}q")
    } else {
        format!("{length} moves")
    }
}

/// `solve` with `--max-length`: every distinct solution of each scramble
/// on stdin up to that length (`run` for lists).
#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn list_batch(args: &Args, options: &Options) -> anyhow::Result<ExitCode> {
    let path = (args.table.as_deref())
        .or(args.table_flag.as_deref())
        .context("no table")?;
    let metric = args.metric.metric;
    check_listing(options, metric)?;
    let cubes = read_scrambles(args.count.unwrap_or(usize::MAX))?;
    if cubes.is_empty() {
        return Err(BadInput("no scrambles on stdin".into()).into());
    }
    let corners = Cocsep::generate_metric(metric)?;
    // SAFETY: Mapped/shared CLI inputs are required to remain unchanged while loaded (README contract).
    let table = unsafe { args.load.open(path) }?;
    let threads = args.threads;
    let lanes = cli::lanes(args.lanes, path, table.meta().k, threads);
    let concurrency = args.concurrency;
    log::info!(
        "{} cubes, at most {concurrency} at once (0: all), {metric}, {options:?}",
        cubes.len()
    );
    let solver = solver_of(&corners, &table, metric, path)?;
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
    let done = |i: usize, list: Solutions, stats: Stats, seconds: f64| {
        let ok = list_ok(cubes.get(i), &list, metric);
        nodes.fetch_add(stats.nodes, Ordering::Relaxed);
        n_done.fetch_add(1, Ordering::Relaxed);
        if !ok {
            bad.store(true, Ordering::Relaxed);
        }
        // (A cube's lines together.)
        let mut out = stdout.lock().unwrap_or_else(PoisonError::into_inner);
        if json {
            let line = Line::Solutions {
                index: i,
                listing: Listing::new(&list, metric),
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
            let _ = write_list(&mut *out, i, &list, stats, seconds, metric, options);
        }
        let _ = out.flush();
    };
    let start = Instant::now();
    egg_solver::lanes::share::all_solutions(
        &solver, &get, threads, lanes, max_active, options, &done,
    )
    .context("a thread panicked")?;
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
            metric,
        })?;
    } else {
        let metric_note = if metric.is_qtm() { " metric qtm" } else { "" };
        println!(
            "total {n} cubes wall {wall:.6} wall/cube {wall_per_cube:.6} nodes/cube {nodes_per_cube:.0} threads {threads} lanes {lanes} concurrency {concurrency}{metric_note}"
        );
    }
    Ok(if bad.load(Ordering::Relaxed) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// The text lines of cube `i`'s list (`OUTPUT_HELP`): `solutions`,
/// `truncated` if it is, and a `solution` line each.
fn write_list(
    out: &mut impl Write,
    i: usize,
    list: &Solutions,
    stats: Stats,
    seconds: f64,
    metric: Metric,
    options: &Options,
) -> std::io::Result<()> {
    let optimal = list
        .optimal
        .map_or_else(|| "-".to_string(), |optimal| optimal.to_string());
    writeln!(
        out,
        "solutions {i} {} {optimal} {} {} {seconds:.6}",
        list.solutions.len(),
        list.max_length,
        stats.nodes
    )?;
    if list.truncated {
        writeln!(
            out,
            "truncated {i} {} {}",
            options.max_solutions, list.complete_length
        )?;
    }
    for solution in &list.solutions {
        writeln!(
            out,
            "solution {i} {} {}{}",
            metric.length(solution),
            format_moves(solution),
            length_mark(metric, solution)
        )?;
    }
    Ok(())
}

/// `--fast`'s JSON object with a listing.
#[derive(serde::Serialize)]
struct FastListing<'a> {
    table: &'a str,
    threads: usize,
    lanes: usize,
    scramble: &'a str,
    #[serde(flatten)]
    listing: Listing,
    nodes: u64,
    lookups: u64,
    seconds: f64,
    ms: f64,
    nodes_per_second: f64,
}

/// `solve --fast` with `--max-length`: every distinct solution of the
/// supplied scramble up to that length, with all selected threads
/// (`run_fast` for a list).
#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn list_fast(args: &Args, options: &Options) -> anyhow::Result<ExitCode> {
    let path = args
        .table_flag
        .as_deref()
        .context("--fast requires --table PATH")?;
    let metric = args.metric.metric;
    check_listing(options, metric)?;
    let scramble = fast_scramble(args)?;
    let parse = |text: &str| {
        parse_moves(text)
            .and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
            .ok_or_else(|| BadInput(format!("cannot parse {text:?}")))
    };
    let mut cubes = Vec::new();
    let warmup = args.warmup.as_deref().unwrap_or("");
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
    let corners = Cocsep::generate_metric(metric)?;
    // SAFETY: Mapped/shared CLI inputs are required to remain unchanged while loaded (README contract).
    let table = unsafe { args.load.open(path) }?;
    let lanes = cli::lanes(args.lanes, path, table.meta().k, threads);
    let solver = solver_of(&corners, &table, metric, path)?;
    let get = |i: usize| cubes.get(i).copied();
    // (The scramble, after the warm-up cube if any.)
    let (index, cube) = cubes
        .iter()
        .enumerate()
        .next_back()
        .context("no scramble")?;
    let result = Mutex::new(None);
    let done = |i: usize, list: Solutions, stats: Stats, seconds: f64| {
        if i == index {
            *result.lock().unwrap_or_else(PoisonError::into_inner) = Some((list, stats, seconds));
        }
    };
    egg_solver::lanes::share::all_solutions(&solver, &get, threads, lanes, 1, options, &done)
        .context("a thread panicked")?;
    let result = result.into_inner().unwrap_or_else(PoisonError::into_inner);
    let (list, stats, seconds) = result.context("not solved")?;
    anyhow::ensure!(
        list.optimal.is_some() || list.max_length < metric.max_target(),
        "no solution within {} moves ({metric}): is the table damaged?",
        metric.max_target()
    );
    anyhow::ensure!(
        list_ok(Some(cube), &list, metric),
        "BAD: a listed solution does not solve the scramble"
    );
    // Match the precision of the plain-text solve output.
    let seconds: f64 = format!("{seconds:.6}").parse()?;
    let rate = if seconds > 0.0 {
        stats.nodes as f64 / seconds
    } else {
        0.0
    };
    if args.json {
        print_json(&FastListing {
            table: path,
            threads,
            lanes,
            scramble: &scramble,
            listing: Listing::new(&list, metric),
            nodes: stats.nodes,
            lookups: stats.lookups,
            seconds,
            ms: seconds * 1000.0,
            nodes_per_second: rate,
        })?;
        return Ok(ExitCode::SUCCESS);
    }
    print_fast_list(&list, metric, options);
    println!(
        "time     {:.3} ms   ({:.1} M nodes, {:.0} M nodes/s)",
        seconds * 1000.0,
        stats.nodes as f64 / 1e6,
        rate / 1e6
    );
    Ok(ExitCode::SUCCESS)
}

/// `--fast`'s text lines of a list (`OUTPUT_HELP`): the optimal length,
/// how many solutions are listed up to which length (and whether there are
/// more), then the solutions under a line per length.
fn print_fast_list(list: &Solutions, metric: Metric, options: &Options) {
    let limit = length_text(metric, usize::from(list.max_length));
    match list.optimal {
        Some(optimal) => println!("optimal  {}", length_text(metric, usize::from(optimal))),
        None => println!("optimal  more than {limit}"),
    }
    let count = list.solutions.len();
    println!(
        "listed   {count} solution{} of at most {limit} (verified)",
        if count == 1 { "" } else { "s" }
    );
    if list.truncated {
        println!(
            "truncated: there are more than {} (--max-solutions); complete up to {}",
            options.max_solutions,
            length_text(metric, usize::from(list.complete_length))
        );
    }
    let mut solutions = list.solutions.iter();
    for (length, count) in list.counts(metric) {
        println!(
            "{}: {count} solution{}",
            length_text(metric, length),
            if count == 1 { "" } else { "s" }
        );
        for solution in solutions.by_ref().take(count) {
            println!("  {}", format_moves(solution));
        }
    }
}
