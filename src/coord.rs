//! H48 coordinates.
//!
//! * cocsep: corner orientation (3^7) x corner tetrad separation (C(8,4)=70),
//!   reduced by the 48 symmetries to 3393 classes.
//! * msplit: which of the 8 non-E-slice positions hold S-slice edges (70).
//! * eo: orientation of 11 edges (2^11).
//! * eslot: E-slice edge placement (495 values) packed into 512 slots, so
//!   that slots 64k and 64k+1 are free for a 4-bit group minimum.
//!
//! The full h11 key (computed after conjugating the cube by the symmetry
//! that maps its corners to the class representative) is
//! `X = ((class * 70 + msplit) << 20) | (eo11 << 9) | eslot` and the hk index
//! is `((X >> (11 - k)) & !0x1FF) | (X & 0x1FF)` (cube48opt's layout).

use crate::simd::{Cube, A64};
#[cfg(miri)]
use std::arch::x86_64::_pext_u64;
use std::arch::x86_64::{
    __m256i, _mm256_add_epi8, _mm256_load_si256, _mm256_movemask_epi8, _mm256_shuffle_epi8,
    _mm256_slli_epi16, _mm256_xor_si256, _pdep_u32, _pext_u32,
};

pub const N_CO: usize = 2187;
pub const N_CSEP: usize = 70;
pub const N_COCSEP: usize = N_CO * N_CSEP; // 153090
pub const N_CLASSES: usize = 3393;
pub const N_MSPLIT: usize = 70;
pub const N_EO: usize = 2048;
pub const N_SLOTS: usize = 512;

/// Binomial coefficient `n` choose `k` (for small `n`: no overflow checks).
const fn binom(n: u32, k: u32) -> u32 {
    if k > n {
        return 0;
    }
    let mut r = 1;
    let mut i = 0;
    while i < k {
        // (Exact: a product of i + 1 consecutive numbers is divisible by (i + 1)!.)
        r = r * (n - i) / (i + 1);
        i += 1;
    }
    r
}

/// Colex rank of a bit mask with `k` set bits.
const fn comb_rank(mask: u32) -> u32 {
    let mut r = 0;
    let mut j = 0;
    let mut p = 0;
    while p < 32 {
        if mask >> p & 1 == 1 {
            j += 1;
            r += binom(p, j);
        }
        p += 1;
    }
    r
}

/// E-slice mask -> slot: masks in descending order fill slots 2..510,
/// skipping slots 64k and 64k+1 (reserved for the group minimum).
const fn mk_eslot() -> ([u16; 4096], [u16; 512]) {
    let mut slot = [0xFFFFu16; 4096];
    let mut mask = [0u16; 512];
    let mut v: u16 = 2;
    let mut m: u16 = 4096;
    while m > 0 {
        m -= 1;
        if m.count_ones() == 4 {
            slot[m as usize] = v;
            mask[v as usize] = m;
            if (v + 1).is_multiple_of(64) {
                v += 3;
            } else {
                v += 1;
            }
        }
    }
    (slot, mask)
}
const ESLOT_TABLES: ([u16; 4096], [u16; 512]) = mk_eslot();

#[expect(
    clippy::cast_possible_truncation,
    reason = "ranks of 8-bit masks are below 70 (no `try_from` in const fns)"
)]
const fn mk_8_rank() -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut m: u8 = 0;
    loop {
        t[m as usize] = if m.count_ones() == 4 {
            comb_rank(m as u32) as u8
        } else {
            0xFF
        };
        if m == u8::MAX {
            return t;
        }
        m += 1;
    }
}

const fn mk_8_unrank() -> [u8; 70] {
    let mut t = [0u8; 70];
    let mut m: u8 = 0;
    loop {
        if m.count_ones() == 4 {
            t[comb_rank(m as u32) as usize] = m;
        }
        if m == u8::MAX {
            return t;
        }
        m += 1;
    }
}

/// co rank of 4 corners given 2-bit fields (8 bits), times `mul`.
const fn mk_co_part(mul: u16, n: usize) -> [u16; 256] {
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
        table[fields as usize] = rank * mul;
        fields += 1;
    }
    table
}

