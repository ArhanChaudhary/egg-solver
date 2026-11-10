//! Cache prefetch hints for every target: `prefetch*` on x86, `prfm` on
//! aarch64, nothing elsewhere (wasm has no prefetch; there `read_once` is a
//! load). A prefetch never faults and has no effect on the program's state,
//! so the pointers need not be valid (except for `read_once`).

/// Prefetch for a read used once (table lookups): x86 `prefetchnta`
/// (measured better than T0 for the table, 6ccd6c8: +2.6%), aarch64
/// `pldl1strm`, wasm32 a one-byte load (below).
///
/// # Safety
/// `p` points into an allocation: on wasm32 its byte is read.
#[inline(always)]
pub unsafe fn read_once<T>(p: *const T) {
    // wasm has no prefetch, but a load whose value is dropped starts the
    // cache miss just the same: the CPU runs on past it (the next lanes)
    // while it is pending. (Measured when added, h6, 100 depth-17 cubes, 1
    // thread, 40 lanes: wasmtime +53% (187.4/190.7 s -> 122.4/125.1 s), V8
    // +45% with its optimizing compiler (188.8 -> 130.6 s). Clamping `p`
    // into linear memory instead of requiring it valid cost 2-4%.)
    #[cfg(target_arch = "wasm32")]
    // SAFETY: as the caller promises.
    unsafe {
        std::ptr::read_volatile(p.cast::<u8>());
    }
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
