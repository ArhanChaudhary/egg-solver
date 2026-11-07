# A short glossary

[Documentation home](../README.md) · [Comparison guide](../comparisons/README.md)

| Term | Meaning |
|---|---|
| Optimal solution | A solution with the fewest moves under the stated move metric |
| Half-turn metric | A 90°, 180°, or 270° turn of one face counts as one move |
| IDA* | A search that tries increasing move budgets and prunes using lower bounds |
| Pruning table | Precomputed information that helps prove a branch needs too many moves |
| Admissible bound | A lower bound that never overestimates the true remaining distance |
| Node / lookup | A visited search state / a table access; implementations count these differently |
| Latency | Time for one cube to finish |
| Throughput | Completed cubes per unit time across a workload |
| Worker | A thread that performs search work |
| Lane | One independently advanced search state within a worker |
| Prefetch | Starting a memory access before the program needs its value |
| SIMD | CPU instructions that operate on several small values together |
| Huge page / THP | A larger virtual-memory page / the operating system's transparent huge-page mechanism |
| TLB | The CPU cache for translating virtual addresses to physical memory; page size affects its coverage |
| NUMA | A machine layout where memory-access cost depends on which processor accesses which memory |
| SMT | Multiple logical CPU threads sharing a physical core's resources |
| PGO | Profile-guided optimization: compiler decisions informed by a training workload |
| LTO / BOLT | Link-time optimization / a tool for optimizing machine code after linking |
| PMU | Hardware performance-monitoring counters, such as cycles, instructions and cache events |
| Paired A/B run | A baseline and candidate measured under matched conditions, with their ratio recorded |
| Miri | An interpreter used to detect undefined behavior in tested Rust executions; finite runs are not a universal proof |

The [full report](../comparisons/RESULTS.md) defines its exact timing formulas and sampling rules. Use those definitions when interpreting a table.
