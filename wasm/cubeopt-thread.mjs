// A thread of the threads build (wasi-threads): a pool worker of
// `cubeopt.mjs`. It instantiates the module on the shared memory, then waits
// in its control slot; `thread-spawn` (in any thread) hands it a thread,
// which it runs with `wasi_thread_start`, and it waits again.

import { wasiImports, threadSpawn, slotBase, SLOT_IDLE, SLOT_RUN } from './cubeopt.mjs';

const post = (message) => self.postMessage(message);

/**
 * A thread that traps (a Rust panic aborts) never tells the threads waiting
 * for it: they would wait forever. Say so, and in Bun end the process (a page
 * stays stuck: its solver worker may be the one waiting).
 */
function fatal(message) {
  console.error(message);
  globalThis.process?.kill(globalThis.process.pid, 'SIGTERM');
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

self.onmessage = (e) => serve(e.data);
