# Technique and table inventories

[Performance guide](../../performance/README.md) · [Experiment history](../../history/DELETED_EXPERIMENTS.md)

These documents explain how other solvers and performance techniques were investigated during development. They are historical working analyses: “pending” items, “ours” and “now” refer to egg-solver (then named cube-opt) at the commit each document names. The final comparison and the commit records provide the later outcomes.

| Review | Purpose |
|---|---|
| [Audit of inline measured-gain comments (main 52ab88f)](an1-annotation-audit.md) | Checks every optimization comment in the source against the gain recorded in its commit |
| [cube48opt10 pruning-table file layout compared with h10](cube48opt10-table.md) | Static reading of cube48opt10's table format, and a byte-comparison recipe |
| [Assessment of optimization ideas from Tom Rokicki and Arhan Chaudhary (2026-11-04)](ideas-assessment.md) | Status of each listed idea and the follow-up it received |
| [The Rust Performance Book: technique audit](perf-book-audit.md) | Each technique in the book, its status in the solver, and the results of new trials |
| [cube48opt-rust technique inventory](x-cube48opt-rust-inventory.md) | cube48opt-rust's techniques compared with the solver's, with two measured prototypes |
| [nissy-core (H48) technique inventory](x-h48-inventory.md) | nissy-core's techniques, an admissibility finding in its h11 table, and a larger-table feasibility study |
| [nxopt technique inventory](x-nxopt-inventory.md) | nxopt's techniques compared with the solver's, with measured prototypes |
| [vcube technique inventory](x-vcube-inventory.md) | vcube's techniques, per-operation costs, and an index-chain and cache working-set study |
| [cube48opt-rust benchmark setup](xcr-bench-entry.md) | How cube48opt-rust was built, given its tables and run for the comparison |

## Reading the labels

- Short labels such as m1–m8, c7, c8, l6, p5, s8 or s9 name development series. They appear in branch and merge names, for example `opt/m8-unsafe` or `opt/s9-pin-v2`, in the [experiment history](../../history/DELETED_EXPERIMENTS.md).
- Branch names that start with `neg/` are rejected or neutral experiments; the [negative-experiment index](../../history/DELETED_EXPERIMENTS.md) links each to its commit record. A few working branches and patch files named in these documents were not retained; their results are recorded only here.
- 1T and 48T mean one and 48 worker threads. A7, B3 and similar codes refer to rows within the same document; “idea 2.4” and similar refer to the numbered idea list in the ideas assessment.
- Source paths and line numbers refer to the commit each document names, not to the current checkout. Local file paths from the development machine have been replaced by descriptions.
