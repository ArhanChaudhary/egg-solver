// The cube as its 54 stickers, for the demo page: moves, scrambles, the
// check of a painted cube, and the steps of a solution. The solver itself
// (the WebAssembly module) never depends on this file; the page uses it to
// draw cubes, explain mistakes and verify solutions independently.
//
// Sticker order (a "facelet string"): faces U R F D L B, nine stickers each,
// read row by row as the face is seen from outside, the order of
// egg-solver's facelet strings and of Kociemba's solvers. U is seen with B at
// the top, D with F at the top, and R, F, L, B with U at the top. Laid out
// as the usual net (U above F; L, F, R, B in a row; D below F), neighbouring
// stickers of the net touch on the cube.
//
// Moves: U R F D L B turn that face a quarter turn clockwise as seen from
// outside, X' anticlockwise, X2 half; X3 and X1 are accepted as X' and X,
// as in egg-solver's parser.

export const FACES = ['U', 'R', 'F', 'D', 'L', 'B'];
export const SOLVED = FACES.map((f) => f.repeat(9)).join('');

// For each face: its outward normal, then the directions in which the
// picture's columns (left to right) and rows (top to bottom) run, with x
// pointing to R, y to U and z to F.
const FRAMES = [
  [[0, 1, 0], [1, 0, 0], [0, 0, 1]], // U: B at the top
  [[1, 0, 0], [0, 0, -1], [0, -1, 0]], // R
  [[0, 0, 1], [1, 0, 0], [0, -1, 0]], // F
  [[0, -1, 0], [1, 0, 0], [0, 0, -1]], // D: F at the top
  [[-1, 0, 0], [0, 0, 1], [0, -1, 0]], // L
  [[0, 0, -1], [-1, 0, 0], [0, -1, 0]], // B
];

const add = (a, b) => a.map((x, i) => x + b[i]);
const scale = (a, s) => a.map((x) => x * s);
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];

/** Each sticker's cubie position (coordinates -1, 0, 1) and outward normal. */
export const STICKERS = [];
for (let face = 0; face < 6; face++) {
  const [normal, col, row] = FRAMES[face];
  for (let i = 0; i < 9; i++) {
    const r = Math.floor(i / 3) - 1;
    const c = (i % 3) - 1;
    STICKERS.push({ position: add(normal, add(scale(col, c), scale(row, r))), normal });
  }
}
const where = new Map(STICKERS.map((s, i) => [`${s.position}|${s.normal}`, i]));

/** A quarter turn clockwise about the outward `axis` (seen from outside). */
const turn = (axis, v) => add(scale(axis, dot(axis, v)), scale(cross(axis, v), -1));

/**
 * `QUARTER[face][i]`: where a clockwise quarter turn of `face` takes sticker
 * i (stickers off the layer stay).
 */
const QUARTER = FRAMES.map(([axis]) => STICKERS.map((s, i) => {
  if (dot(s.position, axis) !== 1) return i;
  return where.get(`${turn(axis, s.position)}|${turn(axis, s.normal)}`);
}));

/** A move: { face (0..5), turns (1, 2 or 3 quarter turns clockwise) }. */
export function moveName({ face, turns }) {
  return FACES[face] + ['', '', '2', "'"][turns];
}

/**
 * Parse moves as egg-solver does: face letters with an optional suffix
 * (2, ', 3 or 1), spaces optional. Returns null if the text has anything
 * else.
 */
export function parseMoves(text) {
  const moves = [];
  const chars = [...text];
  for (let i = 0; i < chars.length; i++) {
    const ch = chars[i];
    if (/\s/.test(ch)) continue;
    const face = FACES.indexOf(ch);
    if (face < 0) return null;
    const suffix = chars[i + 1];
    let turns = 1;
    if (suffix === '2') turns = 2;
    else if (suffix === "'" || suffix === '3') turns = 3;
    if ('123\''.includes(suffix ?? 'x')) i++;
    moves.push({ face, turns });
  }
  return moves;
}

export const formatMoves = (moves) => moves.map(moveName).join(' ');

/** The moves that undo `moves`: reversed, each turned the other way. */
export function invertMoves(moves) {
  return moves.slice().reverse().map(({ face, turns }) => ({ face, turns: 4 - turns }));
}

/** The stickers (a 54-character string) after `move`. */
export function applyMove(stickers, { face, turns }) {
  let s = [...stickers];
  for (let t = 0; t < turns; t++) {
    const out = s.slice();
    QUARTER[face].forEach((to, from) => { out[to] = s[from]; });
    s = out;
  }
  return s.join('');
}

export const applyMoves = (stickers, moves) => moves.reduce(applyMove, stickers);

/** True if every face shows one colour (the cube is solved, held any way). */
export function isSolved(stickers) {
  for (let f = 0; f < 6; f++) {
    for (let i = 0; i < 9; i++) if (stickers[9 * f + i] !== stickers[9 * f + 4]) return false;
  }
  return true;
}

// --- The pieces, for checking a painted cube ---

