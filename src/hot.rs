//! Fixed placement of the hot loops in the binary.
//!
//! On this machine (Zen 5), the 1-thread stream throughput can depend on
//! where `bench::stream_thread` (the stream loop, with `run_lanes` inlined)
//! starts within a 64-byte line. Functions are 16-byte aligned, so it can
//! start at offset 0, 16, 32 or 48. On main abce87c, the same code at these
//! offsets gave about 25.5, 25.0, 24.9 and 25.2 solves/s (3 layouts each).
//! On 735dd25 the four pads were within the noise of each other (+-0.7%).
//!
//! The linker places functions one after another, so the offset changed
//! with any change anywhere in the binary, and it moved A/B results by up
//! to the size of most kernel changes. Now each hot function goes into a
//! section of its own. `hot_section!` aligns that section to 64 bytes and
//! starts it with a pad, so the function starts at the pad's offset in
//! every build.
//!
//! A change to the hot code can change the best pad: re-sweep it with the
//! build variables `CUBEOPT_STREAM_PAD` and `CUBEOPT_SHARE_PAD` (0, 16, 32
//! or 48), and use the same pads on both sides of an A/B.

/// Offset of `bench::stream_thread` in its 64-byte line: 16, where 735dd25
/// had it (its four pads were within the noise of each other at 1T).
pub const STREAM_PAD: usize = pad(option_env!("CUBEOPT_STREAM_PAD"), 16);

/// Offset of the share mode's round loop (`share::Worker::rounds`) in its
/// 64-byte line: 16. (With the loop visiting the lanes by pointer, m8 on
/// def2cd0, 1T share against main at its pad 0, 2 pairs per pad: 0 +1.3
/// +1.8%, 16 +5.3 +3.2%, 32 +1.6 +1.0%, 48 +1.7 +1.9%. Before: at 48T the
/// four pads were within the noise, +-0.3%; at 1T share, 0 against 48:
/// +1.7 +0.1 +0.0%.)
pub const SHARE_PAD: usize = pad(option_env!("CUBEOPT_SHARE_PAD"), 16);

/// The value of a pad build variable (decimal), or `default`.
///
/// # Panics
/// At compile time (it is used in constants) if the value is not 0, 16, 32
/// or 48.
#[must_use]
pub const fn pad(var: Option<&str>, default: usize) -> usize {
    let p = match var {
        None => default,
        Some(s) => {
            let digits = s.as_bytes();
            let mut v = 0;
            let mut i = 0;
            while i < digits.len() {
                assert!(digits[i].is_ascii_digit(), "a pad is a decimal number");
                v = v * 10 + (digits[i] - b'0') as usize;
                i += 1;
            }
            v
        }
    };
    assert!(p < 64 && p.is_multiple_of(16), "a pad is 0, 16, 32 or 48");
    p
}

/// Starts section `$name` on a 64-byte boundary, followed by `$pad` bytes
/// (int3, never executed). The hot function marked
/// `#[unsafe(link_section = $name)]`, the section's only one, then starts
/// at offset `$pad` of a 64-byte line. (Module-level asm is emitted before
/// any function, and the function's own alignment, 16, divides the pad.)
/// Only on x86-64 Linux (ELF section syntax, tuned on Zen 5): elsewhere the
/// hot functions stay where the linker puts them (their `link_section` is
/// set under the same condition).
#[macro_export]
macro_rules! hot_section {
    ($name:literal, $pad:expr) => {
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        ::core::arch::global_asm!(
            concat!(".pushsection ", $name, ",\"ax\",@progbits"),
            ".p2align 6",
            ".skip {pad}, 0xcc",
            ".popsection",
            pad = const $pad,
        );
    };
}
