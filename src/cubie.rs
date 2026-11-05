//! Scalar ("cubie level") cube model, derived from 3D geometry.
//!
//! A cube state is described piece by piece: which corner/edge piece sits in
//! each position, and how it is twisted (corners: 0..3) or flipped (edges:
//! 0..2). This module is not performance critical. It is the ground truth
//! from which the fast SIMD representation (`simd`), the move tables and the
//! 48 symmetries are derived, and against which they are tested.
//!
//! Piece numbering follows nissy so that cheap bit tricks give the H48
//! coordinates:
//! * corners `UFR UBL DFL DBR | UFL UBR DFR DBL` (bit 2 = tetrad),
//! * edges `UF UB DB DF | UR UL DL DR | FR FL BL BR` (bits 2-3 = slice).
//!
//! Axes: x points to R, y to U, z to F.
//!
//! Composition convention: `a.compose(&b)` is "a then b". `corner_perm[i]`
//! is the piece in position `i`, so
//! `a.compose(&b).corner_perm[i] = a.corner_perm[b.corner_perm[i]]`.

/// An integer 3D vector (positions, normals).
pub type Vec3 = [i32; 3];

/// Centers of the 8 corner positions.
pub const CORNER_POSITIONS: [Vec3; 8] = [
    [1, 1, 1],    // UFR
    [-1, 1, -1],  // UBL
    [-1, -1, 1],  // DFL
    [1, -1, -1],  // DBR
    [-1, 1, 1],   // UFL
    [1, 1, -1],   // UBR
    [1, -1, 1],   // DFR
    [-1, -1, -1], // DBL
];

/// Centers of the 12 edge positions.
pub const EDGE_POSITIONS: [Vec3; 12] = [
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

/// Outward normals of the faces in move order: U, D, R, L, F, B.
///
/// Move `m` turns face `m / 3` clockwise by `m % 3 + 1` quarter turns (so
/// the moves are U U2 U' D D2 D' ... B').
pub const FACE_NORMALS: [Vec3; 6] = [
    [0, 1, 0],
    [0, -1, 0],
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
];
/// Face letters in move order.
pub const FACE_NAMES: [char; 6] = ['U', 'D', 'R', 'L', 'F', 'B'];

/// A cube state, piece by piece.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CubieCube {
    /// The corner piece in each corner position.
    pub corner_perm: [u8; 8],
    /// The twist of each corner position's piece: by how many clockwise
    /// steps (0..3) its U/D sticker is turned away from the U/D face.
    pub corner_orient: [u8; 8],
    /// The edge piece in each edge position.
    pub edge_perm: [u8; 12],
    /// Whether each edge position's piece is flipped (0 or 1).
    pub edge_orient: [u8; 12],
}

impl CubieCube {
    pub const SOLVED: CubieCube = CubieCube {
        corner_perm: [0, 1, 2, 3, 4, 5, 6, 7],
        corner_orient: [0; 8],
        edge_perm: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        edge_orient: [0; 12],
    };

    /// "`self` then `other`".
    #[must_use]
    pub fn compose(&self, other: &CubieCube) -> CubieCube {
        let mut result = CubieCube::SOLVED;
        for i in 0..8 {
            let from = usize::from(other.corner_perm[i]);
            result.corner_perm[i] = self.corner_perm[from];
            result.corner_orient[i] = (self.corner_orient[from] + other.corner_orient[i]) % 3;
        }
        for i in 0..12 {
            let from = usize::from(other.edge_perm[i]);
            result.edge_perm[i] = self.edge_perm[from];
            result.edge_orient[i] = (self.edge_orient[from] + other.edge_orient[i]) % 2;
        }
        result
    }

    /// The state that undoes this one: `c.compose(&c.inverse())` is solved.
    #[must_use]
    pub fn inverse(&self) -> CubieCube {
        let mut result = CubieCube::SOLVED;
        let corners = self.corner_perm.iter().zip(&self.corner_orient);
        for (i, (&piece, &orient)) in (0..).zip(corners) {
            result.corner_perm[usize::from(piece)] = i;
            result.corner_orient[usize::from(piece)] = (3 - orient) % 3;
        }
        let edges = self.edge_perm.iter().zip(&self.edge_orient);
        for (i, (&piece, &orient)) in (0..).zip(edges) {
            result.edge_perm[usize::from(piece)] = i;
            result.edge_orient[usize::from(piece)] = orient;
        }
        result
    }

