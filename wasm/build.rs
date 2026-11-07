//! For the threads build (wasm32-wasip1-threads): `cfg(wasm_threads)` (its
//! `atomics` target feature is not visible to `cfg(target_feature)`), and all
//! of wasm32's memory from the start, so that it never grows.
//!
//! Memory that grows while threads run is not safe in V8: a bulk memory
//! operation (`memory.fill`, `memory.copy`) checks its bounds against the
//! size the thread's instance last saw, which a grow updates only later. A
//! `memory.fill` of a block fresh from `malloc` then trapped ("memory access
//! out of bounds"; 1 in ~100 runs of the module's tests under load). With
//! 4 GiB less one 64 KiB page from the start (so that the heap's end,
//! `__heap_end`, still fits in 32 bits), wasi-libc's `malloc` has it all
//! without growing; the engine maps it, untouched pages cost no RAM.

// No intentional panics in shipped code (user request; asserts, which no
// lint covers, are checked by tests/no_runtime_panics.rs).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]

fn main() {
    println!("cargo::rustc-check-cfg=cfg(wasm_threads)");
    if std::env::var("TARGET").is_ok_and(|target| target.ends_with("-threads")) {
        println!("cargo::rustc-cfg=wasm_threads");
        println!("cargo::rustc-cdylib-link-arg=--initial-memory=4294901760");
    }
}
