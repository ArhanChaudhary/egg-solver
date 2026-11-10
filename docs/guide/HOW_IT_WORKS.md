# How the search works

[Documentation home](../README.md) · [Performance studies](../performance/README.md)

The easiest starting point in the source is the recursive reference search in [`src/solver.rs`](../reference/source/src/solver.rs.html). The fast implementations preserve its core search problem while scheduling work differently.

## Search with a growing move budget

The solver uses **IDA-star** (`IDA*`), iterative deepening A*. It searches with a move limit, then increases that limit until it finds a solution. At each node, a table supplies a lower bound on the moves still needed. A branch can be skipped when its lower bound exceeds the remaining budget.

With valid lower bounds and complete search of shorter limits, the first successful limit establishes the shortest length. The implementation also uses move-order rules, symmetry reductions and pruning rules whose correctness must preserve that search property. A checksum protects file integrity; it is not a mathematical proof that a table's bounds are admissible.

## Keep useful work in flight

Large pruning tables do not fit in CPU caches. The search loops maintain several **lanes** per worker. A lane computes a lookup address, issues a prefetch, and yields. Other lanes advance while that memory access is outstanding. The worker later returns to use the value.

```mermaid
flowchart LR
    A[Advance a lane] --> B[Compute table index]
    B --> C[Prefetch table value]
    C --> D[Visit other lanes]
    D --> E[Read earlier lookup]
    E --> F[Prune or generate a child]
    F --> A
```

A lane is search state, not an operating-system thread. Increasing lane count can improve memory overlap, but also increases state and scheduling work. That is why the benchmark records include lane sweeps rather than assuming that more is always faster.

## Share a cube across workers

The shared-search mode divides a cube's search into jobs. Workers can donate pieces of a search to idle workers. Job ordering aims to find a solution earlier in the successful iteration; synchronization must still ensure that shorter iterations are handled correctly and that each published job is safe to read.

The [safety report](../safety/NO_UB.md) discusses the ownership and publication checks, including a reproduced concurrency defect that was fixed in the audited source.

## Source map

| File | What to read it for |
|---|---|
| [`src/solver.rs`](../reference/source/src/solver.rs.html) | Reference IDA*, pruning, table index and value lookup |
| [`src/lanes.rs`](../reference/source/src/lanes.rs.html) | Fast lane scheduler, `run_lanes`, and `Lane::advance` |
| [`src/lanes/share.rs`](../reference/source/src/lanes/share.rs.html) | Work sharing and worker rounds for latency and batches |
| [`src/coord.rs`](../reference/source/src/coord.rs.html) | Cube coordinates and table addressing |
| [`src/tables.rs`](../reference/source/src/tables.rs.html) | Pruning-table representation and memory |
| [`src/symstart.rs`](../reference/source/src/symstart.rs.html) | Symmetry filtering of starting jobs |
| [`src/bin/solve.rs`](../reference/source/src/bin/solve.rs.html) | Interactive/batch command-line interface |
| [`tests/correctness.rs`](../reference/source/tests/correctness.rs.html) | Independent search and optimal-length cross-checks |

The linked source views are snapshots from release `a247321`. Historical performance records name their own source commits; line numbers and mechanisms can differ in those older versions.

See [parallelism measurements](../performance/PARALLELISM.md) for the distinction between CPU workers, software lanes, query-slot occupancy and measured speedup.
