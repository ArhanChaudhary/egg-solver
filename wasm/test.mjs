#!/usr/bin/env bun
// Smoke test of the built modules (wasm/build.sh) in Bun, run by check.sh:
// both builds generate h0 and solve short scrambles one at a time and as a
// batch; every solution must undo its scramble (solving "scramble +
// solution" gives the empty solution), be no longer than the scramble, and
// agree in length with the other modes; a facelet string must give the cube
// of its moves. A table exported as a file (`exportTable`, kept by the demo
// page in IndexedDB) must load back. The demo page's sticker model
// (cube-model.mjs) must agree with the module: its stickers of a scramble
// are that scramble's cube, and the module's solutions solve its cubes.
// Then the quarter-turn metric: QTM solves on the HTM table and on a QTM h0
// must agree, replay, be no shorter than the HTM ones and have the corner
// permutation's parity (known: R U R' U' 4q, the checkerboard 12q); an HTM
// solve on the QTM table is refused, and the QTM table exports and loads back
// as one. And all solutions up to a length (`solveAll`): the known lists of
// the 6-half-turn checkerboard (6 optimal solutions, 12 more of 8 moves),
// each solution replayed, in order and once; the limits and the cap
// (`truncated`); QTM lengths; what no search reaches is refused.

import { EggSolver } from './egg-solver.mjs';
import * as model from './cube-model.mjs';

