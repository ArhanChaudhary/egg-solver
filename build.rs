//! Generates move and symmetry constants (in the SIMD byte layout of
//! `src/simd.rs`) from the geometric cube model in `src/cubie.rs`, as Rust
//! source in `$OUT_DIR/consts.rs` (included by `simd.rs`), and the cubie
//! level moves, symmetries and facelet tables in `$OUT_DIR/cubie_consts.rs`
//! (included by `cubie.rs`). A failure of the model fails the build.

#[allow(dead_code)]
#[path = "src/cubie.rs"]
mod cubie;

use std::fmt::Write as _;

/// A build error: the message of a failed step.
type Error = Box<dyn std::error::Error>;

/// The 32 bytes of a cube in the SIMD layout.
fn simd_bytes(cube: &cubie::CubieCube) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    let corners = cube.corner_perm.iter().zip(&cube.corner_orient);
    for (byte, (&piece, &twist)) in bytes.iter_mut().zip(corners) {
        *byte = piece | (twist << 5);
    }
    let edges = cube.edge_perm.iter().zip(&cube.edge_orient);
    for (byte, (&piece, &flip)) in bytes.iter_mut().skip(16).zip(edges) {
        *byte = piece | (flip << 4);
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

/// Rust source of a `CubieCube` constant.
fn cubie_literal(cube: &cubie::CubieCube) -> String {
    format!(
        "CubieCube {{ corner_perm: {:?}, corner_orient: {:?}, edge_perm: {:?}, edge_orient: {:?} }}",
        cube.corner_perm, cube.corner_orient, cube.edge_perm, cube.edge_orient
    )
}

/// The source of `cubie_consts.rs` (see `cubie.rs`): the moves, symmetries
/// and facelet tables of the sticker model.
fn cubie_consts(
    moves: &[cubie::CubieCube],
    syms: &[cubie::Symmetry],
    inverses: &[u8; 48],
) -> Result<String, Error> {
    let mut s = String::new();
    writeln!(s, "static MOVE_CUBES: [CubieCube; N_MOVES] = [")?;
    for m in moves {
        writeln!(s, "  {},", cubie_literal(m))?;
    }
    writeln!(s, "];")?;
    writeln!(s, "static SYMMETRIES: [Symmetry; 48] = [")?;
    for sym in syms {
        writeln!(
            s,
            "  Symmetry {{ matrix: {:?}, mirror: {}, cube: {}, cube_inverse: {}, move_map: {:?} }},",
            sym.matrix,
            sym.mirror,
            cubie_literal(&sym.cube),
            cubie_literal(&sym.cube_inverse),
            sym.move_map
        )?;
    }
    writeln!(s, "];")?;
    writeln!(s, "static SYMMETRY_INVERSES: [u8; 48] = {inverses:?};")?;
    let corners = cubie::model::corner_facelets().ok_or("cube model: no corner facelets")?;
    writeln!(s, "const CORNER_FACELETS: [[u8; 3]; 8] = {corners:?};")?;
    let edges = cubie::model::edge_facelets().ok_or("cube model: no edge facelets")?;
    writeln!(s, "const EDGE_FACELETS: [[u8; 2]; 12] = {edges:?};")?;
    Ok(s)
}

fn main() -> Result<(), Error> {
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
    // The library's cubie.rs takes its moves, symmetries and facelet tables
    // from `cubie_consts.rs` (this script's copy derives them).
    println!("cargo::rustc-cfg=cubie_generated");
    let moves = cubie::model::move_cubes().ok_or("cube model: no move cubes")?;
    let syms = cubie::model::symmetries().ok_or("cube model: no symmetries")?;
    let inverses = cubie::model::symmetry_inverses(&syms).ok_or("cube model: no inverses")?;
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    std::fs::write(
        out_dir.join("cubie_consts.rs"),
        cubie_consts(&moves, &syms, &inverses)?,
    )?;
    let mut s = String::new();
    writeln!(s, "pub const MOVES: [Aligned32; 18] = [")?;
    for m in &moves {
        writeln!(s, "  {},", aligned32(&simd_bytes(m)))?;
    }
    writeln!(s, "];")?;
    // Orientation part of each move (the `ORIENT_BITS` term of `compose`).
    writeln!(s, "pub const MOVE_ORIENTS: [Aligned32; 18] = [")?;
    for m in &moves {
        let mut bytes = simd_bytes(m);
        for (i, x) in bytes.iter_mut().enumerate() {
            *x &= if i < 8 { 0x60 } else { 0x10 };
        }
        writeln!(s, "  {},", aligned32(&bytes))?;
    }
    writeln!(s, "];")?;
    // Premove cubes: applying move m to the inverse = premove of inverse(m).
    writeln!(s, "pub const INVERSE_MOVES: [Aligned32; 18] = [")?;
    for m in 0..18u8 {
        let inverse = moves
            .get(usize::from(cubie::inverse_move(m)))
            .ok_or("no inverse move")?;
        writeln!(s, "  {},", aligned32(&simd_bytes(inverse)))?;
    }
    writeln!(s, "];")?;
    writeln!(s, "pub static SYM_CUBES: [Aligned32; 48] = [")?;
    for sym in &syms {
        writeln!(s, "  {},", aligned32(&simd_bytes(&sym.cube)))?;
    }
    writeln!(s, "];")?;
    writeln!(s, "pub static SYM_CUBE_INVERSES: [Aligned32; 48] = [")?;
    for sym in &syms {
        writeln!(s, "  {},", aligned32(&simd_bytes(&sym.cube_inverse)))?;
    }
    writeln!(s, "];")?;
    // Edge-only conjugation tables (see `coord::edge_key_sym`), per
    // symmetry: `classes` classifies a piece j of T (edge lane: bit 7 =
    // E-slice, bit 6 = S-slice, bit 4 = eo of T at j), `perm` is the
    // position permutation Tinv (edge lane; 0x80 = zero elsewhere), and
    // `perm_eo` the eo of Tinv by source position: byte j (edge lane) is bit
    // 4 of Tinv at the position i that Tinv maps from j (`Tinv[i] & 15 ==
    // j`), so that xoring it before the position permutation adds Tinv's eo.
    writeln!(s, "pub const SYM_EDGES: [SymEdge; 48] = [")?;
    for sym in &syms {
        let t = simd_bytes(&sym.cube);
        let t_inverse = simd_bytes(&sym.cube_inverse);
        let (mut classes, mut perm, mut perm_eo) = ([0u8; 32], [0u8; 32], [0u8; 32]);
        for (class, &x) in classes.iter_mut().zip(&t).skip(16).take(12) {
            *class = ((x & 0x0C) << 4) | (x & 0x10);
        }
        for (i, (p, &x)) in perm.iter_mut().zip(&t_inverse).enumerate() {
            *p = if (16..28).contains(&i) { x } else { 0x80 };
        }
        for &x in t_inverse.iter().skip(16).take(12) {
            *perm_eo
                .get_mut(16 + usize::from(x & 15))
                .ok_or("edge out of range")? = x & 0x10;
        }
        writeln!(
            s,
            "  SymEdge {{ classes: {}, perm: {}, perm_eo: {}, pad: Aligned32([0; 32]) }},",
            aligned32(&classes),
            aligned32(&perm),
            aligned32(&perm_eo)
        )?;
    }
    writeln!(s, "];")?;
    let mirrors = list(syms.iter().map(|sym| sym.mirror));
    writeln!(s, "pub static SYM_MIRROR: [bool; 48] = [{mirrors}];")?;
    let inverses = list(inverses);
    writeln!(s, "pub static SYM_INVERSES: [u8; 48] = [{inverses}];")?;
    writeln!(s, "pub static SYM_MOVE_MAP: [[u8; 18]; 48] = [")?;
    for sym in &syms {
        writeln!(s, "  [{}],", list(sym.move_map))?;
    }
    writeln!(s, "];")?;
    std::fs::write(out_dir.join("consts.rs"), s)?;
    Ok(())
}