pub(crate) const ESLOT_T: [u16; 4096] = ESLOT_TABLES.0;
pub(crate) const RANK8_T: [u8; 256] = mk_8_rank();
pub static ESLOT: [u16; 4096] = ESLOT_T;
pub static SLOT_EMASK: [u16; 512] = ESLOT_TABLES.1;
pub static RANK8: [u8; 256] = RANK8_T;
pub static UNRANK8: [u8; 70] = mk_8_unrank();
/// co rank parts pre-multiplied by `N_CSEP` (saves a multiply on the
/// cube -> cocsep data dependency chain).
#[expect(
    clippy::cast_possible_truncation,
    reason = "`N_CSEP` is 70 (no `try_from` in const fns)"
)]
const fn mk_co_part70(mul: u32, n: usize) -> [u32; 256] {
    let t = mk_co_part(1, n);
    let mut r = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        r[i] = t[i] as u32 * mul * N_CSEP as u32;
        i += 1;
    }
    r
}
pub(crate) const CO_LO_T: [u32; 256] = mk_co_part70(1, 4);
pub(crate) const CO_HI_T: [u32; 64] = {
    let t = mk_co_part70(81, 3);
    let mut r = [0u32; 64];
    let mut i = 0;
    while i < 64 {
        r[i] = t[i];
        i += 1;
    }
    r
};

/// cocsep coordinate in 0..153090.
#[must_use]
#[inline(always)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "hot path: `usize` is 64 bits and the pext results are below 2^14"
)]
pub fn cocsep(c: Cube) -> usize {
    let x = c.corners_u64();
    unsafe {
        let t = &crate::simd::TABS;
        let (co2, sep): (u64, u64);
        // co2: orientations of corners 0..6 (14 bits); sep: tetrad mask.
        // (Miri cannot run inline asm: the same pexts as intrinsics.)
        #[cfg(miri)]
        {
            co2 = _pext_u64(x, t.pext_co);
            sep = _pext_u64(x, t.pext_sep);
        }
        #[cfg(not(miri))]
        std::arch::asm!(
            "pext {co2}, {x}, qword ptr [{t} + {oc}]",
            "pext {sep}, {x}, qword ptr [{t} + {os}]",
            co2 = out(reg) co2,
            sep = lateout(reg) sep,
            x = in(reg) x,
            t = in(reg) t,
            oc = const std::mem::offset_of!(crate::simd::Tabs, pext_co),
            os = const std::mem::offset_of!(crate::simd::Tabs, pext_sep),
            options(pure, readonly, nostack, preserves_flags)
        );
        std::hint::assert_unchecked(co2 < 1 << 14 && sep < 1 << 8);
        (*t.co_lo.get_unchecked((co2 & 0xFF) as usize)
            + *t.co_hi.get_unchecked((co2 >> 8) as usize)
            + u32::from(*t.rank8.get_unchecked(sep as usize))) as usize
    }
}

/// `3 * cocsep(c)` (the byte offset of its packed entry), from tables
/// scaled by 3 (no multiply on the index path).
#[must_use]
#[inline(always)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "hot path: `usize` is 64 bits and the pext results are below 2^14"
)]
pub fn cocsep3(c: Cube) -> usize {
    let x = c.corners_u64();
    unsafe {
        let t = &crate::simd::TABS;
        let (co2, sep): (u64, u64);
        #[cfg(miri)]
        {
            co2 = _pext_u64(x, t.pext_co);
            sep = _pext_u64(x, t.pext_sep);
        }
        #[cfg(not(miri))]
        std::arch::asm!(
            "pext {co2}, {x}, qword ptr [{t} + {oc}]",
            "pext {sep}, {x}, qword ptr [{t} + {os}]",
            co2 = out(reg) co2,
            sep = lateout(reg) sep,
            x = in(reg) x,
            t = in(reg) t,
            oc = const std::mem::offset_of!(crate::simd::Tabs, pext_co),
            os = const std::mem::offset_of!(crate::simd::Tabs, pext_sep),
            options(pure, readonly, nostack, preserves_flags)
        );
        std::hint::assert_unchecked(co2 < 1 << 14 && sep < 1 << 8);
        (*t.co_lo3.get_unchecked((co2 & 0xFF) as usize)
            + *t.co_hi3.get_unchecked((co2 >> 8) as usize)
            + u32::from(*t.sep3.get_unchecked(sep as usize))) as usize
    }
}

