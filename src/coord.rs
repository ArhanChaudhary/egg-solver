//! Coordinates: the numbers that index the pruning tables.
//!
//! A pruning table stores, for each value of a coordinate (a function of
//! the cube state), a lower bound on the number of moves to solve any cube
//! with that value. The H48 coordinates (from nissy and cube48opt):
//!
//! * cocsep: corner orientation (3^7) x corner tetrad separation: which 4
//!   of the 8 positions hold the corners `UFL UBR DFR DBL` (C(8,4) = 70;
//!   half turns keep these 4 among themselves). Reduced by the 48
//!   symmetries to 3393 classes.
//! * msplit: which 4 of the 8 non-E-slice positions hold S-slice edges (70).
//! * eo: edge orientation (2^11; the 12th flip follows from the others).
//! * eslot: which 4 positions hold E-slice edges (C(12,4) = 495 values),
//!   packed into 512 slots so that slots 64k and 64k+1 are free for a 4-bit
//!   group minimum.
//!
//! The full h11 key (computed after conjugating the cube by the symmetry
//! that maps its corners to the class representative) is
//! `X = ((class * 70 + msplit) << 20) | (eo11 << 9) | eslot`. The table
//! `hk` (k = 0..=11) keeps only the top `k` eo bits: its index is
//! `((X >> (11 - k)) & !0x1FF) | (X & 0x1FF)` (cube48opt's layout).
//!
//! The hot path (`cocsep_x3`, `index_sym`) reads the cube's bytes with
//! `pext` (gather the bits under a mask) and `movemask` (gather the top bit
//! of each byte), then looks the parts up in small tables.

use crate::simd::Cube;
use std::arch::x86_64::{
    _mm256_add_epi8, _mm256_movemask_epi8, _mm256_shuffle_epi8, _mm256_slli_epi16,
    _mm256_xor_si256, _pdep_u32, _pext_u32, _pext_u64,
};

/// Corner orientations (the 8th follows from the others).
pub const N_CO: usize = 2187;
/// Corner tetrad separations.
pub const N_CSEP: usize = 70;
/// cocsep coordinates: 153090.
pub const N_COCSEP: usize = N_CO * N_CSEP;
/// Symmetry classes of cocsep coordinates.
pub const N_CLASSES: usize = 3393;
/// msplit values.
pub const N_MSPLIT: usize = 70;
/// Edge orientations (11 bits).
pub const N_EO: usize = 2048;
/// eslot slots (495 used).
pub const N_SLOTS: usize = 512;

/// Binomial coefficient `n` choose `k` (for small `n`: no overflow checks).
const fn binomial(n: u32, k: u32) -> u32 {
    if k > n {
        return 0;
    }
    let mut result = 1;
    let mut i = 0;
    while i < k {
        // (Exact: a product of i + 1 consecutive numbers is divisible by (i + 1)!.)
        result = result * (n - i) / (i + 1);
        i += 1;
    }
    result
}

/// Colexicographic rank of a bit mask among the masks with as many set
/// bits: the sum of `binomial(p, j)` over its `j`-th set bit `p`.
const fn colex_rank(mask: u32) -> u32 {
    let mut rank = 0;
    let mut ones = 0;
    let mut bit = 0;
    while bit < 32 {
        if mask >> bit & 1 == 1 {
            ones += 1;
            rank += binomial(bit, ones);
        }
        bit += 1;
    }
    rank
}

/// E-slice mask -> slot, and slot -> mask: masks in descending order fill
/// slots 2..510, skipping slots 64k and 64k+1 (reserved for the group
/// minimum).
const fn eslot_tables() -> ([u16; 4096], [u16; 512]) {
    let mut slot_of = [0xFFFFu16; 4096];
    let mut mask_of = [0u16; 512];
    let mut slot: u16 = 2;
    let mut mask: u16 = 4096;
    while mask > 0 {
        mask -= 1;
        if mask.count_ones() == 4 {
            slot_of[mask as usize] = slot;
            mask_of[slot as usize] = mask;
            if (slot + 1).is_multiple_of(64) {
                slot += 3;
            } else {
                slot += 1;
            }
        }
    }
    (slot_of, mask_of)
}
const ESLOT_TABLES: ([u16; 4096], [u16; 512]) = eslot_tables();

/// Rank (0..70) of each 8-bit mask with 4 set bits; 0xFF for other masks.
#[expect(
    clippy::cast_possible_truncation,
    reason = "ranks of 8-bit masks are below 70 (no `try_from` in const fns)"
)]
const fn rank8_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut mask: u8 = 0;
    loop {
        table[mask as usize] = if mask.count_ones() == 4 {
            colex_rank(mask as u32) as u8
        } else {
            0xFF
        };
        if mask == u8::MAX {
            return table;
        }
        mask += 1;
    }
}

