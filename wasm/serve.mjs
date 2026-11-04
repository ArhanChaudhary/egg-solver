#!/usr/bin/env node
// A static file server for the browser demo (index.html): serves wasm/
// with the headers that make the page cross-origin isolated (needed for
// SharedArrayBuffer, so for the threads build), and optionally a folder of
// table files under /tables/.
//
//   node wasm/serve.mjs [--port 8000] [--tables ~/cube-tables]
//   then open http://localhost:8000/

import { createServer } from 'node:http';
import { createReadStream, statSync } from 'node:fs';
import { extname, join, normalize, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const port = Number(option('--port', 8000));
const root = fileURLToPath(new URL('.', import.meta.url));
const tables = option('--tables');
const types = {
  '.html': 'text/html; charset=utf-8',
  '.mjs': 'text/javascript',
  '.js': 'text/javascript',
  '.wasm': 'application/wasm',
  '.css': 'text/css',
};

createServer((request, response) => {
  const path = decodeURIComponent(new URL(request.url, 'http://x').pathname);
  let file;
  if (tables && path.startsWith('/tables/')) {
    file = join(resolve(tables), normalize(path.slice('/tables/'.length)).replace(/^(\.\.[/\\])+/, ''));
  } else {
    file = join(root, normalize(path === '/' ? '/index.html' : path).replace(/^(\.\.[/\\])+/, ''));
  }
  let size;
  try {
    const stat = statSync(file);
    if (!stat.isFile()) throw new Error('not a file');
    size = stat.size;
  } catch {
    response.writeHead(404).end('not found');
    return;
  }
  response.writeHead(200, {
    'Content-Type': types[extname(file)] ?? 'application/octet-stream',
    'Content-Length': size,
    // Cross-origin isolation (SharedArrayBuffer, wasm threads).
    'Cross-Origin-Opener-Policy': 'same-origin',
    'Cross-Origin-Embedder-Policy': 'require-corp',
    'Cache-Control': 'no-cache',
  });
  if (request.method === 'HEAD') {
    response.end();
  } else {
    createReadStream(file).pipe(response);
  }
}).listen(port, () => {
  console.log(`serving ${root} on http://localhost:${port}/` + (tables ? ` (tables: ${tables} at /tables/)` : ''));
});