    /// Negate all corner twists (mod 3): a mirror reverses the turning
    /// direction (see `conjugate`).
    #[must_use]
    pub fn negate_corner_orient(&self) -> CubieCube {
        let mut result = *self;
        for orient in &mut result.corner_orient {
            *orient = (3 - *orient) % 3;
        }
        result
    }

    /// This state followed by move `mv` (0..18).
    #[must_use]
    pub fn apply_move(&self, mv: usize) -> CubieCube {
        self.compose(&move_cubes()[mv])
    }

    /// This state followed by a move sequence.
    #[must_use]
    pub fn apply_moves(&self, moves: &[u8]) -> CubieCube {
        let mut cube = *self;
        for &mv in moves {
            cube = cube.apply_move(usize::from(mv));
        }
        cube
    }

    /// Parity (0 even, 1 odd) of the corner permutation.
    #[must_use]
    pub fn corner_parity(&self) -> u32 {
        permutation_parity(&self.corner_perm)
    }

    /// Parity (0 even, 1 odd) of the edge permutation.
    #[must_use]
    pub fn edge_parity(&self) -> u32 {
        permutation_parity(&self.edge_perm)
    }

    /// Is this a reachable cube position? (Valid permutations, total twist
    /// and flip 0, equal corner and edge parities.)
    #[must_use]
    pub fn is_solvable(&self) -> bool {
        let mut seen_corners = [false; 8];
        let mut seen_edges = [false; 12];
        for &piece in &self.corner_perm {
            if piece >= 8 || seen_corners[usize::from(piece)] {
                return false;
            }
            seen_corners[usize::from(piece)] = true;
        }
        for &piece in &self.edge_perm {
            if piece >= 12 || seen_edges[usize::from(piece)] {
                return false;
            }
            seen_edges[usize::from(piece)] = true;
        }
        let total = |orients: &[u8]| orients.iter().map(|&x| u32::from(x)).sum::<u32>();
        total(&self.corner_orient).is_multiple_of(3)
            && total(&self.edge_orient).is_multiple_of(2)
            && self.corner_parity() == self.edge_parity()
    }

    /// Uniformly random reachable position.
    pub fn random(rng: &mut Rng) -> CubieCube {
        loop {
            let mut cube = CubieCube::SOLVED;
            // (Fisher-Yates shuffles.)
            for i in (1..8).rev() {
                let j = rng.below_usize(i + 1);
                cube.corner_perm.swap(i, j);
            }
            for i in (1..12).rev() {
                let j = rng.below_usize(i + 1);
                cube.edge_perm.swap(i, j);
            }
            // The last twist and flip make the totals valid.
            let mut twist = 0;
            for i in 0..7 {
                cube.corner_orient[i] = rng.below_u8(3);
                twist += cube.corner_orient[i];
            }
            cube.corner_orient[7] = (3 - twist % 3) % 3;
            let mut flip = 0;
            for i in 0..11 {
                cube.edge_orient[i] = rng.below_u8(2);
                flip += cube.edge_orient[i];
            }
            cube.edge_orient[11] = flip % 2;
            // Half of all such states have mismatched parities: retry.
            if cube.corner_parity() == cube.edge_parity() {
                return cube;
            }
        }
    }
}

/// Parity of a permutation (0 even, 1 odd): that of its number of
/// inversions.
fn permutation_parity(perm: &[u8]) -> u32 {
    let mut parity = 0;
    for (i, &a) in perm.iter().enumerate() {
        for &b in &perm[i + 1..] {
            if a > b {
                parity ^= 1;
            }
        }
    }
    parity
}

