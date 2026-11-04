//! hk tables from tables of a larger k with the same base, and table
//! comparison.
//!
//! An hk entry is the minimum over the hK entries (K > k) that differ only in
//! the eo bits hk drops, i.e. over the 2^(K - k) rows `r << (K - k) ..` of
//! hK at the same slot; a group minimum is the minimum of those groups'
//! minima. With the same base the 2-bit codes collapse the same way
//! (`min(code)` is the code of the minimum), so for example h7..h10 of base
//! 10 follow from h11, and this checks directly generated tables.

use crate::tables::{
    entry_low_bits, h48_bytes, h48_rows, H48Meta, H48Table, Memory, WORDS_PER_ROW,
};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Per 2-bit field minimum of two words of codes.
#[inline(always)]
fn min_codes(x: u64, y: u64) -> u64 {
    const LO: u64 = 0x5555_5555_5555_5555;
    let (x0, x1) = (x & LO, (x >> 1) & LO);
    let (y0, y1) = (y & LO, (y >> 1) & LO);
    let hi = x1 & y1;
    // Equal high bits: the low bits' AND; else the low bit of the one with
    // high bit 0.
    let lo = ((!(x1 ^ y1)) & x0 & y0) | (x1 & !y1 & y0) | (!x1 & y1 & x0);
    (hi << 1) | (lo & LO)
}

/// The word `w` (of a row) of the collapse of `rows` (`16 * n` words, n
/// rows of the larger table).
#[inline(always)]
fn collapsed_word(rows: &[AtomicU64], w: usize) -> u64 {
    let mut out = u64::MAX;
    let mut min = 15;
    for r in rows.as_chunks::<WORDS_PER_ROW>().0 {
        let x = r[w].load(Relaxed);
        out = min_codes(out, x);
        min = min.min(x & 15);
    }
    if w.is_multiple_of(2) {
        (out & !15) | min
    } else {
        out
    }
}

/// hk of the same base from `src` (k <= src's k), in anonymous memory.
///
/// # Panics
/// If `k` is above `src`'s.
#[must_use]
pub fn derive(source: &H48Table, k: u32, threads: usize) -> H48Table {
    assert!(k <= source.meta.k);
    let factor = 1usize << (source.meta.k - k);
    let memory = Memory::anonymous(usize::try_from(h48_bytes(k)).unwrap());
    let rows = usize::try_from(h48_rows(k)).unwrap();
    let words = memory.words();
    let source_words = source.memory.words();
    par_rows(rows, threads, |row| {
        let covered = &source_words[row * factor * 16..(row + 1) * factor * 16];
        for w in 0..16 {
            words[row * 16 + w].store(collapsed_word(covered, w), Relaxed);
        }
    });
    H48Table {
        memory,
        meta: H48Meta {
            k,
            base: source.meta.base,
        },
    }
}

/// Differences found by `compare`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Diff {
    /// Real entries compared, and those whose codes differ.
    pub entries: u64,
    pub codes: u64,
    /// Group minima that differ, among those that matter (at most the base
    /// on either side; larger ones are never read: a group minimum is used
    /// only for a code-0 entry, whose depth is at most the base).
    pub group_minima: u64,
    /// Group minima above the base that differ (harmless).
    pub unread_group_minima: u64,
}

/// Compare `table` with the collapse of `larger` to `table`'s k (the same
/// table if the ks are equal).
///
/// # Panics
/// If the bases differ or `table`'s k is above `larger`'s.
#[must_use]
pub fn compare(table: &H48Table, larger: &H48Table, threads: usize) -> Diff {
    assert_eq!(
        table.meta.base, larger.meta.base,
        "tables of different bases"
    );
    assert!(table.meta.k <= larger.meta.k);
    let base = u64::from(table.meta.base);
    let factor = 1usize << (larger.meta.k - table.meta.k);
    let rows = usize::try_from(h48_rows(table.meta.k)).unwrap();
    let (words, larger_words) = (table.memory.words(), larger.memory.words());
    let total = [(); 4].map(|()| AtomicU64::new(0));
    par_rows_chunked(rows, threads, |first, end| {
        let mut counts = [0u64; 4];
        for row in first..end {
            let collapsed = &larger_words[row * factor * 16..(row + 1) * factor * 16];
            for w in 0..16 {
                let x = words[row * 16 + w].load(Relaxed);
                let y = if factor == 1 {
                    collapsed[w].load(Relaxed)
                } else {
                    collapsed_word(collapsed, w)
                };
                let mask = entry_low_bits(w as u64);
                counts[0] += u64::from(mask.count_ones());
                let differ = (x ^ y) | ((x ^ y) >> 1);
                counts[1] += u64::from((differ & mask).count_ones());
                if w.is_multiple_of(2) && (x & 15) != (y & 15) {
                    if (x & 15).min(y & 15) <= base {
                        counts[2] += 1;
                    } else {
                        counts[3] += 1;
                    }
                }
            }
        }
        for (sum, count) in total.iter().zip(counts) {
            sum.fetch_add(count, Relaxed);
        }
    });
    let [entries, codes, group_minima, unread_group_minima] = total.map(AtomicU64::into_inner);
    Diff {
        entries,
        codes,
        group_minima,
        unread_group_minima,
    }
}

fn par_rows(rows: usize, threads: usize, f: impl Fn(usize) + Sync) {
    par_rows_chunked(rows, threads, |a, b| (a..b).for_each(&f));
}

fn par_rows_chunked(rows: usize, threads: usize, f: impl Fn(usize, usize) + Sync) {
    let chunk = 1 << 12;
    let next = std::sync::atomic::AtomicUsize::new(0);
    crate::parallel::on_threads(threads, |_| loop {
        let a = next.fetch_add(chunk, Relaxed);
        if a >= rows {
            break;
        }
        f(a, (a + chunk).min(rows));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_codes_is_fieldwise_min() {
        for x in 0..256u64 {
            for y in 0..256u64 {
                let m = min_codes(x, y);
                for f in 0..4 {
                    let (a, b) = ((x >> (2 * f)) & 3, (y >> (2 * f)) & 3);
                    assert_eq!((m >> (2 * f)) & 3, a.min(b), "{x:#x} {y:#x} field {f}");
                }
            }
        }
    }
}
