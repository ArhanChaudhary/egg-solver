# Cubeopt comparison audit

Audit completed 2026-11-08 for the historical comparison and fresh 14/48-thread checks. A separate investigation of the current web solver and pruning tables follows this audit; the Mac implementation remains unidentified.

The published 5.4× and 20.3× figures describe an older x86 Linux `cube48opt10` binary on the EPYC VM. They are not measurements against the closed-source cubeopt used on your friend's M4 Pro; its exact release and execution environment have not been supplied. I should have identified that binary and the limits of the comparison prominently, rather than letting the short comparison table stand in for a claim about current cubeopt generally.

## What your friend's numbers say

You supplied: MacBook M4 Pro, 14 threads, h10, `dataset/random_state.txt`, 4.88 million nodes/solve and 2.77 ns/node.

`4.88e6 × 2.77e-9 = 0.0135176 seconds/solve`, or **73.98 solves/s**. These numbers are internally consistent. The exact command, binary version/hash, native-versus-browser execution environment, number of input positions, page configuration and concurrency mode are not available. The supplied Linux binary is about five months old according to you; that does not establish the age or identity of the Mac implementation. The dataset name alone does not establish that the measured input prefixes are identical.

## What the old report actually measured

The historical 1-thread result used the first 50 states; the 48-thread result used the first 200 states and three process runs. The retained input file has 500 states. The old harness parsed `tt=... ms` from each cube's output and averaged it; it did not divide cube48opt10's latency by its thread count.

| Old cube48opt10 measurement | Mean time/cube | Equivalent serial completion rate | Mean nodes/cube |
|---|---:|---:|---:|
| 1 thread, first 50 states | 368.82422 ms | 2.711/s | 6,993,163.78 |
| 48 threads, run 1, first 200 states | 48.88043 ms | 20.458/s | 7,112,389.26 |
| 48 threads, run 2, first 200 states | 48.89141 ms | 20.453/s | 7,214,157.06 |
| 48 threads, run 3, first 200 states | 49.34675 ms | 20.265/s | 7,183,479.15 |

The 48-thread median is 48.89141 ms. The corresponding egg-solver h10 logs give **68.40798 ms** at one thread, and **2.707375 / 2.409495 / 2.414160 ms** across the three 48-thread runs. Thus the independently recalculated ratios are **5.39154×** and **20.25193×** (ratio of medians). The printed 5.4× and 20.3× figures match that arithmetic. Egg’s 48-thread end-to-end batch wall time was 2.416–2.422 ms/cube in the latter two runs, so its short per-cube timer did not hide an order-of-magnitude cleanup cost. These are resident-table measurements: loading and process startup are excluded on both sides.

The executable used by the historical harness is byte-for-byte identical to `/home/super/cube48opt10`:

- SHA-256: `6a20791d0b90ff946e6addc7e48565f0cb154260b38056b82052affffe94bf56`.
- ELF x86-64 Linux executable; GNU build ID `8169ccfa29e63d769214885e5e1c7c6191be74eb`.
- Embedded compiler string: GCC 9.4.0, Ubuntu 20.04 build.
- You report that this binary is about five months old. The compiler string and file modification time do not independently establish its release date.
- Table: `/data2/competitors/cube48opt10/h48prun31h1.dat`, 31,130,910,720 bytes.

## Where my conclusion exceeded the evidence

1. **Binary identity was too weak in the short report.** A row named `cube48opt10` did not make clear that it was this particular older Linux executable, with no verified current release identity. The raw result can be reproducible while its use as a statement about every current cubeopt implementation is unjustified.
2. **I did not establish a best-thread-count comparison.** Matching both programs at 48 threads answers a fixed-configuration question. It does not prove that 48 threads is the fastest setting for this competitor. The fresh 14-thread check below was slower than 48 threads for this old executable; it does not explain the Mac result.
3. **There was no paired M4 Pro measurement.** Architecture and operating system differ between the reports; memory behavior, implementation version and execution mode may also differ. A real speedup on the friend's machine requires timing egg-solver and that cubeopt binary there, on identical positions and equivalent modes.
4. **I carried forward measurements made on an earlier egg-solver build.** The README did identify `229b99f`, and the detailed report disclosed later differences, but that is another reason these numbers cannot establish a current, universal ranking.

The README did name the EPYC host and comparison commit, and the detailed documentation explicitly rejected a universal speed ranking. I should not claim those disclosures were absent. The error I can establish is that I endorsed a stronger ranking than this experiment supports and did not make the old competitor executable’s identity prominent in the short table. I have not established that the retained timing arithmetic is fabricated or that the M4 result proves a particular timing bug. I am responsible for checking the inherited benchmark evidence before endorsing its claims in the publication material.

## Evidence retained

Historical raw logs, summaries and page measurements are under `/data/agents/f1/final/{raw,sum,meta}/cube48opt10_*`. The audit's independently recalculated values are in `/data/agents/cubeopt-audit/old-cubeopt-rows.json`. The historical harness is `/data/agents/f1/cube_solver_test/runtest.sh`, function `cubeopt`.

