Current CLI, layout and deployment revision after a247321.
Full native/backend checks: PASS, including Clippy with -D warnings.
Miri: two focused lane tests passed. The earlier broader run was deliberately interrupted during the long split-enumeration test; it is not a completed Miri pass. Historical complete Miri certificates still describe their recorded source.
Browser: Chromium, ordinary HTTP without isolation headers, repository subpath, h0 generation, explicit 5-move solve, docs navigation: PASS.
GitHub-hosted deployment itself requires pushing the repository and enabling Pages with GitHub Actions.
