# Historical multi-solver comparison

[Benchmarks](README.md) · [Timing definitions](METHODS.md) · [Full dated matrix](archive/MATRIX_20261107.md)

This is the **November 7, 2026 snapshot**, measured on the 48-core EPYC VM with egg source **`229b99f`**. It is not the latest throughput study and does not describe every later source revision. For the complete 500-state, h6–h10 finite-batch test, use the [November 8 nissy comparison](NISSY_THROUGHPUT.md).

## What is retained

The [full matrix](archive/MATRIX_20261107.md) preserves every original numeric table, source/build screen, page measurement, sample interval, distance-20 check and coverage ledger. It records **230 completed report cells** across nissy, nxopt, vcube, supplied cubeopt binaries and cube48opt-rust. The [raw index](../data/comparison/run-index.csv) lists the collected run groups; eligible groups and report cells are different counts.

## Read the old tables correctly

| Table family | What is timed | Main qualification |
|---|---|---|
| Latency | One cube at a time, with workers cooperating | cube48opt-rust uses its stated 18-thread cap; other rows can use 48 |
| Batch / stream throughput | Concurrent or fixed-window work, as stated per section | Not interchangeable with one-cube latency |
| Nxopt / vcube harness time | Sum of cube durations divided by workers and cubes | Can omit the idle finishing tail; not measured batch wall time |
| One-thread interval | Variation across the same sampled cubes | Not process-to-process variability |
| Multi-thread range | Median and range over the stated repeats | Repeat count and process boundaries are part of the result |

Times in the detailed matrix are **seconds per cube under that section's formula**. Smaller is better. GB is decimal table size. `1G` means 1 GiB table pages; `THP` coverage can be partial. Node counts are implementation-specific. [Methods](METHODS.md) expands the compact notation.

## Follow-up corrections and newer evidence

- [Cubeopt audit](CUBEOPT_AUDIT.md): the 5.4× / 20.3× arithmetic describes one supplied older Linux executable, not a current Mac release.
- [Large nxopt table](NXOPT32.md): the later 62.474 GB experiment is separate from the smaller nxopt22/31 rows.
- [New nissy throughput](NISSY_THROUGHPUT.md): complete-batch wall time on all 500 states with tuned allocations.
- [Safety assessment](../safety/NO_UB.md): validation is bound to its audited source fingerprint, not inferred from timing.

The later `842a504` source had separate h11 A/B validation against `229b99f`: stream throughput changed by −1.28% at one worker and −1.01% at 48 workers; the stated latency checks changed by +0.16%, +2.84% and +1.20% time. Those records remain in the full matrix. Historical gains are not added together or silently attributed to the latest checkout.

## Detailed sections and old bookmarks

These links retain the earlier report's entry points. Each opens the matching section of the dated matrix.

<details>
<summary>Show historical section links</summary>

<a id="cubeopt-comparison-all-currently-known-results"></a>