/// The mask of each rank of `rank8_table`.
const fn unrank8_table() -> [u8; 70] {
    let mut table = [0u8; 70];
    let mut mask: u8 = 0;
    loop {
        if mask.count_ones() == 4 {
            table[colex_rank(mask as u32) as usize] = mask;
        }
        if mask == u8::MAX {
            return table;
        }
        mask += 1;
    }
}

/// Base-3 value of `n` corner orientations given as 2-bit fields (as
/// gathered by `pext`), times `scale`.
const fn co_rank_table(scale: u16, n: usize) -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut fields: u16 = 0;
    while fields < (1 << (2 * n)) {
        let mut rank = 0;
        let mut power = 1;
        let mut i = 0;
        while i < n {
            rank += ((fields >> (2 * i)) & 3) * power;
            power *= 3;
            i += 1;
        }
        table[fields as usize] = rank * scale;
        fields += 1;
    }
    table
}

/// Compile-time copy of `ESLOT` (consts cannot read statics).
pub(crate) const ESLOT_CONST: [u16; 4096] = ESLOT_TABLES.0;
/// Compile-time copy of `RANK8`.
pub(crate) const RANK8_CONST: [u8; 256] = rank8_table();
/// eslot of each E-slice position mask (12 bits).
pub static ESLOT: [u16; 4096] = ESLOT_CONST;
/// E-slice position mask of each eslot.
pub static SLOT_EMASK: [u16; 512] = ESLOT_TABLES.1;
/// Rank of each 8-bit mask with 4 set bits (tetrads, msplit).
pub static RANK8: [u8; 256] = RANK8_CONST;
/// Inverse of `RANK8`.
pub static UNRANK8: [u8; 70] = unrank8_table();

/// `co_rank_table` pre-multiplied by `N_CSEP` (saves a multiply on the
/// cube -> cocsep data dependency chain; part of m1-micro's +2.2% at 48T).
#[expect(
    clippy::cast_possible_truncation,
    reason = "`N_CSEP` is 70 (no `try_from` in const fns)"
)]
const fn co_rank_table_x70(scale: u32, n: usize) -> [u32; 256] {
    let ranks = co_rank_table(1, n);
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = ranks[i] as u32 * scale * N_CSEP as u32;
        i += 1;
    }
    table
}
/// cocsep part of the orientations of corners 0..4.
pub(crate) const CO_LOW: [u32; 256] = co_rank_table_x70(1, 4);
/// cocsep part of the orientations of corners 4..7 (3^4 times higher).
pub(crate) const CO_HIGH: [u32; 64] = {
    let full = co_rank_table_x70(81, 3);
    let mut table = [0u32; 64];
    let mut i = 0;
    while i < 64 {
        table[i] = full[i];
        i += 1;
    }
    table
};

/// The corner bits of a cube: the orientations of corners 0..7 (2 bits
/// each, 14 bits) and the tetrad mask (8 bits), by two `pext`s.
#[inline(always)]
fn corner_fields(cube: Cube) -> (u64, u64) {
    let corners = cube.corners_u64();
    let (co_fields, tetrads): (u64, u64);
    unsafe {
        let tabs = &crate::simd::HOT_TABLES;
        // (Miri cannot run inline asm: the same pexts as intrinsics.)
        #[cfg(miri)]
        {
            co_fields = _pext_u64(corners, tabs.co_pext_mask);
            tetrads = _pext_u64(corners, tabs.tetrad_pext_mask);
        }
        // (The masks are memory operands: no 64-bit immediates to load;
        // +1.7% when added, part of m4-micro's +7.7% at 48T.)
        #[cfg(not(miri))]
        std::arch::asm!(
            "pext {co2}, {x}, qword ptr [{t} + {oc}]",
            "pext {sep}, {x}, qword ptr [{t} + {os}]",
            co2 = out(reg) co_fields,
            sep = lateout(reg) tetrads,
            x = in(reg) corners,
            t = in(reg) tabs,
            oc = const std::mem::offset_of!(crate::simd::HotTables, co_pext_mask),
            os = const std::mem::offset_of!(crate::simd::HotTables, tetrad_pext_mask),
            options(pure, readonly, nostack, preserves_flags)
        );
        // (A range hint, not an index here: the lookups of `cocsep` and
        // `cocsep_x3` are unchecked already.)
        std::hint::assert_unchecked(co_fields < 1 << 14 && tetrads < 1 << 8);
    }
    (co_fields, tetrads)
}

