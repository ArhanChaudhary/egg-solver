# Reading the benchmark tables

[Benchmarks](README.md) · [Benchmark catalog](../BENCHMARK_CATALOG.md)

## Choose the metric first

| Metric | Calculation | Better direction | What it answers |
|---|---|---|---|
| Complete-batch throughput | Cubes completed ÷ batch wall seconds | Higher | How quickly does the complete workload finish? |
| Batch time per cube | Batch wall seconds ÷ cubes | Lower | The throughput-equivalent cost per cube; not an individual cube's response time |
| Single-cube latency | Elapsed time for that cube | Lower | How long does one solve take? |
| Normalized worker time | Sum of cube durations ÷ workers ÷ cubes | Lower | A harness normalization; does not include idle time at the end of a batch |

The newest nissy study uses complete-batch wall time. Some older nxopt/vcube studies use normalized worker time. Do not invert the latter and label it a measured end-to-end completion rate.

For throughput, **egg rate advantage = egg solves/s ÷ competitor solves/s**. With identical batch sizes this also equals competitor batch time ÷ egg batch time. For latency, an advantage uses competitor time ÷ egg time. A value above 1 favors egg; the table must still identify the metric and measured source.

## Units and configuration

| Label | Meaning |
|---|---|
| MB / GB | Decimal: 1,000,000 / 1,000,000,000 bytes |
| MiB / GiB | Binary: 1,048,576 / 1,073,741,824 bytes |
| h6, h10, etc. | H48 table setting; implementations can have different layouts and byte sizes at the same h |
| Table data | Logical pruning bytes, excluding metadata and page padding |
| Mapped table RAM | Allocation rounded to the actual page size; excludes the solver's other memory |
| Worker / thread | An operating-system execution thread; a cube-worker process can use several threads |
| Lane | Interleaved search state within a worker; not a CPU core |
| 1 GiB pages | Explicit huge pages backing the table, with coverage checked for the run |
| THP | Transparent huge pages; historical coverage can be partial |

For example, the same 486,420,480 bytes are 486.42 MB or 463.89 MiB. A different displayed unit does not establish a stronger table.

## Samples and variation

Three runs of the same 500 states are **1,500 solves of 500 distinct states**. A median over fresh processes, a range over repeated groups in one process, and an interval over different cubes describe different variation. They are not interchangeable confidence estimates.

The tables identify whether loading, generation, warm-up, result verification and output are timed. A resident-table search result does not predict cold startup time. Node counts can change with parallel stop timing and search order, even when all returned optimal lengths agree.

## Historical matrix notation

The dated detailed matrix retains its original numbers and compact notation:

| Historical label | Read it as |
|---|---|
| 1T / 48T | One / 48 configured worker threads, except explicitly capped competitors |
| 1G | 1 GiB table pages |
| `sec/solv` | Seconds per cube under that section's timing formula |
| `median [min–max, repeats]` | Median and observed range over the stated repeats |
| `±…%` at 1T | The report's interval over sampled cube times; not run-to-run variation |
| GB·s | Decimal table GB × the section's seconds-per-cube value |
| `size` | Competitor table size divided by the paired egg table size |
| `shipped`, `native`, `pgo`, `pf…` | Historical build labels; see the matrix's build-screen section and raw metadata |

## Scope of the claims

A result describes the stated CPU, source, executable, table, input bytes and timing boundary. It does not establish a universal fastest solver. In particular, an unidentified current Mac cubeopt release cannot be substituted for the supplied older Linux executable. Correct replay and matching known optimal lengths are checks on the tested inputs, not a proof about every program execution.
