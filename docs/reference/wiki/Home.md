# Reference copy: cube_solver_test wiki home page

This page is a copy of the home page of the [cube_solver_test wiki](https://github.com/cs0x7f/cube_solver_test/wiki) (repository [cs0x7f/cube_solver_test](https://github.com/cs0x7f/cube_solver_test)), taken for the November 2026 solver comparison. It is not egg-solver documentation. Its timing tables were measured on an Intel Core i7-10750H, not on this project's machine. The text below this note is unchanged from the copy. [`final-report.py`](../../reproduce/final-report.py) reads its tables as the published reference values; the corresponding measurements on this project's machine are in [the comparison results](../../data/comparison/results.md#cube-solver-test-wiki-metrics-this-machine).

# Solvers

| Solver                                                                                     | Language | Variations                                       |
|--------------------------------------------------------------------------------------------|----------|--------------------------------------------------|
| [nxopt](https://github.com/rokicki/cube20src)                                              |   c++    | nxopt11, nxopt21, nxopt31, nxopt22               |
| [vcube](https://github.com/Voltara/vcube/)                                                 |   c++    | vcube104, vcube108, vcube112, vcube208, vcube308 |
| [reid's optimal solver](https://www.cflmath.com/Rubik/optimal_solver.html)                 |    c     | reid                                             |
| [Kociemba's optimal solver](https://github.com/hkociemba/RubiksCube-OptimalSolver)         |  python  | kocpy                                            |
| [Cube Explorer](https://kociemba.org/cube.htm)                                             |  pascal  | CE, CEhuge                                       |
| Open Source [Cube Explorer](https://github.com/cs0x7f/CubeExplorer/tree/fpc)               |  pascal  | kocce0, kocce1, kocce2                           |
| [3ColorCube](https://github.com/mfeather1/3ColorCube)                                      |javascript| 3Color835M, 3ColorOpt                            |
| [stickersolve](https://github.com/TAR-ALEX/stickersolve)                                   |   c++    | sticker                                          |
| [Rubiks-Cube-Optimal-Solver](https://github.com/abdelmaged92/Rubiks-Cube-Optimal-Solver)   |   java   | optjava                                          |
| [nissy-classic](https://github.com/sebastianotronto/nissy-classic)                         |    c     | nissyc                                           |
| [nissy-core](https://github.com/sebastianotronto/nissy-core)                               |    c     | h48h0, h48h1, .., h48h11                         |
| [cubeopt](https://github.com/cs0x7f/cubeopt)                                               |   c++    | cubeopt, cube48opt, cubenp                       |

# Space-node product

Pruning table size * number of nodes visited, less is better.

Average of 100 solves, in units of 1e15 Bytes (1PB).

| Solver     | Table/MB |  15f  |  16f   |   17f   |   inf   |
|------------|----------|-------|--------|---------|---------|
| nxopt11    | 20.3366  | 0.085 |  1.030 |  13.757 |         |
| nxopt21    | 325.386  | 0.147 |  1.367 |  16.563 |         |
| nxopt31    | 2603.08  | 0.315 |  2.246 |  20.307 | 183.431 |
| nxopt22    | 7809.26  | 0.664 |  2.936 |  28.020 | 216.542 |
| vcube104   | 20.3366  | 0.087 |  1.028 |  13.720 |         |
| vcube108   | 325.386  | 0.140 |  1.398 |  17.019 |         |
| vcube112   | 2603.08  | 0.279 |  2.306 |  20.400 | 182.078 |
| vcube208   | 7809.26  | 0.586 |  2.874 |  28.184 | 217.277 |
| vcube308   | 22777.0  | 0.501 |  3.439 |  28.038 | 221.597 |
| reid       | 35.22*2  | 0.907 | 11.360 |  95.391 |         |
| kocpy      | 833.172  | 0.916 | 11.057 | 110.312 |         |
| kocce0     | 28.1816  | 0.166 |  2.021 |  20.483 |         |
| kocce1     | 705.886  | 0.514 |  6.263 |  62.805 | 451.286 |
| kocce2     | 1972.71  | 0.532 |  6.269 |  63.334 | 450.707 |
| CE         | 28.1816  | 0.169 |  2.021 |  20.483 |         |
| CEhuge     | 1972.71  | 0.509 |  6.269 |  63.334 | 436.803 |
| 3Color835M | 1043.65  | 1.514 | 15.561 | 163.006 |         |
| 3ColorOpt  | 2959.11  | 1.497 | 14.085 | 138.270 | 763.862 |
| sticker    | 12884.9  | 7.524 | 52.351 | 508.051 |         |
| nissyc     | 2465.89  | 0.284 |  3.134 |  33.405 | 249.992 |
| optjava    | 1972.71  | 0.696 |  7.654 |  76.829 | 545.364 |
| h48h0      | 31.6839  | 0.385 |  2.674 |         |         |
| h48h1      | 61.3072  | 0.414 |  2.649 |         |         |
| h48h2      | 120.553  | 0.465 |  2.621 |  29.264 |         |
| h48h3      | 239.047  | 0.559 |  2.875 |  28.383 |         |
| h48h4      | 476.033  | 0.824 |  3.660 |  33.336 |         |
| h48h5      | 950.006  | 1.085 |  5.305 |  32.799 | 285.473 |
| h48h6      | 1897.95  | 1.646 |  7.148 |  33.918 | 274.457 |
| h48h7      | 3793.84  | 2.713 |  5.990 |  37.688 | 277.246 |
| h48h8      | 7585.62  | 5.158 | 10.491 |  54.859 | 343.333 |
| h48h9      | 15169.1  | 8.904 | 14.881 |  56.217 | 312.741 |
| cubeopt10  | 20.3366  | 0.076 |  0.920 |  12.304 |         |
| cubeopt01  | 88.9728  | 0.155 |  1.911 |  21.347 |         |
| cubeopt20  | 325.386  | 0.122 |  1.340 |  15.355 |         |
| cubeopt11  | 1423.56  | 0.212 |  2.117 |  22.377 | 213.026 |
| cubeopt30  | 2603.08  | 0.185 |  1.859 |  18.323 | 175.937 |
| cube48opt1 | 30.4012  | 0.205 |  2.369 |  25.802 |         |
| cube48opt2 | 121.605  | 0.203 |  2.083 |  23.104 |         |
| cube48opt3 | 243.210  | 0.208 |  2.127 |  22.780 |         |
| cube48opt4 | 486.420  | 0.230 |  1.948 |  21.867 |         |
| cube48opt5 | 972.840  | 0.228 |  1.927 |  22.786 | 200.037 |
| cube48opt6 | 1945.68  | 0.230 |  1.985 |  22.940 | 197.870 |
| cube48opt7 | 3891.36  | 0.241 |  1.965 |  23.780 | 199.615 |
| cube48opt8 | 7782.72  | 0.335 |  2.483 |  26.780 | 197.300 |
| cube48opt9 | 15565.4  | 0.342 |  2.755 |  27.722 | 200.622 |
| cubenp10   | 22.7620  | 0.052 |  0.634 |   8.532 |         |
| cubenp01   | 99.5841  | 0.112 |  1.318 |  15.085 |         |
| cubenp20   | 364.193  | 0.083 |  0.870 |  10.307 |         |
| cubenp11   | 1593.34  | 0.158 |  1.585 |  16.383 |         |
| cubenp30   | 2913.54  | 0.125 |  1.227 |  12.718 | 114.453 |

# Number of nodes visited

Average of 100 solves, in units of 1 million, less is better.

"X-move" denotes random-move scramble of specific number of moves, "inf-move" denotes random-state scramble.

| Solver     | 15-move | 16-move | 17-move | inf-move |
|------------|---------|---------|---------|----------|
| nxopt11    |  4.180  | 50.627  | 676.488 |          |
| nxopt21    |  0.451  |  4.200  |  50.902 |          |
| nxopt31    |  0.121  |  0.863  |   7.801 |  70.467  |
| nxopt22    |  0.085  |  0.376  |   3.588 |  27.729  |
| vcube104   |  4.295  | 50.568  | 674.668 |          |
| vcube108   |  0.431  |  4.297  |  52.306 |          |
| vcube112   |  0.107  |  0.886  |   7.837 |  69.947  |
| vcube208   |  0.075  |  0.368  |   3.609 |  27.823  |
| vcube308   |  0.022  |  0.151  |   1.231 |   9.729  |
| reid       | 12.877  | 161.25  |1353.948 |          |
| kocpy      |  1.100  | 13.271  | 132.400 |          |
| kocce0     |  5.899  | 71.722  | 726.828 |          |
| kocce1     |  0.729  |  8.873  |  88.973 | 639.318  |
| kocce2     |  0.270  |  3.178  |  32.105 | 228.471  |
| CE         |  6.004  | 71.721  | 726.827 |          |
| CEhuge     |  0.258  |  3.178  |  32.105 | 221.423  |
| 3Color835M |  1.451  | 14.910  | 156.188 |          |
| 3ColorOpt  |  0.506  |  4.760  |  46.727 | 258.139  |
| sticker    |  0.584  |  4.063  |  39.430 |          |
| nissyc     |  0.115  |  1.271  |  13.547 | 101.380  |
| optjava    |  0.353  |  3.880  |  38.946 | 276.454  |
| h48h0      | 12.141  | 84.394  |         |          |
| h48h1      |  6.759  | 43.212  |         |          |
| h48h2      |  3.860  | 21.745  | 242.747 |          |
| h48h3      |  2.337  | 12.025  | 118.734 |          |
| h48h4      |  1.732  |  7.689  | 70.029  |          |
| h48h5      |  1.142  |  5.584  | 34.525  | 300.496  |
| h48h6      |  0.867  |  3.766  | 17.871  | 144.607  |
| h48h7      |  0.715  |  1.579  |  9.934  |  73.078  |
| h48h8      |  0.680  |  1.383  |  7.232  |  45.261  |
| h48h9      |  0.587  |  0.981  |  3.706  |  20.617  |
| cubeopt10  |  3.733  | 45.261  | 605.024 |          |
| cubeopt01  |  1.743  | 21.482  | 239.927 |          |
| cubeopt20  |  0.375  |  4.119  |  47.191 |          |
| cubeopt11  |  0.149  |  1.487  |  15.719 | 149.643  |
| cubeopt30  |  0.071  |  0.714  |   7.039 |  67.588  |
| cube48opt1 |  6.747  | 77.913  | 848.722 |          |
| cube48opt2 |  1.666  | 17.132  | 189.996 |          |
| cube48opt3 |  0.855  |  8.746  |  93.665 |          |
| cube48opt4 |  0.472  |  4.005  |  44.955 |          |
| cube48opt5 |  0.234  |  1.981  |  23.422 | 205.622  |
| cube48opt6 |  0.118  |  1.020  |  11.790 | 101.697  |
| cube48opt7 |  0.062  |  0.505  |   6.111 |  51.297  |
| cube48opt8 |  0.043  |  0.319  |   3.441 |  25.351  |
| cube48opt9 |  0.022  |  0.177  |   1.781 |  12.889  |
| cubenp10   |  2.302  | 27.869  | 374.873 |          |
| cubenp01   |  1.125  | 13.239  | 151.479 |          |
| cubenp20   |  0.229  |  2.388  |  28.302 |          |
| cubenp11   |  0.099  |  0.995  |  10.282 |          |
| cubenp30   |  0.043  |  0.421  |   4.365 |  39.283  |

# Average time per node

Average of 100 solves, in units of 1 nanosecond, less is better.

Please note that the following results are highly dependent on the executing environment.

## Intel Core i7-10750H (6-core 12-thread)

transparent hugepage is set to "always"

Single-thread results

| Solver     | ns/node | sec/solv | GB*sec |
|------------|---------|----------|--------|
| nxopt31    |  162.57 |  11.456  | 29.821 |
| nxopt22    |  220.52 |   6.115  | 47.752 |
| vcube112   |  106.64 |   7.459  | 19.417 |
| vcube208   |  147.54 |   4.105  | 32.057 |
| vcube308   |  210.14 |   2.044  | 46.566 |
| kocce2     |  123.37 |  28.187  | 55.605 |
| h48h5      |   80.20 |  24.099  | 22.894 |
| h48h6      |   74.39 |  10.758  | 20.417 |
| h48h7      |   76.95 |   5.624  | 21.335 |
| h48h8      |   80.10 |   3.625  | 27.500 |
| h48h9      |   98.18 |   2.024  | 30.706 |
| cubeopt30  |   70.74 |   4.781  | 12.445 |
| cubenp30   |  106.40 |   4.180  | 12.152 |
| cube48opt5 |   40.51 |   8.329  |  8.103 |
| cube48opt6 |   40.43 |   4.112  |  8.001 |
| cube48opt7 |   41.02 |   2.104  |  8.188 |
| cube48opt8 |   46.45 |   1.178  |  9.164 |
| cube48opt9 |   48.26 |   0.622  |  9.681 |

12-thread results

| Solver     | ns/node | sec/solv | GB*sec |
|------------|---------|----------|--------|
| nxopt31    |  20.747 |   1.462  |  3.806 |
| nxopt22    |  28.762 |   0.798  |  6.228 |
| vcube112   |  17.904 |   1.252  |  3.260 |
| vcube208   |  22.424 |   0.624  |  4.872 |
| vcube308   |  30.253 |   0.294  |  6.704 |
| kocce2     |  15.231 |   3.480  |  6.865 |
| h48h5      |  15.230 |   4.577  |  4.348 |
| h48h6      |  14.877 |   2.151  |  4.083 |
| h48h7      |  14.883 |   1.088  |  4.126 |
| h48h8      |  15.462 |   0.700  |  5.309 |
| h48h9      |  16.353 |   0.337  |  5.114 |
| cubeopt30  |  11.392 |   0.770  |  2.004 |
| cubenp30   |  17.040 |   0.669  |  1.946 |
| cube48opt5 |   6.920 |   1.423  |  1.384 |
| cube48opt6 |   6.852 |   0.697  |  1.356 |
| cube48opt7 |   6.927 |   0.355  |  1.383 |
| cube48opt8 |   6.788 |   0.172  |  1.339 |
| cube48opt9 |   7.445 |   0.096  |  1.494 |
| CEhuge[1]  |  19.570 |   4.333  |  8.548 |
| optjava    |  33.587 |   9.285  | 18.317 |

[1] Huge page is not enabled in windows.