- [CUBEOPT comparison: all currently known results](archive/MATRIX_20261107.md#cubeopt-comparison-all-currently-known-results)

<a id="final-source-validation"></a>

- [Final source validation](archive/MATRIX_20261107.md#final-source-validation)

<a id="fresh-miri-validation"></a>

- [Fresh Miri validation](archive/MATRIX_20261107.md#fresh-miri-validation)

<a id="latency-one-cube-at-a-time-on-all-threads-egg-solver-solve-c-1-vs-nissy-h48-cubeopt30-cube48opt10"></a>

- [Latency: one cube at a time on all threads (egg-solver solve -c 1 vs nissy H48, cubeopt30, cube48opt10)](archive/MATRIX_20261107.md#latency-one-cube-at-a-time-on-all-threads-egg-solver-solve-c-1-vs-nissy-h48-cubeopt30-cube48opt10)

<a id="2-6-3-9-gb-tables-latency"></a>

- [~2.6-3.9 GB tables (latency)](archive/MATRIX_20261107.md#2-6-3-9-gb-tables-latency)

<a id="7-6-7-8-gb-tables-latency"></a>

- [~7.6-7.8 GB tables (latency)](archive/MATRIX_20261107.md#7-6-7-8-gb-tables-latency)

<a id="large-tables-1-gib-pages-only-latency"></a>

- [large tables (1 GiB pages only) (latency)](archive/MATRIX_20261107.md#large-tables-1-gib-pages-only-latency)

<a id="throughput-many-cubes-in-flight-egg-solver-solve-c-0-bench-stream-vs-nxopt-t-vcube-w-which-solve-one-cube-per-thread"></a>

- [Throughput: many cubes in flight (egg-solver solve -c 0 / bench stream vs nxopt -t, vcube -w, which solve one cube per thread)](archive/MATRIX_20261107.md#throughput-many-cubes-in-flight-egg-solver-solve-c-0-bench-stream-vs-nxopt-t-vcube-w-which-solve-one-cube-per-thread)

<a id="2-6-3-9-gb-tables-throughput"></a>

- [~2.6-3.9 GB tables (throughput)](archive/MATRIX_20261107.md#2-6-3-9-gb-tables-throughput)

<a id="7-6-7-8-gb-tables-throughput"></a>

- [~7.6-7.8 GB tables (throughput)](archive/MATRIX_20261107.md#7-6-7-8-gb-tables-throughput)

<a id="large-tables-1-gib-pages-only-throughput"></a>

- [large tables (1 GiB pages only) (throughput)](archive/MATRIX_20261107.md#large-tables-1-gib-pages-only-throughput)

<a id="headline-same-page-type-same-semantics-same-size-group"></a>

- [Headline (same page type, same semantics, same size group)](archive/MATRIX_20261107.md#headline-same-page-type-same-semantics-same-size-group)

<a id="cube-solver-test-wiki-metrics-this-machine"></a>

- [cube_solver_test wiki metrics (this machine)](archive/MATRIX_20261107.md#cube-solver-test-wiki-metrics-this-machine)

<a id="space-node-product-table-bytes-x-nodes-visited-in-1e15-bytes-less-is-better"></a>

- [Space-node product (table bytes x nodes visited, in 1e15 bytes; less is better)](archive/MATRIX_20261107.md#space-node-product-table-bytes-x-nodes-visited-in-1e15-bytes-less-is-better)

<a id="number-of-nodes-visited-millions-per-cube-less-is-better"></a>

- [Number of nodes visited (millions per cube; less is better)](archive/MATRIX_20261107.md#number-of-nodes-visited-millions-per-cube-less-is-better)

<a id="average-time-per-node-random-state-harness-tt-ns-node-sec-solv-nodes-per-cube-of-the-same-run"></a>

- [Average time per node (random_state; harness tt, ns/node = sec/solv / nodes per cube of the same run)](archive/MATRIX_20261107.md#average-time-per-node-random-state-harness-tt-ns-node-sec-solv-nodes-per-cube-of-the-same-run)

<a id="reference-the-wiki-s-published-values-intel-core-i7-10750h-6-cores-12-threads-thp-always-not-this-machine"></a>

- [Reference: the wiki's published values (Intel Core i7-10750H, 6 cores / 12 threads, THP always; NOT this machine)](archive/MATRIX_20261107.md#reference-the-wiki-s-published-values-intel-core-i7-10750h-6-cores-12-threads-thp-always-not-this-machine)

<a id="node-counts-vs-the-wiki-same-datasets-first-100-cubes-nodes-do-not-depend-on-the-machine"></a>

- [Node counts vs the wiki (same datasets, first 100 cubes; nodes do not depend on the machine)](archive/MATRIX_20261107.md#node-counts-vs-the-wiki-same-datasets-first-100-cubes-nodes-do-not-depend-on-the-machine)

<a id="distance-20-positions-cube20-org"></a>

- [Distance-20 positions (cube20.org)](archive/MATRIX_20261107.md#distance-20-positions-cube20-org)

<a id="2-6-3-9-gb-tables-distance-20"></a>

- [~2.6-3.9 GB tables (distance 20)](archive/MATRIX_20261107.md#2-6-3-9-gb-tables-distance-20)

<a id="7-6-7-8-gb-tables-distance-20"></a>

- [~7.6-7.8 GB tables (distance 20)](archive/MATRIX_20261107.md#7-6-7-8-gb-tables-distance-20)

<a id="large-tables-1-gib-pages-only-distance-20"></a>

- [large tables (1 GiB pages only) (distance 20)](archive/MATRIX_20261107.md#large-tables-1-gib-pages-only-distance-20)

<a id="build-screening-shipped-flags-vs-o3-march-native-and-vs-pgo"></a>

- [Build screening (shipped flags vs -O3 -march=native and vs PGO)](archive/MATRIX_20261107.md#build-screening-shipped-flags-vs-o3-march-native-and-vs-pgo)

<a id="solution-lengths-optimality-cross-check"></a>

- [Solution lengths (optimality cross-check)](archive/MATRIX_20261107.md#solution-lengths-optimality-cross-check)

<a id="rejected-runs-page-placement-outside-load-or-interference-validation"></a>

- [Rejected runs (page placement, outside load, or interference validation)](archive/MATRIX_20261107.md#rejected-runs-page-placement-outside-load-or-interference-validation)

<a id="harness-correction-2026-11-05"></a>

- [Harness correction (2026-11-05)](archive/MATRIX_20261107.md#harness-correction-2026-11-05)

<a id="fairness"></a>

- [Fairness](archive/MATRIX_20261107.md#fairness)

<a id="cube48opt10-s-table-vs-egg-solver-h10"></a>

- [cube48opt10's table vs egg-solver h10](archive/MATRIX_20261107.md#cube48opt10-s-table-vs-egg-solver-h10)

<a id="competitor-implementation-findings"></a>

- [Competitor implementation findings](archive/MATRIX_20261107.md#competitor-implementation-findings)

<a id="phack-hardware-counter-analysis"></a>

- [phack hardware-counter analysis](archive/MATRIX_20261107.md#phack-hardware-counter-analysis)

<a id="runs"></a>

- [Runs](archive/MATRIX_20261107.md#runs)

<a id="tables"></a>

- [Tables](archive/MATRIX_20261107.md#tables)

<a id="memory-and-hugepage-steps"></a>

- [Memory and hugepage steps](archive/MATRIX_20261107.md#memory-and-hugepage-steps)

<a id="complete-coverage-ledger"></a>

- [Complete coverage ledger](archive/MATRIX_20261107.md#complete-coverage-ledger)

<a id="safety-evidence"></a>

- [Safety evidence](archive/MATRIX_20261107.md#safety-evidence)

<a id="bugs-and-issues-found-in-other-solvers-for-the-final-report-the-user-will-report-them-upstream"></a>

[Historical section](archive/MATRIX_20261107.md#bugs-and-issues-found-in-other-solvers-for-the-final-report-the-user-will-report-them-upstream).

<a id="competitor-bug-findings"></a>

[Historical section](archive/MATRIX_20261107.md#competitor-bug-findings).

<a id="no-panic-source-inventory"></a>

[Historical section](archive/MATRIX_20261107.md#no-panic-source-inventory).

<a id="no-panic-integration-remaining-implicit-check-inventory"></a>

[Historical section](archive/MATRIX_20261107.md#no-panic-integration-remaining-implicit-check-inventory).

</details>
