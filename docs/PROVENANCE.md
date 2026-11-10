# Source identities and evidence scope

[Documentation](README.md) · [Benchmarks](comparisons/README.md) · [Verification](reproduce/README.md)

Usage guides describe the commands in this source checkout. Benchmark and safety reports describe the specific source bytes they tested. A dated source identifier is not a claim that it is still the repository's current `main`.

The repository history was rebuilt on 2026-11-10: the public repository had started from one squashed commit, and the full development history is now included. Agent commits are attributed to Claude, and two commit messages were reworded to remove excluded backend timing comparisons, so every commit has a new ID; file contents are unchanged. The guides cite current IDs. Raw records under `data/` keep the IDs that were current when they were written. The table maps the 172 old IDs that raw records cite. For example, the published source used by the November 8 studies, `947c20d` in raw records, is now `1b243c0`.

<details>
<summary>Old commit IDs in raw records</summary>

| Date | Old ID | Current ID |
|---|---|---|
| 2026-11-02 | `ec7e133` | `900ed03` |
| 2026-11-02 | `9090310` | `e68e619` |
| 2026-11-02 | `046c33f` | `bf0bcff` |
| 2026-11-02 | `dc26266` | `aa80cec` |
| 2026-11-02 | `9f9a4d3` | `4ffb6f2` |
| 2026-11-02 | `7bce67d` | `ce1cee9` |
| 2026-11-02 | `0c48355` | `448f4fb` |
| 2026-11-02 | `2271355` | `9e5e320` |
| 2026-11-02 | `c52cb63` | `856204b` |
| 2026-11-02 | `31c2878` | `99c6237` |
| 2026-11-02 | `2bb148d` | `c49c3fd` |
| 2026-11-02 | `a0681ad` | `1da36db` |
| 2026-11-02 | `0b65adf` | `7ebb3f1` |
| 2026-11-02 | `d2674d5` | `a482e6b` |
| 2026-11-02 | `aa43192` | `fd48890` |
| 2026-11-02 | `d63535b` | `bc453e7` |
| 2026-11-02 | `5cea944` | `3a7b790` |
| 2026-11-02 | `59ce145` | `2b88284` |
| 2026-11-03 | `42a7c98` | `43d148f` |
| 2026-11-03 | `ae4fc49` | `3540328` |
| 2026-11-03 | `f99ebc4` | `417a74d` |
| 2026-11-03 | `3957bbc` | `f1e2092` |
| 2026-11-03 | `ebd6c20` | `6ccd6c8` |
| 2026-11-03 | `5b0f6ab` | `7c6c7b2` |
| 2026-11-03 | `e9d751e` | `cedc001` |
| 2026-11-03 | `6d265dd` | `af142e4` |
| 2026-11-03 | `d708f1c` | `388b911` |
| 2026-11-03 | `814ae96` | `1176dad` |
| 2026-11-03 | `042f13c` | `cc8c00c` |
| 2026-11-03 | `fe58740` | `95d1619` |
| 2026-11-03 | `7a173a1` | `169da9e` |
| 2026-11-03 | `b90b706` | `82da6e7` |
| 2026-11-03 | `8b2cf71` | `3b10f59` |
| 2026-11-03 | `70567ff` | `ffbd072` |
| 2026-11-03 | `45a5a0b` | `ef92dda` |
| 2026-11-03 | `18ef7f0` | `ca2921f` |
| 2026-11-03 | `ff2b7a4` | `c897107` |
| 2026-11-03 | `c914d66` | `8de8048` |
| 2026-11-03 | `496b096` | `1e7ccce` |
| 2026-11-03 | `1326046` | `403680a` |
| 2026-11-03 | `60153a7` | `7652948` |
| 2026-11-03 | `eee3432` | `d482047` |
| 2026-11-03 | `a741d66` | `cff820d` |
| 2026-11-03 | `c509dd1` | `30810e7` |
| 2026-11-03 | `35c608c` | `55120b7` |
| 2026-11-03 | `431e46a` | `12a7bce` |
| 2026-11-03 | `236bf33` | `567e005` |
| 2026-11-03 | `ee18653` | `0fc4f92` |
| 2026-11-03 | `68dfaf6` | `3356857` |
| 2026-11-03 | `9caf0e2` | `32a42c5` |
| 2026-11-03 | `82ff5c8` | `0dc6f47` |
| 2026-11-03 | `4911370` | `80fa308` |
| 2026-11-03 | `5c3efa9` | `a5248b0` |
| 2026-11-03 | `78ce33e` | `ea4da12` |
| 2026-11-03 | `ecd311e` | `1560974` |
| 2026-11-03 | `26b4f4c` | `94e405d` |
| 2026-11-03 | `85e9c79` | `d0ab36f` |
| 2026-11-03 | `621b70a` | `c564dc3` |
| 2026-11-03 | `d2a3c2e` | `84b03da` |
| 2026-11-03 | `82b291e` | `2b14dad` |
| 2026-11-03 | `cb8cb70` | `00d998c` |
| 2026-11-03 | `6f033a4` | `2f5a122` |
| 2026-11-03 | `4088803` | `a8ae5f9` |
| 2026-11-03 | `fcc8d7b` | `7233222` |
| 2026-11-03 | `f947454` | `1b1ac4f` |
| 2026-11-03 | `7853462` | `7ea2a6c` |
| 2026-11-03 | `b2f4b13` | `bc6be36` |
| 2026-11-03 | `8953929` | `88afbf5` |
| 2026-11-03 | `b84fb7c` | `29de940` |
| 2026-11-03 | `f0efcde` | `4f559fa` |
| 2026-11-03 | `47b5da2` | `75e1e9a` |
| 2026-11-03 | `1235cdd` | `865b1f8` |
| 2026-11-03 | `c5d7705` | `8a1f55c` |
| 2026-11-04 | `36668cc` | `08d637a` |
| 2026-11-03 | `21e3ca9` | `bc310cc` |
| 2026-11-04 | `26af596` | `0354803` |
| 2026-11-04 | `0b35690` | `af1b296` |
| 2026-11-04 | `2203a07` | `3430808` |
| 2026-11-04 | `f2cb1f4` | `97e1abd` |
| 2026-11-04 | `595e424` | `8163366` |
| 2026-11-04 | `abce87c` | `44b7d8a` |
| 2026-11-04 | `d13944f` | `78bc088` |
| 2026-11-04 | `71d69ae` | `13a13fa` |
| 2026-11-04 | `55e048e` | `e7b094e` |
| 2026-11-04 | `b2f2c9a` | `555a6ea` |
| 2026-11-04 | `301115f` | `e591236` |
| 2026-11-04 | `4453092` | `1e0ede1` |
| 2026-11-04 | `ece816a` | `1f9dcaf` |
| 2026-11-04 | `854dfe8` | `d5f3c52` |
| 2026-11-04 | `3cc6ee3` | `ff86d68` |
| 2026-11-04 | `3f7054d` | `c6c7c5f` |
| 2026-11-04 | `ab5d21a` | `3e59901` |
| 2026-11-04 | `b608890` | `f66ec5a` |
| 2026-11-04 | `dda14e2` | `56f5b58` |
| 2026-11-04 | `c9a1c4d` | `9e4bc24` |
| 2026-11-04 | `735dd25` | `b3f0bfb` |
| 2026-11-04 | `33ca449` | `3bf5530` |
| 2026-11-04 | `085a9fd` | `0339dd2` |
| 2026-11-04 | `1f4c85a` | `7c46566` |
| 2026-11-04 | `9e69243` | `5029d24` |
| 2026-11-04 | `5cce44d` | `6a05ec8` |
| 2026-11-04 | `1c31eca` | `1c2749a` |
| 2026-11-04 | `f8a4919` | `387034c` |
| 2026-11-04 | `673d49e` | `42b986b` |
| 2026-11-04 | `84a4aa1` | `5f130cd` |
| 2026-11-04 | `fe926ed` | `d4125fd` |
| 2026-11-04 | `a546429` | `0abb347` |
| 2026-11-04 | `857a0c6` | `f07b109` |
| 2026-11-04 | `8926c95` | `887c892` |
| 2026-11-04 | `def2cd0` | `f2cf828` |
| 2026-11-04 | `7eaede8` | `fa2b454` |
| 2026-11-04 | `876a97e` | `db0b8ac` |
| 2026-11-04 | `8977134` | `57b3a5d` |
| 2026-11-04 | `33a8042` | `710716d` |
| 2026-11-04 | `95cbb11` | `03abdfc` |
| 2026-11-04 | `7608a28` | `d28335f` |
| 2026-11-04 | `2a0574f` | `81eae95` |
| 2026-11-05 | `3a9a50a` | `7075875` |
| 2026-11-05 | `9be649d` | `ee62734` |
| 2026-11-05 | `1571c47` | `2ad24b8` |
| 2026-11-05 | `c620d69` | `942d44c` |
| 2026-11-05 | `a7ce28f` | `76a488f` |
| 2026-11-05 | `ca31e63` | `cc43b77` |
| 2026-11-05 | `585b3b6` | `d690a46` |
| 2026-11-05 | `89dca7b` | `ee9e887` |
| 2026-11-05 | `66c4f03` | `6ff4350` |
| 2026-11-05 | `aee1645` | `13ea1af` |
| 2026-11-05 | `0d1a0e8` | `64877d5` |
| 2026-11-04 | `2f4fb7a` | `9c45b7a` |
| 2026-11-04 | `bb07598` | `828737e` |
| 2026-11-05 | `31525f1` | `2bdc042` |
| 2026-11-05 | `4c74267` | `965ff32` |
| 2026-11-05 | `508cbe5` | `0e453b6` |
| 2026-11-05 | `ca820b6` | `6244eaa` |
| 2026-11-05 | `6990603` | `9a338aa` |
| 2026-11-05 | `55bca49` | `102023f` |
| 2026-11-05 | `0d6a899` | `5f9e56b` |
| 2026-11-05 | `a5c7a6f` | `6353621` |
| 2026-11-05 | `43fc26b` | `f5e5400` |
| 2026-11-05 | `0c4bfe2` | `d68e49e` |
| 2026-11-05 | `b9fc895` | `d786c0d` |
| 2026-11-05 | `b6d7821` | `c4bf6a8` |
| 2026-11-05 | `760ad75` | `1b5ecd9` |
| 2026-11-05 | `229b99f` | `979dcf6` |
| 2026-11-05 | `80620c4` | `7d8b606` |
| 2026-11-05 | `18c4a66` | `52ab88f` |
| 2026-11-05 | `7f4950d` | `68047f2` |
| 2026-11-05 | `0b5b5a6` | `1f5eb70` |
| 2026-11-05 | `2403aa3` | `6a131a3` |
| 2026-11-05 | `10977c2` | `c2fd2a1` |
| 2026-11-05 | `ffd2394` | `6ae0bb2` |
| 2026-11-05 | `4bf1b5e` | `67bddf5` |
| 2026-11-05 | `ae25e0a` | `5739dd0` |
| 2026-11-05 | `ae49125` | `9d41ad7` |
| 2026-11-05 | `3eff157` | `76c3714` |
| 2026-11-05 | `db87483` | `0a5b0cf` |
| 2026-11-05 | `2627ad3` | `f4c064a` |
| 2026-11-06 | `4b1707a` | `baa021d` |
| 2026-11-06 | `e127307` | `b6913e8` |
| 2026-11-06 | `a1fce13` | `bbc455e` |
| 2026-11-06 | `935cb7f` | `01e7683` |
| 2026-11-06 | `0ed8002` | `75f5e8c` |
| 2026-11-06 | `cd963dc` | `5489b2d` |
| 2026-11-06 | `af9a7e7` | `050ecb3` |
| 2026-11-06 | `56ace9c` | `c551a06` |
| 2026-11-06 | `447e432` | `ce92f83` |
| 2026-11-06 | `12bb580` | `4dda0ab` |
| 2026-11-06 | `93867a4` | `da134bc` |
| 2026-11-07 | `988f289` | `e18e971` |
| 2026-11-07 | `316597f` | `13307ba` |
| 2026-11-07 | `842a504` | `a247321` |
| 2026-11-07 | `947c20d` | `1b243c0` |

