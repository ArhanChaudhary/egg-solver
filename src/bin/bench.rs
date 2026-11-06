//! Optimal-solve throughput benchmark.
//!
//! Usage (`bench --help` and `bench <command> --help` have all options):
//!   bench stream <table> [--warmup S] [--window S] [-t threads] [-l lanes] [--seed N]
//!       Endless deterministic stream of uniformly random states; reports
//!       solves completed per second during the window (steady state).
//!   bench batch <table> <scramble file | random:N:SEED> [-n count] [-t threads] [-l lanes]
//!       Solves a fixed batch and verifies every solution.
//!   bench tune <table> [--window S]
//!       Short stream runs at 1 thread and at all threads over lane counts;
//!       prints the best and saves it next to the table (`<table>.tune`).
//! lanes = 0 uses the recursive single-cube solver; the default is
//! `<table>.tune`'s if `tune` made one, else `tables::file::untuned_lanes`.
//! `--share[=C]` uses the work-sharing mode (`lanes::share`) with at most C
//! cubes in progress at once (default: threads * lanes; 1 solves one cube
//! at a time with all threads). `--json` prints the result as JSON.
//!
//! The form before subcommands, which scripts use, still works (`legacy`):
//!   bench <table> stream [warmup] [window] [threads] [lanes] [seed] [share[:C]]
//!   bench <table> <scramble file | random:N:SEED> [count] [threads] [lanes] [share[:C]]
//!   bench <table> tune [seconds]

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
use clap::{Args, Parser, Subcommand};
use egg_solver::cli::{self, BadInput, LoadArgs, Verbosity};
use egg_solver::cubie::{parse_moves, CubieCube, Rng};
use egg_solver::lanes::share::Pool;
use egg_solver::solver::{Solver, Stats};
use egg_solver::tables::Cocsep;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
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
///
/// # Errors
/// If a cube got no solution or a wrong one, or a thread could not be
/// started or panicked.
fn run(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    threads: usize,
    lanes: usize,
    duration: Option<f64>,
    share: Option<usize>,
) -> anyhow::Result<(Vec<Record>, f64)> {
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let pool = share.map(|max_active| {
        let max_active = if max_active == 0 {
            threads * lanes
        } else {
            max_active
        };
        egg_solver::lanes::share::Pool::new(cubes, max_active, threads)
    });
    let records = Mutex::new(Vec::new());
    let bad = AtomicU64::new(0);
    let t0 = Instant::now();
    // (Each mode's thread code is a function of its own, not inlined here:
    // `share_thread`, and `stream_thread`, which is the stream's hot loop.)
    let duration = duration
        .map(std::time::Duration::try_from_secs_f64)
        .transpose()
        .context("benchmark duration is out of range")?;
    let threads_ok = std::thread::scope(|s| {
        let mut spawned = Vec::with_capacity(threads);
        let mut result = Ok(());
        for _ in 0..threads {
            let thread = std::thread::Builder::new().spawn_scoped(s, || {
                let record = |i: usize, solution: Option<Vec<u8>>, stats: Stats, seconds: f64| {
                    // (No solution, or one that does not solve the cube: bad.)
                    let solves_cube = |moves: &[u8]| {
                        cubes(i).and_then(|cube| cube.apply_moves(moves)) == Some(CubieCube::SOLVED)
                    };
                    if !solution.as_deref().is_some_and(solves_cube) {
                        bad.fetch_add(1, Ordering::Relaxed);
                    }
                    records
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .push(Record {
                            finished: t0.elapsed().as_secs_f64(),
                            seconds,
                            length: solution.map_or(0, |moves| moves.len()),
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
            match thread {
                Ok(thread) => spawned.push(thread),
                Err(error) => {
                    stop.store(true, Ordering::Relaxed);
                    result = Err(anyhow::Error::from(error).context("cannot start a thread"));
                    break;
                }
            }
        }
        if let Some(duration) = duration.filter(|_| result.is_ok()) {
            std::thread::sleep(duration);
            stop.store(true, Ordering::Relaxed);
        }
        let mut all_ok = true;
        for thread in spawned {
            if thread.join().is_err() {
                stop.store(true, Ordering::Relaxed);
                all_ok = false;
            }
        }
        result?;
        Ok::<_, anyhow::Error>(all_ok)
    })?;
    anyhow::ensure!(threads_ok, "a thread panicked");
    let bad = bad.load(Ordering::Relaxed);
    anyhow::ensure!(bad == 0, "{bad} bad solutions");
    if let Some(pool) = &pool {
        eprintln!("share: {}", pool.counters());
    }
    let records = records.into_inner().unwrap_or_else(PoisonError::into_inner);
    Ok((records, t0.elapsed().as_secs_f64()))
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
    record: impl FnMut(usize, Option<Vec<u8>>, Stats, f64),
) {
    pool.run(solver, lanes, stop, record);
}

// `stream_thread`'s section: it starts at a fixed offset of a 64-byte line
// in every build (see `egg_solver::hot`).
// Match the measured stream function's page offset (0xa90 with pad 16).
// Together with cold completion reporting: +0.83% 1T h8 throughput when
// added; 48T neutral (+0.19% median, five pairs).
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
core::arch::global_asm!(
    ".pushsection .text.hot_stream,\"ax\",@progbits",
    ".p2align 12",
    ".skip 0xa80, 0xcc",
    ".popsection",
);
egg_solver::hot_section!(".text.hot_stream", egg_solver::hot::STREAM_PAD);

/// A thread of a stream-mode run (`run_lanes`).
#[inline(never)]
#[cfg_attr(
    all(target_arch = "x86_64", target_os = "linux"),
    unsafe(link_section = ".text.hot_stream")
)]
fn stream_thread(
    solver: &Solver,
    cubes: &(dyn Fn(usize) -> Option<CubieCube> + Sync),
    next: &AtomicUsize,
    stop: &AtomicBool,
    lanes: usize,
    record: impl FnMut(usize, Option<Vec<u8>>, Stats, f64),
) {
    egg_solver::lanes::run_lanes(solver, cubes, next, stop, lanes, record);
}

/// Stream throughput (solves/s) of `threads` x `lanes` lanes, over a
/// `window` after a warm-up of half as long.
fn stream_throughput(
    solver: &Solver,
    threads: usize,
    lanes: usize,
    window: f64,
) -> anyhow::Result<f64> {
    let cubes = |i: usize| {
        let mut rng = Rng(2026u64.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64));
        rng.next_u64();
        Some(CubieCube::random(&mut rng))
    };
    let warmup = window / 2.0;
    let (records, _) = run(solver, &cubes, threads, lanes, Some(warmup + window), None)?;
    let done = records
        .iter()
        .filter(|r| r.finished >= warmup && r.finished < warmup + window)
        .count();
    #[expect(clippy::cast_precision_loss, reason = "a rate")]
    let rate = done as f64 / window;
    Ok(rate)
}

/// Optimal-solve throughput benchmark: solves per second, nodes and
/// lookups per cube, solution lengths.
#[derive(Parser, Debug)]
#[command(name = "bench", version, after_long_help = LEGACY_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Print the result as JSON (one object per result).
    #[arg(long, global = true, display_order = 100)]
    json: bool,
    #[command(flatten)]
    verbosity: Verbosity,
    /// Print a shell completion script.
    #[arg(long, value_name = "SHELL", exclusive = true, hide = true)]
    completions: Option<clap_complete::Shell>,
}

const LEGACY_HELP: &str = "\
The form before subcommands still works (scripts use it):
  bench <table> stream [warmup] [window] [threads] [lanes] [seed] [share[:C]]
  bench <table> <scramble file | random:N:SEED> [count] [threads] [lanes] [share[:C]]
  bench <table> tune [seconds]";

#[derive(Subcommand, Debug)]
enum Command {
    /// An endless stream of uniformly random states (cube i from its own
    /// seed): solves per second in the window after the warm-up.
    Stream(StreamArgs),
    /// Solve a fixed set of cubes, verify every solution: solves per second
    /// and solve times.
    Batch(BatchArgs),
    /// Measure lane counts at 1 thread and at all threads; save the best
    /// next to the table (`<TABLE>.tune`, the default of `solve` and
    /// `bench`).
    Tune(TuneArgs),
}

/// The table and how to load it.
#[derive(Args, Debug)]
struct TableArgs {
    /// The table file (made by `gen`).
    #[arg(env = egg_solver::env::name("EGG_SOLVER_TABLE"))]
    table: String,
    #[command(flatten)]
    load: LoadArgs,
}

/// Threads, lanes and the mode of a run.
#[derive(Args, Debug)]
struct RunArgs {
    /// Threads.
    #[arg(
        short,
        long,
        env = egg_solver::env::name("EGG_SOLVER_THREADS"),
        default_value_t = egg_solver::thread_count(),
        value_parser = cli::positive(),
    )]
    threads: usize,
    /// Interleaved lanes per thread (0: the recursive single-cube solver)
    /// [default: from `<TABLE>.tune` if `bench tune` made one, else a
    /// default for the table's size].
    #[arg(short, long)]
    lanes: Option<usize>,
    /// The work-sharing mode, with at most C cubes in progress at once
    /// (`--share` alone: threads * lanes; 1: one cube at a time with all
    /// threads).
    #[arg(
        long,
        value_name = "C",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "0"
    )]
    share: Option<usize>,
}