/// Edge masks: (E-slice positions, S-slice positions, eo), 12 bits each.
#[must_use]
#[inline(always)]
pub fn edge_masks(c: Cube) -> (u32, u32, u32) {
    unsafe {
        let e = _mm256_movemask_epi8(_mm256_slli_epi16(c.0, 4)).cast_unsigned() >> 16;
        let s = _mm256_movemask_epi8(_mm256_slli_epi16(c.0, 5)).cast_unsigned() >> 16;
        let o = _mm256_movemask_epi8(_mm256_slli_epi16(c.0, 3)).cast_unsigned() >> 16;
        (e & 0xFFF, s & 0xFFF, o & 0xFFF)
    }
}

/// Edge part of the h11 key of an (already transformed) cube:
/// `(msplit << 20) | (eo11 << 9) | eslot`.
#[must_use]
#[inline(always)]
pub fn edge_key(c: Cube) -> u64 {
    let (e, s, o) = edge_masks(c);
    unsafe {
        let srel = _pext_u32(s, !e & 0xFFF);
        let msplit = u64::from(RANK8[srel as usize]);
        let eslot = u64::from(ESLOT[e as usize]);
        (msplit << 20) | (u64::from(o >> 1) << 9) | eslot
    }
}

/// Edge part of the h11 key of `c` conjugated by symmetry `t`; equals
/// `edge_key(c.transform_edges(t))`. Only the edge classes and orientations
/// of the conjugate are built: one shuffle classifies each piece `p` as
/// `T[p]` (E/S flags, eo of T), xor adds c's eo, and a second shuffle
/// applies the position permutation `Tinv`, whose eo is added on the eo
/// path only.
#[must_use]
#[inline(always)]
pub fn edge_key_sym(c: Cube, t: usize) -> u64 {
    let (msplit, eo11, eslot) = edge_parts_sym(c, t);
    (msplit << 20) | (eo11 << 9) | eslot
}

/// (msplit, eo11, eslot) of `c` conjugated by symmetry `t` (see `edge_key_sym`).
#[must_use]
#[inline(always)]
pub fn edge_parts_sym(c: Cube, t: usize) -> (u64, u64, u64) {
    let (msplit, o, eslot) = edge_parts_sym_raw(c, t);
    (msplit, u64::from(o >> 17), eslot)
}

/// `edge_parts_sym` with the eo part unshifted: the eo of edges 0..11 in
/// bits 16..28 (all else zero).
#[inline(always)]
fn edge_parts_sym_raw(c: Cube, t: usize) -> (u64, u32, u64) {
    unsafe {
        let tabs = &crate::simd::TABS;
        let line: *const A64 = tabs.sym_edge.get_unchecked(t);
        let classify = _mm256_load_si256(line.cast());
        let positions = _mm256_load_si256(line.cast::<__m256i>().add(1));
        let classes = _mm256_xor_si256(_mm256_shuffle_epi8(classify, c.0), c.0);
        let conj = _mm256_shuffle_epi8(classes, positions);
        // Masks of edge positions 0..12, in bits 16..28 (all else zero).
        let e_mask = _mm256_movemask_epi8(conj).cast_unsigned();
        // asm: LLVM would turn movemask(conj + conj) into vptestmb + kmovd,
        // which is ~1.5 cycles longer on Zen 5 (this mask feeds the pext).
        let doubled = _mm256_add_epi8(conj, conj);
        let s_mask: u32;
        #[cfg(miri)]
        {
            s_mask = _mm256_movemask_epi8(doubled).cast_unsigned();
        }
        #[cfg(not(miri))]
        std::arch::asm!("vpmovmskb {s:e}, {zz}", s = out(reg) s_mask, zz = in(ymm_reg) doubled,
            options(pure, nomem, nostack, preserves_flags));
        let eo_flips = _mm256_xor_si256(conj, positions);
        let eo_bits = _mm256_movemask_epi8(_mm256_slli_epi16(eo_flips, 3)).cast_unsigned();
        // The non-E positions (`e_mask` is within bits 16..28): the mask of
        // the pext, and (shifted) the index of the complemented eslot
        // table, which saves a copy of `e_mask`.
        let non_e = e_mask ^ 0x0FFF_0000;
        let msplit = u64::from(*tabs.rank8.get_unchecked(_pext_u32(s_mask, non_e) as usize));
        let eslot = u64::from(*tabs.eslot_c.get_unchecked((non_e >> 16) as usize));
        (msplit, eo_bits, eslot)
    }
}