</details>

| Evidence | Measured source / scope | Record |
|---|---|---|
| November 8 nissy throughput | Published source `1b243c0` (`947c20d` in raw records); retained CLI build inputs match except generator help text not linked into `solve` | [Report](comparisons/NISSY_THROUGHPUT.md), [manifest](data/nissy-throughput-20261108/manifest.json) |
| Earlier multi-solver matrix | Egg `979dcf6`; later `a247321` checked separately | [Historical matrix](comparisons/RESULTS.md) |
| Miri and sanitizer audit | Candidate `13307ba`, integrated as `a247321`; source fingerprint recorded | [Safety assessment](safety/NO_UB.md) |
| Physical Xeon counters | `01e7683` plus the recorded profiling patch; Westmere build | [Counter study](performance/HARDWARE_COUNTERS.md) |
| Other experiments | Their own pinned sources, compilers, inputs and timing formulas | [Catalog](BENCHMARK_CATALOG.md), [deleted experiments](history/DELETED_EXPERIMENTS.md) |

The retained evidence includes raw text/CSV/JSON, datasets, methods, licenses, source snapshots and dated experiment notes. Executables, build caches, pruning-table data and binary PMU recordings are excluded. Machine paths in raw records identify the measurement environment; they are not installation requirements.

`MANIFEST.sha256` checks the current documentation file bytes. `SOURCES.json` records origins and hashes from the original evidence collection; it is a provenance record, not the checksum list for later edited guides. Individual studies also retain their own raw-output hashes. These checks detect changed files, not incorrect scientific claims.

Historical evidence is preserved when a summary is rewritten. Old status messages, pending work, test counts and benchmark formulas are interpreted at their recorded date. Backend correctness identities remain where relevant; the excluded backend performance comparisons are not republished.
