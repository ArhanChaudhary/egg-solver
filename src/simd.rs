//! Fast cube representation: one AVX2 register.
//!
//! Bytes 0..8 hold corners (`piece | co << 5`), bytes 16..28 hold edges
//! (`piece | eo << 4`); everything else is unspecified (zero for cubes
//! built from cubies, garbage after `compose`). Composition is a single
//! `vpshufb` plus orientation fix-up (method from vcube).

use crate::cubie::CubieCube;
use std::arch::x86_64::*;

#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct A32(pub [u8; 32]);

#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct A64(pub [u8; 64]);

include!(concat!(env!("OUT_DIR"), "/consts.rs"));

/// The small lookup tables of the move/index hot path in one static, so that
/// all their accesses share one base register (x86 has no RIP-relative
/// indexed addressing: separate statics cost an `lea` each per use).
#[repr(C, align(64))]
pub struct Tabs {
    pub mv: [A32; 18],
    pub mv_ori: [A32; 18],
    pub inv_mv: [A32; 18],
    pub sym_edge: [A64; 48],
    pub co_lo: [u32; 256],
    pub co_hi: [u32; 64],
    /// `co_lo`, `co_hi` and `rank8` (of tetrad masks) times 3: the byte
    /// offset of a packed cocsep entry (`coord::cocsep3`).
    pub co_lo3: [u32; 256],
    pub co_hi3: [u32; 64],
    pub sep3: [u8; 256],
    /// `coord::ESLOT` indexed by the complement (in 12 bits) of the mask.
    pub eslot_c: [u16; 4096],
    pub rank8: [u8; 256],
    /// `pext` masks of `coord::cocsep` (read from memory: no 64-bit
    /// immediates per call): orientations of corners 0..6, tetrads.
    pub pext_co: u64,
    pub pext_sep: u64,
    /// `lanes::HEUR_PAIR`.
    pub heur_pair: [f32; 256],
    /// `solver::ALLOWED_AFTER`.
    pub allowed_after: [u32; 6],
    /// `!(7 << (3 * f))`: all moves but those of face `f`.
    pub face_keep: [u32; 6],
}

pub static TABS: Tabs = Tabs {
    mv: MOVES,
    mv_ori: MOVES_ORI,
    inv_mv: INV_MOVES,
    sym_edge: SYM_EDGE,
    co_lo: crate::coord::CO_LO_T,
    co_hi: crate::coord::CO_HI_T,
    co_lo3: {
        let mut r = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            r[i] = 3 * crate::coord::CO_LO_T[i];
            i += 1;
        }
        r
    },
    co_hi3: {
        let mut r = [0u32; 64];
        let mut i = 0;
        while i < 64 {
            r[i] = 3 * crate::coord::CO_HI_T[i];
            i += 1;
        }
        r
    },
    sep3: {
        // (Only masks of 4 corners occur; the others keep the 0xFF marker.)
        let mut r = [0xFFu8; 256];
        let mut i = 0;
        while i < 256 {
            if crate::coord::RANK8_T[i] != 0xFF {
                r[i] = 3 * crate::coord::RANK8_T[i];
            }
            i += 1;
        }
        r
    },
    eslot_c: {
        let mut r = [0u16; 4096];
        let mut x = 0;
        while x < 4096 {
            r[x] = crate::coord::ESLOT_T[x ^ 0xFFF];
            x += 1;
        }
        r
    },
    rank8: crate::coord::RANK8_T,
    pext_co: 0x0060_6060_6060_6060,
    pext_sep: 0x0404_0404_0404_0404,
    heur_pair: crate::lanes::HEUR_PAIR,
    allowed_after: crate::solver::ALLOWED_AFTER_T,
    face_keep: {
        let mut r = [0u32; 6];
        let mut f = 0;
        while f < 6 {
            r[f] = !(7 << (3 * f));
            f += 1;
        }
        r
    },
};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Cube(pub __m256i);

