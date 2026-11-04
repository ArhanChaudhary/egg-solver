//! 16-byte vectors on WebAssembly SIMD128 (`v128`). Same semantics as
//! `v16_scalar` (the reference; `tests::matches_scalar` compares them).
//! Engines compile these to SSE/AVX on x86 and NEON on arm64: `swizzle` is
//! `pshufb` after a saturating add (indices from 16 give 0, as `tbl`),
//! `bitmask` is `pmovmskb`.

use std::arch::wasm32::{
    i16x8_shl, u64x2_extract_lane, u8x16_add, u8x16_bitmask, u8x16_min, u8x16_shr, u8x16_splat,
    u8x16_sub, u8x16_swizzle, v128, v128_and, v128_andnot, v128_any_true, v128_bitselect,
    v128_load, v128_or, v128_store, v128_xor,
};

/// 16 bytes in a `v128`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct V16(pub v128);

impl PartialEq for V16 {
    fn eq(&self, other: &V16) -> bool {
        self.xor(*other).is_zero()
    }
}

impl std::fmt::Debug for V16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "V16({:?})", self.to_array())
    }
}

impl V16 {
    #[inline(always)]
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "v128_load needs no alignment")]
    pub fn load(bytes: &[u8; 16]) -> V16 {
        // SAFETY: a 16-byte array.
        V16(unsafe { v128_load(bytes.as_ptr().cast::<v128>()) })
    }

    #[inline(always)]
    #[must_use]
    #[expect(clippy::cast_ptr_alignment, reason = "v128_store needs no alignment")]
    pub fn to_array(self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        // SAFETY: a 16-byte array.
        unsafe { v128_store(bytes.as_mut_ptr().cast::<v128>(), self.0) };
        bytes
    }

    #[inline(always)]
    #[must_use]
    pub fn splat(byte: u8) -> V16 {
        V16(u8x16_splat(byte))
    }

    /// Table lookup: byte i is `self[indices[i]]`, or 0 if `indices[i] >=
    /// 16` (`swizzle`).
    #[inline(always)]
    #[must_use]
    pub fn lookup(self, indices: V16) -> V16 {
        V16(u8x16_swizzle(self.0, indices.0))
    }

    #[inline(always)]
    #[must_use]
    pub fn wrapping_add(self, other: V16) -> V16 {
        V16(u8x16_add(self.0, other.0))
    }

    #[inline(always)]
    #[must_use]
    pub fn wrapping_sub(self, other: V16) -> V16 {
        V16(u8x16_sub(self.0, other.0))
    }

    #[inline(always)]
    #[must_use]
    pub fn min(self, other: V16) -> V16 {
        V16(u8x16_min(self.0, other.0))
    }

    #[inline(always)]
    #[must_use]
    pub fn and(self, other: V16) -> V16 {
        V16(v128_and(self.0, other.0))
    }

    #[inline(always)]
    #[must_use]
    pub fn or(self, other: V16) -> V16 {
        V16(v128_or(self.0, other.0))
    }

    #[inline(always)]
    #[must_use]
    pub fn xor(self, other: V16) -> V16 {
        V16(v128_xor(self.0, other.0))
    }

    /// `self & !other` (`andnot`).
    #[inline(always)]
    #[must_use]
    pub fn and_not(self, other: V16) -> V16 {
        V16(v128_andnot(self.0, other.0))
    }

    /// Each byte shifted left by one bit: added to itself (x86 has no byte
    /// shift, so `i8x16.shl` costs a shift and a mask there; an add is one
    /// `paddb`).
    #[inline(always)]
    #[must_use]
    pub fn shl1(self) -> V16 {
        V16(u8x16_add(self.0, self.0))
    }

    /// Each byte shifted right by one bit.
    #[inline(always)]
    #[must_use]
    pub fn shr1(self) -> V16 {
        V16(u8x16_shr(self.0, 1))
    }

    /// `if choose_a { a } else { b }` (`bitselect`, no branch).
    #[inline(always)]
    #[must_use]
    pub fn select(choose_a: bool, a: V16, b: V16) -> V16 {
        let mask = 0u8.wrapping_sub(u8::from(choose_a));
        V16(v128_bitselect(a.0, b.0, u8x16_splat(mask)))
    }

    /// Bytes of `a` where `mask` (0xFF or 0) is set, else of `b`.
    #[inline(always)]
    #[must_use]
    pub fn select_bytes(mask: u8, a: V16, b: V16) -> V16 {
        V16(v128_bitselect(a.0, b.0, u8x16_splat(mask)))
    }

    #[inline(always)]
    #[must_use]
    pub fn is_zero(self) -> bool {
        !v128_any_true(self.0)
    }

    /// Bytes 0..8 as a little-endian `u64`.
    #[inline(always)]
    #[must_use]
    pub fn low_u64(self) -> u64 {
        u64x2_extract_lane::<0>(self.0)
    }

    /// Bit `BIT` of bytes 0..16, as a 16-bit mask (x86 `pmovmskb` of a
    /// shift): `bitmask` reads bit 7 of each byte. The shift is by 16-bit
    /// lanes (one `psllw`, no mask): bit 7 of each byte still comes from the
    /// same byte, the other bits are ignored.
    #[inline(always)]
    #[must_use]
    pub fn bit_mask<const BIT: u32>(self) -> u32 {
        let shifted = if BIT == 7 {
            self.0
        } else {
            i16x8_shl(self.0, 7 - BIT)
        };
        u32::from(u8x16_bitmask(shifted))
    }

    /// `(bit 7 of self, bit 6 of self, bit 4 of other)` of each byte, as
    /// three 16-bit masks (see `bit_mask`).
    #[inline(always)]
    #[must_use]
    pub fn bit_masks_764(self, other: V16) -> (u32, u32, u32) {
        (
            self.bit_mask::<7>(),
            self.bit_mask::<6>(),
            other.bit_mask::<4>(),
        )
    }
}
