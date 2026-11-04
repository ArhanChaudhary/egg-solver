//! Solve scrambles from stdin, one per line.
//! Usage: cubeopt <table path>
use cubeopt::cubie::{format_moves, parse_moves, CubieCube};
use cubeopt::solver::Solver;
use cubeopt::tables::{Cocsep, H48Table};
use std::io::BufRead;

fn main() {
    let path = std::env::args().nth(1).expect("usage: cubeopt <table>");
    let cs = Cocsep::generate();
    let table = H48Table::open(&path).expect("open table");
    let solver = Solver::new(&cs, &table);
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let Some(ms) = parse_moves(&line) else {
            println!("parse error");
            continue;
        };
        let c = CubieCube::SOLVED.apply_moves(&ms);
        let t = std::time::Instant::now();
        let (sol, st) = solver.solve(&c);
        println!(
            "{} ({}) {:.3}s nodes {}",
            format_moves(&sol),
            sol.len(),
            t.elapsed().as_secs_f64(),
            st.nodes
        );
    }
}