const fail = (message) => {
  console.error(`wasm test: ${message}`);
  process.exit(1);
};
const scrambles = [
  "R U R' U'",
  "F2 D L2 B' U",
  "R U2 F' D L2 B R'",
  "U R2 F B R B2 R U2",
  "D' L F2 U' B R2 D F'",
  "L2 D' R U2 B' F' L U R'",
];
const dist = new URL('./dist/', import.meta.url);
for (const [name, threads] of [['egg-solver.wasm', undefined], ['egg-solver-threads.wasm', 3]]) {
  const start = performance.now();
  const solver = await EggSolver.create({ wasm: new URL(name, dist), threads });
  if (solver.exports.egg_solver_has_threads() !== (threads ? 1 : 0)) fail(`${name}: has_threads`);
  const table = await solver.generateTable(0);
  if (table.k !== 0) fail(`${name}: generated h${table.k}`);
  const single = scrambles.map((s) => solver.solve(s));
  const batch = solver.solveMany(scrambles, { lanes: 4 });
  scrambles.forEach((scramble, i) => {
    const { moves, length } = single[i];
    if (batch[i].length !== length) fail(`${name}: ${scramble}: batch ${batch[i].length}, single ${length}`);
    if (length > scramble.split(' ').length) fail(`${name}: ${scramble}: ${length} moves`);
    const check = solver.solve(`${scramble} ${moves}`);
    if (check.length !== 0) fail(`${name}: ${moves} does not solve ${scramble}`);
    const again = solver.solve(`${scramble} ${batch[i].moves}`);
    if (again.length !== 0) fail(`${name}: ${batch[i].moves} does not solve ${scramble}`);
  });
  const facelets = 'UUFUUFUUFRRRRRRRRRFFDFFDFFDDDBDDBDDBLLLLLLLLLUBBUBBUBB';
  if (solver.solve(facelets).moves !== "R'") fail(`${name}: facelets of R`);
  let refused = false;
  try {
    solver.solve('UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBUB');
  } catch {
    refused = true;
  }
  if (!refused) fail(`${name}: a bad facelet string was accepted`);
  // A table file made by exportTable loads back, checksum included, and solves the same.
  const file = solver.exportTable();
  const loaded = await solver.loadTable(new Uint8Array(await file.arrayBuffer()), { verify: true });
  if (loaded.k !== 0 || loaded.bytes !== table.bytes) fail(`${name}: exported table loads as ${JSON.stringify(loaded)}`);
  if (solver.solve(scrambles[2]).length !== single[2].length) fail(`${name}: exported table solves differently`);
  // The page's sticker model against the module.
  for (const [i, text] of scrambles.entries()) {
    const stickers = model.applyMoves(model.SOLVED, model.parseMoves(text));
    const { moves: solution, length } = solver.solve(stickers);
    if (length !== single[i].length) fail(`${name}: model stickers of ${text} differ from its moves`);
    if (!model.isSolved(model.applyMoves(stickers, model.parseMoves(solution)))) fail(`${name}: model: ${solution} does not solve ${text}`);
    if (model.checkStickers([...stickers])) fail(`${name}: model rejects ${text}`);
  }
  // (Random states are too deep for h0 here: the model only checks that they are cubes.)
  for (const text of solver.randomCubes(7, 20)) {
    if (model.checkStickers([...text])) fail(`${name}: model rejects the random cube ${text}`);
  }
  // All solutions up to a length.
  const checkerboard = 'U2 D2 R2 L2 F2 B2';
  const lengths = (list) => list.solutions.map((solution) => solution.length).join(' ');
  const lengthOf = (moves, metric) =>
    moves ? moves.split(' ').reduce((n, m) => n + (metric === 'qtm' && m.endsWith('2') ? 2 : 1), 0) : 0;
  const replays = (scramble, list, options) => {
    const seen = new Set();
    for (const { moves, length } of list.solutions) {
      if (seen.has(moves)) fail(`${name}: solveAll ${scramble}: ${moves} twice`);
      seen.add(moves);
      if (length !== lengthOf(moves, options?.metric)) fail(`${name}: solveAll ${scramble}: ${moves} is not ${length} moves`);
      if (length > list.max_length) fail(`${name}: solveAll ${scramble}: ${moves} is longer than ${list.max_length}`);
      if (solver.solve(`${scramble} ${moves}`, options).length !== 0) fail(`${name}: solveAll: ${moves} does not solve ${scramble}`);
    }
    if (list.count !== list.solutions.length) fail(`${name}: solveAll ${scramble}: count ${list.count}`);
    const counted = list.by_length.reduce((n, entry) => n + entry.count, 0);
    if (counted !== list.count) fail(`${name}: solveAll ${scramble}: by_length ${JSON.stringify(list.by_length)}`);
  };
  const optimal = solver.solveAll(checkerboard);
  replays(checkerboard, optimal);
  if (optimal.count !== 6 || optimal.optimal !== 6 || optimal.max_length !== 6 || optimal.truncated) {
    fail(`${name}: solveAll ${checkerboard}: ${JSON.stringify(optimal)}`);
  }
  if (optimal.solutions[0].moves !== checkerboard || optimal.metric !== 'htm') fail(`${name}: solveAll: ${JSON.stringify(optimal.solutions[0])}`);
  if (lengths(solver.solveAll(checkerboard, { plus: 0 })) !== lengths(optimal)) fail(`${name}: solveAll plus 0`);
  const plus = solver.solveAll(checkerboard, { plus: 2, lanes: 4 });
  replays(checkerboard, plus);
  if (lengths(plus) !== [...Array(6).fill(6), ...Array(12).fill(8)].join(' ') || plus.max_length !== 8) {
    fail(`${name}: solveAll plus 2: ${lengths(plus)}`);
  }
  if (JSON.stringify(plus.by_length) !== '[{"length":6,"count":6},{"length":8,"count":12}]') fail(`${name}: solveAll: ${JSON.stringify(plus.by_length)}`);
  if (lengths(solver.solveAll(checkerboard, { maxLength: 8 })) !== lengths(plus)) fail(`${name}: solveAll maxLength 8`);
  if (solver.solveAll(checkerboard, { plus: 2, maxLength: 7 }).count !== 6) fail(`${name}: solveAll plus 2, maxLength 7`);
  const none = solver.solveAll(checkerboard, { maxLength: 5 });
  if (none.count !== 0 || none.optimal !== null || none.max_length !== 5) fail(`${name}: solveAll maxLength 5: ${JSON.stringify(none)}`);
  const cut = solver.solveAll(checkerboard, { plus: 2, maxSolutions: 7 });
  replays(checkerboard, cut);
  if (!cut.truncated || cut.count !== 7 || cut.complete_length !== 7 || cut.optimal !== 6) {
    fail(`${name}: solveAll maxSolutions 7: ${JSON.stringify({ ...cut, solutions: undefined })}`);
  }
  for (const scramble of scrambles.slice(0, 4)) {
    const list = solver.solveAll(scramble, { plus: 1 });
    replays(scramble, list);
    if (list.optimal !== solver.solve(scramble).length || list.count < 1) fail(`${name}: solveAll ${scramble}: optimal ${list.optimal}`);
  }
  const solved = solver.solveAll('', { maxLength: 9 });
  if (solved.count !== 1 || solved.solutions[0].moves !== '' || solved.optimal !== 0) fail(`${name}: solveAll of the solved cube: ${JSON.stringify(solved)}`);
  const quarter = solver.solveAll('U2 D2 R2 L2', { metric: 'qtm' });
  replays('U2 D2 R2 L2', quarter, { metric: 'qtm' });
  if (quarter.count !== 2 || quarter.optimal !== 8 || quarter.metric !== 'qtm' || lengths(quarter) !== '8 8') {
    fail(`${name}: solveAll in QTM: ${JSON.stringify(quarter)}`);
  }
  for (const options of [{ maxLength: 21 }, { metric: 'qtm', maxLength: 27 }, { plus: -1 }, { plus: 1.5 }, { maxSolutions: 0 }]) {
    let refused = false;
    try {
      solver.solveAll(checkerboard, options);
    } catch {
      refused = true;
    }
    if (!refused) fail(`${name}: solveAll accepted ${JSON.stringify(options)}`);
  }
  // Quarter-turn metric.
  const qtmLength = (moves) =>
    moves ? moves.split(' ').reduce((n, m) => n + (m.endsWith('2') ? 2 : 1), 0) : 0;
  const parity = (scramble) => qtmLength(scramble) % 2;
  const known = [["R U R' U'", 4], ['U2 D2 F2 B2 L2 R2', 12]];
  const qtmScrambles = [...scrambles, ...known.map(([s]) => s)];
  const onHtm = qtmScrambles.map((s) => solver.solve(s, { metric: 'qtm' }));
  const onHtmMany = solver.solveMany(qtmScrambles, { lanes: 4, metric: 'qtm' });
  const qtmTable = await solver.generateTable(0, 0, { metric: 'qtm' });
  if (qtmTable.metric !== 'qtm') fail(`${name}: generated a ${qtmTable.metric} table`);
  const onQtm = qtmScrambles.map((s) => solver.solve(s));
  qtmScrambles.forEach((scramble, i) => {
    const { moves, length, metric } = onQtm[i];
    if (metric !== 'qtm' || onHtm[i].metric !== 'qtm') fail(`${name}: ${scramble}: metric ${metric}`);
    if (length !== qtmLength(moves)) fail(`${name}: ${scramble}: ${moves} is not ${length}q`);
    if (onHtm[i].length !== length || onHtmMany[i].length !== length) {
      fail(`${name}: ${scramble}: ${onHtm[i].length}q / ${onHtmMany[i].length}q on h0, ${length}q on qtm h0`);
    }
    if (i < scrambles.length && length < single[i].length) fail(`${name}: ${scramble}: ${length}q < HTM`);
    if (length % 2 !== parity(scramble)) fail(`${name}: ${scramble}: ${length}q has the wrong parity`);
    const want = known.find(([s]) => s === scramble)?.[1];
    if (want !== undefined && length !== want) fail(`${name}: ${scramble}: ${length}q, not ${want}q`);
    if (solver.solve(`${scramble} ${moves}`).length !== 0) fail(`${name}: ${moves} does not solve ${scramble}`);
  });
  let htmOnQtm = false;
  try {
    solver.solve(scrambles[0], { metric: 'htm' });
  } catch {
    htmOnQtm = true;
  }
  if (!htmOnQtm) fail(`${name}: an HTM solve on a QTM table was accepted`);
  // A QTM table exported as a file loads back as a QTM table.
  const qtmFile = solver.exportTable();
  const qtmLoaded = await solver.loadTable(new Uint8Array(await qtmFile.arrayBuffer()), { verify: true });
  if (qtmLoaded.metric !== 'qtm') fail(`${name}: an exported QTM table loads as ${JSON.stringify(qtmLoaded)}`);
  if (solver.solve(qtmScrambles[1]).length !== onQtm[1].length) fail(`${name}: exported QTM table solves differently`);
  solver.close();
  console.log(`wasm test: ${name} (${solver.threads} thread(s)) ok in ${((performance.now() - start) / 1000).toFixed(1)} s`);
}
