//! The parts of `coord.rs` that the x86 backend computes with BMI2 (`pext`,
//! `pdep`) and `pmovmskb`, for the portable backend: same results bit for
//! bit (tables are shared between backends). Constant-mask `pext`s are
//! multiply tricks, the variable one is two 6-bit table lookups, and
//! movemasks are `V16::bit_mask`s.

use super::{pieces, Cube, HOT_TABLES, V16};

/// `pext(x, mask)` (gather the bits of `x` under `mask`) for 12-bit values:
/// two 6-bit lookups.
#[inline(always)]
#[must_use]
pub fn pext12(x: u32, mask: u32) -> u32 {
    let tabs = &HOT_TABLES;
    let (mask_low, mask_high) = ((mask & 63) as usize, (mask >> 6 & 63) as usize);
    // SAFETY: all indices are masked to 6 bits.
    unsafe {
        let low = tabs
            .pext6
            .get_unchecked(mask_low)
            .get_unchecked((x & 63) as usize);
        let high = tabs
            .pext6
            .get_unchecked(mask_high)
            .get_unchecked((x >> 6 & 63) as usize);
        u32::from(*low) | u32::from(*high) << tabs.popcnt6.get_unchecked(mask_low)
    }
}

/// `pext(x, mask)` for a contiguous `mask` (the eo bits of `coord::index_sym`):
/// a mask and a shift.
#[inline(always)]
#[must_use]
pub fn pext_contiguous(x: u32, mask: u32) -> u32 {
    // (`mask` is 0 for h0: the shift is then by 32, done in 64 bits.)
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the bits under a 32-bit mask"
    )]
    let bits = (u64::from(x & mask) >> mask.trailing_zeros()) as u32;
    bits
}

/// `pdep(x, mask)` (deposit the low bits of `x` at the set bits of `mask`;
/// generation only).
#[must_use]
pub fn pdep12(x: u32, mask: u32) -> u32 {
    let (mut out, mut rest, mut i) = (0, mask, 0);
    while rest != 0 {
        let bit = rest.isolate_lowest_one();
        if x >> i & 1 == 1 {
            out |= bit;
        }
        rest ^= bit;
        i += 1;
    }
    out
}

/// The corner bits of a cube, as x86's two `pext`s give them: the
/// orientations of corners 0..7 (2 bits each, 14 bits) and the tetrad mask
/// (8 bits).
#[inline(always)]
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "fields of the low bytes")]
pub fn corner_fields(cube: Cube) -> (u64, u64) {
    let corners = cube.corners_u64();
    // Twists at bits 0, 8, 16, 24 (corners 0..4): the multiply puts field j
    // at bit 8j + 6i for each i, and i = 3 - j lands at 18 + 2j; no two
    // terms overlap (so no carries).
    let low4 = (corners >> 5) as u32 & 0x0303_0303;
    let co_low = (low4.wrapping_mul(0x0004_1041) >> 18) & 0xFF;
    // Corners 4..7: the same with three fields (i = 2 - j, at 12 + 2j).
    let high3 = (corners >> 37) as u32 & 0x0003_0303;
    let co_high = (high3.wrapping_mul(0x1041) >> 12) & 0x3F;
    // Tetrad bit (bit 2) of each byte: byte i's bit lands at 56 + i.
    let tetrads =
        ((corners >> 2) & 0x0101_0101_0101_0101).wrapping_mul(0x0102_0408_1020_4080) >> 56;
    (u64::from(co_low | co_high << 8), tetrads)
}

/// Edge masks: (E-slice positions, S-slice positions, flipped edges), 12
/// bits each.
#[inline(always)]
#[must_use]
pub fn edge_masks(cube: Cube) -> (u32, u32, u32) {
    (
        cube.edges.bit_mask::<3>() & 0xFFF,
        cube.edges.bit_mask::<2>() & 0xFFF,
        cube.edges.bit_mask::<4>() & 0xFFF,
    )
}

