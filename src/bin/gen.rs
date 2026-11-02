//! Generate H48 tables.
//! Usage: gen <k: 0|11> <base> <out path> [derived k:base:path ...]
#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]
use cubeopt::tables::*;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!("usage: gen <0|11> <base> <out> [k:base:path ...]");
        std::process::exit(1);
    }
    let k: u32 = a[1].parse().unwrap();
    let base: u8 = a[2].parse().unwrap();
    let out = a[3].clone();
    let derived: Vec<(u32, u8, Option<String>)> = a[4..]
        .iter()
        .map(|s| {
            let p: Vec<&str> = s.splitn(3, ':').collect();
            (
                p[0].parse().unwrap(),
                p[1].parse().unwrap(),
                Some(p[2].to_string()),
            )
        })
        .collect();
    let t = std::time::Instant::now();
    let cs = Cocsep::generate();
    let (main, ds) = cubeopt::generate(&cs, k, base, Some(&out), &derived, cubeopt::nthreads());
    eprintln!("generated in {:.1}s", t.elapsed().as_secs_f64());
    eprintln!(
        "h{} base {} distribution {:?}",
        k,
        base,
        main.distribution()
    );
    for d in &ds {
        eprintln!(
            "h{} base {} distribution {:?}",
            d.meta.k,
            d.meta.base,
            d.distribution()
        );
    }
}
