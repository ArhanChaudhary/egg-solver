//! Scalar ("cubie level") cube model, derived from 3D geometry.
//!
//! This module is not performance critical. It is the ground truth from which
//! the fast SIMD representation, the move tables and the 48 symmetries are
//! derived, and against which they are tested.
//!
//! Piece numbering follows nissy so that cheap bit tricks give the H48
//! coordinates:
//! * corners `UFR UBL DFL DBR | UFL UBR DFR DBL` (bit 2 = tetrad),
//! * edges `UF UB DB DF | UR UL DL DR | FR FL BL BR` (bits 2-3 = slice).
//!
//! Axes: x points to R, y to U, z to F.
//!
//! Composition convention: `a.mul(&b)` is "a then b"; `cp[i]` is the piece in
//! position `i`, so `(a*b).cp[i] = a.cp[b.cp[i]]`.

pub type V3 = [i32; 3];

pub const CORNER_POS: [V3; 8] = [
    [1, 1, 1],    // UFR
    [-1, 1, -1],  // UBL
    [-1, -1, 1],  // DFL
    [1, -1, -1],  // DBR
    [-1, 1, 1],   // UFL
    [1, 1, -1],   // UBR
    [1, -1, 1],   // DFR
    [-1, -1, -1], // DBL
];

pub const EDGE_POS: [V3; 12] = [
    [0, 1, 1],   // UF
    [0, 1, -1],  // UB
    [0, -1, -1], // DB
    [0, -1, 1],  // DF
    [1, 1, 0],   // UR
    [-1, 1, 0],  // UL
    [-1, -1, 0], // DL
    [1, -1, 0],  // DR
    [1, 0, 1],   // FR
    [-1, 0, 1],  // FL
    [-1, 0, -1], // BL
    [1, 0, -1],  // BR
];

/// Faces in move order: U, D, R, L, F, B. Moves are `3*face + (quarter turns - 1)`.
pub const FACE_NORMAL: [V3; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
];
pub const FACE_CHARS: [char; 6] = ['U', 'D', 'R', 'L', 'F', 'B'];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CubieCube {
    pub cp: [u8; 8],
    pub co: [u8; 8],
    pub ep: [u8; 12],
    pub eo: [u8; 12],
}

impl CubieCube {
    pub const SOLVED: CubieCube = CubieCube {
        cp: [0, 1, 2, 3, 4, 5, 6, 7],
        co: [0; 8],
        ep: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        eo: [0; 12],
    };

    /// "self then other".
    pub fn mul(&self, o: &CubieCube) -> CubieCube {
        let mut r = CubieCube::SOLVED;
        for i in 0..8 {
            r.cp[i] = self.cp[o.cp[i] as usize];
            r.co[i] = (self.co[o.cp[i] as usize] + o.co[i]) % 3;
        }
        for i in 0..12 {
            r.ep[i] = self.ep[o.ep[i] as usize];
            r.eo[i] = (self.eo[o.ep[i] as usize] + o.eo[i]) % 2;
        }
        r
    }

    pub fn inverse(&self) -> CubieCube {
        let mut r = CubieCube::SOLVED;
        for i in 0..8 {
            r.cp[self.cp[i] as usize] = i as u8;
            r.co[self.cp[i] as usize] = (3 - self.co[i]) % 3;
        }
        for i in 0..12 {
            r.ep[self.ep[i] as usize] = i as u8;
            r.eo[self.ep[i] as usize] = self.eo[i];
        }
        r
    }

    /// Negate all corner orientations (used for mirror conjugation).
    pub fn invert_co(&self) -> CubieCube {
        let mut r = *self;
        for i in 0..8 {
            r.co[i] = (3 - r.co[i]) % 3;
        }
        r
    }

    pub fn apply_move(&self, m: usize) -> CubieCube {
        self.mul(&move_cubes()[m])
    }

    pub fn apply_moves(&self, ms: &[u8]) -> CubieCube {
        let mut c = *self;
        for &m in ms {
            c = c.apply_move(m as usize);
        }
        c
    }

