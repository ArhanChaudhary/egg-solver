//! H48 coordinates.
//!
//! * cocsep: corner orientation (3^7) x corner tetrad separation (C(8,4)=70),
//!   reduced by the 48 symmetries to 3393 classes.
//! * esep: which edges are in which slice, C(12,4) * C(8,4) = 34650.
//! * eo: orientation of 11 edges (2^11).
//!
//! The full h11 index is `((class * 34650 + esep) << 11) | eo11`, computed
//! after conjugating the cube by the symmetry that maps its corners to the
//! class representative. The hk index is `h11 >> (11 - k)`.

use crate::simd::Cube;
use std::arch::x86_64::*;

pub const N_CO: usize = 2187;
pub const N_CSEP: usize = 70;
pub const N_COCSEP: usize = N_CO * N_CSEP; // 153090
pub const N_CLASSES: usize = 3393;
pub const N_ESEP: usize = 34650;
pub const N_EO: usize = 2048;

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

const fn mk_e_rank() -> [u16; 4096] {
    let mut t = [0u16; 4096];
    let mut m = 0;
    while m < 4096 {
        t[m] = if popcnt(m as u32) == 4 {
            comb_rank(m as u32) as u16
        } else {
            0xFFFF
        };
        m += 1;
    }
    t
}

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

const fn mk_e_unrank() -> [u16; 495] {
    let mut t = [0u16; 495];
    let mut m = 0;
    while m < 4096 {
        if popcnt(m as u32) == 4 {
            t[comb_rank(m as u32) as usize] = m as u16;
        }
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

pub static E_RANK: [u16; 4096] = mk_e_rank();
pub static RANK8: [u8; 256] = mk_8_rank();
pub static E_UNRANK: [u16; 495] = mk_e_unrank();
pub static UNRANK8: [u8; 70] = mk_8_unrank();
static CO_LO: [u16; 256] = mk_co_part(1, 4);
static CO_HI: [u16; 256] = mk_co_part(81, 3);

/// cocsep coordinate in 0..153090.
#[inline(always)]
pub fn cocsep(c: Cube) -> usize {
    let x = c.corners_u64();
    unsafe {
        let co2 = _pext_u64(x, 0x6060606060606060);
        let sep = _pext_u64(x, 0x0404040404040404);
        let co =
            CO_LO[(co2 & 0xFF) as usize] as usize + CO_HI[((co2 >> 8) & 0x3F) as usize] as usize;
        co * N_CSEP + RANK8[sep as usize] as usize
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

/// esep * 2048 + eo11, of an (already transformed) cube.
#[inline(always)]
pub fn edge_coord(c: Cube) -> u64 {
    let (e, s, o) = edge_masks(c);
    unsafe {
        let srel = _pext_u32(s, !e & 0xFFF);
        let esep = E_RANK[e as usize] as u64 * N_CSEP as u64 + RANK8[srel as usize] as u64;
        (esep << 11) | (o >> 1) as u64
    }
}

/// Edge bytes (lane 1) for each esep value: E positions get pieces 8..11,
/// S positions get 4..7, M positions get 0..3, all oriented.
pub fn esep_edges() -> &'static [[u8; 12]] {
    use std::sync::OnceLock;
    static T: OnceLock<Vec<[u8; 12]>> = OnceLock::new();
    T.get_or_init(|| {
        let mut v = vec![[0u8; 12]; N_ESEP];
        for (esep, out) in v.iter_mut().enumerate() {
            let e = E_UNRANK[esep / N_CSEP] as u32;
            let srel = UNRANK8[esep % N_CSEP] as u32;
            let s = unsafe { _pdep_u32(srel, !e & 0xFFF) };
            let (mut ne, mut ns, mut nm) = (8u8, 4u8, 0u8);
            for (p, o) in out.iter_mut().enumerate() {
                if e >> p & 1 == 1 {
                    *o = ne;
                    ne += 1;
                } else if s >> p & 1 == 1 {
                    *o = ns;
                    ns += 1;
                } else {
                    *o = nm;
                    nm += 1;
                }
            }
        }
        v
    })
}

/// Build a cube from corners of `corner_src` and the given edge coordinate.
pub fn cube_from_edge_coord(corner_src: Cube, ec: u64) -> Cube {
    let esep = (ec >> 11) as usize;
    let eo11 = (ec & 0x7FF) as u32;
    let eo12 = (eo11 << 1) | (eo11.count_ones() & 1);
    let mut b = corner_src.to_bytes();
    let edges = esep_edges()[esep];
    for p in 0..12 {
        b[16 + p] = edges[p] | (((eo12 >> p) & 1) << 4) as u8;
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
        let mut seen = vec![false; 495];
        for m in 0..4096u32 {
            if m.count_ones() == 4 {
                let r = E_RANK[m as usize] as usize;
                assert!(!seen[r]);
                seen[r] = true;
                assert_eq!(E_UNRANK[r] as u32, m);
            }
        }
        for m in 0..256u32 {
            if m.count_ones() == 4 {
                assert_eq!(UNRANK8[RANK8[m as usize] as usize] as u32, m);
            }
        }
    }

    #[test]
    fn edge_coord_roundtrip() {
        let mut rng = Rng(11);
        for _ in 0..1000 {
            let c = Cube::from_cubie(&CubieCube::random(&mut rng));
            let ec = edge_coord(c);
            let d = cube_from_edge_coord(c, ec);
            assert_eq!(edge_coord(d), ec);
            assert!(ec < (N_ESEP * N_EO) as u64);
        }
    }
}
