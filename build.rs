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
    let strings: Vec<String> = values.into_iter().map(|x| format!("{x}")).collect();
    strings.join(",")
}

/// Rust source of an `Aligned32` constant.
fn aligned32(bytes: &[u8; 32]) -> String {
    format!("Aligned32([{}])", list(bytes))
}

fn main() {
    println!("cargo:rerun-if-changed=src/cubie.rs");
    println!("cargo:rerun-if-changed=build.rs");
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
    // Edge-only conjugation tables (see `coord::edge_key_sym`), one 64-byte
    // line per symmetry: bytes 0..32 classify a piece j of T (edge lane:
    // bit 7 = E-slice, bit 6 = S-slice, bit 4 = eo of T at j), bytes 32..64
    // are the position permutation Tinv (edge lane; 0x80 = zero elsewhere).
    writeln!(s, "pub const SYM_EDGES: [Aligned64; 48] = [").unwrap();
    for sym in syms {
        let t = simd_bytes(&sym.cube);
        let t_inverse = simd_bytes(&sym.cube_inverse);
        let mut line = [0u8; 64];
        for j in 0..12 {
            let x = t[16 + j];
            line[16 + j] = ((x & 0x0C) << 4) | (x & 0x10);
        }
        for i in 0..32 {
            line[32 + i] = if (16..28).contains(&i) {
                t_inverse[i]
            } else {
                0x80
            };
        }
        writeln!(s, "  Aligned64([{}]),", list(line)).unwrap();
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