    pub fn corner_parity(&self) -> u32 {
        perm_parity(&self.cp)
    }

    pub fn edge_parity(&self) -> u32 {
        perm_parity(&self.ep)
    }

    /// Is this a reachable cube position?
    pub fn is_solvable(&self) -> bool {
        let mut cs = [false; 8];
        let mut es = [false; 12];
        for &p in &self.cp {
            if p >= 8 || cs[p as usize] {
                return false;
            }
            cs[p as usize] = true;
        }
        for &p in &self.ep {
            if p >= 12 || es[p as usize] {
                return false;
            }
            es[p as usize] = true;
        }
        self.co.iter().map(|&x| x as u32).sum::<u32>() % 3 == 0
            && self.eo.iter().map(|&x| x as u32).sum::<u32>() % 2 == 0
            && self.corner_parity() == self.edge_parity()
    }

    /// Uniformly random reachable position.
    pub fn random(rng: &mut Rng) -> CubieCube {
        loop {
            let mut c = CubieCube::SOLVED;
            for i in (1..8).rev() {
                let j = rng.below(i as u64 + 1) as usize;
                c.cp.swap(i, j);
            }
            for i in (1..12).rev() {
                let j = rng.below(i as u64 + 1) as usize;
                c.ep.swap(i, j);
            }
            let mut s = 0;
            for i in 0..7 {
                c.co[i] = rng.below(3) as u8;
                s += c.co[i];
            }
            c.co[7] = (3 - s % 3) % 3;
            let mut s = 0;
            for i in 0..11 {
                c.eo[i] = rng.below(2) as u8;
                s += c.eo[i];
            }
            c.eo[11] = s % 2;
            if c.corner_parity() == c.edge_parity() {
                return c;
            }
        }
    }
}

fn perm_parity(p: &[u8]) -> u32 {
    let mut par = 0;
    for i in 0..p.len() {
        for j in i + 1..p.len() {
            if p[i] > p[j] {
                par ^= 1;
            }
        }
    }
    par
}

/// Small deterministic PRNG (splitmix64), so benchmarks are reproducible.
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        ((self.next_u64() as u128 * n as u128) >> 64) as u64
    }
}

// ---------------------------------------------------------------------------
// Sticker model
// ---------------------------------------------------------------------------

/// A sticker location: a cubie position vector and the outward normal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Sticker {
    p: V3,
    n: V3,
}

fn axis_vec(axis: usize, s: i32) -> V3 {
    let mut v = [0; 3];
    v[axis] = s;
    v
}

fn det3(a: V3, b: V3, c: V3) -> i32 {
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}

/// Stickers of corner position `i` in clockwise order, starting from the U/D one.
fn corner_stickers(i: usize) -> [Sticker; 3] {
    let p = CORNER_POS[i];
    let n0 = axis_vec(1, p[1]);
    let a = axis_vec(0, p[0]);
    let b = axis_vec(2, p[2]);
    let (n1, n2) = if det3(n0, a, b) > 0 { (a, b) } else { (b, a) };
    [
        Sticker { p, n: n0 },
        Sticker { p, n: n1 },
        Sticker { p, n: n2 },
    ]
}

/// Stickers of edge position `i`, the reference (U/D, else F/B) one first.
fn edge_stickers(i: usize) -> [Sticker; 2] {
    let p = EDGE_POS[i];
    let (r, o) = if p[1] != 0 {
        let o = if p[0] != 0 {
            axis_vec(0, p[0])
        } else {
            axis_vec(2, p[2])
        };
        (axis_vec(1, p[1]), o)
    } else {
        (axis_vec(2, p[2]), axis_vec(0, p[0]))
    };
    [Sticker { p, n: r }, Sticker { p, n: o }]
}

fn all_stickers() -> Vec<Sticker> {
    let mut v = Vec::with_capacity(48);
    for i in 0..8 {
        v.extend_from_slice(&corner_stickers(i));
    }
    for i in 0..12 {
        v.extend_from_slice(&edge_stickers(i));
    }
    v
}