/// cocsep coordinate in 0..153090.
#[must_use]
#[inline(always)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "hot path: `usize` is 64 bits and the pext results are below 2^14"
)]
pub fn cocsep(cube: Cube) -> usize {
    let (co_fields, tetrads) = corner_fields(cube);
    unsafe {
        let tabs = &crate::simd::HOT_TABLES;
        (*tabs.co_low.get_unchecked((co_fields & 0xFF) as usize)
            + *tabs.co_high.get_unchecked((co_fields >> 8) as usize)
            + u32::from(*tabs.rank8.get_unchecked(tetrads as usize))) as usize
    }
}

/// `3 * cocsep(cube)` (the byte offset of its packed entry), from tables
/// scaled by 3 (no multiply on the index path; part of m6-micro's index
/// patch: +0.7% at 1T, -0.4% at 48T).
#[must_use]
#[inline(always)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "hot path: `usize` is 64 bits and the pext results are below 2^14"
)]
pub fn cocsep_x3(cube: Cube) -> usize {
    let (co_fields, tetrads) = corner_fields(cube);
    unsafe {
        let tabs = &crate::simd::HOT_TABLES;
        (*tabs.co_low_x3.get_unchecked((co_fields & 0xFF) as usize)
            + *tabs.co_high_x3.get_unchecked((co_fields >> 8) as usize)
            + u32::from(*tabs.tetrads_x3.get_unchecked(tetrads as usize))) as usize
    }
}

/// Edge masks: (E-slice positions, S-slice positions, flipped edges), 12
/// bits each.
#[must_use]
#[inline(always)]
pub fn edge_masks(cube: Cube) -> (u32, u32, u32) {
    // (Shifting a bit of each byte into the top bit lets `movemask` gather
    // it; edges are bytes 16..28.)
    let mask = |bits: i32| (bits.cast_unsigned() >> 16) & 0xFFF;
    unsafe {
        (
            mask(_mm256_movemask_epi8(_mm256_slli_epi16(cube.0, 4))),
            mask(_mm256_movemask_epi8(_mm256_slli_epi16(cube.0, 5))),
            mask(_mm256_movemask_epi8(_mm256_slli_epi16(cube.0, 3))),
        )
    }
}

/// Edge part of the h11 key of an (already transformed) cube:
/// `(msplit << 20) | (eo11 << 9) | eslot`.
#[must_use]
#[inline(always)]
pub fn edge_key(cube: Cube) -> u64 {
    let (e_slice, s_slice, flips) = edge_masks(cube);
    unsafe {
        // (msplit ranks the S-slice positions among the non-E positions.)
        let s_among_non_e = _pext_u32(s_slice, !e_slice & 0xFFF);
        let msplit = u64::from(RANK8[s_among_non_e as usize]);
        let eslot = u64::from(ESLOT[e_slice as usize]);
        (msplit << 20) | (u64::from(flips >> 1) << 9) | eslot
    }
}

/// Edge part of the h11 key of `cube` conjugated by symmetry `sym`; equals
/// `edge_key(cube.transform_edges(sym))`. Only the edge classes and
/// orientations of the conjugate are built: one shuffle classifies each
/// piece `p` as `T[p]` (E/S flags, eo of T), xor adds the cube's eo and
/// Tinv's (by source position: one three-way xor), and a second shuffle
/// applies the position permutation `Tinv`. (Instead of a full
/// conjugation: +5.5% at 48T when added, edge-conj.)
#[must_use]
#[inline(always)]
pub fn edge_key_sym(cube: Cube, sym: usize) -> u64 {
    let (msplit, eo11, eslot) = edge_parts_sym(cube, sym);
    (msplit << 20) | (eo11 << 9) | eslot
}

/// (msplit, eo11, eslot) of `cube` conjugated by `sym` (see `edge_key_sym`).
#[must_use]
#[inline(always)]
pub fn edge_parts_sym(cube: Cube, sym: usize) -> (u64, u64, u64) {
    let (msplit, eo_bits, eslot) = edge_parts_sym_raw(cube, sym);
    (msplit, u64::from(eo_bits >> 17), eslot)
}

