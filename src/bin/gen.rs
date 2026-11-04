//! Generate H48 tables: table hk (k = 0 or 11) by BFS, and optionally
//! tables of smaller k derived from it on the way.
//!
//! Usage: gen <k: 0|11> <base> <out path> [derived k:base:path ...]
use cubeopt::tables::Cocsep;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: gen <0|11> <base> <out> [k:base:path ...]");
        std::process::exit(1);
    }
    let k: u32 = args[1].parse().unwrap();
    let base: u8 = args[2].parse().unwrap();
    let out = args[3].clone();
    let derived: Vec<(u32, u8, Option<String>)> = args[4..]
        .iter()
        .map(|spec| {
            let parts: Vec<&str> = spec.splitn(3, ':').collect();
            (
                parts[0].parse().unwrap(),
                parts[1].parse().unwrap(),
                Some(parts[2].to_string()),
            )
        })
        .collect();
    let start = std::time::Instant::now();
    let corners = Cocsep::generate();
    let (table, derived_tables) = cubeopt::generate(
        &corners,
        k,
        base,
        Some(&out),
        &derived,
        cubeopt::thread_count(),
    );
    eprintln!("generated in {:.1}s", start.elapsed().as_secs_f64());
    eprintln!(
        "h{} base {} distribution {:?}",
        k,
        base,
        table.distribution()
    );
    for derived_table in &derived_tables {
        eprintln!(
            "h{} base {} distribution {:?}",
            derived_table.meta.k,
            derived_table.meta.base,
            derived_table.distribution()
        );
    }
}