fn sticker_index(all: &[Sticker], s: Sticker) -> usize {
    all.iter().position(|&x| x == s).expect("bad sticker")
}

type Mat = [[i32; 3]; 3];

fn mat_apply(m: &Mat, v: V3) -> V3 {
    let mut r = [0; 3];
    for (i, ri) in r.iter_mut().enumerate() {
        *ri = m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2];
    }
    r
}

fn mat_det(m: &Mat) -> i32 {
    det3(m[0], m[1], m[2])
}

/// Rotation by -90 degrees about unit axis `u` (clockwise seen from outside).
fn quarter_turn_matrix(u: V3) -> Mat {
    // R v = -(u x v) + u (u.v)
    let mut m = [[0; 3]; 3];
    for j in 0..3 {
        let v = axis_vec(j, 1);
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
        for i in 0..3 {
            m[i][j] = -cross[i] + u[i] * dot;
        }
    }
    m
}

/// A sticker state: `s[loc] = identity` (home location index) of the sticker at `loc`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StickerCube(pub [u8; 48]);

impl StickerCube {
    pub fn solved() -> StickerCube {
        let mut s = [0u8; 48];
        for (i, x) in s.iter_mut().enumerate() {
            *x = i as u8;
        }
        StickerCube(s)
    }

    /// Apply a face move (`face` in 0..6, `q` quarter turns clockwise).
    pub fn apply_face(&self, face: usize, q: usize) -> StickerCube {
        let all = all_stickers();
        let u = FACE_NORMAL[face];
        let m = quarter_turn_matrix(u);
        let mut cur = self.clone();
        for _ in 0..q {
            let mut next = cur.clone();
            for (li, l) in all.iter().enumerate() {
                let dot = l.p[0] * u[0] + l.p[1] * u[1] + l.p[2] * u[2];
                if dot > 0 {
                    let to = Sticker {
                        p: mat_apply(&m, l.p),
                        n: mat_apply(&m, l.n),
                    };
                    next.0[sticker_index(&all, to)] = cur.0[li];
                }
            }
            cur = next;
        }
        cur
    }

    /// Conjugate by a spatial symmetry: `out[M(L)] = M(s[L])`.
    pub fn conjugate(&self, m: &Mat) -> StickerCube {
        let all = all_stickers();
        let map = |idx: usize| -> usize {
            let l = all[idx];
            sticker_index(
                &all,
                Sticker {
                    p: mat_apply(m, l.p),
                    n: mat_apply(m, l.n),
                },
            )
        };
        let mut out = [0u8; 48];
        for li in 0..48 {
            out[map(li)] = map(self.0[li] as usize) as u8;
        }
        StickerCube(out)
    }

    /// State with `s[L] = M(L)`: the cubie-level "cube" of a symmetry.
    fn symmetry_state(m: &Mat) -> StickerCube {
        StickerCube::solved().conjugate_identity_only(m)
    }

    fn conjugate_identity_only(&self, m: &Mat) -> StickerCube {
        let all = all_stickers();
        let mut out = [0u8; 48];
        for (li, o) in out.iter_mut().enumerate() {
            let l = all[self.0[li] as usize];
            *o = sticker_index(
                &all,
                Sticker {
                    p: mat_apply(m, l.p),
                    n: mat_apply(m, l.n),
                },
            ) as u8;
        }
        StickerCube(out)
    }

    pub fn to_cubie(&self) -> CubieCube {
        let mut c = CubieCube::SOLVED;
        for i in 0..8 {
            let ref_id = self.0[3 * i] as usize;
            let piece = ref_id / 3;
            c.cp[i] = piece as u8;
            for k in 0..3 {
                if self.0[3 * i + k] as usize == 3 * piece {
                    c.co[i] = k as u8;
                }
            }
        }
        for i in 0..12 {
            let id = self.0[24 + 2 * i] as usize - 24;
            c.ep[i] = (id / 2) as u8;
            c.eo[i] = (id % 2) as u8;
        }
        c
    }

