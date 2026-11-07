## Final source validation

The comparison tables measure commit `229b99f`. The later final source `842a5043abfd` adds
preflight, shared-memory support, the no-panic integration, the measured stream follow-up
and the completed egg-solver rename.
Its performance was checked separately against the immutable comparison binaries on h11
(1 GiB pages, 40 lanes; stream warmup/window 10/60 seconds at 1T and 20/30 at 48T).
The timing tables below retain their original measured provenance. Search node/lookup
counts and solution-length histograms match exactly on the verification datasets.

| check | paired runs | final / measured | change |
|---|---:|---:|---:|
| Stream throughput 1T | 8 | 0.987217 | -1.28% |
| Stream throughput 48T | 5 | 0.989851 | -1.01% |
| Latency 48T random_state | 6 | 1.001574 | +0.16% time |
| Latency 48T random_move_17f | 6 | 1.028380 | +2.84% time |
| Latency 48T random_state_depth17 | 6 | 1.012014 | +1.20% time |

Ratios are medians of paired ratios; higher throughput and lower latency are better.

**Final stream throughput regressed by more than 0.5%:** 1T (-1.28%), 48T (-1.01%).
This measured difference remains disclosed; the competitor tables were not measured on this later build.

The project rename passed full native/scalar/ARM/WASM checks and was separately
validated for identical hot instructions, required placement and stack frames.
Existing environment names remain fallback aliases and legacy tables remain readable.

Final renamed main: `842a5043abfd550313a86a3d9bb5254f4b457540` in `~/egg-solver`. Full native/scalar/ARM/WASM checks, legacy h8/h11 search identity and hot-code comparisons passed. The temporary old repository-path alias has been removed.
