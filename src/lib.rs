#![allow(clippy::needless_range_loop, clippy::manual_is_multiple_of)]
pub mod coord;
pub mod cubie;
pub mod gen;
pub mod simd;
pub mod solver;
pub mod tables;

use tables::*;

/// Generate a table by BFS on h0 or h11, plus derived tables.
/// Returns the main table and the derived tables (each with its own memory).
pub fn generate(
    cs: &Cocsep,
    k: u32,
    base: u8,
    out: Option<&str>,
    derived: &[(u32, u8, Option<String>)],
    threads: usize,
) -> (H48Table, Vec<H48Table>) {
    assert!(k == 0 || k == 11, "BFS only on true coordinates h0/h11");
    let alloc = |kk: u32, path: Option<&str>| -> Mem {
        let len = h48_bytes(kk) as usize;
        match path {
            Some(p) => Mem::file(p, len, true).expect("create table file"),
            None => Mem::anon(len),
        }
    };
    let mem = alloc(k, out);
    let dmems: Vec<Mem> = derived
        .iter()
        .map(|(dk, _, p)| alloc(*dk, p.as_deref()))
        .collect();
    let g = gen::Gen {
        cs,
        k,
        base,
        threads,
        derived: derived
            .iter()
            .zip(dmems.iter())
            .map(|((dk, db, _), m)| (H48Meta { k: *dk, base: *db }, m))
            .collect(),
    };
    g.run(&mem);
    let meta = H48Meta { k, base };
    if let Some(p) = out {
        write_meta(p, &meta).unwrap();
    }
    let dts = derived
        .iter()
        .zip(dmems)
        .map(|((dk, db, p), m)| {
            let meta = H48Meta { k: *dk, base: *db };
            if let Some(p) = p {
                write_meta(p, &meta).unwrap();
            }
            H48Table { mem: m, meta }
        })
        .collect();
    (H48Table { mem, meta }, dts)
}

pub fn nthreads() -> usize {
    std::thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(1)
}