/// Small deterministic PRNG (splitmix64), so benchmarks are reproducible.
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`: the high half of the 128-bit product of a
    /// random `u64` and `n` (no division).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64().carrying_mul(n, 0).1
    }

    /// `below` for small numbers (moves, orientations).
    ///
    /// # Panics
    /// Cannot panic: the result is below `n`.
    pub fn below_u8(&mut self, n: u8) -> u8 {
        u8::try_from(self.below(n.into())).expect("below n")
    }

    /// `below` for indices.
    ///
    /// # Panics
    /// Cannot panic: the result is below `n` (and `usize` is 64 bits).
    pub fn below_usize(&mut self, n: usize) -> usize {
        let n = u64::try_from(n).expect("64-bit usize");
        usize::try_from(self.below(n)).expect("below n")
    }
}

// ---------------------------------------------------------------------------
// Sticker model: moves and symmetries are rotations and reflections of the
// 48 stickers; the cubie-level moves and symmetries are read off from them.
// ---------------------------------------------------------------------------

/// A sticker location: a cubie position vector and the outward normal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Sticker {
    position: Vec3,
    normal: Vec3,
}

/// The unit vector `sign` along `axis`.
fn axis_vector(axis: usize, sign: i32) -> Vec3 {
    let mut v = [0; 3];
    v[axis] = sign;
    v
}

/// Determinant of the matrix with rows `a`, `b`, `c`.
fn determinant(a: Vec3, b: Vec3, c: Vec3) -> i32 {
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}

/// Stickers of corner position `i` in clockwise order, starting from the U/D one.
fn corner_stickers(i: usize) -> [Sticker; 3] {
    let position = CORNER_POSITIONS[i];
    let up_down = axis_vector(1, position[1]);
    let a = axis_vector(0, position[0]);
    let b = axis_vector(2, position[2]);
    // (Clockwise seen from outside: a positive determinant.)
    let (second, third) = if determinant(up_down, a, b) > 0 {
        (a, b)
    } else {
        (b, a)
    };
    [up_down, second, third].map(|normal| Sticker { position, normal })
}

/// Stickers of edge position `i`, the reference (U/D, else F/B) one first.
fn edge_stickers(i: usize) -> [Sticker; 2] {
    let position = EDGE_POSITIONS[i];
    let (reference, other) = if position[1] != 0 {
        let other = if position[0] != 0 {
            axis_vector(0, position[0])
        } else {
            axis_vector(2, position[2])
        };
        (axis_vector(1, position[1]), other)
    } else {
        (axis_vector(2, position[2]), axis_vector(0, position[0]))
    };
    [reference, other].map(|normal| Sticker { position, normal })
}

/// All 48 stickers: 3 per corner position, then 2 per edge position.
fn all_stickers() -> Vec<Sticker> {
    let mut stickers = Vec::with_capacity(48);
    for i in 0..8 {
        stickers.extend_from_slice(&corner_stickers(i));
    }
    for i in 0..12 {
        stickers.extend_from_slice(&edge_stickers(i));
    }
    stickers
}

/// Index of a sticker in `all_stickers()`.
fn sticker_index(all: &[Sticker], sticker: Sticker) -> u8 {
    let index = all.iter().position(|&x| x == sticker).expect("bad sticker");
    u8::try_from(index).expect("48 stickers")
}

/// A 3x3 integer matrix (rotations and reflections), row major.
type Matrix = [[i32; 3]; 3];

/// `m` times the column vector `v`.
fn matrix_apply(m: &Matrix, v: Vec3) -> Vec3 {
    let mut result = [0; 3];
    for (row, x) in m.iter().zip(&mut result) {
        *x = row[0] * v[0] + row[1] * v[1] + row[2] * v[2];
    }
    result
}

fn matrix_determinant(m: &Matrix) -> i32 {
    determinant(m[0], m[1], m[2])
}

fn matrix_product(a: &Matrix, b: &Matrix) -> Matrix {
    let mut product = [[0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            product[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    product
}

/// The image of a sticker under `m`.
fn matrix_apply_sticker(m: &Matrix, sticker: Sticker) -> Sticker {
    Sticker {
        position: matrix_apply(m, sticker.position),
        normal: matrix_apply(m, sticker.normal),
    }
}

/// Rotation by -90 degrees about unit axis `u` (clockwise seen from outside).
#[expect(clippy::needless_range_loop, reason = "matrix indices")]
fn quarter_turn_matrix(u: Vec3) -> Matrix {
    // Column j is the image of the unit vector e_j: R v = -(u x v) + u (u.v).
    let mut m = [[0; 3]; 3];
    for j in 0..3 {
        let v = axis_vector(j, 1);
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

/// A sticker state: `self.0[location]` is the identity (home location
/// index) of the sticker at `location`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StickerCube(pub [u8; 48]);

impl StickerCube {
    #[must_use]
    pub fn solved() -> StickerCube {
        let mut stickers = [0u8; 48];
        for (x, i) in stickers.iter_mut().zip(0..) {
            *x = i;
        }
        StickerCube(stickers)
    }

    /// Apply a face move (`face` in 0..6, clockwise quarter turns).
    #[must_use]
    pub fn apply_face(&self, face: usize, quarter_turns: usize) -> StickerCube {
        let all = all_stickers();
        let axis = FACE_NORMALS[face];
        let turn = quarter_turn_matrix(axis);
        let mut current = self.clone();
        for _ in 0..quarter_turns {
            let mut next = current.clone();
            for (from, &sticker) in all.iter().enumerate() {
                // (The face's layer: the stickers on its side of the center.)
                let p = sticker.position;
                if p[0] * axis[0] + p[1] * axis[1] + p[2] * axis[2] > 0 {
                    let to = sticker_index(&all, matrix_apply_sticker(&turn, sticker));
                    next.0[usize::from(to)] = current.0[from];
                }
            }
            current = next;
        }
        current
    }

    /// Conjugate by a spatial symmetry `m`: `out[m(L)] = m(self[L])`.
    #[must_use]
    pub fn conjugate(&self, m: &Matrix) -> StickerCube {
        let all = all_stickers();
        let map = |index: u8| -> u8 {
            sticker_index(&all, matrix_apply_sticker(m, all[usize::from(index)]))
        };
        let mut out = [0u8; 48];
        for (location, &identity) in (0..).zip(&self.0) {
            out[usize::from(map(location))] = map(identity);
        }
        StickerCube(out)
    }

    /// State with `self[L] = m(L)`: the cubie-level "cube" of a symmetry.
    fn symmetry_state(m: &Matrix) -> StickerCube {
        let all = all_stickers();
        let mut out = [0u8; 48];
        for (x, &sticker) in out.iter_mut().zip(&all) {
            *x = sticker_index(&all, matrix_apply_sticker(m, sticker));
        }
        StickerCube(out)
    }

    /// The cubie state of this sticker state (which must be one).
    #[must_use]
    pub fn to_cubie(&self) -> CubieCube {
        let mut cube = CubieCube::SOLVED;
        for i in 0..8 {
            // The piece's reference sticker is 3 * piece; where it is gives
            // the twist.
            let piece = self.0[3 * i] / 3;
            cube.corner_perm[i] = piece;
            for (k, &identity) in (0..).zip(&self.0[3 * i..3 * i + 3]) {
                if identity == 3 * piece {
                    cube.corner_orient[i] = k;
                }
            }
        }
        for i in 0..12 {
            let identity = self.0[24 + 2 * i] - 24;
            cube.edge_perm[i] = identity / 2;
            cube.edge_orient[i] = identity % 2;
        }
        cube
    }

    #[must_use]
    pub fn from_cubie(cube: &CubieCube) -> StickerCube {
        let mut stickers = [0u8; 48];
        for i in 0..8 {
            let piece = cube.corner_perm[i];
            for k in 0..3 {
                // piece sticker j sits at position sticker (twist + j) % 3
                let j = (k + 3 - cube.corner_orient[i]) % 3;
                stickers[3 * i + usize::from(k)] = 3 * piece + j;
            }
        }
        for i in 0..12 {
            let piece = cube.edge_perm[i];
            for k in 0..2 {
                let j = (k + cube.edge_orient[i]) % 2;
                stickers[24 + 2 * i + usize::from(k)] = 24 + 2 * piece + j;
            }
        }
        StickerCube(stickers)
    }
}

// ---------------------------------------------------------------------------
// Facelet strings
// ---------------------------------------------------------------------------

/// The faces of a facelet string in its order (U R F D L B, as in Kociemba's
/// solvers), each as its outward normal and the directions of "right" and
/// "down" as the face is seen from outside: U with B at the top, D with F at
/// the top, the side faces with U at the top.
const FACELET_FACES: [(Vec3, Vec3, Vec3); 6] = [
    ([0, 1, 0], [1, 0, 0], [0, 0, 1]),
    ([1, 0, 0], [0, 0, -1], [0, -1, 0]),
    ([0, 0, 1], [1, 0, 0], [0, -1, 0]),
    ([0, -1, 0], [1, 0, 0], [0, 0, -1]),
    ([-1, 0, 0], [0, 0, 1], [0, -1, 0]),
    ([0, 0, -1], [-1, 0, 0], [0, -1, 0]),
];

/// The sticker of facelet `i` (0..54: 9 per face, row by row).
fn facelet_sticker(i: usize) -> Sticker {
    let (normal, right, down) = FACELET_FACES[i / 9];
    let offset = |x: usize| i32::try_from(x).expect("below 3") - 1;
    let (row, column) = (offset(i % 9 / 3), offset(i % 3));
    Sticker {
        position: std::array::from_fn(|a| normal[a] + column * right[a] + row * down[a]),
        normal,
    }
}

/// The face letter of an outward normal.
fn face_letter(normal: Vec3) -> char {
    let face = FACE_NORMALS.iter().position(|&n| n == normal);
    FACE_NAMES[face.expect("a face normal")]
}

/// Why a facelet string is not a cube (`CubieCube::from_facelets`).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FaceletError {
    /// Not 54 stickers (whitespace aside).
    #[error("{0} stickers, a cube has 54")]
    StickerCount(usize),
    /// Two centers of one color.
    #[error("two centers are {0:?}")]
    SameCenters(char),
    /// A sticker (numbered from 1) of a color that no center has.
    #[error("sticker {sticker} is {color:?}, no center's color")]
    UnknownColor { sticker: usize, color: char },
    /// The stickers at a corner position are no corner's colors.
    #[error("corner {0}: no such piece")]
    NoSuchCorner(usize),
    /// The stickers at an edge position are no edge's colors.
    #[error("edge {0}: no such piece")]
    NoSuchEdge(usize),
    /// Pieces of a cube, but in a position no face turns reach.
    #[error(
        "not a reachable position (a piece twice, a twisted corner, a flipped edge or two pieces swapped)"
    )]
    Unreachable,
}

impl CubieCube {
    /// The cube of a facelet string: 54 stickers, face by face in the order
    /// U R F D L B, each face row by row as seen from outside (U with B at
    /// the top, D with F at the top, the others with U at the top), as in
    /// Kociemba's solvers. Any six symbols can stand for the colors: a face's
    /// color is the one of its center. Whitespace is ignored.
    ///
    /// # Errors
    /// If the string is not 54 stickers of six colors, one per center, or
    /// does not describe pieces of a cube, or a position that can be reached
    /// by turning faces (e.g. a single twisted corner).
    ///
    /// # Panics
    /// Never (every sticker is a facelet).
    pub fn from_facelets(text: &str) -> Result<CubieCube, FaceletError> {
        let colors: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        if colors.len() != 54 {
            return Err(FaceletError::StickerCount(colors.len()));
        }
        // The face of each color, from the centers.
        let centers: [char; 6] = std::array::from_fn(|face| colors[9 * face + 4]);
        for (face, center) in centers.iter().enumerate() {
            if centers[..face].contains(center) {
                return Err(FaceletError::SameCenters(*center));
            }
        }
        let normal_at = |sticker: Sticker| -> Result<Vec3, FaceletError> {
            let i = (0..54)
                .find(|&i| facelet_sticker(i) == sticker)
                .expect("every sticker has a facelet");
            let face = centers.iter().position(|&c| c == colors[i]);
            face.map(|face| FACELET_FACES[face].0)
                .ok_or(FaceletError::UnknownColor {
                    sticker: i + 1,
                    color: colors[i],
                })
        };
        let mut cube = CubieCube::SOLVED;
        for position in 0..8 {
            let [first, second, third] = corner_stickers(position).map(&normal_at);
            let seen = [first?, second?, third?];
            // The piece whose stickers' colors are these, turned by `twist`.
            let found = (0..8).flat_map(|piece| (0..3).map(move |twist| (piece, twist)));
            let found = found.into_iter().find(|&(piece, twist)| {
                let home = corner_stickers(piece);
                (0..3).all(|j| seen[(twist + j) % 3] == home[j].normal)
            });
            let (piece, twist) = found.ok_or(FaceletError::NoSuchCorner(position))?;
            cube.corner_perm[position] = u8::try_from(piece).expect("below 8");
            cube.corner_orient[position] = u8::try_from(twist).expect("below 3");
        }
        for position in 0..12 {
            let [first, second] = edge_stickers(position).map(&normal_at);
            let seen = [first?, second?];
            let found = (0..12).flat_map(|piece| (0..2).map(move |flip| (piece, flip)));
            let found = found.into_iter().find(|&(piece, flip)| {
                let home = edge_stickers(piece);
                (0..2).all(|j| seen[(flip + j) % 2] == home[j].normal)
            });
            let (piece, flip) = found.ok_or(FaceletError::NoSuchEdge(position))?;
            cube.edge_perm[position] = u8::try_from(piece).expect("below 12");
            cube.edge_orient[position] = u8::try_from(flip).expect("below 2");
        }
        if cube.is_solvable() {
            Ok(cube)
        } else {
            Err(FaceletError::Unreachable)
        }
    }

    /// The facelet string of the cube (see `from_facelets`), with the face
    /// letters U R F D L B as colors.
    #[must_use]
    pub fn to_facelets(self) -> String {
        (0..54)
            .map(|i| {
                let sticker = facelet_sticker(i);
                if i % 9 == 4 {
                    return face_letter(sticker.normal);
                }
                for position in 0..8 {
                    if let Some(k) = corner_stickers(position).iter().position(|&s| s == sticker) {
                        let (piece, twist) =
                            (self.corner_perm[position], self.corner_orient[position]);
                        let j = (k + 3 - usize::from(twist)) % 3;
                        return face_letter(corner_stickers(usize::from(piece))[j].normal);
                    }
                }
                for position in 0..12 {
                    if let Some(k) = edge_stickers(position).iter().position(|&s| s == sticker) {
                        let (piece, flip) = (self.edge_perm[position], self.edge_orient[position]);
                        let j = (k + usize::from(flip)) % 2;
                        return face_letter(edge_stickers(usize::from(piece))[j].normal);
                    }
                }
                unreachable!("every facelet is a center, corner or edge sticker")
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Moves and symmetries
// ---------------------------------------------------------------------------

/// Number of moves (turns of 90, 180 and 270 degrees of the 6 faces).
pub const N_MOVES: usize = 18;

/// The cube state of each move.
pub fn move_cubes() -> &'static [CubieCube; N_MOVES] {
    use std::sync::OnceLock;
    static MOVES: OnceLock<[CubieCube; N_MOVES]> = OnceLock::new();
    MOVES.get_or_init(|| {
        let mut moves = [CubieCube::SOLVED; N_MOVES];
        for face in 0..6 {
            for turns in 1..=3 {
                moves[3 * face + turns - 1] =
                    StickerCube::solved().apply_face(face, turns).to_cubie();
            }
        }
        moves
    })
}

/// Singmaster name of a move: `U`, `U2`, `U'`, ...
#[must_use]
pub fn move_name(mv: u8) -> String {
    let mut name = String::with_capacity(2);
    push_move_name(&mut name, mv);
    name
}

