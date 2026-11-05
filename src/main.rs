//! Solve scrambles from stdin, one per line, with the recursive solver
//! (`solver::Solver`), and print each solution, its length, time and nodes.
//! For speed use `solve` (the `lanes` search) instead.
//!
//! Usage: cubeopt <table path>
use anyhow::Context as _;
use clap::Parser;
use cubeopt::cli::{self, LoadArgs, Verbosity};
use cubeopt::cubie::{format_moves, parse_moves, CubieCube};
use cubeopt::solver::Solver;
use cubeopt::tables::Cocsep;
use std::io::BufRead;
use std::process::ExitCode;

/// Solve scrambles from stdin, one per line, with the plain recursive
/// solver (the reference search; `solve` is much faster).
#[derive(Parser, Debug)]
#[command(name = "cubeopt", version)]
struct Args {
    /// The table file (made by `gen`).
    #[arg(env = "CUBEOPT_TABLE", required_unless_present = "completions")]
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
    cli::exit_status("cubeopt", run(&args))
}

fn run(args: &Args) -> anyhow::Result<ExitCode> {
    let path = args.table.as_deref().context("no table")?;
    let corners = Cocsep::generate();
    let table = args.load.open(path)?;
    let solver = Solver::new(&corners, &table);
    for line in std::io::stdin().lock().lines() {
        let line = line.context("cannot read stdin")?;
        let Some(moves) = parse_moves(&line) else {
            println!("parse error");
            continue;
        };
        let cube = CubieCube::SOLVED.apply_moves(&moves);
        let start = std::time::Instant::now();
        let (solution, stats) = solver.solve(&cube);
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
