//! 16-byte vectors on aarch64 NEON (Apple Silicon and other `ARMv8` cores).
//! Same semantics as `v16_scalar` (the reference; the tests run on both).

use std::arch::aarch64::{
    uint8x16_t, vaddq_u8, vaddv_u8, vandq_u8, vbicq_u8, vbslq_u8, vdupq_n_u8, veorq_u8,
    vget_high_u8, vget_low_u8, vgetq_lane_u64, vld1q_u8, vmaxvq_u8, vminq_u8, vorrq_u8, vpaddq_u8,
    vqtbl1q_u8, vreinterpretq_u64_u8, vshlq_n_u8, vshrq_n_u8, vst1q_u8, vsubq_u8, vtstq_u8,
};

/// 16 bytes in a NEON register.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct V16(pub uint8x16_t);

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

// SAFETY (every `unsafe` below): NEON is a target feature of every aarch64
// target this backend is built for (`backend = "neon"` requires it), and the
// loads and stores are of 16-byte arrays.
impl V16 {
    #[inline(always)]
    #[must_use]
    pub fn load(bytes: &[u8; 16]) -> V16 {
        V16(unsafe { vld1q_u8(bytes.as_ptr()) })
    }

    #[inline(always)]
    #[must_use]
    pub fn to_array(self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        unsafe { vst1q_u8(bytes.as_mut_ptr(), self.0) };
        bytes
    }

    #[inline(always)]
    #[must_use]
    pub fn splat(byte: u8) -> V16 {
        V16(unsafe { vdupq_n_u8(byte) })
    }

    /// Table lookup: byte i is `self[indices[i]]`, or 0 if `indices[i] >=
    /// 16` (`tbl`).
    #[inline(always)]
    #[must_use]
    pub fn lookup(self, indices: V16) -> V16 {
        V16(unsafe { vqtbl1q_u8(self.0, indices.0) })
    }

    #[inline(always)]
    #[must_use]
    pub fn wrapping_add(self, other: V16) -> V16 {
        V16(unsafe { vaddq_u8(self.0, other.0) })
    }

    #[inline(always)]
    #[must_use]
    pub fn wrapping_sub(self, other: V16) -> V16 {
        V16(unsafe { vsubq_u8(self.0, other.0) })
    }

    #[inline(always)]
    #[must_use]
    pub fn min(self, other: V16) -> V16 {
        V16(unsafe { vminq_u8(self.0, other.0) })
    }

    #[inline(always)]
    #[must_use]
    pub fn and(self, other: V16) -> V16 {
        V16(unsafe { vandq_u8(self.0, other.0) })
    }

    #[inline(always)]
    #[must_use]
    pub fn or(self, other: V16) -> V16 {
        V16(unsafe { vorrq_u8(self.0, other.0) })
    }

    #[inline(always)]
    #[must_use]
    pub fn xor(self, other: V16) -> V16 {
        V16(unsafe { veorq_u8(self.0, other.0) })
    }

    /// `self & !other` (`bic`).
    #[inline(always)]
    #[must_use]
    pub fn and_not(self, other: V16) -> V16 {
        V16(unsafe { vbicq_u8(self.0, other.0) })
    }

    /// Each byte shifted left by one bit.
    #[inline(always)]
    #[must_use]
    pub fn shl1(self) -> V16 {
        V16(unsafe { vshlq_n_u8::<1>(self.0) })
    }

    /// Each byte shifted right by one bit.
    #[inline(always)]
    #[must_use]
    pub fn shr1(self) -> V16 {
        V16(unsafe { vshrq_n_u8::<1>(self.0) })
    }

    /// `if choose_a { a } else { b }` (`bsl`, no branch).
    #[inline(always)]
    #[must_use]
    pub fn select(choose_a: bool, a: V16, b: V16) -> V16 {
        let mask = 0u8.wrapping_sub(u8::from(choose_a));
        V16(unsafe { vbslq_u8(vdupq_n_u8(mask), a.0, b.0) })
    }

    /// Bytes of `a` where `mask` (0xFF or 0) is set, else of `b` (`bsl`).
    #[inline(always)]
    #[must_use]
    pub fn select_bytes(mask: u8, a: V16, b: V16) -> V16 {
        V16(unsafe { vbslq_u8(vdupq_n_u8(mask), a.0, b.0) })
    }

    #[inline(always)]
    #[must_use]
    pub fn is_zero(self) -> bool {
        unsafe { vmaxvq_u8(self.0) == 0 }
    }

    /// Bytes 0..8 as a little-endian `u64`.
    #[inline(always)]
    #[must_use]
    pub fn low_u64(self) -> u64 {
        unsafe { vgetq_lane_u64::<0>(vreinterpretq_u64_u8(self.0)) }
    }

    /// The weight of each byte's bit in a 16-bit mask (two 8-bit halves).
    #[inline(always)]
    fn bit_weights() -> uint8x16_t {
        const WEIGHTS: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128];
        unsafe { vld1q_u8(WEIGHTS.as_ptr()) }
    }

    /// Bit `BIT` of bytes 0..16, as a 16-bit mask (x86 `pmovmskb` of a
    /// shift): each byte's flag weighted by its bit, summed per half.
    #[inline(always)]
    #[must_use]
    pub fn bit_mask<const BIT: u32>(self) -> u32 {
        unsafe {
            let flags = vtstq_u8(self.0, vdupq_n_u8(1 << BIT));
            let weighted = vandq_u8(flags, Self::bit_weights());
            u32::from(vaddv_u8(vget_low_u8(weighted)))
                | u32::from(vaddv_u8(vget_high_u8(weighted))) << 8
        }
    }

    /// `(bit 7 of self, bit 6 of self, bit 4 of other)` of each byte, as
    /// three 16-bit masks: three flag vectors weighted by bit, reduced
    /// together by four pairwise adds (cheaper than three separate
    /// reductions).
    #[inline(always)]
    #[must_use]
    pub fn bit_masks_764(self, other: V16) -> (u32, u32, u32) {
        unsafe {
            let weights = Self::bit_weights();
            let bit7 = vandq_u8(vtstq_u8(self.0, vdupq_n_u8(0x80)), weights);
            let bit6 = vandq_u8(vtstq_u8(self.0, vdupq_n_u8(0x40)), weights);
            let bit4 = vandq_u8(vtstq_u8(other.0, vdupq_n_u8(0x10)), weights);
            let sums = vpaddq_u8(vpaddq_u8(bit7, bit6), vpaddq_u8(bit4, bit4));
            let sums = vpaddq_u8(sums, sums);
            // Bytes: bit 7's low/high half, bit 6's, bit 4's.
            let packed = vgetq_lane_u64::<0>(vreinterpretq_u64_u8(sums));
            #[expect(clippy::cast_possible_truncation, reason = "16-bit fields")]
            let masks = (
                u32::from(packed as u16),
                u32::from((packed >> 16) as u16),
                u32::from((packed >> 32) as u16),
            );
            masks
        }
    }
}
