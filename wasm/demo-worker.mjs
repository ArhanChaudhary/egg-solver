// The browser demo's solver (index.html): runs the WebAssembly module in a
// worker, so the page stays responsive and the threads build may block.
// Messages in: { id, cmd, ... }; out: { id, result } or { id, error }, and
// { progress } while a table loads.

import { EggSolver } from './egg-solver.mjs';

let solver = null;

const commands = {
  async init({ threads }) {
    solver?.close();
    const wasm = new URL(threads > 1 ? 'dist/egg-solver-threads.wasm' : 'dist/egg-solver.wasm', import.meta.url);
    solver = await EggSolver.create({ wasm, threads: threads > 1 ? threads : undefined });
    return { threads: solver.threads };
  },
  generate({ k, base }) {
    return solver.generateTable(k, base);
  },
  load({ source, verify }) {
    const onProgress = (done, size) => postMessage({ progress: done / size });
    return solver.loadTable(source, { verify, onProgress });
  },
  solve({ cube, lanes }) {
    const start = performance.now();
    const result = solver.solve(cube, { lanes });
    return { ...result, wall: (performance.now() - start) / 1000 };
  },
  random({ seed }) {
    return solver.randomCubes(seed, 1)[0];
  },
};

onmessage = async ({ data: { id, cmd, ...args } }) => {
  try {
    postMessage({ id, result: await commands[cmd](args) });
  } catch (e) {
    postMessage({ id, error: e.message });
  }
};
