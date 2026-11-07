//! Spot check of a BFS table (h0 or h11) against its own neighbor relation.
//!
//! The table holds coset distances, so for a coset at distance `d` every
//! neighbor (one move away) is at distance `d - 1`, `d` or `d + 1`, and some
//! neighbor is at `d - 1`. On the 2-bit codes (0: `<= base`, 1: `base + 1`,
//! 2: `base + 2`, 3: `>= base + 3`) that gives:
//! * every neighbor's code is at least `code - 1` (no neighbor is too low:
//!   this is what makes the bounds admissible) and at most `code + 1`
//!   (exact for codes 0..2);
//! * codes 1 and 2 have a neighbor with `code - 1`.
//!
//! It also checks that a decoded entry indexes back to itself and that the
//! self-symmetric variants of an entry agree.
//!
//! ```text
//! tabcheck <table> <samples> <seed> [threads]
//! ```
use egg_solver::coord::{cube_from_edge_key, split_index, N_SLOTS};
use egg_solver::cubie::{Move, Rng};
use egg_solver::tables::{entry_class, entry_sym_to_rep, h48_rows, Cocsep, H48Table};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Violations found, by kind.
#[derive(Default)]
struct Counts {
    samples: AtomicU64,
    too_low_neighbor: AtomicU64,
    too_high_neighbor: AtomicU64,
    no_lower_neighbor: AtomicU64,
    index_mismatch: AtomicU64,
    variant_mismatch: AtomicU64,
    /// Samples by code.
    codes: [AtomicU64; 4],
}

fn report(what: &str, index: u64, detail: &str) {
    eprintln!("VIOLATION {what}: index {index} {detail}");
}

fn random_index(rows: u64, rng: &mut Rng) -> u64 {
    // Slots 0 and 1 of each group hold metadata; slot 511 is unused.
    let slot = loop {
        let value = rng.below(N_SLOTS as u64);
        if value % 64 >= 2 && value != 511 {
            break value;
        }
    };
    rng.below(rows) * N_SLOTS as u64 + slot
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let samples: u64 = args[2].parse().unwrap();
    let seed: u64 = args[3].parse().unwrap();
    let threads: u64 = args.get(4).map_or(8, |x| x.parse().unwrap());
    let corners = Cocsep::generate().unwrap();
    // SAFETY: Mapped/shared CLI inputs are required to remain unchanged while loaded (README contract).
    let table = unsafe { H48Table::open(&args[1]) }.expect("open table");
    let k = table.meta().k;
    assert!(
        k == 0 || k == 11,
        "only tables of true coordinates (h0, h11)"
    );
    let rows = h48_rows(k);
    let counts = Counts::default();
    std::thread::scope(|s| {
        for t in 0..threads {
            let (corners, table, counts, report) = (&corners, &table, &counts, &report);
            s.spawn(move || {
                let mut rng = Rng(seed ^ (t << 40));
                for _ in 0..samples / threads {
                    let index = random_index(rows, &mut rng);
                    let (class, edge_key) = split_index(index, k);
                    let (Some(cube), Some((code, _))) = (
                        cube_from_edge_key(corners.representatives()[class], edge_key),
                        table.get(index),
                    ) else {
                        counts.index_mismatch.fetch_add(1, Relaxed);
                        report("index", index, "invalid coordinate");
                        continue;
                    };
                    counts.samples.fetch_add(1, Relaxed);
                    counts.codes[usize::from(code)].fetch_add(1, Relaxed);
                    if corners.index(cube, k) != index {
                        counts.index_mismatch.fetch_add(1, Relaxed);
                        report("index", index, &format!("{cube:?}"));
                    }
                    // Self-symmetric variants of the entry (the same coset seen
                    // through a symmetry fixing the class representative).
                    let mut sym = corners.self_symmetries()[class] & !1;
                    while sym != 0 {
                        let s = sym.trailing_zeros() as usize;
                        sym ^= sym.isolate_lowest_one();
                        let variant = cube.transform(s);
                        let entry = corners.entries()[egg_solver::coord::cocsep(variant)];
                        debug_assert_eq!(entry_class(entry), class);
                        let variant_index = corners.index(variant, k);
                        if table
                            .get(variant_index)
                            .is_none_or(|(value, _)| value != code)
                        {
                            counts.variant_mismatch.fetch_add(1, Relaxed);
                            report(
                                "variant",
                                index,
                                &format!("sym {s} ttrep {}", entry_sym_to_rep(entry)),
                            );
                        }
                    }
                    let mut has_lower = false;
                    for m in Move::ALL {
                        let Some((neighbor_code, _)) =
                            table.get(corners.index(cube.apply_move(m), k))
                        else {
                            counts.index_mismatch.fetch_add(1, Relaxed);
                            report("neighbor", index, "invalid coordinate");
                            continue;
                        };
                        if neighbor_code + 1 < code {
                            counts.too_low_neighbor.fetch_add(1, Relaxed);
                            report(
                                "too low neighbor",
                                index,
                                &format!("code {code} move {} code {neighbor_code}", m.index()),
                            );
                        }
                        if neighbor_code > code + 1 && code < 3 {
                            counts.too_high_neighbor.fetch_add(1, Relaxed);
                            report(
                                "too high neighbor",
                                index,
                                &format!("code {code} move {} code {neighbor_code}", m.index()),
                            );
                        }
                        has_lower |= neighbor_code + 1 == code;
                    }
                    if (code == 1 || code == 2) && !has_lower {
                        counts.no_lower_neighbor.fetch_add(1, Relaxed);
                        report("no lower neighbor", index, &format!("code {code}"));
                    }
                }
            });
        }
    });
    println!(
        "codes {:?} samples {} too_low_neighbor {} too_high_neighbor {} no_lower_neighbor {} index_mismatch {} variant_mismatch {}",
        counts.codes.each_ref().map(|c| c.load(Relaxed)),
        counts.samples.load(Relaxed),
        counts.too_low_neighbor.load(Relaxed),
        counts.too_high_neighbor.load(Relaxed),
        counts.no_lower_neighbor.load(Relaxed),
        counts.index_mismatch.load(Relaxed),
        counts.variant_mismatch.load(Relaxed),
    );
}