#[derive(Args, Debug)]
struct StreamArgs {
    #[command(flatten)]
    table: TableArgs,
    /// Seconds of warm-up (not counted).
    #[arg(long, value_name = "S", default_value_t = 3.0, value_parser = seconds)]
    warmup: f64,
    /// Seconds counted.
    #[arg(long, value_name = "S", default_value_t = 15.0, value_parser = seconds)]
    window: f64,
    /// Seed of the stream (the same seed gives the same cubes).
    #[arg(long, default_value_t = 2026)]
    seed: u64,
    #[command(flatten)]
    run: RunArgs,
}

#[derive(Args, Debug)]
struct BatchArgs {
    #[command(flatten)]
    table: TableArgs,
    /// The cubes: a file of scrambles (one per line), or `random:N:SEED`
    /// for N uniformly random states.
    #[arg(value_name = "FILE|random:N:SEED")]
    set: String,
    /// Solve only the first COUNT cubes.
    #[arg(short = 'n', long)]
    count: Option<usize>,
    #[command(flatten)]
    run: RunArgs,
}

#[derive(Args, Debug)]
struct TuneArgs {
    #[command(flatten)]
    table: TableArgs,
    /// Seconds counted per measurement (after half as many of warm-up).
    #[arg(long, value_name = "S", default_value_t = 6.0, value_parser = seconds)]
    window: f64,
}

