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

use crate::simd::Cube;
use std::arch::x86_64::*;

pub const N_CO: usize = 2187;
pub const N_CSEP: usize = 70;
pub const N_COCSEP: usize = N_CO * N_CSEP; // 153090
pub const N_CLASSES: usize = 3393;
pub const N_MSPLIT: usize = 70;
pub const N_EO: usize = 2048;
pub const N_SLOTS: usize = 512;

const fn binom(n: u32, k: u32) -> u32 {
    if k > n {
        return 0;
    }
    let mut r: u64 = 1;
    let mut i = 0;
    while i < k {
        r = r * (n - i) as u64 / (i + 1) as u64;
        i += 1;
    }
    r as u32
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

const fn popcnt(x: u32) -> u32 {
    x.count_ones()
}

/// E-slice mask -> slot: masks in descending order fill slots 2..510,
/// skipping slots 64k and 64k+1 (reserved for the group minimum).
const fn mk_eslot() -> ([u16; 4096], [u16; 512]) {
    let mut slot = [0xFFFFu16; 4096];
    let mut mask = [0u16; 512];
    let mut v = 2usize;
    let mut m = 4096;
    while m > 0 {
        m -= 1;
        if popcnt(m as u32) == 4 {
            slot[m] = v as u16;
            mask[v] = m as u16;
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

const fn mk_8_rank() -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut m = 0;
    while m < 256 {
        t[m] = if popcnt(m as u32) == 4 {
            comb_rank(m as u32) as u8
        } else {
            0xFF
        };
        m += 1;
    }
    t
}

const fn mk_8_unrank() -> [u8; 70] {
    let mut t = [0u8; 70];
    let mut m = 0;
    while m < 256 {
        if popcnt(m as u32) == 4 {
            t[comb_rank(m as u32) as usize] = m as u8;
        }
        m += 1;
    }
    t
}

/// co rank of 4 corners given 2-bit fields (8 bits), times `mul`.
const fn mk_co_part(mul: u16, n: usize) -> [u16; 256] {
    let mut t = [0u16; 256];
    let mut x = 0;
    while x < (1 << (2 * n)) {
        let mut r = 0u16;
        let mut p = 1u16;
        let mut i = 0;
        while i < n {
            r += ((x >> (2 * i)) & 3) as u16 * p;
            p *= 3;
            i += 1;
        }
        t[x] = r * mul;
        x += 1;
    }
    t
}

pub(crate) const ESLOT_T: [u16; 4096] = ESLOT_TABLES.0;
pub(crate) const RANK8_T: [u8; 256] = mk_8_rank();
pub static ESLOT: [u16; 4096] = ESLOT_T;
pub static SLOT_EMASK: [u16; 512] = ESLOT_TABLES.1;
pub static RANK8: [u8; 256] = RANK8_T;
pub static UNRANK8: [u8; 70] = mk_8_unrank();
/// co rank parts pre-multiplied by `N_CSEP` (saves a multiply on the
/// cube -> cocsep data dependency chain).
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
#[inline(always)]
pub fn cocsep(c: Cube) -> usize {
    let x = c.corners_u64();
    unsafe {
        let co2 = _pext_u64(x, 0x6060606060606060);
        let sep = _pext_u64(x, 0x0404040404040404);
        let t = &crate::simd::TABS;
        (*t.co_lo.get_unchecked((co2 & 0xFF) as usize)
            + *t.co_hi.get_unchecked(((co2 >> 8) & 0x3F) as usize)
            + *t.rank8.get_unchecked(sep as usize) as u32) as usize
    }
}

/// Edge masks: (E-slice positions, S-slice positions, eo), 12 bits each.
#[inline(always)]
pub fn edge_masks(c: Cube) -> (u32, u32, u32) {
    unsafe {
        let e = _mm256_movemask_epi8(_mm256_slli_epi16(c.0, 4)) as u32 >> 16;
        let s = _mm256_movemask_epi8(_mm256_slli_epi16(c.0, 5)) as u32 >> 16;
        let o = _mm256_movemask_epi8(_mm256_slli_epi16(c.0, 3)) as u32 >> 16;
        (e & 0xFFF, s & 0xFFF, o & 0xFFF)
    }
}

/// Edge part of the h11 key of an (already transformed) cube:
/// `(msplit << 20) | (eo11 << 9) | eslot`.
#[inline(always)]
pub fn edge_key(c: Cube) -> u64 {
    let (e, s, o) = edge_masks(c);
    unsafe {
        let srel = _pext_u32(s, !e & 0xFFF);
        let msplit = RANK8[srel as usize] as u64;
        let eslot = ESLOT[e as usize] as u64;
        (msplit << 20) | (((o >> 1) as u64) << 9) | eslot
    }
}

/// Edge part of the h11 key of `c` conjugated by symmetry `t`; equals
/// `edge_key(c.transform_edges(t))`. Only the edge classes and orientations
/// of the conjugate are built: one shuffle classifies each piece `p` as
/// `T[p]` (E/S flags, eo of T), xor adds c's eo, and a second shuffle
/// applies the position permutation `Tinv`, whose eo is added on the eo
/// path only.
#[inline(always)]
pub fn edge_key_sym(c: Cube, t: usize) -> u64 {
    let (msplit, eo11, eslot) = edge_parts_sym(c, t);
    (msplit << 20) | (eo11 << 9) | eslot
}

/// (msplit, eo11, eslot) of `c` conjugated by symmetry `t` (see `edge_key_sym`).
#[inline(always)]
pub fn edge_parts_sym(c: Cube, t: usize) -> (u64, u64, u64) {
    unsafe {
        let tabs = &crate::simd::TABS;
        let tab = tabs.sym_edge.get_unchecked(t).0.as_ptr();
        let q = _mm256_load_si256(tab as *const __m256i);
        let p = _mm256_load_si256(tab.add(32) as *const __m256i);
        let y = _mm256_xor_si256(_mm256_shuffle_epi8(q, c.0), c.0);
        let z = _mm256_shuffle_epi8(y, p);
        // Masks of edge positions 0..12, in bits 16..28 (all else zero).
        let e = _mm256_movemask_epi8(z) as u32;
        // asm: LLVM would turn movemask(z + z) into vptestmb + kmovd, which
        // is ~1.5 cycles longer on Zen 5 (this mask feeds the pext).
        let zz = _mm256_add_epi8(z, z);
        let s: u32;
        std::arch::asm!("vpmovmskb {s:e}, {zz}", s = out(reg) s, zz = in(ymm_reg) zz,
            options(pure, nomem, nostack, preserves_flags));
        let o = _mm256_movemask_epi8(_mm256_slli_epi16(_mm256_xor_si256(z, p), 3)) as u32;
        let msplit = *tabs
            .rank8
            .get_unchecked(_pext_u32(s, e ^ 0x0FFF_0000) as usize) as u64;
        let eslot = *tabs.eslot.get_unchecked((e >> 16) as usize) as u64;
        (msplit, (o >> 17) as u64, eslot)
    }
}

/// hk index of class `class` and edge parts `(msplit, eo11, eslot)`; equals
/// `reduce_key((class * 70 + msplit) << 20 | eo11 << 9 | eslot, k)` but with
/// a shorter dependency chain from msplit.
#[inline(always)]
pub fn index_from_parts(class: usize, parts: (u64, u64, u64), k: u32) -> u64 {
    let (msplit, eo11, eslot) = parts;
    let row = class as u64 * N_MSPLIT as u64 + msplit;
    (row << (k + 9)) | ((eo11 >> (11 - k)) << 9) | eslot
}

/// Full h11 key from a corner class and transformed edges.
#[inline(always)]
pub fn h11_key(class: usize, e: Cube) -> u64 {
    ((class as u64 * N_MSPLIT as u64) << 20) + edge_key(e)
}

/// Reduce an h11 key to an hk index.
#[inline(always)]
pub fn reduce_key(x: u64, k: u32) -> u64 {
    ((x >> (11 - k)) & !0x1FF) | (x & 0x1FF)
}

/// Inverse of `reduce_key` for k in {0, 11}: (class, h11 edge key).
#[inline(always)]
pub fn split_index(idx: u64, k: u32) -> (usize, u64) {
    let eslot = idx & 0x1FF;
    let rest = idx >> 9;
    let eo = (rest & ((1 << k) - 1)) << (11 - k);
    let cm = rest >> k;
    let class = (cm / N_MSPLIT as u64) as usize;
    let msplit = cm % N_MSPLIT as u64;
    (class, (msplit << 20) | (eo << 9) | eslot)
}

/// Build a cube from the corners of `corner_src` and an h11 edge key.
pub fn cube_from_edge_key(corner_src: Cube, ek: u64) -> Cube {
    let eslot = (ek & 0x1FF) as usize;
    let eo11 = ((ek >> 9) & 0x7FF) as u32;
    let msplit = (ek >> 20) as usize;
    let e = SLOT_EMASK[eslot] as u32;
    let s = unsafe { _pdep_u32(UNRANK8[msplit] as u32, !e & 0xFFF) };
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
        b[16 + p] = piece | (((eo12 >> p) & 1) << 4) as u8;
    }
    b[8..16].fill(0);
    b[28..32].fill(0);
    let a = crate::simd::A32(b);
    Cube(unsafe { _mm256_load_si256(a.0.as_ptr() as *const __m256i) })
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
                let r = ESLOT[m as usize] as usize;
                assert!(r < 511 && r % 64 >= 2 && !seen[r]);
                seen[r] = true;
                assert_eq!(SLOT_EMASK[r] as u32, m);
            }
        }
        assert_eq!(seen.iter().filter(|&&x| x).count(), 495);
        for m in 0..256u32 {
            if m.count_ones() == 4 {
                assert_eq!(UNRANK8[RANK8[m as usize] as usize] as u32, m);
            }
        }
    }

    #[test]
    fn edge_key_sym_matches() {
        let mut rng = Rng(13);
        for _ in 0..500 {
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
