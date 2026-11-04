pub mod coord;
pub mod cubie;
pub mod gen;
pub mod lanes;
pub mod simd;
pub mod solver;
pub mod tables;

use tables::{h48_bytes, write_meta, Cocsep, H48Meta, H48Table, Mem};

/// Generate a table by BFS on h0 or h11, plus derived tables.
/// Returns the main table and the derived tables (each with its own memory).
///
/// # Panics
/// If `k` is not 0 or 11, or a table file cannot be created or written.
#[must_use]
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
        let len = usize::try_from(h48_bytes(kk)).expect("64-bit usize");
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
    let _counts = g.run(&mem);
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

#[must_use]
pub fn nthreads() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}