/// x86 `coord::edge_parts_sym_raw`: (msplit, eo of edges 0..12 in bits
/// 16..28, eslot) of `cube` conjugated by symmetry `sym`. One lookup
/// classifies each piece (E/S flags, eo of T), xor adds the cube's eo and
/// Tinv's (by source position), a second lookup applies the position
/// permutation Tinv.
#[inline(always)]
#[must_use]
pub fn edge_parts_sym_raw(cube: Cube, sym: usize) -> (u64, u32, u64) {
    let tabs = &HOT_TABLES;
    // SAFETY: symmetries are below 48.
    let line = unsafe { tabs.sym_edges.get_unchecked(sym) };
    let classes = V16::load(&line.classes.0)
        .lookup(pieces(cube.edges))
        .xor(cube.edges.xor(V16::load(&line.source_flips.0)));
    let conj = classes.lookup(V16::load(&line.positions.0));
    // (Bytes 12..16 of `conj` are 0: their position index is out of range.)
    let (e_slice, s_slice, flipped) = conj.bit_masks_764(conj);
    let non_e = !e_slice & 0xFFF;
    // SAFETY: a 12-bit pext with 8 mask bits is below 256, and `non_e` is
    // below 4096.
    let (msplit, eslot) = unsafe {
        (
            u64::from(*tabs.rank8.get_unchecked(pext12(s_slice, non_e) as usize)),
            u64::from(*tabs.eslot_complement.get_unchecked(non_e as usize)),
        )
    };
    (msplit, flipped << 16, eslot)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bit-by-bit `pext`.
    fn pext_reference(x: u64, mask: u64) -> u64 {
        let (mut out, mut n) = (0, 0);
        for bit in 0..64 {
            if mask >> bit & 1 == 1 {
                out |= (x >> bit & 1) << n;
                n += 1;
            }
        }
        out
    }

    #[test]
    fn pext_replacements_match() {
        for mask in 0..4096u32 {
            for x in [0u32, 0xFFF, 0x5A5, 0xA5A, 0x123, mask, !mask & 0xFFF] {
                let want = pext_reference(u64::from(x), u64::from(mask));
                assert_eq!(u64::from(pext12(x, mask)), want);
                let low = u32::MAX.unbounded_shr(32 - mask.count_ones());
                assert_eq!(pext12(pdep12(x, mask), mask), x & low);
            }
        }
        for k in 0..=11u32 {
            let mask = u32::MAX.unbounded_shr(32 - k) << (28 - k);
            for x in [0u32, 0x0FFF_0000, 0x0ABC_0000, 0x0555_0000] {
                let want = pext_reference(u64::from(x), u64::from(mask));
                assert_eq!(u64::from(pext_contiguous(x, mask)), want);
            }
        }
        let mut rng = crate::cubie::Rng(3);
        for _ in 0..100_000 {
            let corners = rng.next_u64();
            let mut bytes = [0u8; 32];
            bytes[..8].copy_from_slice(&corners.to_le_bytes());
            let (co_fields, tetrads) = corner_fields(Cube::from_bytes(bytes));
            assert_eq!(co_fields, pext_reference(corners, 0x0060_6060_6060_6060));
            assert_eq!(tetrads, pext_reference(corners, 0x0404_0404_0404_0404));
        }
    }
}

/// The portable backend gives the x86 backend's results (cube operations on
/// the used bytes, and every coordinate), along random move sequences (so
/// with the unspecified bytes as they come out of the operations).
#[cfg(all(test, backend = "x86"))]
mod cross_check {
    use super::*;
    use crate::cubie::{CubieCube, Rng};
    use crate::simd::Cube as X86Cube;

    /// `coord::cocsep` from the portable parts.
    fn cocsep(cube: Cube) -> usize {
        let (co_fields, tetrads) = corner_fields(cube);
        let tabs = &HOT_TABLES;
        let index = |x: u64| usize::try_from(x).unwrap();
        let sum = tabs.co_low[index(co_fields & 0xFF)]
            + tabs.co_high[index(co_fields >> 8)]
            + u32::from(tabs.rank8[index(tetrads)]);
        usize::try_from(sum).unwrap()
    }

    fn check_coordinates(p: Cube, x: X86Cube) {
        assert_eq!(p.to_bytes(), x.to_bytes());
        assert_eq!(cocsep(p), crate::coord::cocsep(x));
        assert_eq!(edge_masks(p), crate::coord::edge_masks(x));
        for sym in 0..48 {
            let (msplit, eo_bits, eslot) = edge_parts_sym_raw(p, sym);
            assert_eq!(
                (msplit, u64::from(eo_bits >> 17), eslot),
                crate::coord::edge_parts_sym(x, sym)
            );
            for k in [0, 1, 7, 10, 11] {
                let eo_mask = u32::MAX.unbounded_shr(32 - k) << (28 - k);
                let row_mul = 1 << (k + 9);
                let row = 1234 * crate::coord::N_MSPLIT as u64 + msplit;
                let index =
                    (row * row_mul) | u64::from(pext_contiguous(eo_bits, eo_mask)) << 9 | eslot;
                assert_eq!(
                    index,
                    crate::coord::index_sym(x, sym, 1234, row_mul, eo_mask)
                );
            }
        }
    }

    #[test]
    fn portable_matches_x86() {
        let mut rng = Rng(21);
        for _ in 0..40 {
            let a = CubieCube::random(&mut rng);
            let (mut p, mut x) = (Cube::from_cubie(&a), X86Cube::from_cubie(&a));
            let (mut pi, mut xi) = (p.inverse(), x.inverse());
            assert_eq!(pi.to_bytes(), xi.to_bytes());
            for _ in 0..30 {
                let mv = usize::try_from(rng.below(18)).unwrap();
                (p, x) = (p.apply_move(mv), x.apply_move(mv));
                (pi, xi) = (pi.premove_inverse(mv), xi.premove_inverse(mv));
                check_coordinates(p, x);
                check_coordinates(pi, xi);
                let sym = usize::try_from(rng.below(48)).unwrap();
                assert_eq!(p.transform(sym).to_bytes(), x.transform(sym).to_bytes());
                assert_eq!(p.compose(pi).to_bytes(), x.compose(xi).to_bytes());
                for mask in [0, !0] {
                    assert_eq!(
                        Cube::child_lookup(p, pi, mv, mask).to_bytes(),
                        X86Cube::child_lookup(x, xi, mv, mask).to_bytes()
                    );
                }
                assert_eq!(p.is_solved(), x.is_solved());
            }
        }
    }
}