/// Appends the Singmaster name of `mv` to `text`.
fn push_move_name(text: &mut String, mv: u8) {
    text.push(FACE_NAMES[usize::from(mv / 3)]);
    match mv % 3 {
        0 => {}
        1 => text.push('2'),
        _ => text.push('\''),
    }
}

/// The move that undoes `mv` (same face, opposite direction).
#[must_use]
pub fn inverse_move(mv: u8) -> u8 {
    3 * (mv / 3) + 2 - mv % 3
}

/// Parse "R U2 F' D3 B1" style sequences (both Singmaster and 1/2/3 suffixes).
#[must_use]
pub fn parse_moves(text: &str) -> Option<Vec<u8>> {
    let mut moves = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(letter) = chars.next() {
        if letter.is_whitespace() {
            continue;
        }
        let face = FACE_NAMES.iter().zip(0..).find(|&(&c, _)| c == letter)?.1;
        // An optional suffix right after the letter.
        let quarter_turns = match chars.next_if(|c| matches!(c, '1' | '2' | '3' | '\'')) {
            Some('2') => 2,
            Some('3' | '\'') => 3,
            _ => 1,
        };
        moves.push(3 * face + quarter_turns - 1);
    }
    Some(moves)
}

/// A move sequence in Singmaster notation, separated by spaces.
#[must_use]
pub fn format_moves(moves: &[u8]) -> String {
    // (One String, not a String per move, a Vec and a join: `solve -c 1`
    // allocates 6.5 instead of 25.2 times per cube.)
    let mut text = String::with_capacity(3 * moves.len());
    for (i, &mv) in moves.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }
        push_move_name(&mut text, mv);
    }
    text
}

