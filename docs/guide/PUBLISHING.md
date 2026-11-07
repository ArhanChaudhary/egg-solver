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

The site runs the solver in your browser. Generate a small table there or
select your own table file; large pruning tables are not uploaded to Pages.
The demo uses one thread when cross-origin isolation is unavailable, including
standard GitHub Pages hosting. Local `wasm/serve.mjs` enables multiple threads
by serving the required isolation headers.

To build the same site locally:

```sh
rustup target add wasm32-wasip1 wasm32-wasip1-threads
scripts/build-pages.sh /tmp/egg-solver-pages
python3 -m http.server 8000 --directory /tmp/egg-solver-pages
```

Choose an output directory that does not already exist. Open
`http://localhost:8000/`. This plain HTTP server exercises the one-thread
hosting mode. Build artifacts are generated locally and are not committed.

Documentation HTML is included for offline browsing. After editing Markdown,
run `bun scripts/render-docs.mjs` before committing. Historical benchmark
results keep their recorded build and hardware provenance.
