# Undefined-behavior evidence

[Safety overview](README.md) · [Complete dated assessment](NO_UB_20261107.md) · [Validation records](../evidence/validation/ub-audit/REPORT.md)

**Assessment recorded November 7, 2026.** The identified defects below were fixed and the stated checks passed on the audited source. **This is not a proof that every possible execution is free of undefined behavior.** Checksums, solution replay and finite Miri runs answer different questions.

## Source covered by the audit

The tested candidate was `316597f64a9bd749e8a01b67f96c108c03611e68`, integrated as `842a5043abfd550313a86a3d9bb5254f4b457540`. The recorded source fingerprint is:

```text
93478bc5c45ba267255574f8cb466f02f006f1c7bf558fb8f9d088ec4307a4c9
```

These identifiers state what was audited; they do not name today's repository `main`. Later source changes need applicable validation. The current documentation cleanup does not change the Rust search implementation or create a new Miri certificate.

## Reproduced defects and fixes

| Defect | Fix / contract | Recorded evidence |
|---|---|---|
| Out-of-range public table read | Checked getter and validated backing size | [Miri diagnostic](../evidence/validation/ub-audit/x86-table_index.log) |
| Invalid symmetry index 48 | Reject invalid symmetry indices | [Miri diagnostic](../evidence/validation/ub-audit/release-x86-symmetry_index.log) |
| Malformed cube bytes | Validate construction; restrict raw fields | [Miri diagnostic](../evidence/validation/ub-audit/release-x86-malformed_cube_rank.log) |
| Reserved edge-coordinate slot | Reject invalid keys/ranks | [Miri diagnostic](../evidence/validation/ub-audit/release-x86-malformed_coordinate.log) |
| Tracing after shared-search publication | Finish tracing before another worker may claim/rebuild the state | [Seed-15 diagnostic](../evidence/validation/ub-audit/trace-before-fix.log) |
| Shared-memory trailer publication, found by review | Read payload after observing the Release/Acquire marker | [Focused validation](../evidence/validation/ub-audit/focused-miri-validation.json) |

Review also ended staged-sort mutable argument borrows before publication. Mapping and shared-memory APIs state unsafe storage contracts; safe copying owns private storage. These changes address concrete mechanisms. They do not exhaust every ownership transition, external writer or platform behavior.

## Completed validation

| Check | Recorded result | Scope |
|---|---|---|
| Full x86 Miri catalogue | 50 of 56 named tests passed; 6 explicitly excluded | Tested interpreted paths |
| Full portable Miri catalogue | 48 of 54 named tests passed; 6 explicitly excluded | Tested interpreted paths |
| Shared-search scheduling | 16 seeds per backend passed | Repeated schedules, not exhaustive interleavings |
| Focused strict-provenance Miri | Passed on both tested backends | Boundary and publication helpers |
| AddressSanitizer | Passed | Instrumented native library/integration paths |
| ThreadSanitizer | Passed | Instrumented shared-search and boundary paths |
| Native/backend checks | Passed with documented harness exclusions | Native, ARM/QEMU, WASI/wasmtime and Bun checks |

[Native validation](../evidence/validation/ub-audit/native-validation.json), [focused Miri](../evidence/validation/ub-audit/focused-miri-validation.json), [ASan](../evidence/validation/ub-audit/asan-validation.json) and [TSan](../evidence/validation/ub-audit/tsan-validation.json) bind the results to source and tool settings. The [full dated assessment](NO_UB_20261107.md) retains test names, compiler versions, all six exclusions, earlier failed attempts and the detailed invariant review.

## Limits and caller obligations

- Full Miri runs used default provenance handling because the synchronization dependency uses exposed provenance. Warnings are retained; no UB checks were disabled. Focused strict-provenance passes do not replace this qualification.
- Actual file mappings, System V attachments, native assembly/AVX-512, NEON and WASM host behavior are outside the interpreted runs. Native/backend checks provide different evidence.
- Unit-test catalogues do not cover every integration test, doctest, input or weak-memory execution.
- Mapped/shared table storage must remain valid and immutable while borrowed. Private ownership after copying avoids external file-mutation dependencies.
- Metadata/checksums establish storage shape and integrity, not mathematical admissibility of every pruning bound.
- Removing intentional panic calls does not prove that dependencies or implicit operations can never panic.

The strongest evidence concerns the reproduced failures and the fixes that remove their triggering conditions. There is no defensible percentage of all possible UB eliminated. [Optimality checks](README.md) and [benchmark replay evidence](../comparisons/NISSY_THROUGHPUT.md) must not be presented as a universal memory-safety proof.
