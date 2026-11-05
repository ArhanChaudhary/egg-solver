#!/usr/bin/env bun
// A static file server for the browser demo (index.html), in Bun: serves
// wasm/ with the headers that make the page cross-origin isolated (needed for
// SharedArrayBuffer, so for the threads build), and optionally a folder of
// table files under /tables/.
//
//   bun wasm/serve.mjs [--port 8000] [--tables ~/cube-tables]
//   then open http://localhost:8000/

import { join, normalize, resolve } from 'node:path';

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const root = import.meta.dir;
const tables = option('--tables');
// (Paths stay inside their folder: leading `..` steps are dropped.)
const inside = (folder, path) => join(folder, normalize(`/${path}`).replace(/^(\/\.\.)+/, ''));

const server = Bun.serve({
  port: Number(option('--port', 8000)),
  async fetch(request) {
    const path = decodeURIComponent(new URL(request.url).pathname);
    const file = Bun.file(
      tables && path.startsWith('/tables/')
        ? inside(resolve(tables), path.slice('/tables/'.length))
        : inside(root, path === '/' ? 'index.html' : path),
    );
    if (!(await file.exists())) return new Response('not found', { status: 404 });
    // (Bun sends Content-Length and the type from the extension.)
    return new Response(request.method === 'HEAD' ? null : file, {
      headers: {
        // Cross-origin isolation (SharedArrayBuffer, wasm threads).
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Embedder-Policy': 'require-corp',
        'Cache-Control': 'no-cache',
      },
    });
  },
});
console.log(`serving ${root} on ${server.url}` + (tables ? ` (tables: ${tables} at /tables/)` : ''));
