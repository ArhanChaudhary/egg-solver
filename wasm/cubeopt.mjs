// cubeopt in WebAssembly: a JavaScript wrapper for Node and browsers.
//
//   import { Cubeopt } from './cubeopt.mjs';
//   const solver = await Cubeopt.create({ wasm: 'dist/cubeopt.wasm' });
//   await solver.loadTable('/path/to/h6');        // or await generateTable(4)
//   solver.solve("R U R' U' F2 D L2");             // { moves, length, nodes, ... }
//
// The module is built for WASI (wasm32-wasip1, or wasm32-wasip1-threads for
// several threads): this file provides the few WASI functions it imports
// (clock, console output, sleep), and for the threads build the
// `thread-spawn` of wasi-threads with a pool of workers (`cubeopt-thread.mjs`)
// on a shared memory. In browsers the threads build needs a cross-origin
// isolated page (COOP/COEP headers, see `serve.mjs`) and must run in a worker
// (the main thread may not block). See README.md, "WebAssembly".

const isNode = typeof process !== 'undefined' && process.versions?.node !== undefined;

const ERRNO_SUCCESS = 0;
const ERRNO_AGAIN = 6;
const ERRNO_BADF = 8;
const ERRNO_NOSYS = 52;

// Thread pool slots in the control array (Int32Array): word 0 is the last
// thread id; slot i is words 4 + 4i .. 8 + 4i: state, thread id, argument.
const SLOT_STARTING = 0;
const SLOT_IDLE = 1;
const SLOT_CLAIMED = 2;
const SLOT_RUN = 3;
const slotBase = (i) => 4 + 4 * i;

/** A copy of wasm memory bytes (TextDecoder and others refuse shared views). */
function copyOut(memory, ptr, len) {
  const copy = new Uint8Array(len);
  copy.set(new Uint8Array(memory.buffer, ptr, len));
  return copy;
}

/**
 * The WASI preview 1 functions the module imports (others answer ENOSYS):
 * no files, no arguments or environment; stdout and stderr go to the
 * console, line by line. (Addresses arrive as signed i32: `>>> 0` makes
 * those from 2 GiB up positive.)
 */
