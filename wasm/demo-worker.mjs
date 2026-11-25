// The browser demo's solver (index.html): runs the WebAssembly module in a
// worker, so the page stays responsive and the threads build may block.
// Messages in: { id, cmd, ... }; out: { id, result } or { id, error }, and
// { progress } while a table file loads. A solve cannot be interrupted: the
// page stops one by ending the worker and starting a new one. A list of
// solutions (`solveAll`) can: the page sets the cancel word of `init`'s
// `cancelHandle` (the threads build), and the list returns what it found.

import { EggSolver } from './egg-solver.mjs';

let solver = null;

const commands = {
  async init({ threads }) {
    solver?.close();
    const wasm = new URL(threads > 1 ? 'dist/egg-solver-threads.wasm' : 'dist/egg-solver.wasm', import.meta.url);
    solver = await EggSolver.create({ wasm, threads: threads > 1 ? threads : undefined });
    // (`cancelHandle`: shared memory of the threads build, with which the page stops a list; null otherwise.)
    return { threads: solver.threads, cancelHandle: solver.cancelHandle };
  },
  async generate({ k, base, metric }) {
    const start = performance.now();
    const table = await solver.generateTable(k, base, { metric });
    return { ...table, seconds: (performance.now() - start) / 1000 };
  },
  load({ source, verify }) {
    const onProgress = (done, size) => postMessage({ progress: done / size });
    return solver.loadTable(source, { verify, onProgress });
  },
  /** The table as a file (a Blob), to keep in IndexedDB. */
  export() {
    return solver.exportTable();
  },
  solve({ cube, lanes, metric }) {
    const start = performance.now();
    const result = solver.solve(cube, { lanes, metric });
    return { ...result, wall: (performance.now() - start) / 1000 };
  },
  /** Every solution of at most `plus` moves more than the shortest (0: every shortest one). */
  solveAll({ cube, plus, maxSolutions, timeLimit, metric }) {
    const start = performance.now();
    const result = solver.solveAll(cube, { plus, maxSolutions, timeLimit, metric });
    return { ...result, wall: (performance.now() - start) / 1000 };
  },
};

onmessage = async ({ data: { id, cmd, ...args } }) => {
  try {
    postMessage({ id, result: await commands[cmd](args) });
  } catch (e) {
    postMessage({ id, error: e.message });
  }
};
