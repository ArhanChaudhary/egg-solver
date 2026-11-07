#!/usr/bin/env bun
// Solve cubes with the WebAssembly build in Bun.
//
//   bun wasm/demo.mjs [options] < scrambles       one scramble or facelet string per line
//   bun wasm/demo.mjs --random 20:2026 [options]  random states (as `bench random:20:2026`)
//
// Options:
//   --table <file>    a table file (`gen <k> auto <file>`); or
//   --gen <k>[:base]  generate hk in memory first (h0..h4: seconds to minutes)
//   --threads <n>     use dist/egg-solver-threads.wasm with n threads
//   --lanes <n>       lanes per thread (default 16)
//   --batch           solve all cubes together (throughput: each lane its own
//                     cube) instead of one at a time (latency)
//   --bench W:S       stream throughput instead (as the native `bench <table>
//                     stream W S`): random cubes for S seconds after W
//   --seed <n>        the stream's seed (default 2026)
//   --verify          check the table's checksum when loading
//   --wasm <file>     the module (default: dist/egg-solver.wasm, or
//                     dist/egg-solver-threads.wasm with --threads)
//
// Prints one line per cube (length, time, nodes, solution), then totals.

import { EggSolver } from './egg-solver.mjs';

const args = process.argv.slice(2);
const option = (name) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : undefined;
};
const threads = option('--threads') ? Number(option('--threads')) : undefined;
const lanes = Number(option('--lanes') ?? 0);
const dist = new URL('./dist/', import.meta.url);
const wasm = option('--wasm') ?? new URL(threads ? 'egg-solver-threads.wasm' : 'egg-solver.wasm', dist);

let start = performance.now();
const solver = await EggSolver.create({ wasm, threads });
console.error(`module ready in ${((performance.now() - start) / 1000).toFixed(2)} s, ${solver.threads} thread(s)`);

start = performance.now();
let table;
if (option('--table')) {
  table = await solver.loadTable(option('--table'), { verify: args.includes('--verify') });
} else if (option('--gen')) {
  const [k, base] = option('--gen').split(':').map(Number);
  table = await solver.generateTable(k, base ?? 0);
} else {
  console.error('usage: demo.mjs (--table <file> | --gen <k>[:base]) [--threads n] [--lanes n] [--batch] [--random N:SEED] < cubes');
  process.exit(2);
}
console.error(`table h${table.k} base ${table.base} (${(table.bytes / 2 ** 30).toFixed(2)} GiB) ready in ${((performance.now() - start) / 1000).toFixed(1)} s`);


if (option('--bench')) {
  const [warmup, window] = option('--bench').split(':').map(Number);
  const r = solver.bench({ warmup, window, lanes, seed: Number(option('--seed') ?? 2026) });
  console.log(`threads ${r.threads} lanes ${r.lanes} window ${window}s`);
  console.log(`nodes/cube ${r.nodes_per_cube} lookups/cube ${r.lookups_per_cube}`);
  console.log(`throughput ${r.solves_per_second.toFixed(3)}`);
  solver.close();
  process.exit(0);
}

let cubes;
if (option('--random')) {
  const [count, seed] = option('--random').split(':').map(Number);
  cubes = solver.randomCubes(seed, count);
} else {
  cubes = (await Bun.stdin.text()).split('\n').filter((l) => l.trim() !== '');
}

start = performance.now();
let results;
if (args.includes('--batch')) {
  results = solver.solveMany(cubes, { lanes });
} else {
  results = cubes.map((cube, index) => {
    const t = performance.now();
    const result = { ...solver.solve(cube, { lanes }), index, wall: (performance.now() - t) / 1000 };
    return result;
  });
}
const wall = (performance.now() - start) / 1000;
for (const r of results) {
  const time = r.wall ?? r.seconds;
  console.log(`${r.index} length ${r.length} time ${time.toFixed(3)} s nodes ${r.nodes} ${r.moves}`);
}
const n = Math.max(1, results.length);
const nodes = results.reduce((sum, r) => sum + r.nodes, 0);
const lookups = results.reduce((sum, r) => sum + r.lookups, 0);
const lengths = {};
for (const r of results) lengths[r.length] = (lengths[r.length] ?? 0) + 1;
console.log(`cubes ${results.length} wall ${wall.toFixed(3)} s solves/s ${(results.length / wall).toFixed(3)} ` +
  `nodes/cube ${Math.round(nodes / n)} lookups/cube ${Math.round(lookups / n)} ` +
  `Mnodes/s ${(nodes / wall / 1e6).toFixed(1)}`);
console.log(`lengths ${Object.entries(lengths).map(([l, c]) => `${l}:${c}`).join(' ')}`);
solver.close();
