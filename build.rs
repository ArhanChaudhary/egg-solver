//! Generates move and symmetry constants (in the SIMD byte layout of
//! `src/simd.rs`) from the geometric cube model in `src/cubie.rs`, as Rust
//! source in `$OUT_DIR/consts.rs` (included by `simd.rs`).

#[allow(dead_code)]
#[path = "src/cubie.rs"]
mod cubie;

use std::fmt::Write as _;

/// The 32 bytes of a cube in the SIMD layout.
fn simd_bytes(cube: &cubie::CubieCube) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes[..8].iter_mut().enumerate() {
        *byte = cube.corner_perm[i] | (cube.corner_orient[i] << 5);
    }
    for (i, byte) in bytes[16..28].iter_mut().enumerate() {
        *byte = cube.edge_perm[i] | (cube.edge_orient[i] << 4);
    }
    bytes
}

/// Rust source of a list of numbers: `1,2,3`.
fn list<T: std::fmt::Display>(values: impl IntoIterator<Item = T>) -> String {
    let mut text = String::new();
    for (i, value) in values.into_iter().enumerate() {
        if i > 0 {
            text.push(',');
        }
        let _ = write!(text, "{value}");
    }
    text
}

/// Rust source of an `Aligned32` constant.
fn aligned32(bytes: &[u8; 32]) -> String {
    format!("Aligned32([{}])", list(bytes))
}

/// The cube backend for the target: `x86` (AVX2 + BMI2, `simd.rs`), `neon`
/// (aarch64), `wasm` (wasm32 with SIMD128) or `scalar` (anything else, or
/// the `scalar` feature); all but `x86` are `portable/`. Set as
/// `cfg(backend = "...")`.
fn backend(has: impl Fn(&str) -> bool) -> &'static str {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if std::env::var_os("CARGO_FEATURE_SCALAR").is_some() {
        "scalar"
    } else if arch == "x86_64" && has("avx2") && has("bmi2") {
        "x86"
    } else if arch == "aarch64" && has("neon") {
        "neon"
    } else if arch == "wasm32" && has("simd128") {
        "wasm"
    } else {
        "scalar"
    }
}

fn main() {
    println!("cargo:rerun-if-changed=src/cubie.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let features = std::env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let has = |f: &str| features.split(',').any(|x| x == f);
    println!(
        "cargo::rustc-check-cfg=cfg(backend, values(\"x86\", \"neon\", \"wasm\", \"scalar\"))"
    );
    println!("cargo::rustc-check-cfg=cfg(avx512)");
    // The wasm threads target (its `atomics` feature is not visible to
    // `cfg(target_feature)`): `tables::Memory` allocates there differently.
    println!("cargo::rustc-check-cfg=cfg(wasm_threads)");
    if std::env::var("TARGET").is_ok_and(|t| t.starts_with("wasm32") && t.ends_with("-threads")) {
        println!("cargo::rustc-cfg=wasm_threads");
    }
    let backend = backend(has);
    println!("cargo::rustc-cfg=backend=\"{backend}\"");
    // AVX-512 helpers of the lanes kernel (`lanes.rs`) on the x86 backend.
    if backend == "x86" && has("avx512f") && has("avx512bw") && has("avx512vl") {
        println!("cargo::rustc-cfg=avx512");
    }
    let mut s = String::new();
    let moves = cubie::move_cubes();
    writeln!(s, "pub const MOVES: [Aligned32; 18] = [").unwrap();
    for m in moves {
        writeln!(s, "  {},", aligned32(&simd_bytes(m))).unwrap();
    }
    writeln!(s, "];").unwrap();
    // Orientation part of each move (the `ORIENT_BITS` term of `compose`).
    writeln!(s, "pub const MOVE_ORIENTS: [Aligned32; 18] = [").unwrap();
    for m in moves {
        let mut bytes = simd_bytes(m);
        for (i, x) in bytes.iter_mut().enumerate() {
            *x &= if i < 8 { 0x60 } else { 0x10 };
        }
        writeln!(s, "  {},", aligned32(&bytes)).unwrap();
    }
    writeln!(s, "];").unwrap();
    // Premove cubes: applying move m to the inverse = premove of inverse(m).
    writeln!(s, "pub const INVERSE_MOVES: [Aligned32; 18] = [").unwrap();
    for m in 0..18u8 {
        let inverse = cubie::inverse_move(m);
        writeln!(
            s,
            "  {},",
            aligned32(&simd_bytes(&moves[usize::from(inverse)]))
        )
        .unwrap();
    }
    writeln!(s, "];").unwrap();
    let syms = cubie::symmetries();
    writeln!(s, "pub static SYM_CUBES: [Aligned32; 48] = [").unwrap();
    for sym in syms {
        writeln!(s, "  {},", aligned32(&simd_bytes(&sym.cube))).unwrap();
    }
    writeln!(s, "];").unwrap();
    writeln!(s, "pub static SYM_CUBE_INVERSES: [Aligned32; 48] = [").unwrap();
    for sym in syms {
        writeln!(s, "  {},", aligned32(&simd_bytes(&sym.cube_inverse))).unwrap();
    }
    writeln!(s, "];").unwrap();
    // Edge-only conjugation tables (see `coord::edge_key_sym`), per
    // symmetry: `classes` classifies a piece j of T (edge lane: bit 7 =
    // E-slice, bit 6 = S-slice, bit 4 = eo of T at j), `perm` is the
    // position permutation Tinv (edge lane; 0x80 = zero elsewhere), and
    // `perm_eo` the eo of Tinv by source position: byte j (edge lane) is bit
    // 4 of Tinv at the position i that Tinv maps from j (`Tinv[i] & 15 ==
    // j`), so that xoring it before the position permutation adds Tinv's eo.
    writeln!(s, "pub const SYM_EDGES: [SymEdge; 48] = [").unwrap();
    for sym in syms {
        let t = simd_bytes(&sym.cube);
        let t_inverse = simd_bytes(&sym.cube_inverse);
        let (mut classes, mut perm, mut perm_eo) = ([0u8; 32], [0u8; 32], [0u8; 32]);
        for j in 0..12 {
            let x = t[16 + j];
            classes[16 + j] = ((x & 0x0C) << 4) | (x & 0x10);
        }
        for i in 0..32 {
            perm[i] = if (16..28).contains(&i) {
                t_inverse[i]
            } else {
                0x80
            };
        }
        for &x in &t_inverse[16..28] {
            perm_eo[16 + usize::from(x & 15)] = x & 0x10;
        }
        writeln!(
            s,
            "  SymEdge {{ classes: {}, perm: {}, perm_eo: {}, pad: Aligned32([0; 32]) }},",
            aligned32(&classes),
            aligned32(&perm),
            aligned32(&perm_eo)
        )
        .unwrap();
    }
    writeln!(s, "];").unwrap();
    let mirrors = list(syms.iter().map(|sym| sym.mirror));
    writeln!(s, "pub static SYM_MIRROR: [bool; 48] = [{mirrors}];").unwrap();
    let inverses = list(cubie::symmetry_inverses());
    writeln!(s, "pub static SYM_INVERSES: [u8; 48] = [{inverses}];").unwrap();
    writeln!(s, "pub static SYM_MOVE_MAP: [[u8; 18]; 48] = [").unwrap();
    for sym in syms {
        writeln!(s, "  [{}],", list(sym.move_map)).unwrap();
    }
    writeln!(s, "];").unwrap();
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("consts.rs");
    std::fs::write(out, s).unwrap();
}
