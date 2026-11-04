//! 16-byte vectors as plain arrays: the reference for the other `V16`s, and
//! the fallback where there is no SIMD backend (LLVM still vectorizes most
//! of these loops, e.g. with SSE2 on x86).

/// 16 bytes, operated on lane by lane.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C, align(16))]
pub struct V16(pub [u8; 16]);

impl V16 {
    #[inline(always)]
    #[must_use]
    pub fn load(bytes: &[u8; 16]) -> V16 {
        V16(*bytes)
    }

    #[inline(always)]
    #[must_use]
    pub fn to_array(self) -> [u8; 16] {
        self.0
    }

    #[inline(always)]
    #[must_use]
    pub fn splat(byte: u8) -> V16 {
        V16([byte; 16])
    }

    #[inline(always)]
    fn map2(self, other: V16, f: impl Fn(u8, u8) -> u8) -> V16 {
        V16(std::array::from_fn(|i| f(self.0[i], other.0[i])))
    }

    /// Table lookup: byte i is `self[indices[i]]`, or 0 if `indices[i] >= 16`
    /// (NEON `tbl`, wasm `swizzle`; x86 `pshufb` differs: it ignores bits
    /// 4..6 of the index).
    #[inline(always)]
    #[must_use]
    pub fn lookup(self, indices: V16) -> V16 {
        V16(indices
            .0
            .map(|j| self.0.get(usize::from(j)).copied().unwrap_or(0)))
    }

    #[inline(always)]
    #[must_use]
    pub fn wrapping_add(self, other: V16) -> V16 {
        self.map2(other, u8::wrapping_add)
    }

    #[inline(always)]
    #[must_use]
    pub fn wrapping_sub(self, other: V16) -> V16 {
        self.map2(other, u8::wrapping_sub)
    }

    #[inline(always)]
    #[must_use]
    pub fn min(self, other: V16) -> V16 {
        self.map2(other, std::cmp::Ord::min)
    }

    #[inline(always)]
    #[must_use]
    pub fn and(self, other: V16) -> V16 {
        self.map2(other, |a, b| a & b)
    }

    #[inline(always)]
    #[must_use]
    pub fn or(self, other: V16) -> V16 {
        self.map2(other, |a, b| a | b)
    }

    #[inline(always)]
    #[must_use]
    pub fn xor(self, other: V16) -> V16 {
        self.map2(other, |a, b| a ^ b)
    }

    /// `self & !o`.
    #[inline(always)]
    #[must_use]
    pub fn and_not(self, other: V16) -> V16 {
        self.map2(other, |a, b| a & !b)
    }

    /// Each byte shifted left by one bit.
    #[inline(always)]
    #[must_use]
    pub fn shl1(self) -> V16 {
        self.map2(self, |a, _| a << 1)
    }

    /// Each byte shifted right by one bit.
    #[inline(always)]
    #[must_use]
    pub fn shr1(self) -> V16 {
        self.map2(self, |a, _| a >> 1)
    }

    /// `if choose_a { a } else { b }`.
    #[inline(always)]
    #[must_use]
    pub fn select(choose_a: bool, a: V16, b: V16) -> V16 {
        let mask = V16::splat(0u8.wrapping_sub(u8::from(choose_a)));
        a.and(mask).or(b.and_not(mask))
    }

    /// Bytes of `a` where `mask` (0xFF or 0) is set, else of `b`.
    #[inline(always)]
    #[must_use]
    pub fn select_bytes(mask: u8, a: V16, b: V16) -> V16 {
        let mask = V16::splat(mask);
        a.and(mask).or(b.and_not(mask))
    }

    #[inline(always)]
    #[must_use]
    pub fn is_zero(self) -> bool {
        u128::from_le_bytes(self.0) == 0
    }

    /// Bytes 0..8 as a little-endian `u64`.
    #[inline(always)]
    #[must_use]
    pub fn low_u64(self) -> u64 {
        #[expect(clippy::cast_possible_truncation, reason = "the low half")]
        let low = u128::from_le_bytes(self.0) as u64;
        low
    }

    /// Bit `BIT` of bytes 0..16, as a 16-bit mask (`pmovmskb` of a shift).
    #[inline(always)]
    #[must_use]
    pub fn bit_mask<const BIT: u32>(self) -> u32 {
        let mut m = 0;
        for i in 0..16 {
            m |= u32::from(self.0[i] >> BIT & 1) << i;
        }
        m
    }

    /// `(bit 7 of self, bit 6 of self, bit 4 of other)` of each byte, as three
    /// 16-bit masks (see `bit_mask`).
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