/// `edge_parts_sym` with the eo part unshifted: the eo of edges 0..11 in
/// bits 16..28 (all else zero).
#[expect(
    clippy::cast_possible_truncation,
    reason = "x86-64: the u64 pext result indexes a 256-entry table"
)]
#[inline(always)]
fn edge_parts_sym_raw(cube: Cube, sym: usize) -> (u64, u32, u64) {
    unsafe {
        let tabs = &crate::simd::HOT_TABLES;
        let entry = tabs.sym_edges.get_unchecked(sym);
        let classify = crate::simd::load(&entry.classes);
        let positions = crate::simd::load(&entry.perm);
        // Tinv's eo (by source position) is folded into the xor that adds
        // the cube's eo: one vpternlog, and no xor after the position
        // shuffle (part of m7's µop stack: +1.5% at 1T averaged over code
        // offsets, even at 48T).
        let position_eo = crate::simd::load(&entry.perm_eo);
        let classes = _mm256_xor_si256(
            _mm256_xor_si256(_mm256_shuffle_epi8(classify, cube.0), cube.0),
            position_eo,
        );
        let conj = _mm256_shuffle_epi8(classes, positions);
        // Masks of edge positions 0..12, in bits 16..28 (all else zero).
        let e_mask = _mm256_movemask_epi8(conj).cast_unsigned();
        // asm: LLVM would turn movemask(conj + conj) into vptestmb + kmovd,
        // which is ~1.5 cycles longer on Zen 5 (this mask feeds the pext;
        // part of m1-micro's +2.2% at 48T).
        let doubled = _mm256_add_epi8(conj, conj);
        // (A 64-bit output: the 32-bit write zero-extends, and the 64-bit
        // pext below then needs no zero extension of `s_mask`.)
        let s_mask: u64;
        #[cfg(miri)]
        {
            s_mask = u64::from(_mm256_movemask_epi8(doubled).cast_unsigned());
        }
        #[cfg(not(miri))]
        std::arch::asm!("vpmovmskb {s:e}, {zz}", s = out(reg) s_mask, zz = in(ymm_reg) doubled,
            options(pure, nomem, nostack, preserves_flags));
        let eo_bits = _mm256_movemask_epi8(_mm256_slli_epi16(conj, 3)).cast_unsigned();
        // The non-E positions (`e_mask` is within bits 16..28): the mask of
        // the pext, and (shifted) the index of the complemented eslot
        // table, which saves a copy of `e_mask`.
        let non_e = e_mask ^ 0x0FFF_0000;
        // (64-bit pext: no zero extension of the index.)
        let msplit = u64::from(
            *tabs
                .rank8
                .get_unchecked(_pext_u64(s_mask, u64::from(non_e)) as usize),
        );
        let eslot = u64::from(*tabs.eslot_complement.get_unchecked((non_e >> 16) as usize));
        (msplit, eo_bits, eslot)
    }
}

/// `index_from_parts(class, edge_parts_sym(cube, sym), k)`, with the shifts
/// by `k` done as a multiply by `row_mul` = `1 << (k + 9)` and a pext by
/// `eo_mask` = the top `k` eo bits, `((1 << k) - 1) << (28 - k)` (memory
/// operands: no variable shift counts to keep in registers). (The index
/// straight from the parts: part of packed-cocsep's +2.8% at 48T; the
/// multiply and pext: part of m6-micro's index patch.)
#[must_use]
#[inline(always)]
pub fn index_sym(cube: Cube, sym: usize, class: usize, row_mul: u64, eo_mask: u32) -> u64 {
    let (msplit, eo_bits, eslot) = edge_parts_sym_raw(cube, sym);
    let row = class as u64 * N_MSPLIT as u64 + msplit;
    let eo = u64::from(unsafe { _pext_u32(eo_bits, eo_mask) });
    (row * row_mul) | (eo << 9) | eslot
}

/// hk index of class `class` and edge parts `(msplit, eo11, eslot)`; equals
/// `reduce_key((class * 70 + msplit) << 20 | eo11 << 9 | eslot, k)` but with
/// a shorter dependency chain from msplit.
#[must_use]
#[inline(always)]
pub fn index_from_parts(class: usize, parts: (u64, u64, u64), k: u32) -> u64 {
    let (msplit, eo11, eslot) = parts;
    let row = class as u64 * N_MSPLIT as u64 + msplit;
    (row << (k + 9)) | ((eo11 >> (11 - k)) << 9) | eslot
}

/// Full h11 key from a corner class and transformed edges.
#[must_use]
#[inline(always)]
pub fn h11_key(class: usize, edges: Cube) -> u64 {
    ((class as u64 * N_MSPLIT as u64) << 20) + edge_key(edges)
}

/// Reduce an h11 key to an hk index (drop the low `11 - k` eo bits).
#[must_use]
#[inline(always)]
pub fn reduce_key(key: u64, k: u32) -> u64 {
    ((key >> (11 - k)) & !0x1FF) | (key & 0x1FF)
}

