//! Scalar ("cubie level") cube model, derived from 3D geometry.
//!
//! A cube state is described piece by piece: which corner/edge piece sits in
//! each position, and how it is twisted (corners: 0..3) or flipped (edges:
//! 0..2). This module is not performance critical. It is the ground truth
//! from which the fast SIMD representation (`simd`), the move tables and the
//! 48 symmetries are derived, and against which they are tested.
//!
//! The derivation (`model`: moves and symmetries as rotations and
//! reflections of the 48 stickers) runs in build.rs, which writes the moves,
//! symmetries and facelet tables as constants for the library
//! (`cfg(cubie_generated)`), and in the tests.
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
///
/// Valid by construction: the fields are private to the crate, and every
/// way to make one (`SOLVED`, moves, `compose`, `inverse`, `random`,
/// `from_facelets`, the SIMD cube's `to_cubie`) gives permutations of the
/// pieces and orientations in range, so the pieces index the tables below
/// in range.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct CubieCube {
    /// The corner piece in each corner position.
    pub(crate) corner_perm: [u8; 8],
    /// The twist of each corner position's piece: by how many clockwise
    /// steps (0..3) its U/D sticker is turned away from the U/D face.
    pub(crate) corner_orient: [u8; 8],
    /// The edge piece in each edge position.
    pub(crate) edge_perm: [u8; 12],
    /// Whether each edge position's piece is flipped (0 or 1).
    pub(crate) edge_orient: [u8; 12],
}