    pub fn from_cubie(c: &CubieCube) -> StickerCube {
        let mut s = [0u8; 48];
        for i in 0..8 {
            let p = c.cp[i] as usize;
            for k in 0..3 {
                // piece sticker j sits at position sticker (co + j) % 3
                let j = (k + 3 - c.co[i] as usize) % 3;
                s[3 * i + k] = (3 * p + j) as u8;
            }
        }
        for i in 0..12 {
            let p = c.ep[i] as usize;
            for k in 0..2 {
                let j = (k + c.eo[i] as usize) % 2;
                s[24 + 2 * i + k] = (24 + 2 * p + j) as u8;
            }
        }
        StickerCube(s)
    }
}

// ---------------------------------------------------------------------------
// Moves and symmetries
// ---------------------------------------------------------------------------

pub const NMOVES: usize = 18;

pub fn move_cubes() -> &'static [CubieCube; 18] {
    use std::sync::OnceLock;
    static M: OnceLock<[CubieCube; 18]> = OnceLock::new();
    M.get_or_init(|| {
        let mut r = [CubieCube::SOLVED; 18];
        for f in 0..6 {
            for q in 1..=3 {
                r[3 * f + q - 1] = StickerCube::solved().apply_face(f, q).to_cubie();
            }
        }
        r
    })
}

pub fn move_name(m: u8) -> String {
    let f = FACE_CHARS[(m / 3) as usize];
    match m % 3 {
        0 => format!("{f}"),
        1 => format!("{f}2"),
        _ => format!("{f}'"),
    }
}

pub fn inverse_move(m: u8) -> u8 {
    3 * (m / 3) + 2 - m % 3
}

/// Parse "R U2 F' D3 B1" style sequences (both Singmaster and 1/2/3 suffixes).
pub fn parse_moves(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        let f = FACE_CHARS.iter().position(|&c| c == ch)? as u8;
        i += 1;
        let mut q = 1;
        if i < chars.len() {
            match chars[i] {
                '1' => {
                    i += 1;
                }
                '2' => {
                    q = 2;
                    i += 1;
                }
                '3' | '\'' => {
                    q = 3;
                    i += 1;
                }
                _ => {}
            }
        }
        out.push(3 * f + q - 1);
    }
    Some(out)
}

pub fn format_moves(ms: &[u8]) -> String {
    ms.iter()
        .map(|&m| move_name(m))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A symmetry of the cube.
#[derive(Clone, Debug)]
pub struct Sym {
    pub mat: Mat,
    pub mirror: bool,
    /// Cubie "cube" `T` such that conjugation is `T * c * T^-1`
    /// (followed by corner-orientation inversion for mirrors).
    pub t: CubieCube,
    pub tinv: CubieCube,
    /// Image of each move under conjugation.
    pub move_map: [u8; 18],
}

/// All 48 symmetries; index 0 is the identity.
pub fn symmetries() -> &'static Vec<Sym> {
    use std::sync::OnceLock;
    static S: OnceLock<Vec<Sym>> = OnceLock::new();
    S.get_or_init(|| {
        let mut mats: Vec<Mat> = Vec::new();
        let perms = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        for p in perms {
            for signs in 0..8 {
                let mut m = [[0; 3]; 3];
                for i in 0..3 {
                    m[i][p[i]] = if signs >> i & 1 == 1 { -1 } else { 1 };
                }
                mats.push(m);
            }
        }
        // identity first
        let id = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
        let pos = mats.iter().position(|m| *m == id).unwrap();
        mats.swap(0, pos);
        mats.iter()
            .map(|m| {
                let mirror = mat_det(m) < 0;
                let t = StickerCube::symmetry_state(m).to_cubie();
                let tinv = t.inverse();
                let mut move_map = [0u8; 18];
                for f in 0..6 {
                    let n = mat_apply(m, FACE_NORMAL[f]);
                    let f2 = FACE_NORMAL.iter().position(|&x| x == n).unwrap();
                    for q in 0..3 {
                        let q2 = if mirror { 2 - q } else { q };
                        move_map[3 * f + q] = (3 * f2 + q2) as u8;
                    }
                }
                Sym {
                    mat: *m,
                    mirror,
                    t,
                    tinv,
                    move_map,
                }
            })
            .collect()
    })
}