/// A duration in seconds: a finite number, not negative.
fn seconds(text: &str) -> Result<f64, String> {
    match text.parse::<f64>() {
        Ok(s) if std::time::Duration::try_from_secs_f64(s).is_ok() => Ok(s),
        _ => Err("expected seconds (a number, 0 or more)".to_string()),
    }
}

/// The arguments in the subcommand form. The form before subcommands
/// (`bench <table> <mode> [values...] [share[:C]]`, positional values) is
/// rewritten to it: `bench <mode> <table> --option value ... [--share[=C]]`,
/// where a mode that is neither `stream` nor `tune` is the set of `batch`.
/// Values stop at the first option (`-...`), which is passed on as is.
fn legacy(args: Vec<OsString>) -> Vec<OsString> {
    let text = |i: usize| args.get(i).and_then(|a| a.to_str()).unwrap_or("-");
    let first = text(1);
    let is_subcommand = ["stream", "batch", "tune", "help"].contains(&first);
    if args.len() < 3 || is_subcommand || first.starts_with('-') {
        return args;
    }
    let mode = text(2).to_string();
    let mut args = args.into_iter();
    let (program, table) = (args.next(), args.next());
    let mut values: Vec<OsString> = args.skip(1).collect();
    let share = match values.last().and_then(|a| a.to_str()) {
        Some("share") => Some("--share".to_string()),
        Some(last) => last.strip_prefix("share:").map(|c| format!("--share={c}")),
        None => None,
    };
    if share.is_some() {
        values.pop();
    }
    let mut out: Vec<OsString> = program.into_iter().collect();
    let options: &[&str] = match mode.as_str() {
        "stream" => &["--warmup", "--window", "--threads", "--lanes", "--seed"],
        "tune" => &["--window"],
        _ => &["--count", "--threads", "--lanes"],
    };
    match mode.as_str() {
        "stream" | "tune" => out.extend([mode.into(), table.unwrap_or_default()]),
        _ => out.extend(["batch".into(), table.unwrap_or_default(), mode.into()]),
    }
    let mut values = values.into_iter().peekable();
    for option in options {
        if values
            .peek()
            .is_none_or(|v| v.to_string_lossy().starts_with('-'))
        {
            break;
        }
        out.push(option.into());
        out.extend(values.next());
    }
    out.extend(values);
    out.extend(share.map(OsString::from));
    out
}

