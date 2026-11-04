//! `cfg(wasm_threads)` for the threads build (wasm32-wasip1-threads): its
//! `atomics` target feature is not visible to `cfg(target_feature)`.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(wasm_threads)");
    if std::env::var("TARGET").is_ok_and(|target| target.ends_with("-threads")) {
        println!("cargo::rustc-cfg=wasm_threads");
    }
}