/// Structural validity of the SIMD byte layout. Reachability is separate:
/// coordinate generation also uses cubes with mismatched permutation parity.
pub(crate) fn valid_cube_bytes(bytes: &[u8; 32]) -> bool {
    let mut corners = 0u16;
    let mut edges = 0u16;
    for &byte in &bytes[..8] {
        let piece = byte & 31;
        if piece >= 8 || byte >> 5 >= 3 {
            return false;
        }
        corners |= 1 << piece;
    }
    for &byte in &bytes[16..28] {
        let piece = byte & 15;
        if piece >= 12 || byte >> 4 >= 2 {
            return false;
        }
        edges |= 1 << piece;
    }
    corners == 0xFF && edges == 0xFFF
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

    /// This state followed by move `mv`.
    #[cfg(cubie_generated)]
    #[must_use]
    pub fn apply_move(&self, mv: Move) -> CubieCube {
        self.compose(&MOVE_CUBES[mv.index()])
    }

    /// This state followed by a move sequence (moves numbered as `Move`),
    /// or `None` if a number is not a move.
    #[cfg(cubie_generated)]
    #[must_use]
    pub fn apply_moves(&self, moves: &[u8]) -> Option<CubieCube> {
        moves.iter().try_fold(*self, |cube, &mv| {
            Some(cube.apply_move(Move::from_index(usize::from(mv))?))
        })
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
            match seen_corners.get_mut(usize::from(piece)) {
                Some(seen @ false) => *seen = true,
                _ => return false,
            }
        }
        for &piece in &self.edge_perm {
            match seen_edges.get_mut(usize::from(piece)) {
                Some(seen @ false) => *seen = true,
                _ => return false,
            }
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
        for &b in perm.iter().skip(i + 1) {
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
    #[expect(clippy::cast_possible_truncation, reason = "below n, a u8")]
    pub fn below_u8(&mut self, n: u8) -> u8 {
        self.below(n.into()) as u8
    }

    /// `below` for indices.
    #[expect(clippy::cast_possible_truncation, reason = "below n, a usize")]
    pub fn below_usize(&mut self, n: usize) -> usize {
        self.below(n as u64) as usize
    }
}

// (`below_usize` widens a `usize` to `u64`.)
const _: () = assert!(usize::BITS <= u64::BITS);

/// Number of moves (turns of 90, 180 and 270 degrees of the 6 faces).
pub const N_MOVES: usize = 18;

/// A move: face `index / 3` (U D R L F B) turned clockwise by `index % 3 +
/// 1` quarter turns; `X3` is X'. Only the 18 moves exist, so a `Move`
/// indexes the 18-entry move tables without a check.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u8)]
pub enum Move {
    U,
    U2,
    U3,
    D,
    D2,
    D3,
    R,
    R2,
    R3,
    L,
    L2,
    L3,
    F,
    F2,
    F3,
    B,
    B2,
    B3,
}

impl Move {
    /// All moves, in order of their numbers.
    pub const ALL: [Move; N_MOVES] = [
        Move::U,
        Move::U2,
        Move::U3,
        Move::D,
        Move::D2,
        Move::D3,
        Move::R,
        Move::R2,
        Move::R3,
        Move::L,
        Move::L2,
        Move::L3,
        Move::F,
        Move::F2,
        Move::F3,
        Move::B,
        Move::B2,
        Move::B3,
    ];

    /// The move numbered `index`, if there is one (`index < 18`).
    #[must_use]
    #[inline(always)]
    pub fn from_index(index: usize) -> Option<Move> {
        Move::ALL.get(index).copied()
    }

    /// The lowest move of a move mask (bit `m` for move `m`), or `None` if
    /// it has none. (For masks of moves only, as the search's are: then
    /// `while let Some(mv) = Move::lowest(mask)` visits all of them.)
    #[must_use]
    #[inline(always)]
    pub fn lowest(mask: u32) -> Option<Move> {
        Move::from_index(mask.trailing_zeros() as usize)
    }

    /// Its number (0..18).
    #[must_use]
    #[inline(always)]
    pub const fn index(self) -> usize {
        self as usize
    }
}

// (A `Move` is its number: `index` is the identity.)
const _: () = {
    let mut i = 0;
    while i < N_MOVES {
        assert!(Move::ALL[i] as usize == i);
        i += 1;
    }
};

// ---------------------------------------------------------------------------
// Facelet strings
// ---------------------------------------------------------------------------

/// The face letters of a facelet string, in its order of faces (U R F D L
/// B, as in Kociemba's solvers).
const FACELET_LETTERS: [char; 6] = ['U', 'R', 'F', 'D', 'L', 'B'];

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

// (Every facelet number in the tables is below 54, so the tables index a
// facelet string, and a facelet number / 9 the faces, in range.)
#[cfg(cubie_generated)]
const _: () = {
    let mut i = 0;
    while i < 8 * 3 {
        assert!(CORNER_FACELETS[i / 3][i % 3] < 54);
        i += 1;
    }
    let mut i = 0;
    while i < 12 * 2 {
        assert!(EDGE_FACELETS[i / 2][i % 2] < 54);
        i += 1;
    }
};

#[cfg(cubie_generated)]
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
    pub fn from_facelets(text: &str) -> Result<CubieCube, FaceletError> {
        let colors: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        let colors: [char; 54] = colors
            .try_into()
            .map_err(|colors: Vec<char>| FaceletError::StickerCount(colors.len()))?;
        // The face of each color, from the centers (each face's fifth sticker).
        let mut centers = ['\0'; 6];
        for (center, face) in centers.iter_mut().zip(colors.as_chunks::<9>().0) {
            *center = face[4];
        }
        for (face, center) in centers.iter().enumerate() {
            if centers.iter().take(face).any(|c| c == center) {
                return Err(FaceletError::SameCenters(*center));
            }
        }
        // The face (in facelet order) of the color of facelet `facelet`.
        let face_of = |facelet: u8| -> Result<u8, FaceletError> {
            let color = colors[usize::from(facelet)];
            (0..)
                .zip(&centers)
                .find(|&(_, &center)| center == color)
                .map(|(face, _)| face)
                .ok_or(FaceletError::UnknownColor {
                    sticker: usize::from(facelet) + 1,
                    color,
                })
        };
        let mut cube = CubieCube::SOLVED;
        let corners = cube.corner_perm.iter_mut().zip(&mut cube.corner_orient);
        for (position, ((perm, orient), facelets)) in corners.zip(&CORNER_FACELETS).enumerate() {
            let [first, second, third] = facelets.map(face_of);
            let seen = [first?, second?, third?];
            // The piece whose stickers' faces are these, turned by `twist`
            // (the faces of a piece's stickers: those of its home facelets).
            let found = (0u8..).zip(&CORNER_FACELETS).find_map(|(piece, home)| {
                (0u8..3)
                    .find(|&twist| {
                        (0..3).all(|j| seen[(usize::from(twist) + j) % 3] == home[j] / 9)
                    })
                    .map(|twist| (piece, twist))
            });
            (*perm, *orient) = found.ok_or(FaceletError::NoSuchCorner(position))?;
        }
        let edges = cube.edge_perm.iter_mut().zip(&mut cube.edge_orient);
        for (position, ((perm, orient), facelets)) in edges.zip(&EDGE_FACELETS).enumerate() {
            let [first, second] = facelets.map(face_of);
            let seen = [first?, second?];
            let found = (0u8..).zip(&EDGE_FACELETS).find_map(|(piece, home)| {
                (0u8..2)
                    .find(|&flip| (0..2).all(|j| seen[(usize::from(flip) + j) % 2] == home[j] / 9))
                    .map(|flip| (piece, flip))
            });
            (*perm, *orient) = found.ok_or(FaceletError::NoSuchEdge(position))?;
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
        // (Every facelet is written: the centers, then each corner's and
        // edge's stickers.)
        let mut letters = [' '; 54];
        for (face, &letter) in letters
            .as_chunks_mut::<9>()
            .0
            .iter_mut()
            .zip(&FACELET_LETTERS)
        {
            face[4] = letter;
        }
        // The letter of the face of a home facelet.
        let letter = |facelet: u8| FACELET_LETTERS[usize::from(facelet / 9)];
        let corners = self.corner_perm.iter().zip(&self.corner_orient);
        for ((&piece, &twist), facelets) in corners.zip(&CORNER_FACELETS) {
            let home = CORNER_FACELETS[usize::from(piece)];
            for (k, &facelet) in facelets.iter().enumerate() {
                letters[usize::from(facelet)] = letter(home[(k + 3 - usize::from(twist)) % 3]);
            }
        }
        let edges = self.edge_perm.iter().zip(&self.edge_orient);
        for ((&piece, &flip), facelets) in edges.zip(&EDGE_FACELETS) {
            let home = EDGE_FACELETS[usize::from(piece)];
            for (k, &facelet) in facelets.iter().enumerate() {
                letters[usize::from(facelet)] = letter(home[(k + usize::from(flip)) % 2]);
            }
        }
        letters.iter().collect()
    }
}

// ---------------------------------------------------------------------------
// Moves and symmetries
// ---------------------------------------------------------------------------

/// The cube state of each move.
#[cfg(cubie_generated)]
#[must_use]
pub fn move_cubes() -> &'static [CubieCube; N_MOVES] {
    &MOVE_CUBES
}

/// Singmaster name of a move (numbered as `Move`): `U`, `U2`, `U'`, ...
/// (`?` for a number that is not a move).
#[must_use]
pub fn move_name(mv: u8) -> String {
    let mut name = String::with_capacity(2);
    push_move_name(&mut name, mv);
    name
}

/// Appends the Singmaster name of `mv` to `text` (`?` if it is no move).
fn push_move_name(text: &mut String, mv: u8) {
    let Some(&face) = FACE_NAMES.get(usize::from(mv / 3)) else {
        text.push('?');
        return;
    };
    text.push(face);
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

/// Parse "R U2 F' D3 B1" style sequences (both Singmaster and 1/2/3
/// suffixes) into move numbers (as `Move`, so each below 18).
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

/// A 3x3 integer matrix (rotations and reflections), row major.
pub type Matrix = [[i32; 3]; 3];

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
#[cfg(cubie_generated)]
#[must_use]
pub fn symmetries() -> &'static [Symmetry] {
    &SYMMETRIES
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
#[cfg(cubie_generated)]
#[must_use]
pub fn symmetry_inverses() -> &'static [u8; 48] {
    &SYMMETRY_INVERSES
}

// The moves (`MOVE_CUBES`), symmetries (`SYMMETRIES`, `SYMMETRY_INVERSES`)
// and the facelets of each piece's stickers (`CORNER_FACELETS`,
// `EDGE_FACELETS`), derived from the sticker model by build.rs.
#[cfg(cubie_generated)]
include!(concat!(env!("OUT_DIR"), "/cubie_consts.rs"));

/// The sticker model: moves and symmetries are rotations and reflections of
/// the 48 stickers; the cubie-level moves, symmetries and facelet tables are
/// read off from them. Run by build.rs (which writes them as constants for
/// the library) and the tests. Every step returns `None` on a bug in the
/// model instead of panicking (the build then fails with a message).
#[cfg(any(test, not(cubie_generated)))]
pub(crate) mod model {
    use super::{
        CubieCube, Matrix, Symmetry, Vec3, CORNER_POSITIONS, EDGE_POSITIONS, FACE_NORMALS, N_MOVES,
    };

    /// A sticker location: a cubie position vector and the outward normal.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct Sticker {
        position: Vec3,
        normal: Vec3,
    }

    /// The unit vector `sign` along `axis`.
    fn axis_vector(axis: usize, sign: i32) -> Vec3 {
        std::array::from_fn(|a| if a == axis { sign } else { 0 })
    }

    /// Determinant of the matrix with rows `a`, `b`, `c`.
    fn determinant(a: Vec3, b: Vec3, c: Vec3) -> i32 {
        a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0])
    }

    /// Stickers of the corner at `position` in clockwise order, starting
    /// from the U/D one.
    fn corner_stickers(position: Vec3) -> [Sticker; 3] {
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

    /// Stickers of the edge at `position`, the reference (U/D, else F/B)
    /// one first.
    fn edge_stickers(position: Vec3) -> [Sticker; 2] {
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
        let corners = CORNER_POSITIONS.iter().flat_map(|&p| corner_stickers(p));
        let edges = EDGE_POSITIONS.iter().flat_map(|&p| edge_stickers(p));
        corners.chain(edges).collect()
    }

    /// Index of a sticker in `all_stickers()`.
    fn sticker_index(all: &[Sticker], sticker: Sticker) -> Option<u8> {
        let index = all.iter().position(|&x| x == sticker)?;
        u8::try_from(index).ok()
    }

    /// `m` times the column vector `v`.
    fn matrix_apply(m: &Matrix, v: Vec3) -> Vec3 {
        m.map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2])
    }

    fn matrix_determinant(m: &Matrix) -> i32 {
        determinant(m[0], m[1], m[2])
    }

    fn matrix_product(a: &Matrix, b: &Matrix) -> Matrix {
        a.map(|row| std::array::from_fn(|j| row.iter().zip(b).map(|(x, b_row)| x * b_row[j]).sum()))
    }

    /// The image of a sticker under `m`.
    fn matrix_apply_sticker(m: &Matrix, sticker: Sticker) -> Sticker {
        Sticker {
            position: matrix_apply(m, sticker.position),
            normal: matrix_apply(m, sticker.normal),
        }
    }

    /// Rotation by -90 degrees about unit axis `u` (clockwise seen from
    /// outside).
    fn quarter_turn_matrix(u: Vec3) -> Matrix {
        // Column j is the image of the unit vector e_j: R v = -(u x v) + u (u.v).
        let columns: [Vec3; 3] = std::array::from_fn(|j| {
            let v = axis_vector(j, 1);
            let cross = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
            std::array::from_fn(|i| -cross[i] + u[i] * dot)
        });
        std::array::from_fn(|i| columns.map(|column| column[i]))
    }

    /// A sticker state: `self.0[location]` is the identity (home location
    /// index) of the sticker at `location`.
    #[derive(Clone, PartialEq, Eq, Debug)]
    pub(crate) struct StickerCube(pub(crate) [u8; 48]);

    impl StickerCube {
        pub(crate) fn solved() -> StickerCube {
            let mut stickers = [0u8; 48];
            for (x, i) in stickers.iter_mut().zip(0..) {
                *x = i;
            }
            StickerCube(stickers)
        }

        /// Apply a face move (`face` in 0..6, clockwise quarter turns).
        pub(crate) fn apply_face(&self, face: usize, quarter_turns: usize) -> Option<StickerCube> {
            let all = all_stickers();
            let axis = *FACE_NORMALS.get(face)?;
            let turn = quarter_turn_matrix(axis);
            let mut current = self.clone();
            for _ in 0..quarter_turns {
                let mut next = current.clone();
                for (&sticker, &identity) in all.iter().zip(&current.0) {
                    // (The face's layer: the stickers on its side of the center.)
                    let p = sticker.position;
                    if p[0] * axis[0] + p[1] * axis[1] + p[2] * axis[2] > 0 {
                        let to = sticker_index(&all, matrix_apply_sticker(&turn, sticker))?;
                        *next.0.get_mut(usize::from(to))? = identity;
                    }
                }
                current = next;
            }
            Some(current)
        }

        /// Conjugate by a spatial symmetry `m`: `out[m(L)] = m(self[L])`.
        #[cfg(test)]
        pub(crate) fn conjugate(&self, m: &Matrix) -> Option<StickerCube> {
            let all = all_stickers();
            let map = |index: u8| -> Option<u8> {
                sticker_index(&all, matrix_apply_sticker(m, *all.get(usize::from(index))?))
            };
            let mut out = [0u8; 48];
            for (location, &identity) in (0..).zip(&self.0) {
                *out.get_mut(usize::from(map(location)?))? = map(identity)?;
            }
            Some(StickerCube(out))
        }

        /// State with `self[L] = m(L)`: the cubie-level "cube" of a symmetry.
        fn symmetry_state(m: &Matrix) -> Option<StickerCube> {
            let all = all_stickers();
            let mut out = [0u8; 48];
            for (x, &sticker) in out.iter_mut().zip(&all) {
                *x = sticker_index(&all, matrix_apply_sticker(m, sticker))?;
            }
            Some(StickerCube(out))
        }

        /// The cubie state of this sticker state (which must be one).
        pub(crate) fn to_cubie(&self) -> Option<CubieCube> {
            let mut cube = CubieCube::SOLVED;
            let (corners, edges) = self.0.split_first_chunk::<24>()?;
            let corner_slots = cube.corner_perm.iter_mut().zip(&mut cube.corner_orient);
            for ((perm, orient), stickers) in corner_slots.zip(corners.as_chunks::<3>().0) {
                // The piece's reference sticker is 3 * piece; where it is gives
                // the twist.
                let piece = stickers[0] / 3;
                *perm = piece;
                *orient = (0..).zip(stickers).find(|&(_, &x)| x == 3 * piece)?.0;
            }
            let edge_slots = cube.edge_perm.iter_mut().zip(&mut cube.edge_orient);
            for ((perm, orient), stickers) in edge_slots.zip(edges.as_chunks::<2>().0) {
                let identity = stickers[0].checked_sub(24)?;
                *perm = identity / 2;
                *orient = identity % 2;
            }
            Some(cube)
        }

        #[cfg(test)]
        pub(crate) fn from_cubie(cube: &CubieCube) -> StickerCube {
            let mut stickers = [0u8; 48];
            let (corners, edges) = stickers.split_at_mut(24);
            let corner_pieces = cube.corner_perm.iter().zip(&cube.corner_orient);
            for ((&piece, &twist), slots) in corner_pieces.zip(corners.as_chunks_mut::<3>().0) {
                for (k, slot) in (0..).zip(slots) {
                    // piece sticker j sits at position sticker (twist + j) % 3
                    *slot = 3 * piece + (k + 3 - twist) % 3;
                }
            }
            let edge_pieces = cube.edge_perm.iter().zip(&cube.edge_orient);
            for ((&piece, &flip), slots) in edge_pieces.zip(edges.as_chunks_mut::<2>().0) {
                for (k, slot) in (0..).zip(slots) {
                    *slot = 24 + 2 * piece + (k + flip) % 2;
                }
            }
            StickerCube(stickers)
        }
    }

    /// The cube state of each move.
    pub(crate) fn move_cubes() -> Option<[CubieCube; N_MOVES]> {
        let mut moves = [CubieCube::SOLVED; N_MOVES];
        for (mv, cube) in moves.iter_mut().enumerate() {
            *cube = StickerCube::solved()
                .apply_face(mv / 3, mv % 3 + 1)?
                .to_cubie()?;
        }
        Some(moves)
    }

    /// All 48 symmetries; index 0 is the identity.
    pub(crate) fn symmetries() -> Option<Vec<Symmetry>> {
        // Signed permutation matrices: each axis maps to some axis, either way.
        let axis_perms = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let mut matrices: Vec<Matrix> = Vec::with_capacity(48);
        for perm in axis_perms {
            for signs in 0..8 {
                let sign = |i: usize| if signs >> i & 1 == 1 { -1 } else { 1 };
                matrices.push(std::array::from_fn(|i| {
                    std::array::from_fn(|j| if perm[i] == j { sign(i) } else { 0 })
                }));
            }
        }
        // identity first
        let identity = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
        let pos = matrices.iter().position(|m| *m == identity)?;
        matrices.swap(0, pos);
        matrices
            .iter()
            .map(|m| {
                let mirror = matrix_determinant(m) < 0;
                let cube = StickerCube::symmetry_state(m)?.to_cubie()?;
                let mut move_map = [0u8; N_MOVES];
                let faces = move_map.as_chunks_mut::<3>().0;
                for (face_moves, &normal) in faces.iter_mut().zip(&FACE_NORMALS) {
                    let image = matrix_apply(m, normal);
                    let image_face = FACE_NORMALS.iter().zip(0..).find(|&(&x, _)| x == image)?.1;
                    for (turns, image_move) in (0..).zip(face_moves) {
                        // (A mirror turns clockwise into counterclockwise.)
                        let image_turns = if mirror { 2 - turns } else { turns };
                        *image_move = 3 * image_face + image_turns;
                    }
                }
                Some(Symmetry {
                    matrix: *m,
                    mirror,
                    cube,
                    cube_inverse: cube.inverse(),
                    move_map,
                })
            })
            .collect()
    }

    /// Index of the inverse of each symmetry of `syms` (`symmetries()`).
    pub(crate) fn symmetry_inverses(syms: &[Symmetry]) -> Option<[u8; 48]> {
        let identity = syms.first()?.matrix;
        let mut inverses = [0u8; 48];
        for (inverse, sym) in inverses.iter_mut().zip(syms) {
            *inverse = (0..)
                .zip(syms)
                .find(|(_, other)| matrix_product(&sym.matrix, &other.matrix) == identity)?
                .0;
        }
        Some(inverses)
    }

    /// The faces of a facelet string in its order (U R F D L B), each as its
    /// outward normal and the directions of "right" and "down" as the face
    /// is seen from outside: U with B at the top, D with F at the top, the
    /// side faces with U at the top.
    const FACELET_FACES: [(Vec3, Vec3, Vec3); 6] = [
        ([0, 1, 0], [1, 0, 0], [0, 0, 1]),
        ([1, 0, 0], [0, 0, -1], [0, -1, 0]),
        ([0, 0, 1], [1, 0, 0], [0, -1, 0]),
        ([0, -1, 0], [1, 0, 0], [0, 0, -1]),
        ([-1, 0, 0], [0, 0, 1], [0, -1, 0]),
        ([0, 0, -1], [-1, 0, 0], [0, -1, 0]),
    ];

    /// The facelet (0..54: 9 per face, row by row) of a sticker.
    fn facelet(sticker: Sticker) -> Option<u8> {
        let facelets = FACELET_FACES.iter().flat_map(|&(normal, right, down)| {
            (-1..=1).flat_map(move |row| {
                (-1..=1).map(move |column| Sticker {
                    position: std::array::from_fn(|a| {
                        normal[a] + column * right[a] + row * down[a]
                    }),
                    normal,
                })
            })
        });
        (0..)
            .zip(facelets)
            .find(|&(_, x)| x == sticker)
            .map(|(i, _)| i)
    }

    /// The facelet of each sticker of each corner position (in
    /// `corner_stickers` order).
    pub(crate) fn corner_facelets() -> Option<[[u8; 3]; 8]> {
        let mut table = [[0; 3]; 8];
        for (row, &position) in table.iter_mut().zip(&CORNER_POSITIONS) {
            for (x, sticker) in row.iter_mut().zip(corner_stickers(position)) {
                *x = facelet(sticker)?;
            }
        }
        Some(table)
    }

    /// The facelet of each sticker of each edge position (in
    /// `edge_stickers` order).
    pub(crate) fn edge_facelets() -> Option<[[u8; 2]; 12]> {
        let mut table = [[0; 2]; 12];
        for (row, &position) in table.iter_mut().zip(&EDGE_POSITIONS) {
            for (x, sticker) in row.iter_mut().zip(edge_stickers(position)) {
                *x = facelet(sticker)?;
            }
        }
        Some(table)
    }

    /// Reference conjugation through the sticker model (slow; for tests).
    #[cfg(test)]
    pub(crate) fn conjugate_by_stickers(cube: &CubieCube, sym: &Symmetry) -> Option<CubieCube> {
        StickerCube::from_cubie(cube)
            .conjugate(&sym.matrix)?
            .to_cubie()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Facelet strings: known ones (solved, U, R, the superflip as in
    /// Kociemba's examples), round trips with any color symbols, and
    /// rejected non-positions.
    #[test]
    fn facelets() {
        let cube_of = |moves: &str| {
            CubieCube::SOLVED
                .apply_moves(&parse_moves(moves).unwrap())
                .unwrap()
        };
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
            assert_eq!(model::StickerCube::from_cubie(&cube).to_cubie(), Some(cube));
        }
    }

    #[test]
    fn moves_match_sticker_model() {
        let mut rng = Rng(2);
        for _ in 0..50 {
            let mut stickers = model::StickerCube::solved();
            let mut cube = CubieCube::SOLVED;
            for _ in 0..30 {
                let mv = rng.below_usize(N_MOVES);
                stickers = stickers.apply_face(mv / 3, mv % 3 + 1).unwrap();
                cube = cube.apply_move(Move::ALL[mv]);
                assert_eq!(stickers.to_cubie(), Some(cube));
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
            cube = cube.apply_moves(&seq).unwrap();
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
                assert_eq!(
                    Some(conjugate(&cube, sym)),
                    model::conjugate_by_stickers(&cube, sym)
                );
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

    /// The constants from build.rs are the sticker model's.
    #[test]
    fn generated_constants_match_model() {
        assert_eq!(model::move_cubes().as_ref(), Some(move_cubes()));
        let syms = model::symmetries().unwrap();
        assert_eq!(syms.len(), symmetries().len());
        for (a, b) in syms.iter().zip(symmetries()) {
            assert_eq!(
                (a.matrix, a.mirror, a.cube, a.cube_inverse, a.move_map),
                (b.matrix, b.mirror, b.cube, b.cube_inverse, b.move_map)
            );
        }
        assert_eq!(
            model::symmetry_inverses(&syms).as_ref(),
            Some(symmetry_inverses())
        );
        assert_eq!(model::corner_facelets(), Some(CORNER_FACELETS));
        assert_eq!(model::edge_facelets(), Some(EDGE_FACELETS));
        for (mv, index) in Move::ALL.iter().zip(0..) {
            assert_eq!(mv.index(), index);
            assert_eq!(Move::from_index(index), Some(*mv));
            assert_eq!(Move::lowest(1 << index | 1 << 17), Some(*mv));
        }
        assert_eq!(Move::from_index(N_MOVES), None);
        assert_eq!(Move::lowest(0), None);
        assert_eq!(CubieCube::SOLVED.apply_moves(&[0, 18]), None);
        assert_eq!(format_moves(&[0, 18, 17]), "U ? B'");
    }
}
