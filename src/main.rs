//! Solve scrambles from stdin, one per line, with the recursive solver
//! (`solver::Solver`), and print each solution, its length, time and nodes.
//! For speed use `solve` (the `lanes` search) instead.
//!
//! Usage: cubeopt <table path>
use cubeopt::cubie::{format_moves, parse_moves, CubieCube};
use cubeopt::solver::Solver;
use cubeopt::tables::{Cocsep, H48Table};
use std::io::BufRead;

fn main() {
    let path = std::env::args().nth(1).expect("usage: cubeopt <table>");
    let corners = Cocsep::generate();
    let table = H48Table::open(&path).expect("open table");
    let solver = Solver::new(&corners, &table);
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
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
}
