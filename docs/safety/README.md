# Correctness, optimality and memory safety

[Documentation home](../README.md) · [Detailed UB assessment](NO_UB.md) · [Mordant findings](MORDANT.md)

There are three different questions to ask about a fast solver:

| Question | Evidence in this project | Limit |
|---|---|---|
| Does the returned move sequence solve the input? | The solve path checks its result; regression tests and benchmark datasets exercise that behavior | Passing sampled inputs does not cover every possible execution |
| Is the sequence shortest? | Iterative-deepening search with lower bounds; independent short-state search, known optimal lengths, cross-mode checks and comparator agreement | Complete correctness of all pruning rules and generated bounds is not established by a checksum or by finite tests alone |
| Is the implementation free of undefined behavior? | Code review, concrete defect reproductions and fixes, Miri, native/backend checks, ASan and TSan | These validate tested executions and stated invariants; they are not a universal proof |

## Recorded validation scope

The audited source fingerprint is `93478bc5c45ba267255574f8cb466f02f006f1c7bf558fb8f9d088ec4307a4c9`, integrated as `842a504` on November 7, 2026. The complete Miri catalogues executed 50 of 56 x86 tests and 48 of 54 portable tests, with six explicit exclusions per backend. Each backend also ran the shared-search tests under 16 scheduling seeds. Native/backend, focused strict-provenance, AddressSanitizer and ThreadSanitizer checks completed.

The [NO_UB report](NO_UB.md) explains the fixed API, bounds, memory ownership and publication defects, the provenance-mode qualifications, and the OS/assembly/backend exclusions. It retains earlier failed or incomplete snapshots as history; the summary identifies the audited source and links the full dated assessment.

[Mordant](MORDANT.md) produced 14 distinct native and 70 WASI lint diagnostics, with zero compiler errors in the accepted November 7 runs. Those diagnostics were reviewed. A successful lint run is not a claim of zero warnings or a safety proof.

The no-panic policy removes intentional runtime panic calls from shipped first-party code, with the documented test/constant/debug-assert exceptions. That scanner result does not prove absence of every implicit or dependency panic.

For comparing algorithms, see [solution lengths in the complete report](../comparisons/RESULTS.md). For implementation detail, see the [source map](../guide/HOW_IT_WORKS.md#source-map).

The November 8 throughput study additionally replayed all 15,000 final solutions in both independent cube models. [Its raw validation](../data/nissy-throughput-20261108/cross-replay.json) is a result check on those 500 distinct states, not a new Miri run or universal optimality proof.
