// Cross-origin isolation for static hosts that cannot send response headers.
//
// The threads build needs SharedArrayBuffer, which browsers only allow on a cross-origin
// isolated page: one served with `Cross-Origin-Opener-Policy: same-origin` and
// `Cross-Origin-Embedder-Policy: require-corp`. wasm/serve.mjs sends them locally and
// egg-solver.arhan.sh gets them from Cloudflare; this service worker covers hosts that cannot
// send them (such as GitHub Pages without a proxy). It adds both headers to every same-origin
// response it passes through; index.html registers it only on a page that is not isolated, and
// reloads once so that the page itself is served through it. Where a browser refuses, the page
// stays on one thread. (Adapted from omelettehedron's wasm/coi-sw.js.)

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));

self.addEventListener('fetch', (event) => {
  const request = event.request;
  // (a request the worker may not re-issue: leave it to the browser)
  if (request.cache === 'only-if-cached' && request.mode !== 'same-origin') return;
  if (new URL(request.url).origin !== self.location.origin) return;
  event.respondWith(
    (async () => {
      const response = await fetch(request);
      // opaque responses cannot be changed
      if (response.status === 0) return response;
      const headers = new Headers(response.headers);
      headers.set('Cross-Origin-Opener-Policy', 'same-origin');
      headers.set('Cross-Origin-Embedder-Policy', 'require-corp');
      headers.set('Cross-Origin-Resource-Policy', 'same-origin');
      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers,
      });
    })(),
  );
});