#[inline(always)]
fn ld(a: &A32) -> __m256i {
    unsafe { _mm256_load_si256(a.0.as_ptr() as *const __m256i) }
}

const fn set64(a: i64, b: i64, c: i64, d: i64) -> A32 {
    let mut r = [0u8; 32];
    let v = [a, b, c, d];
    let mut i = 0;
    while i < 32 {
        r[i] = (v[i / 8] >> (8 * (i % 8))) as u8;
        i += 1;
    }
    A32(r)
}

static ORIENT: A32 = set64(0x6060606060606060, 0, 0x1010101010101010, 0x10101010);
static CARRY: A32 = set64(0x6060606060606060, 0, 0x2020202020202020, 0x20202020);
static USED: A32 = set64(-1, 0, -1, 0xFFFFFFFF);
static CO2: A32 = set64(0x6060606060606060, 0, 0, 0);
static SOLVED: A32 = set64(0x0706050403020100, 0, 0x0706050403020100, 0x0B0A0908);

impl Cube {
    #[inline(always)]
    pub fn solved() -> Cube {
        Cube(ld(&SOLVED))
    }

    pub fn from_cubie(c: &CubieCube) -> Cube {
        let mut b = A32([0; 32]);
        for i in 0..8 {
            b.0[i] = c.cp[i] | (c.co[i] << 5);
        }
        for i in 0..12 {
            b.0[16 + i] = c.ep[i] | (c.eo[i] << 4);
        }
        Cube(ld(&b))
    }

    pub fn to_bytes(self) -> [u8; 32] {
        let mut b = A32([0; 32]);
        unsafe {
            let x = _mm256_and_si256(self.0, ld(&USED));
            _mm256_store_si256(b.0.as_mut_ptr() as *mut __m256i, x)
        };
        b.0
    }

    pub fn to_cubie(self) -> CubieCube {
        let b = self.to_bytes();
        let mut c = CubieCube::SOLVED;
        for i in 0..8 {
            c.cp[i] = b[i] & 7;
            c.co[i] = b[i] >> 5;
        }
        for i in 0..12 {
            c.ep[i] = b[16 + i] & 15;
            c.eo[i] = (b[16 + i] >> 4) & 1;
        }
        c
    }

    /// "self then other".
    #[inline(always)]
    pub fn compose(self, o: Cube) -> Cube {
        unsafe {
            let ss = _mm256_shuffle_epi8(self.0, o.0);
            let so = _mm256_and_si256(o.0, ld(&ORIENT));
            let ss = _mm256_add_epi8(ss, so);
            let su = _mm256_sub_epi8(ss, ld(&CARRY));
            // Bytes outside USED are left as garbage (they never feed back
            // into used bytes); `equals` and `to_bytes` mask them.
            Cube(_mm256_min_epu8(ss, su))
        }
    }

    #[inline(always)]
    pub fn apply_move(self, m: usize) -> Cube {
        // `compose` with the move's orientation term precomputed.
        unsafe {
            let ss = _mm256_shuffle_epi8(self.0, ld(&TABS.mv[m]));
            let ss = _mm256_add_epi8(ss, ld(&TABS.mv_ori[m]));
            let su = _mm256_sub_epi8(ss, ld(&CARRY));
            Cube(_mm256_min_epu8(ss, su))
        }
    }

    /// Given the inverse `i` of a cube `c`, return the inverse of `c * m`.
    #[inline(always)]
    pub fn premove_inv(self, m: usize) -> Cube {
        Cube(ld(&TABS.inv_mv[m])).compose(self)
    }

    #[inline(always)]
    pub fn invert_co(self) -> Cube {
        unsafe {
            let co = _mm256_and_si256(self.0, ld(&CO2));
            let shl = _mm256_slli_epi32(co, 1);
            let shr = _mm256_srli_epi32(co, 1);
            let newco = _mm256_and_si256(_mm256_or_si256(shl, shr), ld(&CO2));
            let clean = _mm256_xor_si256(self.0, co);
            Cube(_mm256_or_si256(clean, newco))
        }
    }

