#!/usr/bin/env python3
"""Markdown tables of egg-solver's speed from the raw runs (summarize.py's rows): the full
tables of docs/performance/SPEED.md and the short one of the README.
usage: tables.py OUTDIR M2DIR"""
import json, pathlib, statistics as st, subprocess, sys

out, m2 = sys.argv[1], pathlib.Path(sys.argv[2])
here = pathlib.Path(__file__).parent
rows = json.loads(subprocess.run([sys.executable, str(here / "summarize.py"), out], capture_output=True, text=True, check=True).stdout)
SIZE = {"h6": "1.81", "h7": "3.62", "h8": "7.25", "h9": "14.50", "h10": "28.99", "h11": "58.00"}
get = lambda k, c: next((r for r in rows if r["table"] == k and r["cfg"] == c), None)
tables = [k for k in SIZE if get(k, "48")]


def ms(x):
    return f"{x:,.0f}" if x >= 100 else f"{x:.1f}" if x >= 10 else f"{x:.2f}"


def table(head, lines):
    print("| " + " | ".join(head) + " |")
    print("|" + "|".join("---" if i == 0 else "---:" for i in range(len(head))) + "|")
    for l in lines:
        print("| " + " | ".join(l) + " |")
    print()


print("<!-- README -->")
table(["Table (GiB)", "Nodes per solve", "1 thread", "14 threads", "48 threads", "48 threads, batch"],
      [[f"{k} ({SIZE[k]})", f"{get(k, '48')['nodes'] / 1e6:.1f} M"]
       + [f"{ms(get(k, c)['ms'])} ms · {get(k, c)['ns_node']:.2f} ns/node" for c in ("1", "14", "48")]
       + [f"{get(k, 'b48')['solves']:,.0f} solves/s"] for k in tables])

print("<!-- one cube at a time -->")
table(["Table (GiB)", "Threads", "Mean ms/solve", "Median ms", "Slowest ms", "Solves/s", "Nodes per solve", "ns/node"],
      [[f"{k} ({SIZE[k]})", c, ms(r["ms"]), ms(r["median_ms"]), ms(r["max_ms"]), f"{r['solves']:,.1f}",
        f"{r['nodes'] / 1e6:.1f} M", f"{r['ns_node']:.2f}"]
       for k in tables for c in ("1", "8", "14", "48") if (r := get(k, c))])
print("<!-- batch -->")
table(["Table (GiB)", "Solves/s", "ms per cube", "Nodes per solve", "ns/node"],
      [[f"{k} ({SIZE[k]})", f"{r['solves']:,.0f}", ms(r["ms"]), f"{r['nodes'] / 1e6:.1f} M", f"{r['ns_node']:.3f}"]
       for k in tables if (r := get(k, "b48"))])
print("<!-- spread -->")
print(", ".join(f"{r['table']} {r['cfg']}: {ms(r['ms_spread'][0])}–{ms(r['ms_spread'][1])}" for r in rows))
print()
print("<!-- M2 -->")
lines = []
for k in ("h6", "h7"):
    for name, label in (("t1-c1", "1 (a performance core)"), ("t4-c1", "4 (the performance cores)"), ("t8-c1", "8 (all cores)"), ("t8-c0", "8, batch")):
        f = m2 / f"{k}-{name}.jsonl"
        ls = [json.loads(l) for l in f.read_text().splitlines() if l.strip()]
        t, s = ls[-1], [l for l in ls if l.get("type") == "solved"]
        assert all(l["ok"] for l in s) and len(s) == t["cubes"]
        nodes = sum(l["nodes"] for l in s)
        lines.append([f"{k} ({SIZE[k]})", label, str(t["cubes"]), ms(1000 * t["wall"] / t["cubes"]),
                      "" if name.endswith("c0") else ms(1000 * st.median(l["seconds"] for l in s)),
                      f"{t['cubes'] / t['wall']:.2f}", f"{nodes / t['cubes'] / 1e6:.1f} M", f"{1e9 * t['wall'] / nodes:.2f}"])
table(["Table (GiB)", "Threads", "States", "Mean ms/solve", "Median ms", "Solves/s", "Nodes per solve", "ns/node"], lines)
print("<!-- lengths -->")
r = get("h6", "48")
print(r["lengths"], get("h6", "1")["lengths"])
