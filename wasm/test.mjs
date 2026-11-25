#!/usr/bin/env bun
// Smoke test of the built modules (wasm/build.sh) in Bun, run by check.sh:
// both builds generate h0 and solve short scrambles one at a time and as a
// batch; every solution must undo its scramble (solving "scramble +
// solution" gives the empty solution), be no longer than the scramble, and
// agree in length with the other modes; a facelet string must give the cube
// of its moves. A table exported as a file (`exportTable`, kept by the demo
// page in IndexedDB) must load back. The demo page's sticker model
// (cube-model.mjs) must agree with the module: its stickers of a scramble
// are that scramble's cube, and the module's solutions solve its cubes; and
// the page's 3D picture (cube-3d.mjs) must turn layers as the model does.
// The progress words of the threads build (`progress`) are readable and
// sensible after a table and after a solve.
// Then the quarter-turn metric: QTM solves on the HTM table and on a QTM h0
// must agree, replay, be no shorter than the HTM ones and have the corner
// permutation's parity (known: R U R' U' 4q, the checkerboard 12q); an HTM
// solve on the QTM table is refused, and the QTM table exports and loads back
// as one. And all solutions up to a length (`solveAll`): the known lists of
// the 6-half-turn checkerboard (6 optimal solutions, 12 more of 8 moves),
// each solution replayed, in order and once; the limits and the cap
// (`truncated`); QTM lengths; what no search reaches is refused. A list is
// stopped by `timeLimit`, and on the threads build by `cancel` from another
// thread (a worker here, as a page's main thread would while its solver
// worker is blocked): it returns what was found, marked `stopped`.

import { EggSolver, cancel, progress } from './egg-solver.mjs';
import * as model from './cube-model.mjs';
import { NORMALS, rotate, turnAngle } from './cube-3d.mjs';

const fail = (message) => {
  console.error(`wasm test: ${message}`);
  process.exit(1);
};
// The page's 3D picture against its sticker model: turning a layer's stickers by `turnAngle`
// about the face's normal puts every sticker where the model's move puts its colour.
{
  const key = (p, n) => `${p.map(Math.round)}|${n.map(Math.round)}`;
  const slot = new Map(model.STICKERS.map((s, i) => [key(s.position, s.normal), i]));
  const labels = [...Array(54).keys()].map((i) => String.fromCharCode(48 + i)).join('');
  for (let face = 0; face < 6; face++) {
    for (const turns of [1, 2, 3]) {
      const after = model.applyMove(labels, { face, turns });
      const [axis, angle] = [NORMALS[face], turnAngle({ turns })];
      model.STICKERS.forEach((s, i) => {
        const inLayer = s.position.reduce((sum, v, a) => sum + v * axis[a], 0) === 1;
        const to = inLayer ? slot.get(key(rotate(s.position, axis, angle), rotate(s.normal, axis, angle))) : i;
        if (after[to] !== labels[i]) fail(`3D picture: ${model.moveName({ face, turns })} takes sticker ${i} to ${to}, not where the model does`);
      });
    }
  }
}
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
  // The progress words (the threads build: its memory is shared): a finished table is all
  // done, and after a solve they hold its last iteration.
  const watch = solver.progressHandle;
  if ((watch !== null) !== Boolean(threads)) fail(`${name}: progressHandle ${JSON.stringify(watch)}`);
  if (progress(null) !== null) fail(`${name}: progress without a handle`);
  if (watch && progress(watch).generated !== 1) fail(`${name}: a finished table is at ${progress(watch).generated}`);
  const single = scrambles.map((s) => solver.solve(s));
  if (watch) {
    const at = progress(watch);
    const last = single.at(-1).length;
    if (at.target < last || at.target > 20 || at.lengths < 1 || at.jobs < 1 || at.claimed < 1 || at.claimed > at.jobs) {
      fail(`${name}: progress after a ${last}-move solve: ${JSON.stringify(at)}`);
    }
  }
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
  // A list stopped by the clock: what was found, marked; a limit that is
  // not reached changes nothing.
  const long = "R U F2 D L' B";
  const timed = solver.solveAll(long, { maxLength: 16, timeLimit: 0.2 });
  replays(long, timed);
  if (timed.stopped !== 'time' || timed.truncated || timed.optimal !== 6 || timed.complete_length >= 16 || timed.seconds > 20) {
    fail(`${name}: solveAll timeLimit: ${JSON.stringify({ ...timed, solutions: undefined })}`);
  }
  const whole = solver.solveAll(long, { maxLength: timed.complete_length });
  const shortOnes = (list) => list.solutions.filter((solution) => solution.length <= timed.complete_length).map((solution) => solution.moves).join('|');
  if (whole.stopped !== null || shortOnes(timed) !== shortOnes(whole)) fail(`${name}: solveAll timeLimit: not complete up to ${timed.complete_length}`);
  const roomy = solver.solveAll(checkerboard, { plus: 2, timeLimit: 3600 });
  if (roomy.stopped !== null || lengths(roomy) !== lengths(plus)) fail(`${name}: solveAll with a roomy timeLimit: ${lengths(roomy)}`);
  // Cancelled from another thread (the threads build: its memory is shared).
  const handle = solver.cancelHandle;
  if ((handle !== null) !== Boolean(threads)) fail(`${name}: cancelHandle ${JSON.stringify(handle)}`);
  if (cancel(handle) !== Boolean(threads)) fail(`${name}: cancel(handle)`);
  // (A cancel without a running list does nothing: the next one starts afresh.)
  if (solver.solveAll(checkerboard).count !== 6) fail(`${name}: solveAll after an idle cancel`);
  if (handle) {
    const code = `import { cancel } from ${JSON.stringify(new URL('./egg-solver.mjs', import.meta.url).href)};
      self.onmessage = ({ data }) => setTimeout(() => self.postMessage(cancel(data.handle)), data.delay);
      self.postMessage('ready');`;
    const canceller = new Worker(URL.createObjectURL(new Blob([code], { type: 'text/javascript' })), { type: 'module' });
    await new Promise((resolve, reject) => {
      canceller.onmessage = resolve;
      canceller.onerror = (e) => reject(new Error(e.message));
    });
    canceller.postMessage({ handle, delay: 400 });
    // (Blocks this thread, as a page's worker is blocked; minutes if not cancelled.)
    const cancelled = solver.solveAll(long, { maxLength: 17, maxSolutions: 100000000 });
    canceller.terminate();
    replays(long, cancelled);
    if (cancelled.stopped !== 'cancelled' || cancelled.optimal !== 6 || cancelled.count < 4 || cancelled.seconds > 60) {
      fail(`${name}: solveAll cancelled: ${JSON.stringify({ ...cancelled, solutions: undefined })}`);
    }
    const after = solver.solveAll(checkerboard, { plus: 2 });
    if (after.stopped !== null || lengths(after) !== lengths(plus)) fail(`${name}: solveAll after a cancel: ${lengths(after)}`);
  }
  for (const options of [{ maxLength: 21 }, { metric: 'qtm', maxLength: 27 }, { plus: -1 }, { plus: 1.5 }, { maxSolutions: 0 }, { timeLimit: 0 }, { timeLimit: 'soon' }]) {
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
