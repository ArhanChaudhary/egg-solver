//! Throughput benchmark: optimal solves per second.
//! Usage: bench <table> <scramble file | random:N:SEED> [count] [threads]
#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]
use cubeopt::cubie::*;
use cubeopt::solver::Solver;
use cubeopt::tables::*;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let path = &a[1];
    let file = &a[2];
    // "random:N:SEED" gives N uniformly random states, otherwise a scramble file.
    let mut cubes: Vec<CubieCube> = if let Some(spec) = file.strip_prefix("random:") {
        let v: Vec<u64> = spec.split(':').map(|x| x.parse().unwrap()).collect();
        let mut rng = Rng(v[1]);
        (0..v[0]).map(|_| CubieCube::random(&mut rng)).collect()
    } else {
        std::fs::read_to_string(file)
            .unwrap()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| CubieCube::SOLVED.apply_moves(&parse_moves(l).unwrap()))
            .collect()
    };
    if let Some(n) = a.get(3) {
        cubes.truncate(n.parse().unwrap());
    }
    let threads: usize = a
        .get(4)
        .map(|x| x.parse().unwrap())
        .unwrap_or(cubeopt::nthreads());
    let cs = Cocsep::generate();
    let tl = Instant::now();
    let table = H48Table::open(path).expect("open table");
    eprintln!(
        "table h{} base {} loaded in {:.1}s",
        table.meta.k,
        table.meta.base,
        tl.elapsed().as_secs_f64()
    );
    let solver = Solver::new(&cs, &table);
    let next = AtomicUsize::new(0);
    let nodes = AtomicU64::new(0);
    let lookups = AtomicU64::new(0);
    let lens = std::sync::Mutex::new(vec![0usize; 32]);
    let times = std::sync::Mutex::new(Vec::<f64>::new());
    let t = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= cubes.len() {
                    break;
                }
                let ts = Instant::now();
                let (sol, st) = solver.solve(&cubes[i]);
                let dt = ts.elapsed().as_secs_f64();
                times.lock().unwrap().push(dt);
                assert!(
                    cubes[i].apply_moves(&sol) == CubieCube::SOLVED,
                    "bad solution"
                );
                nodes.fetch_add(st.nodes, Ordering::Relaxed);
                lookups.fetch_add(st.lookups, Ordering::Relaxed);
                lens.lock().unwrap()[sol.len()] += 1;
            });
        }
    });
    let el = t.elapsed().as_secs_f64();
    let n = cubes.len() as f64;
    let lens = lens.lock().unwrap();
    let dist: Vec<String> = lens
        .iter()
        .enumerate()
        .filter(|(_, &c)| c > 0)
        .map(|(l, c)| format!("{l}:{c}"))
        .collect();
    println!("cubes {} threads {} time {:.3}s", cubes.len(), threads, el);
    println!("solves/sec {:.3}", n / el);
    println!(
        "nodes/cube {:.0} lookups/cube {:.0}",
        nodes.load(Ordering::Relaxed) as f64 / n,
        lookups.load(Ordering::Relaxed) as f64 / n
    );
    println!(
        "ns/node/thread {:.1}",
        el * 1e9 * threads as f64 / nodes.load(Ordering::Relaxed) as f64
    );
    println!("lengths {}", dist.join(" "));
    let mut times = times.into_inner().unwrap();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let cpu: f64 = times.iter().sum();
    println!(
        "cpu-sec/solve {:.4} (sum {:.1}s) max {:.3}s p50 {:.4}s",
        cpu / n,
        cpu,
        times[times.len() - 1],
        times[times.len() / 2]
    );
    // Steady-state throughput: all threads busy, no end-of-batch tail.
    println!("throughput {:.3}", threads as f64 * n / cpu);
}