/// Lengths and nodes and lookups per cube of a run.
#[derive(serde::Serialize)]
struct Summary {
    nodes_per_cube: f64,
    lookups_per_cube: f64,
    /// Cubes per solution length.
    lengths: BTreeMap<usize, usize>,
}

impl Summary {
    #[expect(clippy::cast_precision_loss, reason = "statistics")]
    fn of(records: &[Record]) -> Summary {
        let n = records.len().max(1) as f64;
        let nodes: u64 = records.iter().map(|r| r.stats.nodes).sum();
        let lookups: u64 = records.iter().map(|r| r.stats.lookups).sum();
        let mut lengths = BTreeMap::new();
        for r in records {
            *lengths.entry(r.length).or_insert(0) += 1;
        }
        Summary {
            nodes_per_cube: nodes as f64 / n,
            lookups_per_cube: lookups as f64 / n,
            lengths,
        }
    }

    /// Print nodes and lookups per cube and the distribution of lengths.
    fn print(&self) {
        let histogram: Vec<String> = self
            .lengths
            .iter()
            .map(|(length, count)| format!("{length}:{count}"))
            .collect();
        println!(
            "nodes/cube {:.0} lookups/cube {:.0}",
            self.nodes_per_cube, self.lookups_per_cube
        );
        println!("lengths {}", histogram.join(" "));
    }
}

/// A `--json` result.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Report {
    /// `stream`: the cubes solved in the window.
    Stream {
        threads: usize,
        lanes: usize,
        window: f64,
        /// Cubes in progress at once in the share mode (0: threads *
        /// lanes), or null for the stream mode.
        share: Option<usize>,
        cubes: usize,
        throughput: f64,
        #[serde(flatten)]
        summary: Summary,
    },
    /// `batch`: all cubes of the set.
    Batch {
        cubes: usize,
        threads: usize,
        lanes: usize,
        seconds: f64,
        share: Option<usize>,
        solves_per_second: f64,
        #[serde(flatten)]
        summary: Summary,
        max_seconds: f64,
        p50_seconds: f64,
    },
    /// `tune`: one measurement.
    Tune {
        threads: usize,
        lanes: usize,
        solves_per_second: f64,
    },
    /// `tune`: the best lanes at a thread count.
    Best {
        threads: usize,
        lanes: usize,
        solves_per_second: f64,
    },
    /// `tune`: where the best lanes were saved (or why not).
    Saved {
        path: Option<String>,
        error: Option<String>,
    },
}

/// Print `report` as JSON if `json`, else `text` (the output before
/// `--json`).
fn print(json: bool, report: &Report, text: impl FnOnce()) -> anyhow::Result<()> {
    if json {
        print_json(report)?;
    } else {
        text();
    }
    Ok(())
}

/// `bench tune`: the best lane count at 1 thread and at all threads (each
/// candidate measured for `window` seconds), saved to `<table>.tune`.
fn tune(solver: &Solver, path: &str, window: f64, json: bool) -> anyhow::Result<()> {
    let all = egg_solver::thread_count();
    let candidates = [8, 12, 16, 20, 24, 32, 40, 48];
    let mut best = Vec::new();
    for threads in if all > 1 { vec![1, all] } else { vec![1] } {
        let mut top = (0.0, 0);
        for &lanes in &candidates {
            let rate = stream_throughput(solver, threads, lanes, window)?;
            let report = Report::Tune {
                threads,
                lanes,
                solves_per_second: rate,
            };
            print(json, &report, || {
                println!("threads {threads} lanes {lanes}: {rate:.2} solves/s");
            })?;
            if rate > top.0 {
                top = (rate, lanes);
            }
        }
        let report = Report::Best {
            threads,
            lanes: top.1,
            solves_per_second: top.0,
        };
        print(json, &report, || {
            println!(
                "best at {threads} threads: {} lanes ({:.2} solves/s)",
                top.1, top.0
            );
        })?;
        best.push((threads, top.1));
    }
    let saved = egg_solver::tables::file::save_tuned_lanes(path, &best);
    let report = Report::Saved {
        path: saved.as_ref().ok().cloned(),
        error: saved.as_ref().err().map(ToString::to_string),
    };
    print(json, &report, || match &saved {
        Ok(tune_path) => println!("saved to {tune_path}"),
        Err(e) => println!("not saved: {e}"),
    })
}

