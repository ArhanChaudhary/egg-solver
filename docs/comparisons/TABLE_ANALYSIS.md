# cstimer h4: table size and compression base

[Benchmarks](README.md) · [Methods](METHODS.md) · [Raw records](../data/cubeopt-audit-20261108/README.md)

Investigation completed 2026-11-08. The target is **cube48opt4 on https://cstimer.net/cubeopt/**, displayed as **486M**.

## Verified size

The deployed Opt4 module reports `h48prun31h4.dat`, **486,420,480 bytes**. Egg h4 has exactly the same data size:

`3393 corner classes × 70 edge separations × 2^4 retained orientation values × 128 bytes/row = 486,420,480 bytes`.

This is **486.420480 decimal MB = 463.886719 MiB**. At the start of the investigation egg displayed **464 MiB** and cstimer **486M**; egg's updated selector displays both units. The displayed size difference is a unit conversion, not additional table capacity. Egg's metadata trailer is additional file overhead, not pruning data.

At the November 8 capture, the web page offered Opt1–9, up to 15,565,455,360 bytes. The supplied Linux cube48opt10 uses a 31,130,910,720-byte h10 table. The freshly downloaded Opt9 WASM and wrapper match the retained copies byte-for-byte (WASM SHA-256 `a041e72e9a05182425722c18a1c262cd0ac6018014c5dcd817069693883a734f`). This does not establish the version of the separately reported Mac executable.

## What can differ despite equal sizes

Both families use symmetry-reduced corner data, edge slice/separation data, a projection of edge orientation, two-bit distance codes, and a four-bit minimum shared by a 16-byte group. Equal file sizes do not imply identical coordinate numbering, orientation conventions, projections, compression bases or distance values. Byte comparisons at equal offsets cannot establish relative pruning strength when index mappings differ.

The compression base changes the represented lower bounds while keeping the table size fixed. Code zero uses a shared group minimum plus a corner bound; the other codes describe the base plus one, two, or at least three moves. Egg native recommends base 9 for h4; its browser previously explicitly requested base 8. The controlled native experiment below measures the base-9 tradeoff.

A smaller node count can also come from search ordering, canonical move pruning, symmetry handling, stopping after the first optimal solution, parallel scheduling or different instrumentation. It is not evidence by itself that a table is stronger.

## Experiment

The exact deployed modules are downloaded without modification for generation. The query oracle adds exports to a checksum-pinned in-memory copy, leaving function code and table values unchanged. It evaluates corresponding physical states rather than assuming matching raw indices.

The corpus contains the 500 published random states followed by their scramble prefixes: **10,568 states**, evaluated in normal and inverse directions. Prefix lengths provide known upper bounds; solved and short prefixes check near-goal behavior. Native egg h4 base 8 and base 9 were generated and compared against fresh web Opt4 data. The tests distinguished direct lower-bound strength, actual search node counts, wall time, and correctness.

The measurements below support changing the browser’s recommended compression base. They do not establish that the web table dominates egg’s table or explain the reported Mac h10 result.

## Exact files and provenance

The freshly downloaded Opt4 WASM SHA-256 is `9980620d29f954557adcd05f692b34deacf2e04c8a45f2c172fcd7cf921ef53e`; its wrapper is `37ac455c506b4f734b87a6717fb7e8322d966301ae69bc43fe189e0fc764c9bd`. Both match the retained upstream assets. The fresh web-generated table SHA-256 is `4e7da22fec3e10a1f3a151d1eed5454c1e6a16a884794f4251f0f90a48bde200`.

Egg table files are 486,424,576 bytes: identical pruning-data capacity plus a 4,096-byte metadata trailer. All artifact hashes are in [h4-files.json](../data/cubeopt-audit-20261108/h4-files.json). Generation completed successfully for web Opt4, egg h4 base 8, egg h4 base 9, and the additionally investigated web Opt9. No new upstream generator release was found in these deployed assets.

## Direct pruning strength

Each comparison evaluates corresponding physical states, including their inverses, with each implementation's own coordinate code and table. The native tool calls the public `Solver::lower_bound`; the web oracle runs the original `step1_query` with checksum-pinned context layouts. Neither substitutes one table's bytes under the other's coordinate mapping.

| Comparison (left versus right) | Queries | Left higher | Equal | Right higher |
|---|---:|---:|---:|---:|
| Web Opt4 / egg h4 base 8; 500 full states, both directions | 1,000 | 217 | 628 | 155 |
| Web Opt4 / egg h4 base 8; 10,568 states, both directions | 21,136 | 1,984 | 16,765 | 2,387 |
| Web Opt4 / egg h4 base 9; 500 full states, both directions | 1,000 | 200 | 607 | 193 |
| Web Opt4 / egg h4 base 9; 10,568 states, both directions | 21,136 | 2,205 | 16,657 | 2,274 |
| Egg base 9 / egg base 8; 500 full states, both directions | 1,000 | 69 | 915 | 16 |
| Egg base 9 / egg base 8; 10,568 states, both directions | 21,136 | 263 | 20,087 | 786 |

