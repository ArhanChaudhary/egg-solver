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
//! * `simd`: the fast cube (one AVX2 register, or two 16-byte vectors on
//!   other targets: `portable`) and its moves and symmetries.
//! * `coord`: coordinates, the table indices of a cube.
//! * `tables`: the corner table and the big H48 table (and its files,
//!   `tables::file`); `gen` builds the latter, for any k.
//! * `solver`: a plain recursive IDA*, the reference search.
//! * `lanes`: the fast search: many cubes per thread, interleaved so that
//!   table loads overlap; `lanes::share` lets all threads work on a few
//!   cubes.
//! * `symstart`: for cubes with a symmetry, one job of each class of
//!   equivalent job prefixes.
//! * `cli` (feature `cli`, on by default): what the binaries share.

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

#[cfg(feature = "cli")]
pub mod cli;
pub mod coord;
pub mod cubie;
pub mod env;
pub mod gen;
pub mod hot;
pub mod lanes;
pub mod parallel;
pub mod prefetch;
/// The cube type and the hot path's small tables: AVX2 on x86 (`simd.rs`),
/// else the portable backend (NEON on aarch64, scalar elsewhere).
#[cfg(backend = "x86")]
pub mod simd;
#[cfg(not(backend = "x86"))]
#[path = "portable/mod.rs"]
pub mod simd;
pub mod solver;
pub mod symstart;
pub mod tables;

/// The portable backend, compiled with the x86 one only for its tests (which
/// cross-check the two).
#[cfg(all(test, backend = "x86"))]
#[path = "portable/mod.rs"]
#[allow(dead_code)]
mod portable_check;

use tables::{h48_bytes, h48_rows, Cocsep, H48Meta, H48Table, Memory, TableError};

/// Generate a table by BFS on h0 or h11, plus derived tables (each
/// `(k, base, file)`). Returns the main table and the derived tables (each
/// with its own memory); tables with a file are written to it.
///
/// # Errors
/// If `k` is not 0 or 11, a derived `k` is above `k`, memory cannot be
/// allocated, or a table file cannot be created or written.
pub fn generate(
    corners: &Cocsep,
    k: u32,
    base: u8,
    out: Option<&str>,
    derived: &[(u32, u8, Option<String>)],
    threads: usize,
) -> Result<(H48Table, Vec<H48Table>), TableError> {
    if k != 0 && k != 11 {
        return Err(TableError::Generate(format!(
            "BFS only on true coordinates (h0, h11), not h{k}"
        )));
    }
    let allocate = |k: u32, path: Option<&str>| -> Result<Memory, TableError> {
        let len = h48_bytes(k);
        let too_large = |_| TableError::TooLarge(k);
        match path {
            Some(path) => {
                // (Room for the trailer after the data.)
                let file_len = tables::file::file_bytes(len, tables::file::is_hugetlbfs(path));
                let file_len = usize::try_from(file_len).map_err(too_large)?;
                Memory::map_file(path, file_len, true).map_err(tables::file::io_error(path))
            }
            None => Memory::anonymous(usize::try_from(len).map_err(too_large)?),
        }
    };
    let memory = allocate(k, out)?;
    let derived_memory: Vec<Memory> = derived
        .iter()
        .map(|(dk, _, path)| allocate(*dk, path.as_deref()))
        .collect::<Result<_, _>>()?;
    let derived_words = derived_memory
        .iter()
        .map(Memory::words)
        .collect::<Result<Vec<_>, _>>()?;
    let generator = gen::Generator {
        corners,
        k,
        base,
        threads,
        derived: derived
            .iter()
            .zip(derived_words)
            .map(|((dk, dbase, _), words)| {
                (
                    H48Meta {
                        k: *dk,
                        base: *dbase,
                    },
                    words,
                )
            })
            .collect(),
    };
    let _counts = generator.run(&memory)?;
    let table = H48Table {
        memory,
        meta: H48Meta { k, base },
    };
    if let Some(path) = out {
        tables::file::write_trailer(path, &table.trailer(threads)?)?;
    }
    let derived_tables = derived
        .iter()
        .zip(derived_memory)
        .map(|((dk, dbase, path), memory)| {
            let table = H48Table {
                memory,
                meta: H48Meta {
                    k: *dk,
                    base: *dbase,
                },
            };
            if let Some(path) = path {
                tables::file::write_trailer(path, &table.trailer(threads)?)?;
            }
            Ok(table)
        })
        .collect::<Result<_, TableError>>()?;
    Ok((table, derived_tables))
}

/// Generate hk of any k (0..=11) in anonymous memory: by BFS for the true
/// coordinates h0 and h11, else by `gen::dfs`.
///
/// # Errors
/// If `k` is above 11 or `base` above 12, memory cannot be allocated, or a
/// thread panicked.
pub fn generate_table(
    corners: &Cocsep,
    k: u32,
    base: u8,
    threads: usize,
    verbose: bool,
) -> Result<H48Table, TableError> {
    if k == 0 || k == 11 {
        return Ok(generate(corners, k, base, None, &[], threads)?.0);
    }
    let settings = gen::dfs::Settings {
        k,
        base,
        threads,
        prefix_depth: gen::dfs::PREFIX_DEPTH.min(base.saturating_add(2)),
        verbose,
    };
    let generator = gen::dfs::Generator::new(corners, settings)?;
    let len = usize::try_from(h48_bytes(k)).map_err(|_| TableError::TooLarge(k))?;
    let memory = Memory::anonymous(len)?;
    let rows = 0..h48_rows(k);
    let part = gen::dfs::Part {
        words: memory.words()?,
        first_slot: 0,
        end_slot: rows.end * coord::N_SLOTS as u64,
    };
    generator.generate(&rows, &part)?;
    Ok(H48Table {
        memory,
        meta: H48Meta { k, base },
    })
}

/// The number of hardware threads (1 if unknown).
#[must_use]
pub fn thread_count() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}