/// Conjugate `c` by symmetry `s` at the cubie level: `T c T^-1` (+ co inversion for mirrors).
pub fn conjugate(c: &CubieCube, s: &Sym) -> CubieCube {
    let r = s.t.mul(c).mul(&s.tinv);
    if s.mirror {
        r.invert_co()
    } else {
        r
    }
}

/// Index of the inverse symmetry.
pub fn sym_inverse_table() -> &'static [u8; 48] {
    use std::sync::OnceLock;
    static T: OnceLock<[u8; 48]> = OnceLock::new();
    T.get_or_init(|| {
        let syms = symmetries();
        let mut r = [0u8; 48];
        for i in 0..48 {
            for j in 0..48 {
                let mut p = [[0; 3]; 3];
                for a in 0..3 {
                    for b in 0..3 {
                        p[a][b] = (0..3).map(|k| syms[i].mat[a][k] * syms[j].mat[k][b]).sum();
                    }
                }
                if p == syms[0].mat {
                    r[i] = j as u8;
                }
            }
        }
        r
    })
}

/// Reference conjugation through the sticker model (slow; for tests).
pub fn conjugate_sticker(c: &CubieCube, s: &Sym) -> CubieCube {
    StickerCube::from_cubie(c).conjugate(&s.mat).to_cubie()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticker_roundtrip() {
        let mut rng = Rng(1);
        for _ in 0..100 {
            let c = CubieCube::random(&mut rng);
            assert_eq!(StickerCube::from_cubie(&c).to_cubie(), c);
        }
    }

    #[test]
    fn moves_match_sticker_model() {
        let mut rng = Rng(2);
        for _ in 0..50 {
            let mut s = StickerCube::solved();
            let mut c = CubieCube::SOLVED;
            for _ in 0..30 {
                let m = rng.below(18) as usize;
                s = s.apply_face(m / 3, m % 3 + 1);
                c = c.apply_move(m);
                assert_eq!(s.to_cubie(), c);
                assert!(c.is_solvable());
            }
        }
    }

    #[test]
    fn move_orders() {
        for m in 0..18 {
            let mc = move_cubes()[m];
            let mut c = CubieCube::SOLVED;
            for _ in 0..4 {
                c = c.mul(&mc);
            }
            assert_eq!(c, CubieCube::SOLVED);
            assert_eq!(mc.inverse(), move_cubes()[inverse_move(m as u8) as usize]);
        }
        // R moves FR edge to UR (front to top).
        let r = move_cubes()[6];
        assert_eq!(r.ep[4], 8);
    }

    #[test]
    fn sexy_move_order_6() {
        let seq = parse_moves("R U R' U'").unwrap();
        let mut c = CubieCube::SOLVED;
        for _ in 0..6 {
            c = c.apply_moves(&seq);
        }
        assert_eq!(c, CubieCube::SOLVED);
    }

    #[test]
    fn symmetry_conjugation_matches_stickers() {
        let mut rng = Rng(3);
        let syms = symmetries();
        assert_eq!(syms.len(), 48);
        for s in syms {
            for _ in 0..20 {
                let c = CubieCube::random(&mut rng);
                assert_eq!(conjugate(&c, s), conjugate_sticker(&c, s));
            }
            for m in 0..18 {
                assert_eq!(
                    conjugate(&move_cubes()[m], s),
                    move_cubes()[s.move_map[m] as usize]
                );
            }
        }
        let inv = sym_inverse_table();
        for i in 0..48 {
            let c = CubieCube::random(&mut rng);
            let d = conjugate(&conjugate(&c, &syms[i]), &syms[inv[i] as usize]);
            assert_eq!(c, d);
        }
    }
}
