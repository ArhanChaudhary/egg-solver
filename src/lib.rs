//! A fast optimal solver for the 3x3x3 Rubik's cube.
//!
//! It finds a shortest move sequence (in the half-turn metric: any turn of
//! a face is one move) with IDA*: depth-first searches with an increasing
//! depth limit, pruned by lower bounds on the distance to solved read from
//! precomputed tables. Each search runs on the cube and on its inverse at
//! once, moving whichever side has fewer moves to try.
//!
//! Modules, from the bottom up:
//! * `cubie`: the reference cube model (pieces and their orientations).
//! * `simd`: the fast cube (one AVX2 register) and its moves and symmetries.
//! * `coord`: coordinates, the table indices of a cube.
//! * `tables`: the corner table and the big H48 table; `gen` builds the
//!   latter.
//! * `solver`: a plain recursive IDA*, the reference search.
//! * `lanes`: the fast search: many cubes per thread, interleaved so that
//!   table loads overlap; `lanes::share` lets all threads work on a few
//!   cubes.

pub mod coord;
pub mod cubie;
pub mod gen;
pub mod lanes;
pub mod simd;
pub mod solver;
pub mod tables;

use tables::{h48_bytes, write_meta, Cocsep, H48Meta, H48Table, Memory};

/// Generate a table by BFS on h0 or h11, plus derived tables (each
/// `(k, base, file)`). Returns the main table and the derived tables (each
/// with its own memory); tables with a file are written to it.
///
/// # Panics
/// If `k` is not 0 or 11, or a table file cannot be created or written.
#[must_use]
pub fn generate(
    corners: &Cocsep,
    k: u32,
    base: u8,
    out: Option<&str>,
    derived: &[(u32, u8, Option<String>)],
    threads: usize,
) -> (H48Table, Vec<H48Table>) {
    assert!(k == 0 || k == 11, "BFS only on true coordinates h0/h11");
    let allocate = |k: u32, path: Option<&str>| -> Memory {
        let len = usize::try_from(h48_bytes(k)).expect("64-bit usize");
        match path {
            Some(path) => Memory::map_file(path, len, true).expect("create table file"),
            None => Memory::anonymous(len),
        }
    };
    let memory = allocate(k, out);
    let derived_memory: Vec<Memory> = derived
        .iter()
        .map(|(dk, _, path)| allocate(*dk, path.as_deref()))
        .collect();
    let generator = gen::Generator {
        corners,
        k,
        base,
        threads,
        derived: derived
            .iter()
            .zip(&derived_memory)
            .map(|((dk, dbase, _), memory)| {
                (
                    H48Meta {
                        k: *dk,
                        base: *dbase,
                    },
                    memory,
                )
            })
            .collect(),
    };
    let _counts = generator.run(&memory);
    let meta = H48Meta { k, base };
    if let Some(path) = out {
        write_meta(path, &meta).unwrap();
    }
    let derived_tables = derived
        .iter()
        .zip(derived_memory)
        .map(|((dk, dbase, path), memory)| {
            let meta = H48Meta {
                k: *dk,
                base: *dbase,
            };
            if let Some(path) = path {
                write_meta(path, &meta).unwrap();
            }
            H48Table { memory, meta }
        })
        .collect();
    (H48Table { memory, meta }, derived_tables)
}

/// The number of hardware threads (1 if unknown).
#[must_use]
pub fn thread_count() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}
