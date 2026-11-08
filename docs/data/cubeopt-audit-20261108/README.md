# Cubeopt audit and exact Opt4 experiment — 2026-11-08

The comparison audit, table analysis and nxopt PS=16 report are in `../../comparisons/`.

`h10-*` preserves the complete 500-state native cube48opt10 and egg h10 output, input, commands and metadata. `h10-SHA256SUMS` uses the original unprefixed filenames. `egg-h4b*-solves.jsonl` contains the two 500-state native h4 batches. `web-opt4-query-web.log` is a deliberately stopped web run: use only the first 100 completed results, as recorded in `web-opt4-run-limit.json`. All 100 lengths and independent replays passed. This web run must not be described as a completed 500-state process.

`analysis.json`, bounds CSVs/JSON, corpus and `analyze.py` retain the semantic comparisons. `h4-files.json` pins the exact generated table bytes; the large table files and third-party compiled solver binaries are not embedded in this documentation. Paths in local scripts describe the measurement VM and require adapting for another host.

Table generation and repeated solves have different costs. The measured improvement changes the browser's recommended compression base; native defaults already selected base 9. No universal solver ranking follows from these experiments.
