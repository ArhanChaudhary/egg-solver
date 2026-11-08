#!/bin/bash
set -euo pipefail
cd /data/agents/nxopt32/src
g++ -DPS=16 -std=c++11 -O3 -g -march=native -DHALF -Wall -DTHREADS -DEO3 -DEP2 -o ../bin/nxopt32 nxopt.cpp cubepos.cpp -lpthread