/// `bench stream`.
#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn stream(solver: &Solver, args: &StreamArgs, lanes: usize, json: bool) -> anyhow::Result<()> {
    let (warmup, window, seed) = (args.warmup, args.window, args.seed);
    let (threads, share) = (args.run.threads, args.run.share);
    // Cube i of the stream: from its own seed, so that it does not
    // depend on the order in which the threads take the cubes.
    let cubes = move |i: usize| {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64));
        rng.next_u64();
        Some(CubieCube::random(&mut rng))
    };
    let (records, _) = run(solver, &cubes, threads, lanes, Some(warmup + window), share)?;
    let in_window: Vec<Record> = records
        .into_iter()
        .filter(|r| r.finished >= warmup && r.finished < warmup + window)
        .collect();
    let throughput = in_window.len() as f64 / window;
    let summary = Summary::of(&in_window);
    if json {
        return print_json(&Report::Stream {
            threads,
            lanes,
            window,
            share,
            cubes: in_window.len(),
            throughput,
            summary,
        });
    }
    let mode = share.map_or(String::new(), |c| format!(" share {c}"));
    println!("threads {threads} lanes {lanes} window {window}s{mode}");
    summary.print();
    println!("throughput {throughput:.3}");
    Ok(())
}

/// The cubes of `bench batch`'s set: a file of scrambles or
/// `random:N:SEED`.
fn cube_set(set: &str) -> anyhow::Result<Vec<CubieCube>> {
    if let Some(spec) = set.strip_prefix("random:") {
        let numbers: Option<Vec<u64>> = spec.split(':').map(|x| x.parse().ok()).collect();
        let Some(&[count, seed]) = numbers.as_deref() else {
            return Err(BadInput(format!("{set}: expected random:N:SEED")).into());
        };
        let mut rng = Rng(seed);
        return Ok((0..count).map(|_| CubieCube::random(&mut rng)).collect());
    }
    let text = std::fs::read_to_string(set).with_context(|| format!("cannot read {set}"))?;
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(n, line)| {
            parse_moves(line)
                .and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
                .ok_or_else(|| {
                    BadInput(format!("{set}: line {}: cannot parse {line:?}", n + 1)).into()
                })
        })
        .collect()
}

/// `bench batch`.
#[expect(clippy::cast_precision_loss, reason = "statistics")]
fn batch(
    solver: &Solver,
    args: &BatchArgs,
    cubes: &[CubieCube],
    lanes: usize,
    json: bool,
) -> anyhow::Result<()> {
    let (threads, share) = (args.run.threads, args.run.share);
    let get = |i: usize| cubes.get(i).copied();
    let (records, elapsed) = run(solver, &get, threads, lanes, None, share)?;
    anyhow::ensure!(records.len() == cubes.len(), "cubes not reported");
    let n = cubes.len() as f64;
    let mut times: Vec<f64> = records.iter().map(|r| r.seconds).collect();
    times.sort_by(f64::total_cmp);
    let (max, p50) = (times[times.len() - 1], times[times.len() / 2]);
    let summary = Summary::of(&records);
    if json {
        return print_json(&Report::Batch {
            cubes: cubes.len(),
            threads,
            lanes,
            seconds: elapsed,
            share,
            solves_per_second: n / elapsed,
            summary,
            max_seconds: max,
            p50_seconds: p50,
        });
    }
    let mode = share.map_or(String::new(), |c| format!(" share {c}"));
    println!(
        "cubes {} threads {threads} lanes {lanes} time {elapsed:.3}s{mode}",
        cubes.len()
    );
    println!("solves/sec {:.3}", n / elapsed);
    summary.print();
    println!("solve time max {max:.3}s p50 {p50:.4}s");
    Ok(())
}

/// Print `value` as one line of JSON (stdout flushes at the newline).
fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse_from(legacy(std::env::args_os().collect()));
    if let Some(shell) = cli.completions {
        cli::print_completions::<Cli>(shell);
        return ExitCode::SUCCESS;
    }
    let Some(command) = &cli.command else {
        let mut command = <Cli as clap::CommandFactory>::command();
        command
            .error(
                clap::error::ErrorKind::MissingSubcommand,
                "a command is needed: stream, batch or tune",
            )
            .exit();
    };
    cli.verbosity.init_logging();
    cli::exit_status(
        "bench",
        run_command(command, cli.json).map(|()| ExitCode::SUCCESS),
    )
}

