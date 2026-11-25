# How fast egg-solver is

[Performance studies](README.md) · [Benchmark methods](../comparisons/METHODS.md) · [Lanes and parallelism](PARALLELISM.md)

Measured **November 25, 2026** with commit `17386a2` (the search code of the November 24 lane change, `5bfcd2a`), on two machines, by egg-solver alone: how long one random cube takes, how many nodes the search visits, and how much time each node costs. The [comparisons](../comparisons/README.md) set egg-solver against other solvers; they were measured earlier, with 1 GiB table pages, and are not repeated here.

## How it was measured

- **Cubes:** `testdata/random_state.txt`, 500 uniformly random states (SHA-256 `616810c3…6905de3`). Their optimal lengths: 15 of 16 moves, 126 of 17, 339 of 18 and 20 of 19. With one thread the first 100 states were solved (6, 27, 63 and 4 of 16 to 19 moves); with more threads all 500.
- **Server:** a 48-core AMD EPYC 9V45 (Zen 5) VM, one thread per core, 94 GB of memory. Tables h6 to h11 were made by `gen K auto` and copied into memory (`solve`'s default load) on transparent huge pages (`solve -v` reported 97 to 100% of each table on them). The earlier studies used 1 GiB pages, which were not available this time; on h10 they were about 10% faster (see below).
- **Laptop:** an Apple M2 (4 performance and 4 efficiency cores, 16 GB) running Fedora Asahi, h6 and h7 only (h8 and up do not fit). One thread ran on a performance core and four on the four performance cores. With one thread it solved the first 50 states, otherwise the first 200.
- **Runs:** `solve --json` with 24 lanes per thread (the default), `-c 1` (one cube at a time, all threads on it) or `-c 0` (the batch), timed by `solve` itself; loading the table is not included. On the server every configuration ran twice, in opposite orders, under the machine's benchmark lock with nothing else running; the tables give the mean of the two runs, which were within 2.6% of each other. The laptop ran each configuration once. Every solution was checked by `solve`.
- **Metrics:** *ms/solve* is the run's wall time divided by its cubes; with one cube at a time that is the mean time to solve a cube. *Solves/s* is its inverse. *Nodes per solve* counts the positions that egg-solver's search generates (its own count; other solvers count differently). *ns/node* is the wall time divided by all the nodes, so with several threads it is the time per node of the whole machine (14 threads at 0.85 ns/node: each thread takes about 12 ns per node).

## One cube at a time

`solve -c 1`: all threads on one cube, then the next. Mean and median are per cube; the slowest is the slowest cube of the two runs.

| Table (GiB) | Threads | Mean ms/solve | Median ms | Slowest ms | Solves/s | Nodes per solve | ns/node |
|---|---:|---:|---:|---:|---:|---:|---:|
| h6 (1.81) | 1 | 1,223 | 624 | 7,289 | 0.8 | 105.6 M | 11.58 |
| h6 (1.81) | 8 | 147 | 79.7 | 2,144 | 6.8 | 100.1 M | 1.47 |
| h6 (1.81) | 14 | 84.9 | 46.4 | 1,280 | 11.8 | 100.3 M | 0.85 |
| h6 (1.81) | 48 | 30.9 | 17.5 | 518 | 32.3 | 103.1 M | 0.30 |
| h7 (3.62) | 1 | 591 | 322 | 3,834 | 1.7 | 50.7 M | 11.64 |
| h7 (3.62) | 8 | 76.9 | 39.4 | 1,196 | 13.0 | 51.6 M | 1.49 |
| h7 (3.62) | 14 | 44.3 | 23.0 | 682 | 22.6 | 51.7 M | 0.86 |
| h7 (3.62) | 48 | 16.2 | 8.80 | 268 | 61.8 | 52.7 M | 0.31 |
| h8 (7.25) | 1 | 273 | 167 | 1,824 | 3.7 | 22.8 M | 11.97 |
| h8 (7.25) | 8 | 35.4 | 20.8 | 264 | 28.2 | 23.0 M | 1.54 |
| h8 (7.25) | 14 | 20.4 | 12.0 | 151 | 49.0 | 23.1 M | 0.88 |
| h8 (7.25) | 48 | 7.53 | 4.72 | 54.6 | 132.7 | 23.6 M | 0.32 |
| h9 (14.50) | 1 | 144 | 87.3 | 984 | 6.9 | 11.5 M | 12.55 |
| h9 (14.50) | 8 | 19.4 | 11.8 | 190 | 51.4 | 11.8 M | 1.65 |
| h9 (14.50) | 14 | 11.2 | 6.83 | 104 | 89.6 | 11.8 M | 0.95 |
| h9 (14.50) | 48 | 4.36 | 2.87 | 32.2 | 229.5 | 12.1 M | 0.36 |
| h10 (28.99) | 1 | 81.7 | 48.8 | 524 | 12.2 | 6.2 M | 13.29 |
| h10 (28.99) | 8 | 10.7 | 6.86 | 77.4 | 93.7 | 6.0 M | 1.79 |
| h10 (28.99) | 14 | 6.29 | 4.13 | 45.0 | 159.0 | 6.0 M | 1.05 |
| h10 (28.99) | 48 | 2.56 | 1.85 | 17.3 | 390.0 | 6.2 M | 0.41 |
| h11 (58.00) | 1 | 51.1 | 28.4 | 289 | 19.6 | 3.5 M | 14.47 |
| h11 (58.00) | 8 | 6.52 | 4.14 | 74.3 | 153.5 | 3.3 M | 1.99 |
| h11 (58.00) | 14 | 3.89 | 2.54 | 42.9 | 257.1 | 3.3 M | 1.18 |
| h11 (58.00) | 48 | 1.77 | 1.30 | 17.5 | 566.2 | 3.5 M | 0.51 |

## A batch of 500

`solve -t 48 -c 0`: all 500 states at once, the threads sharing their searches. *ms per cube* is the batch's wall time divided by 500, not the time any one cube took.

| Table (GiB) | Solves/s | ms per cube | Nodes per solve | ns/node |
|---|---:|---:|---:|---:|
| h6 (1.81) | 34 | 29.8 | 103.6 M | 0.287 |
| h7 (3.62) | 66 | 15.2 | 52.6 M | 0.289 |
| h8 (7.25) | 141 | 7.10 | 24.1 M | 0.295 |
| h9 (14.50) | 265 | 3.77 | 12.0 M | 0.315 |
| h10 (28.99) | 486 | 2.06 | 6.2 M | 0.334 |
| h11 (58.00) | 840 | 1.19 | 3.3 M | 0.359 |

## The laptop

| Table (GiB) | Threads | States | Mean ms/solve | Median ms | Solves/s | Nodes per solve | ns/node |
|---|---:|---:|---:|---:|---:|---:|---:|
| h6 (1.81) | 1 (a performance core) | 50 | 1,542 | 791 | 0.65 | 102.0 M | 15.12 |
| h6 (1.81) | 4 (the performance cores) | 200 | 394 | 225 | 2.54 | 101.1 M | 3.89 |
| h6 (1.81) | 8 (all cores) | 200 | 309 | 178 | 3.24 | 101.4 M | 3.04 |
| h6 (1.81) | 8, batch | 200 | 328 |  | 3.05 | 102.9 M | 3.19 |
| h7 (3.62) | 1 (a performance core) | 50 | 781 | 443 | 1.28 | 51.0 M | 15.30 |
| h7 (3.62) | 4 (the performance cores) | 200 | 202 | 111 | 4.95 | 50.9 M | 3.97 |
| h7 (3.62) | 8 (all cores) | 200 | 157 | 86.9 | 6.39 | 51.1 M | 3.06 |
| h7 (3.62) | 8, batch | 200 | 165 |  | 6.07 | 51.7 M | 3.18 |

## Reading the numbers

- **Bigger tables, fewer nodes.** From h6 to h11 the nodes per solve fall 30-fold (103 to 3.5 million) and the time per solve with one thread 24-fold.
- **Time per node** with one thread is 11.6 ns on h6 and grows with the table, to 14.5 ns on h11: nearly every node waits for a table lookup that misses the caches, and 24 lanes per thread keep about 24 of them in flight. The bigger the table, the more of those lookups also miss the address-translation caches.
- **Threads:** per node, 48 threads one cube at a time were 38 times as fast as one on h6 and h7, 35 on h9, 32 on h10 and 28 on h11 (per node, because one thread solved the first 100 states and more threads all 500). The batch was 4% faster than one cube at a time on h6 and 49% on h11: with one cube at a time, short searches leave threads idle at their ends.
- **The slowest cube** of the first 100 took about six times the mean with one thread (a 19-move state).
- **The laptop:** an M2 performance core took 15.1 to 15.3 ns per node with one thread, against 11.6 on the server. The four performance cores were 3.9 times as fast as one; the four efficiency cores added another 28 to 30%. A batch was slower there than one cube at a time.
- **1 GiB pages against transparent huge pages:** the [cube48opt10 audit](../comparisons/CUBEOPT_AUDIT.md) measured egg-solver on h10 at 48 threads, one cube at a time, at 2.36 ms per solve with 1 GiB pages (November 8, 32 lanes); here it took 2.56 ms, 8% longer. The [nissy throughput study](../comparisons/NISSY_THROUGHPUT.md) measured the h10 batch at 561 solves/s with 1 GiB pages; here 486, 13% fewer. Most of that is the pages: 24 and 32 lanes were within 2.5% on h10 in [the lane screens](PARALLELISM.md).

## Your own machine

```sh
target/release/solve --json --table tables/h7 -t 8 -c 1 < testdata/random_state.txt | tail -1
```

The last line has `wall`, `cubes` and `nodes_per_cube`: ms/solve is 1000 × wall ÷ cubes, and ns/node is 10⁹ × wall ÷ (cubes × nodes_per_cube). `bench tables/h7 tune` measures the best lane count for the machine first (about three minutes per table; see the [user guide](../reference/USER_GUIDE.md#tune-and-benchmark)).

## Files

[The raw runs](../data/speed-20261125/README.md): every `solve --json` output, the scripts that made them, and `tables.py`, which prints the tables above from them.