/// A symmetry of the cube: one of the 48 rotations and reflections that map
/// the cube onto itself.
#[derive(Clone, Debug)]
pub struct Symmetry {
    pub matrix: Matrix,
    /// Whether it is a reflection (it reverses the turning direction).
    pub mirror: bool,
    /// Cubie "cube" `T` such that conjugation is `T * c * T^-1`
    /// (followed by negating the corner twists for mirrors).
    pub cube: CubieCube,
    pub cube_inverse: CubieCube,
    /// Image of each move under conjugation.
    pub move_map: [u8; N_MOVES],
}

/// All 48 symmetries; index 0 is the identity.
///
/// # Panics
/// Never: every symmetry maps faces to faces.
pub fn symmetries() -> &'static [Symmetry] {
    use std::sync::OnceLock;
    static SYMMETRIES: OnceLock<Box<[Symmetry]>> = OnceLock::new();
    SYMMETRIES.get_or_init(|| {
        // Signed permutation matrices: each axis maps to some axis, either way.
        let mut matrices: Vec<Matrix> = Vec::with_capacity(48);
        let axis_perms = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        for perm in axis_perms {
            for signs in 0..8 {
                let mut m = [[0; 3]; 3];
                for i in 0..3 {
                    m[i][perm[i]] = if signs >> i & 1 == 1 { -1 } else { 1 };
                }
                matrices.push(m);
            }
        }
        // identity first
        let identity = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
        let pos = matrices.iter().position(|m| *m == identity).unwrap();
        matrices.swap(0, pos);
        matrices
            .iter()
            .map(|m| {
                let mirror = matrix_determinant(m) < 0;
                let cube = StickerCube::symmetry_state(m).to_cubie();
                let mut move_map = [0u8; N_MOVES];
                for (face, &normal) in (0..).zip(&FACE_NORMALS) {
                    let image = matrix_apply(m, normal);
                    let image_face = FACE_NORMALS
                        .iter()
                        .zip(0..)
                        .find(|&(&x, _)| x == image)
                        .unwrap()
                        .1;
                    for turns in 0..3 {
                        // (A mirror turns clockwise into counterclockwise.)
                        let image_turns = if mirror { 2 - turns } else { turns };
                        move_map[usize::from(3 * face + turns)] = 3 * image_face + image_turns;
                    }
                }
                Symmetry {
                    matrix: *m,
                    mirror,
                    cube,
                    cube_inverse: cube.inverse(),
                    move_map,
                }
            })
            .collect()
    })
}