The captured web table and egg base 9 each give stronger bounds on some states. Thus neither is a pointwise stronger replacement on this corpus. They use different coordinate conventions/projections, and raw byte differences do not identify stronger data. All tested bounds stay below known optimal lengths for the 500 full states and known scramble-length upper bounds for the prefixes. Those checks are evidence on this corpus, not a universal admissibility proof.

Base 9 also has a tradeoff: it is stronger more often on the full random states but weaker more often on the prefix-heavy corpus. Averaging root bounds across arbitrary prefixes is not a substitute for measuring search work. Base 9 increases the encoded ceiling from 11 to 12 while making the code-zero fallback less precise for some low distances.

## Controlled native solve experiment

Both rows use the same retained native egg executable, AMD EPYC 9V45 VM, 48 threads, 40 lanes, batch concurrency, copy loading, all 500 input states once, and serialized runs under the benchmark locks. Loading is excluded. These are ordinary copied tables; explicit 1 GiB page coverage is not claimed for this experiment. Source identity matches the published Rust source at `1b243c01983b85f364ab8101aaeaef268fdd7039` (recorded as `1b243c01983b85f364ab8101aaeaef268fdd7039` in the raw records, before the repository history was rebuilt).

| Egg h4 | Batch wall time | Wall time/cube | Mean nodes/cube |
|---|---:|---:|---:|
| Base 8 | 78.210631 s | 156.421 ms | 531,782,489.040 |
| Base 9 | 64.917146 s | 129.834 ms | 424,622,380.116 |

Base 9 used **20.15% fewer nodes** and **17.00% less wall time**, or 1.205× throughput. This is one process per setting, not a repeated-run confidence estimate. Node accounting is compared within egg; no native-versus-WASM speedup is inferred.

Every one of the 500 solutions from each setting passed egg's replay verification and matched the known optimal lengths from the h10 cross-check. The web Opt4 sample's first 100 completed solutions also matched those lengths; all 100 were independently replayed through egg and returned the solved cube.

The web solve log began with all 500 inputs and was deliberately stopped after 100 complete records to bound this additional experiment. `run-limit.json` records the stop. Its mean was 429,578,004.26 nodes/cube and 3273.759 ms at 14 threads in WASM latency mode. There is no complete-process wall-time rate for this partial run, and its timer is not paired with the native batch timers. The [cubeopt audit](CUBEOPT_AUDIT.md) links the separate full-500 h10 outputs.

## Implemented change

The previous browser comment explicitly chose base 8 to reduce generation time; that was a startup-versus-search tradeoff, not a corrupt or undersized table. Both choices are retained. The browser h4 option now calls `generateTable(4)` through the existing automatic-base path instead of forcing base 8. That selects the already established native recommendation, base 9. The label displays both 486 MB and 464 MiB. An explicit base-8 choice remains available for faster generation. No new Rust search code, table format, unsafe block or panic path was introduced; native defaults were already correct. The generator help text also corrects h8 capacity from 3.6 GiB to 7.2 GiB.

The actual threaded WASM module generated h4 with automatic base 9 and exactly 486,420,480 data bytes. Three short solves and independent replays passed. Both existing single-thread and threaded WASM smoke suites also passed. A Chromium test verified that the automatic h4 option sends base zero and the faster-generation choice sends base eight; that routing test uses a mock worker, separately from the real WASM generation test. The direct browser-option parse still uses the existing `k:base` parser; omitted base passes zero, which the Rust API resolves via `default_base(k)`.

Initialization is the cost: native generation took 2.1 s for base 8 and 7.6 s for base 9 in these runs. The actual eight-thread Bun/WASM base-9 test took 149.0 s to generate. Chrome timing was not measured, and these generator implementations/timings should not be conflated. Reusing a saved table avoids regeneration.

## Conclusion and limits

The exact size observation is explained by decimal MB versus binary MiB. Fresh generation and semantic queries do not show that current cstimer Opt4 contains universally better pruning data. A useful existing table optimization was being bypassed by egg's browser selector; that selector is now corrected and tested. It does not explain 4.88M nodes for the unidentified Mac h10 implementation, and no claim is made about that release or an unmeasured larger table.

Reproduction records are indexed in [the evidence directory](../data/cubeopt-audit-20261108/README.md). It retains `native-h4.sh`, `analyze.py`, the input corpus, logs, asset hashes and `analysis.json`. The original research workspace also held the web scripts `opt4/generate.mjs` and `opt4/query-web.mjs`, an `examples/table_bounds.rs` helper and the pinned web assets; those files are not in this repository. Raw timings, node counts, witnesses and stop reasons are retained. The native generation commands are `gen 4 8 FILE -t 48` and `gen 4 auto FILE -t 48`; solve commands are preserved in `native-h4.sh`.
