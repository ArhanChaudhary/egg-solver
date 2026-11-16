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
// as one.

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