/// `index_from_parts(class, edge_parts_sym(c, t), k)`, with the shifts by
/// `k` done as a multiply by `row_mul` = `1 << (k + 9)` and a pext by
/// `eo_mask` = the top `k` eo bits, `((1 << k) - 1) << (28 - k)` (memory
/// operands: no variable shift counts to keep in registers).
#[must_use]
#[inline(always)]
pub fn index_sym(c: Cube, t: usize, class: usize, row_mul: u64, eo_mask: u32) -> u64 {
    let (msplit, o, eslot) = edge_parts_sym_raw(c, t);
    let row = class as u64 * N_MSPLIT as u64 + msplit;
    let eo = u64::from(unsafe { _pext_u32(o, eo_mask) });
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
pub fn h11_key(class: usize, e: Cube) -> u64 {
    ((class as u64 * N_MSPLIT as u64) << 20) + edge_key(e)
}

/// Reduce an h11 key to an hk index.
#[must_use]
#[inline(always)]
pub fn reduce_key(x: u64, k: u32) -> u64 {
    ((x >> (11 - k)) & !0x1FF) | (x & 0x1FF)
}

/// Inverse of `reduce_key` for k in {0, 11}: (class, h11 edge key).
#[must_use]
#[inline(always)]
pub fn split_index(idx: u64, k: u32) -> (usize, u64) {
    let eslot = idx & 0x1FF;
    let rest = idx >> 9;
    let eo = (rest & u64::MAX.unbounded_shr(64 - k)) << (11 - k);
    let cm = rest >> k;
    let class = (cm / N_MSPLIT as u64) as usize;
    let msplit = cm % N_MSPLIT as u64;
    (class, (msplit << 20) | (eo << 9) | eslot)
}

/// Build a cube from the corners of `corner_src` and an h11 edge key.
#[must_use]
pub fn cube_from_edge_key(corner_src: Cube, ek: u64) -> Cube {
    let eslot = (ek & 0x1FF) as usize;
    let eo11 = ((ek >> 9) & 0x7FF) as u32;
    let msplit = (ek >> 20) as usize;
    let e = u32::from(SLOT_EMASK[eslot]);
    let s = unsafe { _pdep_u32(u32::from(UNRANK8[msplit]), !e & 0xFFF) };
    let eo12 = (eo11 << 1) | (eo11.count_ones() & 1);
    let mut b = corner_src.to_bytes();
    let (mut ne, mut ns, mut nm) = (8u8, 4u8, 0u8);
    for p in 0..12 {
        let piece = if e >> p & 1 == 1 {
            ne += 1;
            ne - 1
        } else if s >> p & 1 == 1 {
            ns += 1;
            ns - 1
        } else {
            nm += 1;
            nm - 1
        };
        let flipped = u8::from(eo12 >> p & 1 == 1);
        b[16 + p] = piece | (flipped << 4);
    }
    b[8..16].fill(0);
    b[28..32].fill(0);
    let a = crate::simd::A32(b);
    Cube(unsafe { _mm256_load_si256(std::ptr::from_ref(&a).cast()) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::*;

    #[test]
    fn rank_tables() {
        assert_eq!(binom(12, 4), 495);
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
    fn cocsep3_is_3_cocsep() {
        let mut rng = Rng(17);
        for _ in 0..2000 {
            let c = Cube::from_cubie(&CubieCube::random(&mut rng));
            assert_eq!(cocsep3(c), 3 * cocsep(c));
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
