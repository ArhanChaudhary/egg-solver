//! Solve scrambles from stdin, one per line, with the recursive solver
//! (`solver::Solver`), and print each solution, its length, time and nodes.
//! For speed use `solve` (the `lanes` search) instead.
//!
//! Usage: egg-solver <table path>

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
use clap::Parser;
use egg_solver::cli::{self, LoadArgs, Verbosity};
use egg_solver::cubie::{format_moves, parse_moves, CubieCube};
use egg_solver::solver::Solver;
use egg_solver::tables::Cocsep;
use std::io::BufRead;
use std::process::ExitCode;

/// Solve scrambles from stdin, one per line, with the plain recursive
/// solver (the reference search; `solve` is much faster).
#[derive(Parser, Debug)]
#[command(name = "egg-solver", version)]
struct Args {
    /// The table file (made by `gen`).
    #[arg(env = egg_solver::env::name("EGG_SOLVER_TABLE"), required_unless_present = "completions")]
    table: Option<String>,
    #[command(flatten)]
    load: LoadArgs,
    #[command(flatten)]
    verbosity: Verbosity,
    /// Print a shell completion script.
    #[arg(long, value_name = "SHELL", exclusive = true, hide = true)]
    completions: Option<clap_complete::Shell>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    if let Some(shell) = args.completions {
        cli::print_completions::<Args>(shell);
        return ExitCode::SUCCESS;
    }
    args.verbosity.init_logging();
    cli::exit_status("egg-solver", run(&args))
}

fn run(args: &Args) -> anyhow::Result<ExitCode> {
    let path = args.table.as_deref().context("no table")?;
    let corners = Cocsep::generate()?;
    let table = args.load.open(path)?;
    let solver = Solver::new(&corners, &table);
    for line in std::io::stdin().lock().lines() {
        let line = line.context("cannot read stdin")?;
        let Some(cube) = parse_moves(&line).and_then(|moves| CubieCube::SOLVED.apply_moves(&moves))
        else {
            println!("parse error");
            continue;
        };
        let start = std::time::Instant::now();
        let (solution, stats) = solver.solve(&cube);
        let Some(solution) = solution else {
            println!("no solution within 20 moves (is the table damaged?)");
            continue;
        };
        println!(
            "{} ({}) {:.3}s nodes {}",
            format_moves(&solution),
            solution.len(),
            start.elapsed().as_secs_f64(),
            stats.nodes
        );
    }
    Ok(ExitCode::SUCCESS)
}
