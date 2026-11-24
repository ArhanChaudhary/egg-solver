#!/usr/bin/env python3
"""The tables of README.md from the raw files of this folder. usage: python3 summarize.py"""
import collections, pathlib, re, statistics as st

here = pathlib.Path(__file__).parent
LANES = [8, 12, 16, 20, 24, 32, 40, 48]


def rows(name, fields):
    for line in (here / name).read_text().splitlines():
        parts = line.split()
        if len(parts) == fields and re.fullmatch(r"h\d+", parts[0]):
            yield parts


def table(header, lines):
    print("| " + " | ".join(header) + " |")
    print("|" + "|".join("---" if i == 0 else "---:" for i in range(len(header))) + "|")
    for line in lines:
        print("| " + " | ".join(line) + " |")
    print()


print("## Code versions at one thread (EPYC 9V45), mean seconds per set\n")
for name, label in [("window-1/ab-h6.txt", "h6, 17-move set, 40 lanes"), ("window-1/ab-h9.txt", "h9, 18-move set, 32 lanes")]:
    times = collections.defaultdict(list)
    for p in rows(name, 7):
        times[(p[2], p[4])].append(float(p[5]))
    out = []
    for cpu in ["native", "znver4"]:
        for mode in ["c1", "c0"]:
            ref = st.mean(times[(mode, "28a1953-" + cpu)])
            cells = [f"{label}, `-{mode[0]} {mode[1]}`, target-cpu={cpu}", str(len(times[(mode, "28a1953-" + cpu)]))]
            for version in ["28a1953", "c3eb909", "dc153b3"]:
                mean = st.mean(times[(mode, f"{version}-{cpu}")])
                cells.append(f"{mean:.3f} ({(mean / ref - 1) * 100:+.1f}%)")
            out.append(cells)
    table(["Run", "Runs each", "28a1953", "c3eb909", "dc153b3"], out)

print("## Code versions on the Apple M2, mean seconds per set (40 lanes)\n")
times = collections.defaultdict(list)
for name in ["m2/m2-a.txt", "m2/m2-b.txt"]:
    for p in rows(name, 9):
        times[(p[0], p[1], p[2], int(p[3][1:]), p[4], p[6])].append(float(p[7]))
out = []
for key in sorted({k[:5] for k in times if k[3] == 40 and (k[0], k[1], k[2], 40, k[4], "28a1953") in times}):
    ref = st.mean(times[key + ("28a1953",)])
    cells = [f"{key[0]}, {key[1][1:]}-move set, `-c {key[2][1:]}`, {key[4][1:]} thread(s)"]
    for version in ["28a1953", "c3eb909", "dc153b3"]:
        mean = st.mean(times[key + (version,)])
        cells.append(f"{mean:.3f} ({(mean / ref - 1) * 100:+.1f}%)")
    out.append(cells)
table(["Run", "28a1953", "c3eb909", "dc153b3"], out)

print("## Lane counts at one thread (EPYC 9V45, dc153b3), mean seconds of two runs\n")
times = collections.defaultdict(list)
for p in rows("window-1/lanes.txt", 7):
    times[(p[0], p[1], p[2], int(p[3][1:]))].append(float(p[5]))
sets = [("h6", "s17"), ("h7", "s17"), ("h8", "s18"), ("h9", "s18")]
for mode in ["c1", "c0"]:
    print(f"`solve -t 1 -c {mode[1]}`:\n")
    table(["Lanes"] + [f"{t} ({s[1:]}-move set)" for t, s in sets],
          [[str(L)] + [f"{st.mean(times[(t, s, mode, L)]):.3f}" for t, s in sets] for L in LANES])

print("## Lane counts on the Apple M2 (dc153b3), mean seconds of two runs\n")
times = collections.defaultdict(list)
for name in ["m2/m2-a.txt", "m2/m2-b.txt"]:
    for p in rows(name, 9):
        if p[6] == "dc153b3":
            times[(p[0], p[1], p[2], p[4], int(p[3][1:]))].append(float(p[7]))
cols = sorted({k[:4] for k in times if len({q[4] for q in times if q[:4] == k[:4]}) > 2})
table(["Lanes"] + [f"{c[0]}, {c[1][1:]}-move set, `-c {c[2][1:]}`, {c[3][1:]}T" for c in cols],
      [[str(L)] + [f"{st.mean(times[c + (L,)]):.3f}" if c + (L,) in times else "" for c in cols] for L in LANES])

print("## The new `bench tune` on h7 against `solve` (EPYC 9V45, 48 threads available)\n")
text = (here / "window-2/h7.txt").read_text()
rate = {(m[1], int(m[2]), int(m[3])): float(m[4]) for m in re.finditer(r"(batch|one) threads (\d+) lanes (\d+): ([\d.]+) M nodes/s", text)}
times = collections.defaultdict(list)
for p in rows("window-2/h7.txt", 7):
    if p[0] == "h7":
        times[(p[4], p[2], int(p[3][1:]))].append(float(p[5]))
head = ["Lanes"]
for threads in [24, 48]:
    for mode, word in [("c1", "one"), ("c0", "batch")]:
        head += [f"{threads}T `-c {mode[1]}`: tune, M nodes/s", f"{threads}T `-c {mode[1]}`: solve, s"]
out = []
for L in LANES:
    cells = [str(L)]
    for threads in [24, 48]:
        for mode, word in [("c1", "one"), ("c0", "batch")]:
            cells += [f"{rate[(word, threads, L)]:.0f}", f"{st.mean(times[(f't{threads}', mode, L)]):.3f}"]
    out.append(cells)
table(head, out)
print("One thread, M nodes/s: " + "; ".join(f"{word}: " + ", ".join(f"{L}: {rate[(word, 1, L)]:.1f}" for L in LANES) for word in ["batch", "one"]) + "\n")

print("## The fix against dc153b3 on h7, 24 lanes, mean seconds of 8 runs\n")
times = collections.defaultdict(list)
for line in text.splitlines():
    p = line.split()
    if p and p[0] == "solve" and len(p) == 6:
        times[("`solve`", p[1], "-c " + p[2][1:], p[3])].append(float(p[4]))
    m = re.search(r"^benchshare (t\d+) share(\d) (\S+) .*time ([\d.]+)s", line)
    if m:
        times[("`bench batch`", m[1], "--share=" + m[2], m[3])].append(float(m[4]))
out = []
for key in sorted({k[:3] for k in times}):
    old, new = st.mean(times[key + ("dc153b3-native",)]), st.mean(times[key + ("tune-fix-native",)])
    out.append([f"{key[0]} `{key[2]}`, {key[1][1:]} thread(s)", f"{old:.3f}", f"{new:.3f}", f"{(new / old - 1) * 100:+.1f}%"])
table(["Run", "dc153b3", "fix", "Difference"], out)
