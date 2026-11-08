# Historical PMU-guided optimization experiments

Recorded November 7 decision: retain the existing native search-loop implementation. Both PMU-guided native experiments remain rejected and are now archived with the required 1-thread and 48-thread before/after tables. The separately explored older-x86 backend remains an unmerged experiment, excluded from the release and perf package.

| Experiment | 1-worker throughput change (12 pairs) | 48-worker throughput change (5 pairs) | Decision |
|---|---:|---:|---|
| Read packed table values through byte-sized loads | -0.83% | -0.28% | Reject |
| Move lane advancement out of the stream loop | -7.44% | -10.72% | Reject |

The experiments use historical baseline `93867a4597ab821b4064a0d407d3c89e133c9013` on this EPYC VM. They are not measurements of the final safety build. Higher throughput is better; headline changes are medians of paired ratios, not ratios of marginal medians.

## Measurement method and limits

The 1T screen used h8, one worker, 24 lanes, seed 2026, a 10-second warm-up and a 60-second measured window. Each experiment was paired with the baseline at four code-padding settings, with three repeats per setting and rotating CPU slots. Every setting had a negative median result. Baseline padding 0, byte-load padding 0 and outlined-advance padding 16 were selected by the original control-normalized screen and frozen before the 48T measurements.

The 48T archival measurements used the same h8 table and frozen binaries, 24 lanes per worker, seed 2026, a 20-second warm-up and a 30-second measured window. Five sequential rounds rotated the three binary orders. One baseline run per round was shared across both experiments. Two exclusive machine holds were separated by a 90-second yield; every accepted run had less than 1.5 cores of measured outside load. The binary hashes were checked before execution.

These are short fixed-duration measurements. Completed-work quantization and run-to-run noise limit small-effect claims. They support retaining the rejection decision; they are not a substitute for longer fixed-work measurements required before adopting a new optimization. The VM has no hardware counters, so these data do not demonstrate changes in instructions or cache-miss events.

The initial review stopped after the negative 1T screen. The user's standing rule also required 48T tables for rejected-experiment commits. The supplementary measurements and archival commits below complete that evidence requirement. The earlier review, release receipt and pre-measurement candidate-evidence snapshot are preserved with their original hashes.

## Complete 1T paired screen

All rates are completed solves/s. Candidate and baseline use the same padding within each row.

| Pad | Repeat | Baseline | Byte loads | Byte / baseline | Outlined advance | Outline / baseline |
|---:|---:|---:|---:|---:|---:|---:|
| 0 | 1 | 4.100 | 4.117 | 1.004146 | 3.867 | 0.943171 |
| 16 | 1 | 4.167 | 4.133 | 0.991841 | 3.867 | 0.928006 |
| 32 | 1 | 4.083 | 4.067 | 0.996081 | 3.750 | 0.918442 |
| 48 | 1 | 4.100 | 4.033 | 0.983659 | 3.800 | 0.926829 |
| 48 | 2 | 4.083 | 4.033 | 0.987754 | 3.767 | 0.922606 |
| 32 | 2 | 4.067 | 4.033 | 0.991640 | 3.783 | 0.930170 |
| 16 | 2 | 4.050 | 4.050 | 1.000000 | 3.850 | 0.950617 |
| 0 | 2 | 4.100 | 4.067 | 0.991951 | 3.717 | 0.906585 |
| 0 | 3 | 4.067 | 4.033 | 0.991640 | 3.717 | 0.913941 |
| 16 | 3 | 4.100 | 4.033 | 0.983659 | 3.800 | 0.926829 |
| 32 | 3 | 4.183 | 4.150 | 0.992111 | 3.867 | 0.924456 |
| 48 | 3 | 4.067 | 4.017 | 0.987706 | 3.733 | 0.917876 |

## Complete 48T paired rounds

Rates are completed solves/s. Each row shares the same baseline. Outside-load values follow the execution order.

| Round | Order | Baseline | Byte loads | Byte / baseline | Outlined advance | Outline / baseline | Outside cores |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | base → bytes → outline | 153.933 | 153.500 | 0.997187 | 137.433 | 0.892811 | 0.1 / 0.1 / 0.1 |
| 2 | bytes → outline → base | 155.233 | 152.067 | 0.979605 | 136.533 | 0.879536 | 0.0 / 0.1 / 0.0 |
| 3 | outline → base → bytes | 154.533 | 153.133 | 0.990940 | 138.867 | 0.898624 | 0.1 / 0.1 / 0.1 |
| 4 | outline → bytes → base | 154.967 | 154.800 | 0.998922 | 139.000 | 0.896965 | 0.0 / 0.0 / 0.0 |
| 5 | bytes → base → outline | 153.267 | 153.300 | 1.000215 | 136.700 | 0.891908 | 0.0 / 0.0 / 0.0 |

## Preserved branches and validation

| Experiment | Negative branch | Commit |
|---|---|---|
| Read packed table values through byte-sized loads | `neg/pmu-byte-packed-load` | `1a6f91913e42a0c1df995f70f5bda82bf6d36719` |
| Move lane advancement out of the stream loop | `neg/pmu-outlined-lane-advance` | `fb892a97f7fece9cbf4a9b430a58420f63edd34d` |

Each archival commit includes both full timing tables and median ratios. Executable statements match the measured source snapshots; only the inline experiment comment was annotated with the completed results. Original patches, source snapshots, check logs and binary hashes remain available. The branches preserve their historical baseline and are not merged into the audited release main.

Both measured experiments passed native library/integration/CLI tests and clippy. The byte decoder additionally passed its exhaustive focused strict-provenance Miri test on both tested backends. No full-source Miri certificate is claimed for these experimental branches. The new deterministic 20-state check (seed 2026) matched across baseline and both candidates: 18,754,454 nodes/cube, 16,192,780 lookups/cube, length histogram `16:1 17:1 18:18`.

The earlier phack PMU collection motivated these mechanisms: reducing packed-lookup work and reducing instruction footprint. Their timing results did not justify adoption.

A separate SSSE3 implementation was explored for older x86-64 machines. Its native correctness, lint and focused vector Miri checks passed. It does not alter the AVX2/BMI2 path used by this VM. The experiment is retained under `/data/agents/pmu-westmere/`. Adoption would require complete validation of a rebased candidate and measurements on the intended physical host; it is excluded from main and the shipped profiling kit.

Evidence: [1T screen](../data/pmu-experiments/screen-summary.json), [48T rounds and identity](../data/pmu-experiments/negative-48t/summary.json), [native/focused checks](../data/pmu-experiments/build-validation.json), [archive receipt](../data/pmu-experiments/negative-archive.json), [initial release decision](../data/pmu-experiments/release-decision.json), [phack PMU analysis](HARDWARE_COUNTERS.md).
