# GitHub Pages

[Documentation home](../README.md)

The repository includes `.github/workflows/pages.yml`. It builds the browser
solver and publishes the documentation at `docs/` on the same site.

1. Push the project, including `.github/` and `docs/`, to your GitHub repository.
2. Open **Settings → Pages → Build and deployment**, and select **GitHub Actions**.
3. Run **GitHub Pages** from the Actions tab, or push to the default branch.

The workflow deploys only the default branch. The deployment URL appears in
the Actions run and the `github-pages` environment. No account, repository name,
or site base path is hardcoded. See GitHub's [custom workflow documentation](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

The public site is <https://egg-solver.arhan.sh/>. It runs the solver in your
browser: on load the demo starts with every core the browser reports and
builds the h4 table (464 MiB) on desktops or h3 (232 MiB) on phones, tablets and low-memory devices, so **Solve** works without further setup.
Generate a larger table there or select your own table file; large pruning
tables are not uploaded to Pages.

Browser threads need cross-origin isolation: every response must carry
`Cross-Origin-Opener-Policy: same-origin` and
`Cross-Origin-Embedder-Policy: require-corp`. GitHub Pages cannot set response
headers, so egg-solver.arhan.sh is served through Cloudflare, which adds both
headers; the public demo therefore uses multiple threads. A deployment without
those headers, such as a plain `github.io` address, is not isolated and the
demo falls back to one thread. Local `wasm/serve.mjs` sends the same headers.

To build the same site locally, install Bun 1.4.4 and the Rust targets:

```sh
rustup target add wasm32-wasip1 wasm32-wasip1-threads
scripts/build-pages.sh /tmp/egg-solver-pages
python3 -m http.server 8000 --directory /tmp/egg-solver-pages
```

Choose an output directory that does not already exist. Open
`http://localhost:8000/`. This plain HTTP server exercises the one-thread
hosting mode. WebAssembly binaries and staged site directories are generated locally and are not committed. The documentation HTML companions are committed for offline reading.

After rendering, `python3 scripts/check-docs.py` checks local links, table/heading structure and the latest benchmark headline against its recorded analysis. The Pages build runs this check before building the browser module.

Markdown is the canonical documentation source. The Pages build regenerates its HTML companions with Bun 1.4.4, so a stale generated page cannot retain removed or duplicated sections. HTML remains included for offline browsing; run `bun scripts/render-docs.mjs` to refresh it locally. Historical benchmark results keep their recorded build and hardware provenance.