/// Conjugate `cube` by `sym` at the cubie level: `T c T^-1` (corner twists
/// negated for mirrors). The result is the same position seen through the
/// symmetry, so it is as far from solved.
#[must_use]
pub fn conjugate(cube: &CubieCube, sym: &Symmetry) -> CubieCube {
    let result = sym.cube.compose(cube).compose(&sym.cube_inverse);
    if sym.mirror {
        result.negate_corner_orient()
    } else {
        result
    }
}

/// Index of the inverse of each symmetry.
pub fn symmetry_inverses() -> &'static [u8; 48] {
    use std::sync::OnceLock;
    static INVERSES: OnceLock<[u8; 48]> = OnceLock::new();
    INVERSES.get_or_init(|| {
        let syms = symmetries();
        let mut inverses = [0u8; 48];
        for (inverse, sym) in inverses.iter_mut().zip(syms) {
            for (j, other) in (0..).zip(syms) {
                if matrix_product(&sym.matrix, &other.matrix) == syms[0].matrix {
                    *inverse = j;
                }
            }
        }
        inverses
    })
}

/// Reference conjugation through the sticker model (slow; for tests).
#[must_use]
pub fn conjugate_by_stickers(cube: &CubieCube, sym: &Symmetry) -> CubieCube {
    StickerCube::from_cubie(cube)
        .conjugate(&sym.matrix)
        .to_cubie()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Facelet strings: known ones (solved, U, R, the superflip as in
    /// Kociemba's examples), round trips with any color symbols, and
    /// rejected non-positions.
    #[test]
    fn facelets() {
        let cube_of = |moves: &str| CubieCube::SOLVED.apply_moves(&parse_moves(moves).unwrap());
        let known = [
            ("", "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB"),
            (
                "U",
                "UUUUUUUUUBBBRRRRRRRRRFFFFFFDDDDDDDDDFFFLLLLLLLLLBBBBBB",
            ),
            (
                "R",
                "UUFUUFUUFRRRRRRRRRFFDFFDFFDDDBDDBDDBLLLLLLLLLUBBUBBUBB",
            ),
            (
                "U R2 F B R B2 R U2 L B2 R U' D' R2 F R' L B2 U2 F2",
                "UBULURUFURURFRBRDRFUFLFRFDFDFDLDRDBDLULBLFLDLBUBRBLBDB",
            ),
        ];
        for (moves, facelets) in known {
            assert_eq!(cube_of(moves).to_facelets(), facelets, "{moves}");
            assert_eq!(CubieCube::from_facelets(facelets), Ok(cube_of(moves)));
        }
        let mut rng = Rng(4);
        for _ in 0..100 {
            let cube = CubieCube::random(&mut rng);
            let facelets = cube.to_facelets();
            assert_eq!(CubieCube::from_facelets(&facelets), Ok(cube));
            let recolored: String = facelets
                .chars()
                .map(|c| match c {
                    'U' => 'W',
                    'D' => 'Y',
                    'F' => 'G',
                    'B' => 'B',
                    'R' => 'R',
                    _ => 'O',
                })
                .collect();
            assert_eq!(CubieCube::from_facelets(&recolored), Ok(cube));
        }
        let solved = CubieCube::SOLVED.to_facelets();
        let with = |changes: &[(usize, char)]| {
            let mut stickers: Vec<char> = solved.chars().collect();
            for &(i, c) in changes {
                stickers[i] = c;
            }
            stickers.into_iter().collect::<String>()
        };
        // No such corner (U9 = F), two R centers, a lone twisted corner (URF:
        // U9 R1 F3), a lone flipped edge (UF: U8 F2), two edges swapped (UF
        // and UR, by their F2 and R2), too few stickers.
        for bad in [
            with(&[(8, 'F')]),
            with(&[(4, 'R')]),
            with(&[(8, 'F'), (9, 'U'), (20, 'R')]),
            with(&[(7, 'F'), (19, 'U')]),
            with(&[(19, 'R'), (10, 'F')]),
            solved[1..].to_string(),
        ] {
            assert!(CubieCube::from_facelets(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn sticker_roundtrip() {
        let mut rng = Rng(1);
        for _ in 0..100 {
            let cube = CubieCube::random(&mut rng);
            assert_eq!(StickerCube::from_cubie(&cube).to_cubie(), cube);
        }
    }

    #[test]
    fn moves_match_sticker_model() {
        let mut rng = Rng(2);
        for _ in 0..50 {
            let mut stickers = StickerCube::solved();
            let mut cube = CubieCube::SOLVED;
            for _ in 0..30 {
                let mv = rng.below_usize(N_MOVES);
                stickers = stickers.apply_face(mv / 3, mv % 3 + 1);
                cube = cube.apply_move(mv);
                assert_eq!(stickers.to_cubie(), cube);
                assert!(cube.is_solvable());
            }
        }
    }

    #[test]
    fn move_orders() {
        for mv in 0..N_MOVES {
            let move_cube = move_cubes()[mv];
            let mut cube = CubieCube::SOLVED;
            for _ in 0..4 {
                cube = cube.compose(&move_cube);
            }
            assert_eq!(cube, CubieCube::SOLVED);
            let inverse = inverse_move(u8::try_from(mv).unwrap());
            assert_eq!(move_cube.inverse(), move_cubes()[usize::from(inverse)]);
        }
        // R moves FR edge to UR (front to top).
        let r = move_cubes()[6];
        assert_eq!(r.edge_perm[4], 8);
    }

    #[test]
    fn sexy_move_order_6() {
        let seq = parse_moves("R U R' U'").unwrap();
        let mut cube = CubieCube::SOLVED;
        for _ in 0..6 {
            cube = cube.apply_moves(&seq);
        }
        assert_eq!(cube, CubieCube::SOLVED);
    }

    #[test]
    fn parse_and_format() {
        let moves = parse_moves("R U2 F' D3 B1 L").unwrap();
        assert_eq!(format_moves(&moves), "R U2 F' D' B L");
        assert_eq!(parse_moves("R X"), None);
        // (A suffix must follow its letter.)
        assert_eq!(parse_moves("R 2"), None);
    }

    #[test]
    fn symmetry_conjugation_matches_stickers() {
        let mut rng = Rng(3);
        let syms = symmetries();
        assert_eq!(syms.len(), 48);
        for sym in syms {
            for _ in 0..20 {
                let cube = CubieCube::random(&mut rng);
                assert_eq!(conjugate(&cube, sym), conjugate_by_stickers(&cube, sym));
            }
            for mv in 0..N_MOVES {
                assert_eq!(
                    conjugate(&move_cubes()[mv], sym),
                    move_cubes()[usize::from(sym.move_map[mv])]
                );
            }
        }
        let inverses = symmetry_inverses();
        for (sym, &inverse) in syms.iter().zip(inverses) {
            let cube = CubieCube::random(&mut rng);
            let back = conjugate(&conjugate(&cube, sym), &syms[usize::from(inverse)]);
            assert_eq!(cube, back);
        }
    }
}
