//! Cache prefetch hints for every target: `prefetch*` on x86, `prfm` on
//! aarch64, nothing elsewhere (wasm has no prefetch). A prefetch never
//! faults and has no effect on the program's state, so the pointers need not
//! be valid.

/// Prefetch for a read used once (table lookups): x86 `prefetchnta`
/// (measured better than T0 for the table, ebd6c20: +2.6%), aarch64
/// `pldl1strm`.
#[inline(always)]
pub fn read_once<T>(p: *const T) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: see the module docs.
    unsafe {
        use std::arch::x86_64::{_mm_prefetch, _MM_HINT_NTA};
        _mm_prefetch(p.cast::<i8>(), _MM_HINT_NTA);
    }
    #[cfg(target_arch = "aarch64")]
    // SAFETY: see the module docs.
    unsafe {
        std::arch::asm!("prfm pldl1strm, [{p}]", p = in(reg) p,
            options(nostack, preserves_flags, readonly));
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let _ = p;
}

/// Prefetch for a read into all cache levels: x86 `prefetcht0`, aarch64
/// `pldl1keep`.
#[inline(always)]
pub fn read<T>(p: *const T) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: see the module docs.
    unsafe {
        use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
        _mm_prefetch(p.cast::<i8>(), _MM_HINT_T0);
    }
    #[cfg(target_arch = "aarch64")]
    // SAFETY: see the module docs.
    unsafe {
        std::arch::asm!("prfm pldl1keep, [{p}]", p = in(reg) p,
            options(nostack, preserves_flags, readonly));
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let _ = p;
}

/// Prefetch for a write: x86 `prefetchw`, aarch64 `pstl1keep`.
#[inline(always)]
pub fn write<T>(p: *const T) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: see the module docs.
    unsafe {
        use std::arch::x86_64::{_mm_prefetch, _MM_HINT_ET0};
        _mm_prefetch(p.cast::<i8>(), _MM_HINT_ET0);
    }
    #[cfg(target_arch = "aarch64")]
    // SAFETY: see the module docs.
    unsafe {
        std::arch::asm!("prfm pstl1keep, [{p}]", p = in(reg) p,
            options(nostack, preserves_flags, readonly));
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let _ = p;
}
