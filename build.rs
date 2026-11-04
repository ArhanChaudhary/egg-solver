//! Generates move and symmetry constants (in the SIMD byte layout) from the
//! geometric cube model in `src/cubie.rs`.

#[allow(dead_code)]
#[path = "src/cubie.rs"]
mod cubie;

use std::fmt::Write as _;

fn simd_bytes(c: &cubie::CubieCube) -> [u8; 32] {
    let mut b = [0u8; 32];
    for (i, byte) in b[..8].iter_mut().enumerate() {
        *byte = c.corner_perm[i] | (c.corner_orient[i] << 5);
    }
    for (i, byte) in b[16..28].iter_mut().enumerate() {
        *byte = c.edge_perm[i] | (c.edge_orient[i] << 4);
    }
    b
}

fn arr(b: &[u8; 32]) -> String {
    let v: Vec<String> = b.iter().map(|x| format!("{x}")).collect();
    format!("A32([{}])", v.join(","))
}

fn main() {
    println!("cargo:rerun-if-changed=src/cubie.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let mut s = String::new();
    let moves = cubie::move_cubes();
    writeln!(s, "pub const MOVES: [A32; 18] = [").unwrap();
    for m in moves {
        writeln!(s, "  {},", arr(&simd_bytes(m))).unwrap();
    }
    writeln!(s, "];").unwrap();
    // Orientation part of each move (the `o & ORIENT` term of `compose`).
    writeln!(s, "pub const MOVES_ORI: [A32; 18] = [").unwrap();
    for m in moves {
        let mut b = simd_bytes(m);
        for (i, x) in b.iter_mut().enumerate() {
            *x &= if i < 8 { 0x60 } else { 0x10 };
        }
        writeln!(s, "  {},", arr(&b)).unwrap();
    }
    writeln!(s, "];").unwrap();
    // Premove cubes: applying move m to the inverse = premove of inverse(m).
    writeln!(s, "pub const INV_MOVES: [A32; 18] = [").unwrap();
    for m in 0..18u8 {
        let im = cubie::inverse_move(m);
        writeln!(s, "  {},", arr(&simd_bytes(&moves[im as usize]))).unwrap();
    }
    writeln!(s, "];").unwrap();
    let syms = cubie::symmetries();
    writeln!(s, "pub static SYM_T: [A32; 48] = [").unwrap();
    for y in syms {
        writeln!(s, "  {},", arr(&simd_bytes(&y.cube))).unwrap();
    }
    writeln!(s, "];").unwrap();
    writeln!(s, "pub static SYM_TINV: [A32; 48] = [").unwrap();
    for y in syms {
        writeln!(s, "  {},", arr(&simd_bytes(&y.cube_inverse))).unwrap();
    }
    writeln!(s, "];").unwrap();
    // Edge-only conjugation tables (see `coord::edge_key_sym`), one 64-byte
    // line per symmetry: bytes 0..32 classify a piece j of T (edge lane:
    // bit 7 = E-slice, bit 6 = S-slice, bit 4 = eo of T at j), bytes 32..64
    // are the position permutation Tinv (edge lane; 0x80 = zero elsewhere).
    writeln!(s, "pub const SYM_EDGE: [A64; 48] = [").unwrap();
    for y in syms {
        let t = simd_bytes(&y.cube);
        let ti = simd_bytes(&y.cube_inverse);
        let mut b = [0u8; 64];
        for j in 0..12 {
            let x = t[16 + j];
            b[16 + j] = ((x & 0x0C) << 4) | (x & 0x10);
        }
        for i in 0..32 {
            b[32 + i] = if (16..28).contains(&i) { ti[i] } else { 0x80 };
        }
        let v: Vec<String> = b.iter().map(|x| format!("{x}")).collect();
        writeln!(s, "  A64([{}]),", v.join(",")).unwrap();
    }
    writeln!(s, "];").unwrap();
    let mirrors: Vec<String> = syms.iter().map(|y| format!("{}", y.mirror)).collect();
    writeln!(
        s,
        "pub static SYM_MIRROR: [bool; 48] = [{}];",
        mirrors.join(",")
    )
    .unwrap();
    let inv: Vec<String> = cubie::symmetry_inverses()
        .iter()
        .map(|x| format!("{x}"))
        .collect();
    writeln!(s, "pub static SYM_INV: [u8; 48] = [{}];", inv.join(",")).unwrap();
    writeln!(s, "pub static SYM_MOVE_MAP: [[u8; 18]; 48] = [").unwrap();
    for y in syms {
        let v: Vec<String> = y.move_map.iter().map(|x| format!("{x}")).collect();
        writeln!(s, "  [{}],", v.join(",")).unwrap();
    }
    writeln!(s, "];").unwrap();
    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("consts.rs");
    std::fs::write(out, s).unwrap();
}