export function wasiImports(getMemory, module) {
  const lines = { 1: '', 2: '' };
  const view = () => new DataView(getMemory().buffer);
  const implemented = {
    args_sizes_get(countPtr, sizePtr) {
      [countPtr, sizePtr] = [countPtr >>> 0, sizePtr >>> 0];
      view().setUint32(countPtr, 0, true);
      view().setUint32(sizePtr, 0, true);
      return ERRNO_SUCCESS;
    },
    args_get: () => ERRNO_SUCCESS,
    environ_sizes_get(countPtr, sizePtr) {
      [countPtr, sizePtr] = [countPtr >>> 0, sizePtr >>> 0];
      view().setUint32(countPtr, 0, true);
      view().setUint32(sizePtr, 0, true);
      return ERRNO_SUCCESS;
    },
    environ_get: () => ERRNO_SUCCESS,
    clock_time_get(id, _precision, outPtr) {
      outPtr >>>= 0;
      // Realtime (0) in ns since 1970, else monotonic.
      const ns = id === 0
        ? BigInt(Date.now()) * 1000000n
        : BigInt(Math.round((performance.timeOrigin + performance.now()) * 1e6));
      view().setBigUint64(outPtr, ns, true);
      return ERRNO_SUCCESS;
    },
    clock_res_get(_id, outPtr) {
      outPtr >>>= 0;
      view().setBigUint64(outPtr, 1000n, true);
      return ERRNO_SUCCESS;
    },
    fd_write(fd, iovs, iovsLen, writtenPtr) {
      [iovs, writtenPtr] = [iovs >>> 0, writtenPtr >>> 0];
      if (fd !== 1 && fd !== 2) return ERRNO_BADF;
      const memory = getMemory();
      let written = 0;
      let text = '';
      const decoder = new TextDecoder();
      for (let i = 0; i < iovsLen; i++) {
        const ptr = view().getUint32(iovs + 8 * i, true);
        const len = view().getUint32(iovs + 8 * i + 4, true);
        text += decoder.decode(copyOut(memory, ptr, len));
        written += len;
      }
      const all = lines[fd] + text;
      const end = all.lastIndexOf('\n');
      if (end >= 0) {
        (fd === 1 ? console.log : console.error)(all.slice(0, end));
      }
      lines[fd] = all.slice(end + 1);
      view().setUint32(writtenPtr, written, true);
      return ERRNO_SUCCESS;
    },
    fd_fdstat_get(fd, statPtr) {
      statPtr >>>= 0;
      if (fd > 2) return ERRNO_BADF;
      const v = view();
      v.setUint8(statPtr, 2); // character device
      v.setUint16(statPtr + 2, 0, true);
      v.setBigUint64(statPtr + 8, 0n, true);
      v.setBigUint64(statPtr + 16, 0n, true);
      return ERRNO_SUCCESS;
    },
    fd_prestat_get: () => ERRNO_BADF,
    fd_close: () => ERRNO_SUCCESS,
    // Sleeps (std::thread::sleep): clock subscriptions only.
    poll_oneoff(inPtr, outPtr, count, eventsPtr) {
      [inPtr, outPtr, eventsPtr] = [inPtr >>> 0, outPtr >>> 0, eventsPtr >>> 0];
      const v = view();
      let timeoutMs = 0;
      for (let i = 0; i < count; i++) {
        const s = inPtr + 48 * i;
        if (v.getUint8(s + 8) === 0) {
          const ns = v.getBigUint64(s + 24, true);
          timeoutMs = Math.max(timeoutMs, Number(ns) / 1e6);
        }
      }
      sleep(timeoutMs);
      for (let i = 0; i < count; i++) {
        const s = inPtr + 48 * i;
        const e = outPtr + 32 * i;
        v.setBigUint64(e, v.getBigUint64(s, true), true);
        v.setUint16(e + 8, ERRNO_SUCCESS, true);
        v.setUint8(e + 10, v.getUint8(s + 8));
      }
      v.setUint32(eventsPtr, count, true);
      return ERRNO_SUCCESS;
    },
    sched_yield: () => ERRNO_SUCCESS,
    random_get(ptr, len) {
      [ptr, len] = [ptr >>> 0, len >>> 0];
      const bytes = new Uint8Array(len);
      for (let i = 0; i < len; i += 65536) {
        crypto.getRandomValues(bytes.subarray(i, Math.min(len, i + 65536)));
      }
      new Uint8Array(getMemory().buffer, ptr, len).set(bytes);
      return ERRNO_SUCCESS;
    },
    proc_exit(code) {
      throw new Error(`cubeopt: exit ${code}`);
    },
  };
  const imports = {};
  for (const { module: from, name } of WebAssembly.Module.imports(module)) {
    if (from === 'wasi_snapshot_preview1') {
      imports[name] = implemented[name] ?? (() => ERRNO_NOSYS);
    }
  }
  return imports;
}

/** Block for `ms` (Atomics.wait where allowed, else a busy wait). */
function sleep(ms) {
  if (ms <= 0) return;
  try {
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
  } catch {
    const end = performance.now() + ms;
    while (performance.now() < end);
  }
}

/**
 * The `thread-spawn` import of wasi-threads: hands the thread to an idle pool
 * worker. The module starts its threads once and never ends them (see
 * `wasm/src/threads.rs`), so the pool has room for all of them. It must not
 * wait for a worker: the spawning thread holds wasi-libc's thread list lock,
 * which a thread needs to end.
 */
export function threadSpawn(control) {
  const slots = (control.length - 4) / 4;
  return (startArg) => {
    for (let i = 0; i < slots; i++) {
      const base = slotBase(i);
      if (Atomics.compareExchange(control, base, SLOT_IDLE, SLOT_CLAIMED) === SLOT_IDLE) {
        const tid = Atomics.add(control, 0, 1) + 1;
        Atomics.store(control, base + 1, tid);
        Atomics.store(control, base + 2, startArg);
        Atomics.store(control, base, SLOT_RUN);
        Atomics.notify(control, base);
        return tid;
      }
    }
    console.error('cubeopt: no idle worker for a new thread (raise `threads`)');
    return -ERRNO_AGAIN;
  };
}

