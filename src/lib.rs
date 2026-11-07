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

// Packed coordinates and on-disk word encoding use little-endian layout.
// Do not let unsupported targets reinterpret bytes as unchecked indices.
#[cfg(target_endian = "big")]
compile_error!("egg-solver currently requires a little-endian target");

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
/// # Safety
/// Output files must not be accessed through other handles or mappings during
/// generation, or changed or truncated while any returned table uses them.
/// Calls with no output files use private anonymous memory and are unrestricted.
pub unsafe fn generate(
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
    if base > 12 || derived.iter().any(|(dk, db, _)| *dk > k || *db > 12) {
        return Err(TableError::Generate("invalid generation metadata".into()));
    }
    let allocate = |k: u32, path: Option<&str>| -> Result<Memory, TableError> {
        let len = h48_bytes(k);
        let too_large = |_| TableError::TooLarge(k);
        match path {
            Some(path) => {
                // (Room for the trailer after the data.)
                let file_len = tables::file::file_bytes(len, tables::file::is_hugetlbfs(path));
                let file_len = usize::try_from(file_len).map_err(too_large)?;
                // SAFETY: the caller grants exclusive access to output files.
                unsafe { Memory::map_file(path, file_len, true) }
                    .map_err(tables::file::io_error(path))
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
        // SAFETY: fresh writable mappings, shared only through atomic words
        // until generation has joined every worker.
        .map(|memory| unsafe { memory.words() })
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
    // SAFETY: fresh writable memory, not exposed to readers before this ends.
    let _counts = unsafe { generator.run(&memory)? };
    let table = H48Table {
        memory,
        meta: H48Meta { k, base },
    };
    if let Some(path) = out {
        // SAFETY: output storage is exclusively owned; no trailer view is live.
        unsafe { tables::file::write_trailer(path, &table.trailer(threads)?) }?;
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
                // SAFETY: output storage is exclusively owned; no trailer view is live.
                unsafe { tables::file::write_trailer(path, &table.trailer(threads)?) }?;
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
        // SAFETY: no output files; all memory is privately owned.
        return Ok(unsafe { generate(corners, k, base, None, &[], threads) }?.0);
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
        // SAFETY: our fresh writable mapping, read only after generation ends.
        words: unsafe { memory.words()? },
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

#[cfg(test)]
mod safe_api_tests {
    use super::*;
    use simd::Cube;

    #[test]
    fn safe_api_rejects_invalid_cubes_and_coordinates() {
        assert!(Cube::from_bytes([4; 32]).is_none());
        assert!(Cube::from_bytes([255; 32]).is_none());
        let solved = Cube::solved();
        for sym in [48, usize::MAX] {
            assert!(coord::edge_parts_sym(solved, sym).is_none());
            assert!(coord::edge_key_sym(solved, sym).is_none());
        }
        for key in [0, 1, 511, 70 << 20, u64::MAX] {
            assert!(coord::cube_from_edge_key(solved, key).is_none());
        }
        let original = solved.to_bytes();
        for position in (0..8).chain(16..28) {
            let mut bytes = original;
            bytes[position] = 255;
            assert!(Cube::from_bytes(bytes).is_none());
        }
        let mut bytes = original;
        bytes[8..16].fill(255);
        bytes[28..32].fill(255);
        assert_eq!(Cube::from_bytes(bytes).unwrap().to_bytes(), original);
        let mut rng = cubie::Rng(2026);
        for _ in 0..64 {
            let cube = Cube::from_cubie(&cubie::CubieCube::random(&mut rng));
            assert_eq!(
                Cube::from_bytes(cube.to_bytes()).unwrap().to_cubie(),
                cube.to_cubie()
            );
            let key = coord::edge_key(cube);
            let restored = coord::cube_from_edge_key(cube, key).unwrap();
            assert_eq!(coord::edge_key(restored), key);
            assert_eq!(
                coord::edge_key_sym(cube, 47),
                Some(coord::edge_key(cube.transform_edges(47)))
            );
        }
    }

    #[test]
    fn safe_api_checks_table_memory_and_metadata() {
        assert!(Memory::anonymous(usize::MAX).is_err());
        for meta in [
            H48Meta { k: 12, base: 2 },
            H48Meta { k: 0, base: 13 },
            H48Meta { k: 0, base: 2 },
        ] {
            assert!(H48Table::from_memory(Memory::anonymous(4096).unwrap(), meta).is_err());
        }
        // An intentionally undersized internal fixture also exercises get's
        // allocation check. External code cannot construct this table.
        let table = H48Table {
            memory: Memory::anonymous(4096).unwrap(),
            meta: H48Meta { k: 0, base: 2 },
        };
        assert_eq!(table.get(0), Some((0, 0)));
        assert_eq!(table.get(16383), Some((0, 0)));
        for index in [16384, tables::h48_slots(0), u64::MAX] {
            assert_eq!(table.get(index), None);
            table.prefetch(index);
        }
    }
}
