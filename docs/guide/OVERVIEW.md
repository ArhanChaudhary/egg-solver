# What egg-solver does

[Documentation home](../README.md) · Next: [Quick start](QUICKSTART.md)

egg-solver accepts a cube state or a scramble and searches for a shortest solution. It uses the **half-turn metric**: turning one face by 90°, 180°, or 270° counts as one move. “Optimal” describes the solution length. Runtime speed is measured separately.

The main tradeoff is memory. A large precomputed **pruning table** tells the search when a partial path cannot reach a solution within its remaining move budget. Better bounds let the solver discard more work. Smaller tables retain optimal-search semantics but usually require more searching.

## Tables and memory

The supported tables are named h0 through h11. Each increase in K approximately doubles the table's storage. Leave RAM for the program, operating system, and other applications.

| Table | Approximate data size | Typical use |
|---|---:|---|
| h0 | 28 MiB | Small examples, tests and learning the workflow |
| h6 | 1.8 GiB | Machines with limited memory |
| h7 | 3.6 GiB | A smaller practical table |
| h8 | 7.3 GiB | A larger laptop or desktop table |
| h9 | 14.5 GiB | Higher-memory desktop |
| h10 | 29 GiB | Workstation |
| h11 | 58 GiB | Large-memory workstation or server |

These are data sizes; files can also contain metadata or page padding. The comparison tables use **decimal GB**, while this guide uses **GiB**. That explains numbers such as 62.26 GB versus 58 GiB for h11.

Generate a table once and reuse it. Table generation and loading are separate costs from searching. The principal comparison timings exclude both, so consult the generation and shared-memory evidence if startup time matters to your application.

## One cube or many cubes

For an interactive solve, several workers can cooperate on one cube. For a batch, workers and interleaved search lanes process many cubes concurrently. These are different workloads:

- **Latency:** how long a particular cube takes to finish.
- **Throughput:** how many cubes finish per second over a batch or stream.

The [comparison guide](../comparisons/README.md) keeps these measurements separate. A throughput lead does not by itself specify interactive response time.

## Supported execution paths

The source includes native x86 and ARM/NEON implementations and a WebAssembly interface for browsers and Bun. Build choices depend on the target CPU. The full [user guide](../reference/USER_GUIDE.md) describes cross-compilation, portable builds, table files, shared-memory loading, and browser deployment.

The code is intended to solve optimally. The evidence includes independent short-state checks, known optimal lengths, agreement across search modes and solvers, and solution verification. Read [Correctness and safety](../safety/README.md) for what those checks establish and what they do not.