    /// Inverse cube (vcube's addition chain for c^27719).
    #[inline(always)]
    pub fn inverse(self) -> Cube {
        unsafe {
            let c = self.0;
            let sh = |a, b| _mm256_shuffle_epi8(a, b);
            let v3 = sh(sh(c, c), c);
            let mut vi = sh(v3, v3);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, v3);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, c);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, vi);
            vi = sh(vi, v3);
            vi = sh(vi, vi);
            vi = sh(vi, c);
            let orient = ld(&ORIENT);
            let vo = _mm256_and_si256(c, orient);
            let vo = sh(vo, vi);
            let vp = _mm256_andnot_si256(orient, vi);
            let r = _mm256_and_si256(_mm256_or_si256(vp, vo), ld(&USED));
            Cube(r).invert_co()
        }
    }

    #[inline(always)]
    pub fn equals(self, o: Cube) -> bool {
        unsafe {
            let x = _mm256_xor_si256(self.0, o.0);
            _mm256_testz_si256(x, ld(&USED)) != 0
        }
    }

    #[inline(always)]
    pub fn is_solved(self) -> bool {
        self.equals(Cube::solved())
    }

    /// Conjugate by symmetry `t`.
    #[inline(always)]
    pub fn transform(self, t: usize) -> Cube {
        let r = Cube(ld(&SYM_T[t]))
            .compose(self)
            .compose(Cube(ld(&SYM_TINV[t])));
        if SYM_MIRROR[t] {
            r.invert_co()
        } else {
            r
        }
    }

    /// Conjugate by symmetry `t`, only edges are guaranteed correct.
    #[inline(always)]
    pub fn transform_edges(self, t: usize) -> Cube {
        Cube(ld(&SYM_T[t]))
            .compose(self)
            .compose(Cube(ld(&SYM_TINV[t])))
    }

    /// `if m { a } else { b }`, as a byte blend (no branch).
    #[inline(always)]
    pub fn select(m: bool, a: Cube, b: Cube) -> Cube {
        unsafe { Cube(_mm256_blendv_epi8(b.0, a.0, _mm256_set1_epi8(-(m as i8)))) }
    }

    /// Low 64 bits: the 8 corner bytes.
    #[inline(always)]
    pub fn corners_u64(self) -> u64 {
        unsafe { _mm_cvtsi128_si64(_mm256_castsi256_si128(self.0)) as u64 }
    }
}

impl std::fmt::Debug for Cube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_cubie())
    }
}

impl PartialEq for Cube {
    fn eq(&self, o: &Cube) -> bool {
        Cube::equals(*self, *o)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::*;

    #[test]
    fn simd_matches_cubie() {
        let mut rng = Rng(5);
        let syms = symmetries();
        // (Fewer under Miri, which is slow.)
        for _ in 0..if cfg!(miri) { 20 } else { 200 } {
            let a = CubieCube::random(&mut rng);
            let b = CubieCube::random(&mut rng);
            let (sa, sb) = (Cube::from_cubie(&a), Cube::from_cubie(&b));
            assert_eq!(sa.to_cubie(), a);
            assert_eq!(sa.compose(sb).to_cubie(), a.mul(&b));
            assert_eq!(sa.inverse().to_cubie(), a.inverse());
            for m in 0..18 {
                assert_eq!(sa.apply_move(m).to_cubie(), a.apply_move(m));
                assert_eq!(
                    sa.inverse().premove_inv(m).to_cubie(),
                    a.apply_move(m).inverse()
                );
            }
            for t in 0..48 {
                assert_eq!(sa.transform(t).to_cubie(), conjugate(&a, &syms[t]));
            }
        }
        assert!(Cube::from_cubie(&CubieCube::SOLVED).is_solved());
    }
}