The canonical published repository is now https://github.com/ArhanChaudhary/egg-solver. Its remote has been fetched and configured locally. Your authored README introduction has not been modified.

## What the saved scratchpad establishes

Claude’s saved scratchpad is accessible at `/data/agents/session-state/scratchpad-copy/`; the old `/tmp/claude-1000` path disappeared after reboot. I should have inspected the saved copy directly rather than asking you to supply access.

`TODO-after-loop.md` records the original plan: equal-sized tables, the same local datasets, and comparisons at one and 48 threads. It explicitly identifies the supplied binaries as closed source. Later entries distinguish latency from throughput and record the discovery that these old native binaries ignore `-g`. The actual harness keeps `ngroup=1`, so this did not introduce a division-by-48 error into the reported cube48opt10 latency. The detailed comparison also disclosed the lack of source and the embedded SSE/BMI2 implementation. Those qualifications existed; I failed to preserve their significance when condensing the result for publication.

A fixed 48-thread comparison was part of the requested experiment. Conducting it was reasonable. Treating that experiment as sufficient evidence of a universal ranking was not. Neither those notes nor the retained binaries establish the performance of the Mac implementation your friend ran.

## Fresh 14-thread check

On 2026-11-08, the supplied Linux binary solved the first 200 local random states three times in one process at 14 threads, with its table verified resident on 29 GiB of 1 GiB pages. The three group means were **62.876945, 62.547540 and 62.847185 ms/cube**. Their median is **62.847185 ms/cube**, or **15.912 solves/s** under this serial-solve timing metric. Mean nodes/cube were 7.050M, 7.042M and 7.026M. These repeated groups are not independent process restarts.

Reducing this old binary to 14 threads on the VM does not reproduce the supplied Mac figure of 13.5176 ms/cube. This does not isolate hardware from software differences, and the tested configurations cannot certify a globally optimal configuration. The all-500-state result is recorded below.

## What needs to change in how I report this

- Label the competitor as the **supplied x86-64 Linux cube48opt10 executable**, with its checksum and an explicit statement that its release identity is unverified. Do not silently equate it with every implementation called cubeopt.
- Keep the machine, table sizes, thread count, solve mode, exact dataset prefix, repeats and measured egg build beside the speedup. “h10” means comparable table sizes here, not interchangeable table files. Each solver uses its own table.
- Separate a fixed-thread benchmark from a best-configuration or best-solver claim. The old binary prints `use sse, bmi2`; this comparison includes implementation, compiler and instruction-set choices, not only search-algorithm differences.
- Do not divide the VM egg timing by the Mac cubeopt timing and call the result a speedup. A defensible Mac comparison requires both implementations on that Mac, on identical input bytes, with equivalent timing boundaries and modes.
- Preserve the old raw logs, clearly label them historical, and attach the fresh all-500-state outputs rather than replacing them with an unexplained new headline.

I should have treated the inherited benchmark as evidence to audit, not as sufficient proof of “best.” Having a plausible ratio and several repeats does not verify the identity of the competitor or justify claims outside the tested conditions.

## Complete 500-state rerun

Input: `/home/super/egg-solver/testdata/random_state.txt`, exactly 500 states once, SHA-256 `616810c31d090a78ea1357684750642ab017ff394bfa08ba131bb65cc6905de3`. This is byte-identical to the retained harness dataset. Both programs used 48 threads, latency mode, their own h10 tables, verified resident 1 GiB pages, and serialized quiet-machine runs. Egg's 34-file build-source fingerprint matches the current source, which matches published `origin/main` `947c20d93564f150aad9c292486ab730a329b86c`.

| Solver | Cubes | Mean solve time | Mean nodes | Sum of per-cube solve times |
|---|---:|---:|---:|---:|
| Supplied Linux cube48opt10 | 500 | 47.654576 ms | 6,700,394.976 | 23.827288 s |
| Current egg h10, 32 lanes | 500 | 2.363502 ms | 6,293,506.514 | 1.181751 s |

All 500 returned solution lengths match. Egg additionally verifies that every solution solves its input. Its complete batch wall time excluding loading was 1.185386 s. The ratio of mean per-cube solve times is **20.163×**. This is one fresh process per solver, not a new estimate of run-to-run variation. The first 200 cubeopt states averaged 48.177805 ms and 7,209,174.765 nodes; all 500 average 6.700M nodes, so the longer dataset does not reproduce the reported 4.88M count.

Complete, untouched stdout and separate stderr are in `/home/super/cubeopt-egg-500/`, alongside commands, metadata, input, derived statistics and SHA256SUMS. The raw logs reproduce the old Linux comparison closely. They establish neither the cause of the Mac difference nor a speedup on that Mac.

## Conclusion

The 5.4×/20.3× arithmetic is supported by the historical logs, and the fresh all-500-state 48-thread test reproduces a similar ratio against the supplied executable. I have not established a numerical timing error. My avoidable failure was endorsing that limited experiment as proof of a broader solver ranking without verifying and prominently identifying the competitor release. The M4 result is a credible reason to investigate the current implementation, not a measurement of egg versus cubeopt on identical hardware. The version, runtime, table contents, input scope and timing mode must be pinned down before assigning the difference to any one cause.