/** The imported memory's limits (pages), read from the module's import section. */
function importedMemoryLimits(bytes) {
  let at = 8;
  const leb = () => {
    let result = 0;
    let shift = 0;
    for (;;) {
      const b = bytes[at++];
      result += (b & 0x7f) * 2 ** shift;
      shift += 7;
      if (b < 0x80) return result;
    }
  };
  const skipName = () => {
    const len = leb();
    const name = new TextDecoder().decode(bytes.subarray(at, at + len));
    at += len;
    return name;
  };
  while (at < bytes.length) {
    const id = bytes[at++];
    const size = leb();
    const end = at + size;
    if (id === 2) {
      for (let n = leb(); n > 0; n--) {
        skipName();
        skipName();
        const kind = bytes[at++];
        if (kind === 0) leb();
        else if (kind === 1) {
          at++;
          const flags = bytes[at++];
          leb();
          if (flags & 1) leb();
        } else if (kind === 2) {
          const flags = bytes[at++];
          const initial = leb();
          const maximum = flags & 1 ? leb() : undefined;
          return { initial, maximum, shared: (flags & 2) !== 0 };
        } else if (kind === 3) at += 2;
        else if (kind === 4) {
          at++;
          leb();
        }
      }
    }
    at = end;
  }
  return undefined;
}

/** The bytes of `source`: a path (Node), a URL, or bytes already. */
async function readSource(source) {
  if (source instanceof Uint8Array) return source;
  if (source instanceof ArrayBuffer) return new Uint8Array(source);
  if (isNode && !/^(https?|data|blob):/.test(String(source))) {
    const { readFile } = await import('node:fs/promises');
    return new Uint8Array(await readFile(source instanceof URL ? source : String(source)));
  }
  const response = await fetch(source);
  if (!response.ok) throw new Error(`${source}: ${response.status} ${response.statusText}`);
  return new Uint8Array(await response.arrayBuffer());
}

/** Start a pool worker (`cubeopt-thread.mjs`) and wait until it is idle. */
async function startWorker(module, memory, control, slot) {
  const url = new URL('./cubeopt-thread.mjs', import.meta.url);
  let worker;
  const ready = new Promise((resolve, reject) => {
    const onMessage = (message) => (message === 'ready' ? resolve() : reject(new Error(message)));
    if (isNode) {
      worker = null;
      import('node:worker_threads').then(({ Worker }) => {
        worker = new Worker(url);
        worker.on('message', onMessage);
        worker.on('error', reject);
        worker.postMessage({ module, memory, control, slot });
      }, reject);
    } else {
      worker = new Worker(url, { type: 'module' });
      worker.onmessage = (e) => onMessage(e.data);
      worker.onerror = (e) => reject(new Error(e.message));
      worker.postMessage({ module, memory, control, slot });
    }
  });
  await ready;
  // (Node: idle pool workers must not keep the process alive.)
  if (isNode) worker.unref();
  return worker;
}

/** The solver: one WebAssembly instance with its table. */
export class Cubeopt {
  /**
   * Instantiate the module. `wasm`: a path (Node), URL, bytes or a compiled
   * `WebAssembly.Module`; `threads`: threads to use with the threads build
   * (default: all cores; the single-thread build uses 1).
   */
  static async create({ wasm, threads } = {}) {
    const bytes = wasm instanceof WebAssembly.Module ? null : await readSource(wasm);
    const module = bytes ? await WebAssembly.compile(bytes) : wasm;
    const needsThreads = WebAssembly.Module.imports(module).some((i) => i.module === 'wasi');
    let memory;
    let control;
    const workers = [];
    const imports = {};
    if (needsThreads) {
      if (typeof SharedArrayBuffer === 'undefined') {
        throw new Error('the threads build needs SharedArrayBuffer (a cross-origin isolated page)');
      }
      // (The threads build imports all 4 GiB at once, see wasm/build.rs.)
      const limits = bytes ? importedMemoryLimits(bytes) : { initial: 65535, maximum: 65536 };
      memory = new WebAssembly.Memory({ ...limits, shared: true });
      const cores = isNode
        ? (await import('node:os')).availableParallelism()
        : navigator.hardwareConcurrency;
      threads = Math.max(1, threads ?? cores ?? 1);
      // One worker per thread a solve can spawn, plus one.
      const slots = threads + 1;
      control = new Int32Array(new SharedArrayBuffer(4 * slotBase(slots)));
      imports.env = { memory };
      imports.wasi = { 'thread-spawn': threadSpawn(control) };
    } else {
      threads = 1;
    }
    let instanceMemory = memory;
    imports.wasi_snapshot_preview1 = wasiImports(() => instanceMemory, module);
    const instance = await WebAssembly.instantiate(module, imports);
    instanceMemory = memory ?? instance.exports.memory;
    instance.exports._initialize?.();
    if (needsThreads) {
      const slots = (control.length - 4) / 4;
      for (let slot = 0; slot < slots; slot++) {
        workers.push(await startWorker(module, memory, control, slot));
      }
    }
    return new Cubeopt(instance, instanceMemory, threads, workers);
  }

