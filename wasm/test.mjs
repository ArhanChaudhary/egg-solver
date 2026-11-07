#!/usr/bin/env bun
// Smoke test of the built modules (wasm/build.sh) in Bun, run by check.sh:
// both builds generate h0 and solve short scrambles one at a time and as a
// batch; every solution must undo its scramble (solving "scramble +
// solution" gives the empty solution), be no longer than the scramble, and
// agree in length with the other modes; a facelet string must give the cube
// of its moves.

import { EggSolver } from './egg-solver.mjs';

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
  solver.close();
  console.log(`wasm test: ${name} (${solver.threads} thread(s)) ok in ${((performance.now() - start) / 1000).toFixed(1)} s`);
}
