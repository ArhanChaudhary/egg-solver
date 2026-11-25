#!/usr/bin/env python3
"""Tables of egg-solver's speed from the raw runs (run-char.sh): one cube at a time at 1, 8, 14
and 48 threads and a batch at 48 threads; the two rounds averaged, their spread given.
usage: summarize.py OUTDIR [TABLE...]"""
import json, pathlib, statistics as st, sys

out = pathlib.Path(sys.argv[1])
tables = sys.argv[2:] or ["h6", "h7", "h8", "h9", "h10", "h11"]
CONFIGS = [("1", "1 thread, one cube at a time (first 100 states)"), ("8", "8 threads, one cube at a time"),
           ("14", "14 threads, one cube at a time"), ("48", "48 threads, one cube at a time"),
           ("b48", "48 threads, all 500 as one batch")]


def run(path):
    lines = [json.loads(l) for l in path.read_text().splitlines() if l.strip()]
    total = lines[-1]
    solved = [l for l in lines if l.get("type") == "solved"]
    assert total["type"] == "total" and len(solved) == total["cubes"] and all(l["ok"] for l in solved), path
    nodes = sum(l["nodes"] for l in solved)
    return {"cubes": total["cubes"], "wall": total["wall"], "nodes": nodes,
            "lengths": [l["length"] for l in solved], "seconds": [l["seconds"] for l in solved]}


rows = []
for k in tables:
    for cfg, label in CONFIGS:
        runs = [run(p) for p in sorted(out.glob(f"{k}-{cfg}-r*.jsonl"))]
        if not runs:
            continue
        ms = [1000 * r["wall"] / r["cubes"] for r in runs]
        nps = [r["nodes"] / r["cubes"] for r in runs]
        nsn = [1e9 * r["wall"] / r["nodes"] for r in runs]
        rows.append({"table": k, "cfg": cfg, "label": label, "runs": len(runs), "cubes": runs[0]["cubes"],
                     "ms": st.mean(ms), "ms_spread": (min(ms), max(ms)), "solves": 1000 / st.mean(ms),
                     "nodes": st.mean(nps), "ns_node": st.mean(nsn),
                     "median_ms": st.mean(1000 * st.median(r["seconds"]) for r in runs) if cfg != "b48" else None,
                     "max_ms": max(1000 * max(r["seconds"]) for r in runs) if cfg != "b48" else None,
                     "lengths": {n: runs[0]["lengths"].count(n) for n in sorted(set(runs[0]["lengths"]))}})
print(json.dumps(rows, indent=1))