  constructor(instance, memory, threads, workers) {
    this.exports = instance.exports;
    this.memory = memory;
    /** Threads used by `solve` and `solveMany` (1 for the single-thread build). */
    this.threads = threads;
    this.workers = workers;
    /** The loaded table's `{ k, base, bytes }`, or null. */
    this.table = null;
  }

  /** The output of the last call (JSON or an error message). */
  output() {
    // (Addresses and sizes are unsigned: `>>> 0`, from 2 GiB up.)
    const ptr = this.exports.cubeopt_output() >>> 0;
    const len = this.exports.cubeopt_output_len() >>> 0;
    return new TextDecoder().decode(copyOut(this.memory, ptr, len));
  }

  /** Run an export; returns its parsed JSON output or throws its error. */
  call(name, ...args) {
    let status;
    try {
      status = this.exports[name](...args);
    } catch (e) {
      // A Rust panic aborts the instance (its message went to the console).
      this.broken = true;
      throw new Error(`cubeopt: ${name} failed (${e.message}); create a new instance`);
    }
    const text = this.output();
    if (status !== 0) throw new Error(`cubeopt: ${text}`);
    return JSON.parse(text);
  }

  /** Pass a string to `f(ptr, len)` in a module buffer. */
  withText(text, f) {
    const bytes = new TextEncoder().encode(text);
    const ptr = this.exports.cubeopt_alloc(bytes.length) >>> 0;
    if (ptr === 0) throw new Error('cubeopt: out of memory');
    try {
      new Uint8Array(this.memory.buffer, ptr, bytes.length).set(bytes);
      return f(ptr, bytes.length);
    } finally {
      this.exports.cubeopt_free(ptr, bytes.length);
    }
  }

  /**
   * Load a table file (made by the native `gen`, or `gen` in wasmtime): a
   * path (Node), a URL (fetched), a `Blob`/`File` (browsers), or its bytes.
   * The file is copied straight into the module's memory (no second copy).
   * `verify` checks the data against the checksum in the file's trailer.
   * Returns `{ k, base, bytes }`.
   */
  async loadTable(source, { verify = false, onProgress } = {}) {
    let size;
    let fill;
    if (source instanceof Uint8Array || source instanceof ArrayBuffer) {
      const bytes = source instanceof Uint8Array ? source : new Uint8Array(source);
      size = bytes.length;
      fill = async (at) => at(0).set(bytes);
    } else if (typeof Blob !== 'undefined' && source instanceof Blob) {
      size = source.size;
      fill = (at) => copyStream(source.stream(), at, onProgress, size);
    } else if (isNode && !/^(https?|data|blob):/.test(String(source))) {
      const fs = await import('node:fs');
      const fd = fs.openSync(source instanceof URL ? source : String(source), 'r');
      try {
        size = fs.fstatSync(fd).size;
        fill = async (at) => {
          const piece = 64 << 20;
          for (let done = 0; done < size; ) {
            const n = fs.readSync(fd, at(done, Math.min(piece, size - done)), 0, Math.min(piece, size - done), done);
            if (n <= 0) throw new Error(`${source}: unexpected end of file`);
            done += n;
            onProgress?.(done, size);
          }
        };
        return await this.fillTable(size, fill, verify);
      } finally {
        fs.closeSync(fd);
      }
    } else {
      const response = await fetch(source);
      if (!response.ok) throw new Error(`${source}: ${response.status} ${response.statusText}`);
      size = Number(response.headers.get('content-length'));
      if (!size) {
        const bytes = new Uint8Array(await response.arrayBuffer());
        return this.loadTable(bytes, { verify });
      }
      fill = (at) => copyStream(response.body, at, onProgress, size);
    }
    return this.fillTable(size, fill, verify);
  }