/// Inverse of `reduce_key` for k in {0, 11}: (class, h11 edge key).
#[must_use]
#[inline(always)]
pub fn split_index(index: u64, k: u32) -> (usize, u64) {
    let eslot = index & 0x1FF;
    let rest = index >> 9;
    let eo = (rest & u64::MAX.unbounded_shr(64 - k)) << (11 - k);
    let row = rest >> k;
    let class = (row / N_MSPLIT as u64) as usize;
    let msplit = row % N_MSPLIT as u64;
    (class, (msplit << 20) | (eo << 9) | eslot)
}

/// A cube with the corners of `corners` and the edges of an h11 edge key:
/// some cube of that coordinate.
#[must_use]
pub fn cube_from_edge_key(corners: Cube, edge_key: u64) -> Cube {
    let eslot = (edge_key & 0x1FF) as usize;
    let eo11 = ((edge_key >> 9) & 0x7FF) as u32;
    let msplit = (edge_key >> 20) as usize;
    let e_slice = u32::from(SLOT_EMASK[eslot]);
    let s_slice = unsafe { _pdep_u32(u32::from(UNRANK8[msplit]), !e_slice & 0xFFF) };
    // (The 12th flip makes the number of flips even.)
    let eo12 = (eo11 << 1) | (eo11.count_ones() & 1);
    let mut bytes = corners.to_bytes();
    // Next piece of each slice: E 8..12, S 4..8, M 0..4.
    let (mut next_e, mut next_s, mut next_m) = (8u8, 4u8, 0u8);
    for position in 0..12 {
        let next = if e_slice >> position & 1 == 1 {
            &mut next_e
        } else if s_slice >> position & 1 == 1 {
            &mut next_s
        } else {
            &mut next_m
        };
        let piece = *next;
        *next += 1;
        let flipped = u8::from(eo12 >> position & 1 == 1);
        bytes[16 + position] = piece | (flipped << 4);
    }
    bytes[8..16].fill(0);
    bytes[28..32].fill(0);
    Cube::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::*;

    #[test]
    fn rank_tables() {
        assert_eq!(binomial(12, 4), 495);
        let mut seen = vec![false; 512];
        for m in 0..4096u32 {
            if m.count_ones() == 4 {
                let r = usize::from(ESLOT[m as usize]);
                assert!(r < 511 && r % 64 >= 2 && !seen[r]);
                seen[r] = true;
                assert_eq!(u32::from(SLOT_EMASK[r]), m);
            }
        }
        assert_eq!(seen.iter().filter(|&&x| x).count(), 495);
        for m in 0..256u32 {
            if m.count_ones() == 4 {
                assert_eq!(u32::from(UNRANK8[usize::from(RANK8[m as usize])]), m);
            }
        }
    }

    #[test]
    fn edge_key_sym_matches() {
        let mut rng = Rng(13);
        // (Fewer under Miri, which is slow.)
        for _ in 0..if cfg!(miri) { 20 } else { 500 } {
            let c = Cube::from_cubie(&CubieCube::random(&mut rng));
            for t in 0..48 {
                let ek = edge_key(c.transform_edges(t));
                assert_eq!(edge_key_sym(c, t), ek);
                for k in 0..=11 {
                    let x = ((1234 * N_MSPLIT as u64) << 20) + ek;
                    assert_eq!(
                        index_from_parts(1234, edge_parts_sym(c, t), k),
                        reduce_key(x, k)
                    );
                }
            }
        }
    }

    #[test]
    fn cocsep_x3_is_3_cocsep() {
        let mut rng = Rng(17);
        for _ in 0..2000 {
            let c = Cube::from_cubie(&CubieCube::random(&mut rng));
            assert_eq!(cocsep_x3(c), 3 * cocsep(c));
        }
    }

    #[test]
    fn edge_key_roundtrip() {
        let mut rng = Rng(11);
        for _ in 0..1000 {
            let c = Cube::from_cubie(&CubieCube::random(&mut rng));
            let ek = edge_key(c);
            let d = cube_from_edge_key(c, ek);
            assert_eq!(edge_key(d), ek);
            assert!(ek < (N_MSPLIT << 20) as u64);
            for k in [0, 11] {
                let x = h11_key(17, c);
                let (cl, ek2) = split_index(reduce_key(x, k), k);
                assert_eq!(cl, 17);
                if k == 11 {
                    assert_eq!(ek2, ek);
                } else {
                    assert_eq!(ek2, ek & !(0x7FF << 9));
                }
            }
        }
    }
}