/** Stickers of each corner and edge position, clockwise for corners (seen from outside). */
const CORNERS = [];
const EDGES = [];
{
  const seen = new Set();
  STICKERS.forEach(({ position }) => {
    const key = `${position}`;
    if (seen.has(key)) return;
    seen.add(key);
    const at = STICKERS.map((s, i) => i).filter((i) => `${STICKERS[i].position}` === key);
    if (at.length === 3) {
      // Order: the U or D sticker first, then clockwise around the corner.
      const first = at.find((i) => STICKERS[i].normal[1] !== 0);
      const others = at.filter((i) => i !== first);
      const [a, b] = others;
      // (U, R, F is clockwise at URF: n_U x n_R points into the cube there; rotations keep this)
      const clockwise = dot(cross(STICKERS[first].normal, STICKERS[a].normal), position) < 0;
      CORNERS.push(clockwise ? [first, a, b] : [first, b, a]);
    } else if (at.length === 2) {
      // The U/D sticker first, else the F/B one.
      const rank = (i) => (STICKERS[i].normal[1] !== 0 ? 0 : STICKERS[i].normal[2] !== 0 ? 1 : 2);
      EDGES.push(at.sort((x, y) => rank(x) - rank(y)));
    }
  });
}
export { CORNERS, EDGES };

const parity = (perm) => {
  let p = 0;
  const seen = perm.map(() => false);
  for (let i = 0; i < perm.length; i++) {
    if (seen[i]) continue;
    let len = 0;
    for (let j = i; !seen[j]; j = perm[j]) { seen[j] = true; len++; }
    p ^= (len - 1) & 1;
  }
  return p;
};

/**
 * Check painted stickers (a 54-entry array of colour letters, or null for a
 * blank). Returns null if they make a cube that can be solved, else
 * { kind, message, stickers } with the sticker indices concerned:
 * kind 'blank', 'counts' (with counts: [[colour, n], ...]), 'centres',
 * 'piece', 'twist', 'flip' or 'parity'. The last four also have `reason`,
 * 0 to 7 in the order of this function's messages (a page with wording of
 * its own picks by it): a corner in mirror order, a corner or an edge with
 * colours that none has, the same corner or edge twice, a twisted corner, a
 * flipped edge, two swapped pieces.
 */
export function checkStickers(cells) {
  const blanks = cells.map((c, i) => (c ? -1 : i)).filter((i) => i >= 0);
  if (blanks.length) {
    return { kind: 'blank', message: `${blanks.length} sticker${blanks.length === 1 ? ' is' : 's are'} still blank`, stickers: blanks };
  }
  const counts = new Map();
  for (const c of cells) counts.set(c, (counts.get(c) ?? 0) + 1);
  const wrong = [...counts].filter(([, n]) => n !== 9);
  if (wrong.length || counts.size !== 6) {
    return {
      kind: 'counts',
      counts: [...counts].sort((a, b) => b[1] - a[1]),
      message: 'each colour should be on exactly 9 stickers',
      stickers: [],
    };
  }
  const centres = FACES.map((_, f) => cells[9 * f + 4]);
  if (new Set(centres).size !== 6) {
    return { kind: 'centres', message: 'two centres have the same colour', stickers: FACES.map((_, f) => 9 * f + 4) };
  }
  const faceOf = (i) => centres.indexOf(cells[i]);
  // Which corner piece (by its home position) is at each position, and its twist.
  const homeCorner = CORNERS.map((stickers) => stickers.map((i) => Math.floor(i / 9)));
  const cornerPerm = [];
  let twist = 0;
  for (const [pos, stickers] of CORNERS.entries()) {
    const faces = stickers.map(faceOf);
    let found = -1;
    let ori = 0;
    for (const [piece, home] of homeCorner.entries()) {
      for (let o = 0; o < 3; o++) {
        if (home.every((f, k) => f === faces[(k + o) % 3])) { found = piece; ori = o; }
      }
    }
    if (found < 0) {
      const mirrored = homeCorner.some((home) => [...home].sort().join() === [...faces].sort().join());
      return {
        kind: 'piece',
        reason: mirrored ? 0 : 1,
        message: mirrored
          ? 'a corner has its colours in the wrong order around it (a mirror image of a real corner)'
          : 'a corner has colours that no corner has',
        stickers,
      };
    }
    cornerPerm[pos] = found;
    twist += ori;
  }
  const homeEdge = EDGES.map((stickers) => stickers.map((i) => Math.floor(i / 9)));
  const edgePerm = [];
  let flip = 0;
  for (const [pos, stickers] of EDGES.entries()) {
    const faces = stickers.map(faceOf);
    let found = -1;
    let ori = 0;
    for (const [piece, home] of homeEdge.entries()) {
      if (home[0] === faces[0] && home[1] === faces[1]) { found = piece; ori = 0; }
      if (home[0] === faces[1] && home[1] === faces[0]) { found = piece; ori = 1; }
    }
    if (found < 0) return { kind: 'piece', reason: 2, message: 'an edge has colours that no edge has', stickers };
    edgePerm[pos] = found;
    flip += ori;
  }
  const twice = (perm, list) => {
    const at = new Map();
    for (const [pos, piece] of perm.entries()) {
      if (at.has(piece)) return [...list[at.get(piece)], ...list[pos]];
      at.set(piece, pos);
    }
    return null;
  };
  const dupCorner = twice(cornerPerm, CORNERS);
  if (dupCorner) return { kind: 'piece', reason: 3, message: 'the same corner appears twice', stickers: dupCorner };
  const dupEdge = twice(edgePerm, EDGES);
  if (dupEdge) return { kind: 'piece', reason: 4, message: 'the same edge appears twice', stickers: dupEdge };
  if (twist % 3) return { kind: 'twist', reason: 5, message: 'one corner is twisted (turned in place)', stickers: CORNERS.flat() };
  if (flip % 2) return { kind: 'flip', reason: 6, message: 'one edge is flipped', stickers: EDGES.flat() };
  if (parity(cornerPerm) !== parity(edgePerm)) {
    return { kind: 'parity', reason: 7, message: 'two pieces are swapped (as if taken out and put back the wrong way)', stickers: [] };
  }
  return null;
}
