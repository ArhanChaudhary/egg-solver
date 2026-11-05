// A thread of the threads build (wasi-threads): a pool worker of
// `cubeopt.mjs`. It instantiates the module on the shared memory, then waits
// in its control slot; `thread-spawn` (in any thread) hands it a thread,
// which it runs with `wasi_thread_start`, and it waits again.

import { wasiImports, threadSpawn, slotBase, SLOT_IDLE, SLOT_RUN } from './cubeopt.mjs';

const isNode = typeof process !== 'undefined' && process.versions?.node !== undefined;
const port = isNode ? (await import('node:worker_threads')).parentPort : self;
const post = (message) => port.postMessage(message);
const fs = isNode ? await import('node:fs') : null;

/**
 * A thread that traps (a Rust panic aborts) never tells the threads waiting
 * for it: they would wait forever. Say so at once (a Node worker's console
 * goes through the main thread, which may be the one waiting) and, in Node,
 * end the process.
 */
function fatal(message) {
  if (isNode) {
    fs.writeSync(2, `${message}\n`);
    process.kill(process.pid, 'SIGTERM');
  } else {
    console.error(message);
  }
}

async function serve({ module, memory, control, slot }) {
  let instance;
  try {
    instance = await WebAssembly.instantiate(module, {
      env: { memory },
      wasi: { 'thread-spawn': threadSpawn(control) },
      wasi_snapshot_preview1: wasiImports(() => memory, module),
    });
  } catch (e) {
    post(`cubeopt thread: ${e.message}`);
    return;
  }
  const base = slotBase(slot);
  Atomics.store(control, base, SLOT_IDLE);
  post('ready');
  for (;;) {
    let state;
    while ((state = Atomics.load(control, base)) !== SLOT_RUN) {
      Atomics.wait(control, base, state);
    }
    const tid = Atomics.load(control, base + 1);
    const startArg = Atomics.load(control, base + 2);
    try {
      instance.exports.wasi_thread_start(tid, startArg);
    } catch (e) {
      fatal(`cubeopt: thread ${tid} failed (${e.message}); the module cannot go on`);
      return;
    }
    Atomics.store(control, base, SLOT_IDLE);
  }
}

if (isNode) {
  port.once('message', serve);
} else {
  self.onmessage = (e) => serve(e.data);
}