  /** Allocate the table buffer, let `fill` write the file into it, install it. */
  async fillTable(size, fill, verify) {
    this.table = null;
    const ptr = this.exports.cubeopt_table_buffer(size) >>> 0;
    if (ptr === 0) {
      throw new Error(`cubeopt: no room for a ${size}-byte table (wasm32 has at most 4 GiB in all)`);
    }
    // (A view of the buffer from `offset`; memory.buffer is re-read since
    // growing the memory replaces it.)
    await fill((offset, len = size - offset) => new Uint8Array(this.memory.buffer, ptr + offset, len));
    this.table = this.call('cubeopt_table_load', verify ? 1 : 0);
    await this.warmUp();
    return this.table;
  }

  /**
   * Generate table hk in memory (base 0: the default for k). h0..h4 take
   * from seconds to a minute or two on one thread (more threads: faster).
   */
  async generateTable(k, base = 0) {
    this.table = null;
    this.table = this.call('cubeopt_generate', k, base, this.threads);
    await this.warmUp();
    return this.table;
  }

  /**
   * Solve one cube: a scramble ("R U2 F' ...") or a facelet string (54
   * stickers in the order U R F D L B, any six symbols). All lanes of all
   * threads work on it. Returns `{ moves, length, nodes, lookups, seconds }`.
   */
  solve(cube, { lanes = 0 } = {}) {
    return this.withText(cube, (ptr, len) => this.call('cubeopt_solve', ptr, len, lanes, this.threads));
  }

  /** Solve many cubes for throughput (each lane solves its own cube); results in input order. */
  solveMany(cubes, { lanes = 0 } = {}) {
    return this.withText(cubes.join('\n'), (ptr, len) =>
      this.call('cubeopt_solve_batch', ptr, len, lanes, this.threads));
  }

  /**
   * Stream throughput, as the native `bench <table> stream`: random cubes
   * solved for `window` seconds after `warmup`. Returns
   * `{ solves_per_second, nodes_per_cube, lookups_per_cube, cubes, threads, lanes }`.
   */
  bench({ seed = 2026, warmup = 5, window = 20, lanes = 0 } = {}) {
    return this.call('cubeopt_bench', seed, warmup, window, lanes, this.threads);
  }

  /**
   * Have the engine compile the search loops with its optimizing compiler
   * now, so that the first real solves get it: V8 (Chrome, Node) runs new
   * code with its baseline compiler and switches a function to optimized
   * code only on its next call, so a long first solve would run its whole
   * search slowly: 187.7 s instead of 129.5 s for 100 depth-17 cubes on h6
   * (Node 26, one thread). Solves short scrambles (cheap with any table)
   * for about `seconds`, then gives the background compiler time to finish.
   * `loadTable` and `generateTable` call it.
   */
  async warmUp({ seconds = 0.3 } = {}) {
    const faces = 'UDRLFB';
    let seed = 1;
    const random = () => (seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648;
    const scramble = (length) => {
      const moves = [];
      for (let i = 0; i < length; i++) {
        moves.push(faces[Math.floor(random() * 6)] + ['', '2', "'"][Math.floor(random() * 3)]);
      }
      return moves.join(' ');
    };
    const start = performance.now();
    while (performance.now() - start < seconds * 1000) {
      const cubes = Array.from({ length: 8 }, () => scramble(7));
      this.solveMany(cubes, { lanes: 8 });
      this.solve(cubes[0]);
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
    this.solve(scramble(6));
  }

  /** `count` random cube states (facelet strings): those of the native `bench random:count:seed`. */
  randomCubes(seed, count) {
    return this.call('cubeopt_random', seed, count);
  }

  /** Stop the thread pool's workers. */
  close() {
    for (const worker of this.workers) worker.terminate();
    this.workers = [];
  }
}

/** Copy a byte stream into the table buffer (`at(offset, len)` views). */
async function copyStream(stream, at, onProgress, size) {
  const reader = stream.getReader();
  let done = 0;
  for (;;) {
    const { value, done: end } = await reader.read();
    if (end) break;
    if (done + value.length > size) throw new Error('the table stream is longer than its size');
    at(done, value.length).set(value);
    done += value.length;
    onProgress?.(done, size);
  }
  if (done !== size) throw new Error(`the table stream ended after ${done} of ${size} bytes`);
}

export { slotBase, SLOT_STARTING, SLOT_IDLE, SLOT_CLAIMED, SLOT_RUN };