fn run_command(command: &Command, json: bool) -> anyhow::Result<()> {
    let table_args = match command {
        Command::Stream(args) => &args.table,
        Command::Batch(args) => &args.table,
        Command::Tune(args) => &args.table,
    };
    // (Before the table, which can take a while to load.)
    let cubes = match command {
        Command::Batch(args) => {
            let mut cubes = cube_set(&args.set)?;
            cubes.truncate(args.count.unwrap_or(usize::MAX));
            anyhow::ensure!(!cubes.is_empty(), "{}: no cubes", args.set);
            cubes
        }
        _ => Vec::new(),
    };
    let path = &table_args.table;
    let corners = Cocsep::generate()?;
    let load_start = Instant::now();
    let table = table_args.load.open(path)?;
    eprintln!(
        "table h{} base {} loaded in {:.1}s",
        table.meta.k,
        table.meta.base,
        load_start.elapsed().as_secs_f64()
    );
    let solver = Solver::new(&corners, &table);
    let lanes = |run: &RunArgs| cli::lanes(run.lanes, path, table.meta.k, run.threads);
    match command {
        Command::Tune(args) => tune(&solver, path, args.window, json),
        Command::Stream(args) => stream(&solver, args, lanes(&args.run), json),
        Command::Batch(args) => batch(&solver, args, &cubes, lanes(&args.run), json),
    }
}

#[cfg(test)]
mod tests {
    use super::{cube_set, legacy};
    use egg_solver::cubie::{CubieCube, Rng};

    #[test]
    fn seconds_reject_unrepresentable_durations() {
        for value in ["-1", "NaN", "inf", "1e300"] {
            assert!(super::seconds(value).is_err(), "{value}");
        }
        assert_eq!(super::seconds("0.5"), Ok(0.5));
    }

    /// `random:N:SEED` is N random states from `Rng(SEED)`, as before.
    #[test]
    fn random_sets() {
        let mut rng = Rng(2026);
        let want: Vec<CubieCube> = (0..3).map(|_| CubieCube::random(&mut rng)).collect();
        assert_eq!(cube_set("random:3:2026").unwrap(), want);
        for bad in ["random:3", "random:x:1", "random:1:2:3", "random:"] {
            assert!(cube_set(bad).is_err(), "{bad}");
        }
    }

    fn rewritten(args: &str) -> String {
        let args = args.split(' ').map(std::ffi::OsString::from).collect();
        let args: Vec<String> = legacy(args)
            .into_iter()
            .map(|a| a.into_string().unwrap())
            .collect();
        args.join(" ")
    }

    /// The forms scripts use become the subcommand forms.
    #[test]
    fn legacy_forms_become_subcommands() {
        for (old, new) in [
            (
                "bench /mnt/huge/h11 stream 20 30 48 24",
                "bench stream /mnt/huge/h11 --warmup 20 --window 30 --threads 48 --lanes 24",
            ),
            (
                "bench t stream 10 30 48",
                "bench stream t --warmup 10 --window 30 --threads 48",
            ),
            (
                "bench t stream 10 20 48 24 7 share",
                "bench stream t --warmup 10 --window 20 --threads 48 --lanes 24 --seed 7 --share",
            ),
            (
                "bench t random:1000:2026 1000 48 24 share",
                "bench batch t random:1000:2026 --count 1000 --threads 48 --lanes 24 --share",
            ),
            (
                "bench t depth18.txt 100 48 20 share:1",
                "bench batch t depth18.txt --count 100 --threads 48 --lanes 20 --share=1",
            ),
            ("bench t f", "bench batch t f"),
            ("bench t tune", "bench tune t"),
            ("bench t tune 3", "bench tune t --window 3"),
            (
                "bench t random:9:1 9 -v --json",
                "bench batch t random:9:1 --count 9 -v --json",
            ),
            (
                "bench stream t --window 3 --share",
                "bench stream t --window 3 --share",
            ),
            ("bench --json batch t f", "bench --json batch t f"),
            ("bench --help", "bench --help"),
            ("bench t", "bench t"),
        ] {
            assert_eq!(rewritten(old), new);
        }
    }
}
