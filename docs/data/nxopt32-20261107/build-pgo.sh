#!/bin/bash
set -euo pipefail
cd /data/agents/nxopt32/src
mode=${1:?gen or use}
mkdir -p ../obj ../profile
flags=(-DPS=16 -std=c++11 -O3 -g -march=native -DHALF -Wall -DTHREADS -DEO3 -DEP2)
case "$mode" in
 gen) profile=(-fprofile-generate=/data/agents/nxopt32/profile -fprofile-update=single); output=nxopt32-pgogen ;;
 use) profile=(-fprofile-use=/data/agents/nxopt32/profile -Werror=missing-profile); output=nxopt32-pgo ;;
 *) exit 2 ;;
esac
g++ "${flags[@]}" "${profile[@]}" -c nxopt.cpp -o ../obj/nxopt.o
g++ "${flags[@]}" "${profile[@]}" -c cubepos.cpp -o ../obj/cubepos.o
g++ "${flags[@]}" "${profile[@]}" ../obj/nxopt.o ../obj/cubepos.o -lpthread -o "../bin/$output"
