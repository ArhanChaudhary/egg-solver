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
  solver.close();
  console.log(`wasm test: ${name} (${solver.threads} thread(s)) ok in ${((performance.now() - start) / 1000).toFixed(1)} s`);
}
