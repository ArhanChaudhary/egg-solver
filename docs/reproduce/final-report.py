"""final-report.py OUT: build OUT/results.md from final-compare.sh's logs (fairness protocol v2).

Every solver is measured on two page types (1 GiB hugetlb and 2 MiB THP), with the same cubes,
interleaved runs, >= 3 repeats at 48 threads (median and range) and one or more at 1 thread (with
the 95% interval of the mean over cubes). Speedups are only stated between rows of the same page
type, the same semantics (latency or throughput) and the same table-size group.

sec/solv (tt) per solver, all excluding table load and generation:
  cube-opt latency   mean of solve's per-cube times (-c 1), as h48 and cubeopt
  cube-opt batch     wall / cube of solve -c 0 (includes the idle tail of the batch)
  cube-opt stream    1 / steady-state throughput of bench stream (random states)
  nxopt, vcube       sum of per-cube times / cubes / nthread (one cube per thread; the harness's
                     formula, which leaves out the idle tail: it favours them)
  h48                mean of the Time printed around each nissy.solve call
  cubeopt30/48opt10  mean of the per-cube tt the binary prints
"""
import os
import re
import statistics
import sys
import json
from bisect import bisect_right

OUT = sys.argv[1]
EXTRA = sys.argv[2:]   # further OUT folders whose runs are read too (e.g. the competitors' runs)

LABEL = "egg-solver"
OURS_MB = {k: 1e-6 * 3891363840 * 2 ** (k - 7) for k in range(0, 12)}
KNOWN_MB = {"nxopt31": 2603.08, "nxopt22": 7809.26, "vcube112": 2603.08, "vcube208": 7809.26,
            "vcube308": 22777.0, "h48h7": 3793.84, "h48h8": 7585.62, "h48h9": 15169.1,
            "h48h10": 30338.2, "h48h11": 60676.4, "cubeopt30": 2602.04, "cube48opt10": 31130.4,
            "c48rsopt7": 3891.36, "c48rsbatchopt7": 3891.36, "c48rsopt8": 7782.73, "c48rsbatchopt8": 7782.73,
            "c48rsopt10": 31130.9, "c48rsbatchopt10": 31130.9}
# Size groups: (title, competitor tables, cube-opt ks).
GROUPS = [
    ("~2.6-3.9 GB tables", ["cubeopt30", "nxopt31", "vcube112", "h48h7", "c48rsopt7", "c48rsbatchopt7"], [7]),
    ("~7.6-7.8 GB tables", ["nxopt22", "vcube208", "h48h8", "c48rsopt8", "c48rsbatchopt8"], [8]),
    ("large tables (1 GiB pages only)", ["vcube308", "cube48opt10", "h48h9", "h48h10", "h48h11", "c48rsopt10",
                                         "c48rsbatchopt10"], [9, 10, 11]),
    ("small tables (smoke tests)", ["nxopt11", "nxopt21", "vcube104", "vcube108", "h48h5", "h48h6"], []),
]
BUILD_CHOICE = ("nxopt", "vcube", "h48", "c48rs-opt7", "c48rs-opt8", "c48rs-opt10")   # open-source solvers: also built -march=native and/or PGO
ALT_BUILDS = ("native", "pgo", "pf", "pfnative", "pfpgo")   # (pf*: cube48opt-rust's optional x86 table prefetch)


def ours(name):
    """cube-opt run name -> (mode, k, page) with mode '', 'batch' or 'stream'."""
    m = re.match(r"^cubeoptrs(thp|batch|batchthp|stream|streamthp)?h(\d+)$", name)
    if not m:
        return None
    v = m.group(1) or ""
    return (v.replace("thp", ""), int(m.group(2)), "thp" if v.endswith("thp") else "1g")


def family(name):
    if ours(name):
        return "cube-opt"
    m = re.match(r"^c48rs(?:batch)?(opt\d+)$", name)
    if m:   # cube48opt-rust: a build is chosen per variant (Opt7, Opt8, Opt10)
        return "c48rs-" + m.group(1)
    for f in ("nxopt", "vcube", "h48", "cube48opt", "cubeopt"):
        if name.startswith(f):
            return f
    return name


def semantics(name):
    o = ours(name)
    if o:
        return "throughput" if o[0] else "latency"
    if name.startswith("c48rs"):
        return "throughput" if name.startswith("c48rsbatch") else "latency"
    return "throughput" if family(name) in ("nxopt", "vcube") else "latency"


def page_col(name, pages):
    o = ours(name)
    if o:
        return o[2]
    return {"1g": "1g", "hugetlb": "1g", "thp": "thp"}.get(pages, "native")


def table_mb(name, tbytes=None):
    o = ours(name)
    if o:
        return OURS_MB[o[1]]
    if name in KNOWN_MB and not tbytes:
        return KNOWN_MB[name]
    return tbytes / 1e6 if tbytes else None


def num(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return 0.0


def read_meta(path):
    meta = {}
    for line in open(path):
        k, _, v = line.rstrip("\n").partition(" ")
        meta.setdefault(k, v)
    return meta


def parse_raw(name, threads, text):
    """-> dict(tt, nodes, lengths {cube index: length}, n, times [per-cube s]) or None."""
    lengths, nodes, times, bytime, bynode = {}, [], [], {}, {}
    o = ours(name)
    if o and o[0] == "stream":
        n = re.search(r"^nodes/cube (\S+)", text, re.M)
        tp = re.search(r"^throughput (\S+)", text, re.M)
        if not (n and tp) or float(tp.group(1)) == 0:
            return None
        return dict(tt=1 / float(tp.group(1)), nodes=float(n.group(1)), lengths={}, n=0, times=[], bytime={}, bynode={})
    if o:
        bytime = {}
        for m in re.finditer(r"^solved (\d+) (\d+) (\d+) (\S+)", text, re.M):
            lengths[int(m.group(1))] = int(m.group(2))
            nodes.append(int(m.group(3)))
            times.append(float(m.group(4)))
            bytime[int(m.group(1))] = float(m.group(4))
            bynode[int(m.group(1))] = int(m.group(3))
        t = re.search(r"^total \d+ cubes wall \S+ wall/cube (\S+)", text, re.M)
        if not t or not nodes:
            return None
        tt = float(t.group(1)) if o[0] == "batch" else sum(times) / len(times)
        return dict(tt=tt, nodes=sum(nodes) / len(nodes), lengths=lengths, n=len(nodes),
                    times=times if o[0] != "batch" else [], bytime=bytime if o[0] != "batch" else {},
                    bynode=bynode)
    if name.startswith("nxopt"):
        for m in re.finditer(r"^Solved (\d+) in (\d+) probes \d+ evals (\d+) time (\S+)", text, re.M):
            lengths[int(m.group(1)) - 1] = int(m.group(2))
            nodes.append(int(m.group(3)))
            times.append(float(m.group(4)))
            bynode[int(m.group(1)) - 1] = int(m.group(3))
        div = threads
    elif name.startswith("vcube"):
        for m in re.finditer(r"^(\d+) (\d+\.\d+) (\d+) (\d+) ", text, re.M):
            lengths[int(m.group(1))] = int(m.group(4))
            nodes.append(int(m.group(3)))
            times.append(float(m.group(2)))
            bynode[int(m.group(1))] = int(m.group(3))
        div = threads
    elif name.startswith("h48"):
        i = -1
        for line in text.splitlines():
            m = re.search(r"Nodes visited: (\d+)", line)
            if m:
                nodes.append(int(m.group(1)))
                bynode[len(nodes) - 1] = int(m.group(1))
            m = re.match(r"^Time: (\S+) s", line)
            if m:
                times.append(float(m.group(1)))
                i += 1
                bytime[i] = float(m.group(1))
            m = re.match(r"^Solution: .*\((\d+)\)$", line)
            if m and i not in lengths:
                lengths[i] = int(m.group(1))
        div = 1
    elif name.startswith("c48rs"):
        # cube48opt-rust: one JSON line per cube ("id" = cube index; "seconds" around the solve only).
        for m in re.finditer(r'^\{"id":"(\d+)".*$', text, re.M):
            line, i = m.group(0), int(m.group(1))
            ln, sec, nd = re.search(r'"length":(\d+)', line), re.search(r'"seconds":([0-9.eE+-]+)', line), re.search(r'"nodes":(\d+)', line)
            if not (sec and nd):
                continue
            if ln and '"cancelled":true' not in line:
                lengths[i] = int(ln.group(1))
            nodes.append(int(nd.group(1)))
            times.append(float(sec.group(1)))
            bytime[i] = float(sec.group(1))
            bynode[i] = int(nd.group(1))
        div = threads if name.startswith("c48rsbatch") else 1
    elif name.startswith("cube"):
        for m in re.finditer(r"Cube(\d+) (finished|unsolved) in (\d+), (\d+)/\d+ node\(s\), tt=([0-9.]+) ms", text):
            if m.group(2) == "finished":
                lengths[int(m.group(1))] = int(m.group(3))
            nodes.append(int(m.group(4)))
            times.append(float(m.group(5)) / 1000)
            bytime[int(m.group(1))] = float(m.group(5)) / 1000
            bynode[int(m.group(1))] = int(m.group(4))
        div = 1
    else:
        return None
    if not nodes or not times:
        return None
    k = min(len(nodes), len(times))
    return dict(tt=sum(times) / len(times) / div, nodes=sum(nodes[:k]) / k, lengths=lengths,
                n=len(times), times=times if div == 1 else [], bytime=bytime if div == 1 else {}, bynode=bynode)


def placement(meta, table_bytes):
    """Measured page placement of the table: (kind, percent) or None."""
    w = meta.get("pages", "").split()
    if len(w) < 5:
        return None
    total, rss, anon, thp, huge = (int(x) for x in w[:5])
    tkb = (table_bytes or 0) / 1024
    if huge > 0:
        return ("1G", 100.0 * huge / tkb if tkb else 100.0)
    if anon > 0 and tkb:
        return ("THP", 100.0 * thp / tkb)
    return None


# ---- load the runs ------------------------------------------------------------------------------
runs = []
for base in [OUT] + EXTRA:
  metadir = os.path.join(base, "meta")
  for f in sorted(os.listdir(metadir)) if os.path.isdir(metadir) else []:
    meta = read_meta(os.path.join(metadir, f))
    run = meta.get("run", "").split()
    if len(run) < 5:
        continue
    name, threads, dataset, ntest, pages = run[0], int(run[1]), run[2], int(run[3]), run[4]
    build = run[5] if len(run) > 5 else "shipped"
    # User excluded backend ablation results from every published comparison.
    if build == "scalar":
        continue
    rep = int(run[6]) if len(run) > 6 else 1
    rawp = os.path.join(base, "raw", f[:-4] + ".log")
    text = open(rawp, errors="replace").read() if os.path.exists(rawp) else ""
    p = parse_raw(name, threads, text)
    offset = 0
    m = re.match(r"^(.*)_from(\d+)$", dataset)
    if m:
        dataset, offset = m.group(1), int(m.group(2))
    if p and family(name) in ("cubeopt", "cube48opt") and p["lengths"] and 0 not in p["lengths"] \
            and p["n"] in p["lengths"]:
        p["lengths"] = {i - 1: l for i, l in p["lengths"].items()}   # "Cube1" is the first cube
        p["bytime"] = {i - 1: t for i, t in p["bytime"].items()}
        p["bynode"] = {i - 1: t for i, t in p["bynode"].items()}
    if p and offset:
        p["lengths"] = {i + offset: l for i, l in p["lengths"].items()}
        p["bytime"] = {i + offset: t for i, t in p["bytime"].items()}
        p["bynode"] = {i + offset: t for i, t in p["bynode"].items()}
    tparts = meta.get("table", "").split()
    tbytes = int(tparts[1]) if len(tparts) > 1 and tparts[1].isdigit() else None
    if ours(name):      # cube-opt's files are whole 1 GiB pages: the table is the data part
        tbytes = int(OURS_MB[ours(name)[1]] * 1e6)
    runs.append(dict(name=name, threads=threads, dataset=dataset, offset=offset, ntest=ntest,
                     pages=page_col(name, pages), build=build, rep=rep, status=meta.get("status"),
                     wall=num(meta.get("wall_s")), ext=num(meta.get("ext_cores")), meta=meta, p=p,
                     tbytes=tbytes, file=f[:-4]))
# The agreed interference check applies to the solver family. Retain the
# parallel validation runs as evidence, but withhold timed nissy cells when
# its check exceeds 2%; only serial reruns then qualify for the report.
SERIAL_D20_FAMILIES = set()
for validation_name, validation_dataset in (("cubeoptrsh8", "distance20_val3"),
                                             ("h48h7", "distance20_val2")):
    validation_runs = [r for r in runs if r["status"] == "0" and r["p"]
                       and r["name"] == validation_name and r["dataset"] == validation_dataset]
    alone = [r for r in validation_runs if r["meta"].get("par", "0").split()[0] == "0"]
    parallel = [r for r in validation_runs if r["meta"].get("par", "0").split()[0] != "0"]
    if alone and parallel and parallel[-1]["p"]["tt"] / alone[-1]["p"]["tt"] > 1.02:
        SERIAL_D20_FAMILIES.add(family(validation_name))

good, rejected = [], []
for r in runs:
    if r["status"] != "0" or not r["p"]:
        continue
    if r["threads"] == 48 and r["ext"] >= 1.5:
        rejected.append((r, "outside load >= 1.5 cores during all-thread measurement"))
        continue
    if (r["threads"] == 1 and r["dataset"].startswith("distance20_random")
            and family(r["name"]) in SERIAL_D20_FAMILIES
            and r["meta"].get("par", "0").split()[0] != "0"):
        rejected.append((r, "parallel interference exceeds 2%; serial rerun required"))
        continue
    # A page column only counts if the table really is on those pages (THP >= 75%, 1G >= 99%).
    pl = placement(r["meta"], r["tbytes"])
    # (THP >= 75%: with ~10 GB free and fragmented, the 7.8 GB tables get 80-85% at best, even right
    # after warming free 2 MiB blocks; their coverage is shown in "pages measured".)
    if r["pages"] == "thp" and not (ours(r["name"]) and pl is None) and (pl is None or pl[0] != "THP" or pl[1] < 75):
        rejected.append((r, "THP %s" % ("%d%%" % pl[1] if pl else "not sampled")))
        continue
    # (cube-opt's 1G rows map their hugetlbfs file by construction; a run of a few milliseconds may
    # have been sampled before solve started, so its sample says nothing.)
    if r["pages"] == "1g" and not ours(r["name"]) and pl is not None and (pl[0] != "1G" or pl[1] < 99):
        rejected.append((r, "%s %d%%" % pl))
        continue
    good.append(r)

# Build choice per family: an alternative build (-march=native, PGO) replaces the shipped one only
# when it is clearly faster: median ratio < 0.99 over its 48T (median of repeats) and 1T (first
# chunk) runs against the shipped build on the same table, pages and threads.
chosen = {}
screen_rows = []
for fam in BUILD_CHOICE:
    best, best_ratio = "shipped", 1.0
    for alt in ALT_BUILDS:
        ratios = []
        keys = sorted({(r["name"], r["pages"], r["threads"], r["ntest"]) for r in good
                       if family(r["name"]) == fam and r["build"] == alt and r["offset"] == 0
                       and r["dataset"] == "random_state"})
        for name, pages, threads, ntest in keys:
            def med(b):
                v = [r["p"]["tt"] for r in good if (r["name"], r["pages"], r["threads"], r["ntest"]) == (name, pages, threads, ntest)
                     and r["build"] == b and r["offset"] == 0 and r["dataset"] == "random_state"]
                return statistics.median(v) if v else None
            a_, b_ = med("shipped"), med(alt)
            if a_ and b_:
                ratios.append(b_ / a_)
                screen_rows.append("| %s %s %dT%s | %.4f | %s %.4f | %+.1f%% |" % (
                    name, pages, threads, " (first chunk)" if threads == 1 else "", a_, alt, b_, 100 * (b_ / a_ - 1)))
        if ratios and statistics.median(ratios) < min(0.99, best_ratio):
            best, best_ratio = alt, statistics.median(ratios)
    chosen[fam] = best

# Hold the screening decision fixed during the follow-up measurements.
selection_path = os.path.join(OUT, "build-selection.json")
if os.path.exists(selection_path):
    frozen = json.load(open(selection_path))["choices"]
    if any(fam not in BUILD_CHOICE or build not in ("shipped",) + ALT_BUILDS
           for fam, build in frozen.items()):
        raise ValueError("invalid frozen build selection")
    chosen.update(frozen)
else:
    screening_path = "/data/agents/f1/phase2-c48rs-a.txt"
    required = []
    for line in open(screening_path):
        words = line.split("#", 1)[0].split()
        if len(words) < 7:
            continue
        name, threads, dataset, count, pages, build, repeat = words[:7]
        required.append(f"{name}_{threads}T_{dataset}_{count}_{pages}"
                        + ("" if build == "shipped" else "_" + build)
                        + ("" if repeat == "1" else "_r" + repeat))
    accepted_ids = {r["file"] for r in good}
    if required and all(run_id in accepted_ids for run_id in required):
        temporary = selection_path + "." + str(os.getpid()) + ".tmp"
        with open(temporary, "w") as output:
            json.dump({"choices": chosen, "basis": "completed initial build screening; fixed for follow-up rows"}, output, indent=2)
            output.write("\n")
        os.replace(temporary, selection_path)


def build_ok(r):
    fam = family(r["name"])
    if fam == "cube-opt":   # (its default rows: the SIMD cube; "scalar" rows are asked for by name)
        return r["build"] == "shipped"
    return r["build"] == chosen.get(fam, r["build"]) if fam in BUILD_CHOICE else True


def cell(name_match, pages, threads, dataset="random_state", build=None):
    """Aggregate a cell: runs of one solver/table, page column, thread count (chosen build, or the
    given one), on one dataset. Per repeat, chunks are merged (cube-weighted); then median and range
    over repeats."""
    rs = [r for r in good if name_match(r["name"]) and r["pages"] == pages and r["threads"] == threads
          and (build_ok(r) if build is None else r["build"] == build) and r["dataset"] == dataset]
    if dataset == "random_state" and threads == 1:
        # Headline timing uses cubes 0–49 for every solver and backend.
        # The 100-cube wiki runs and cubes 50–99 are for wiki_nodes only;
        # mixing them here would compare different cube distributions.
        rs = [r for r in rs if r["offset"] + r["p"]["n"] <= 50]
    if not rs:
        return None
    reps = {}
    for r in rs:
        reps.setdefault(r["rep"], []).append(r)
    vals, nodes, ncubes, times, lengths, ext, plc = [], [], [], [], {}, [], []
    bytime = {}
    for rep, chunks in sorted(reps.items()):
        if all(c["p"]["n"] for c in chunks):
            # Screening, wiki and full runs may overlap (e.g. cubes 0–99
            # plus 50–99). Choose disjoint intervals with maximum coverage;
            # counting both would bias means and inflate the sample size.
            ordered = sorted(chunks, key=lambda c: (c["offset"] + c["p"]["n"], c["offset"], c["file"]))
            ends = [c["offset"] + c["p"]["n"] for c in ordered]
            plans = [(0, [])]
            for i, c in enumerate(ordered):
                previous = bisect_right(ends, c["offset"], 0, i)
                count, selected = plans[previous]
                take = (count + c["p"]["n"], selected + [c])
                skip = plans[-1]
                plans.append(max((skip, take), key=lambda plan: (plan[0], -len(plan[1]))))
            cs = sorted(plans[-1][1], key=lambda c: c["offset"])
        else:
            by_offset = {c["offset"]: c for c in chunks}
            cs = [by_offset[offset] for offset in sorted(by_offset)]
        n = sum(c["p"]["n"] for c in cs) or 1
        if any(c["p"]["n"] == 0 for c in cs):      # stream: no cubes
            vals.append(statistics.mean(c["p"]["tt"] for c in cs))
            nodes.append(statistics.mean(c["p"]["nodes"] for c in cs))
        else:
            vals.append(sum(c["p"]["tt"] * c["p"]["n"] for c in cs) / n)
            nodes.append(sum(c["p"]["nodes"] * c["p"]["n"] for c in cs) / n)
        ncubes.append(sum(c["p"]["n"] for c in cs))
        for c in cs:
            times.extend(c["p"]["times"])
            lengths.update(c["p"]["lengths"])
            for i, t in c["p"].get("bytime", {}).items():
                bytime.setdefault(i, []).append(t)
            ext.append(c["ext"])
            pl = placement(c["meta"], c["tbytes"])
            if pl and not (ours(c["name"]) and c["pages"] == "1g" and pl[0] != "1G"):
                plc.append(pl)
    # Only repeats that cover the full cube set count (e.g. a 25-cube screening run next to the
    # 50-cube row is left out of the cell).
    full = max(ncubes)
    keep = [i for i, n in enumerate(ncubes) if n == full]
    vals = [vals[i] for i in keep]
    nodes = [nodes[i] for i in keep]
    ci = None
    if threads == 1 and len(times) > 2:
        ci = 1.96 * statistics.stdev(times) / len(times) ** 0.5 / statistics.mean(times)
    return dict(ours1g=all(ours(r["name"]) and ours(r["name"])[2] == "1g" for r in rs),
                bytime={i: statistics.median(v) for i, v in bytime.items()},
                med=statistics.median(vals), lo=min(vals), hi=max(vals), reps=len(vals), nodes=statistics.mean(nodes),
                n=max(ncubes), coverage=set(lengths), ci=ci, ext=max(ext) if ext else 0, placement=plc,
                build=rs[0]["build"], tbytes=max((r["tbytes"] or 0) for r in rs) or None)


def paired(theirs, mine):
    """Per-cube ratios theirs / ours on the cubes both solved: geometric mean and the 10-90%
    range of the per-cube ratios (the per-cube spread), or None."""
    common = sorted(set(theirs["bytime"]) & set(mine["bytime"]))
    rs = [theirs["bytime"][i] / mine["bytime"][i] for i in common if mine["bytime"][i] > 0 and theirs["bytime"][i] > 0]
    if len(rs) < 5:
        return None
    import math
    g = math.exp(statistics.mean(math.log(x) for x in rs))
    q = sorted(rs)
    return "%d cubes: geo-mean %.1fx, 10-90%% %.1f-%.1fx" % (len(rs), g, q[len(q) // 10], q[(9 * len(q)) // 10])


def fmt_cell(c, d=4):
    if not c:
        return "-"
    s = "%.*f" % (d, c["med"])
    if c["reps"] > 1:
        s += " [%.*f-%.*f, %d]" % (d, c["lo"], d, c["hi"], c["reps"])
    elif c["ci"] is not None:
        s += " (±%.0f%%)" % (100 * c["ci"])
    return s


def fmt_place(c):
    if c and not c["placement"] and c.get("ours1g"):
        return "1G (hugetlbfs file; run too short to sample)"
    if not c or not c["placement"]:
        return "?"
    kinds = {k for k, _ in c["placement"]}
    low = min(p for _, p in c["placement"])
    return "%s %d%%" % ("/".join(sorted(kinds)), round(min(low, 100)))


# ---- cube_solver_test wiki metrics -------------------------------------------------------------
WIKI_HOME = os.path.join(os.path.dirname(__file__), "..", "reference", "wiki", "Home.md")
WIKI_SETS = [("15f", "random_move_15f"), ("16f", "random_move_16f"), ("17f", "random_move_17f"),
             ("inf", "random_state")]
WIKI_REF = ["nxopt31", "nxopt22", "vcube112", "vcube208", "vcube308", "h48h7", "h48h8", "h48h9",
            "cubeopt30", "cube48opt7", "cube48opt8", "cube48opt9"]


def wiki_tables(path):
    """The wiki's tables: {section title: {solver: [cells]}}."""
    out, title = {}, None
    try:
        lines = open(path).read().splitlines()
    except OSError:
        return out
    for line in lines:
        if line.startswith("#"):
            title = line.strip("# ").strip()
        elif line.startswith("|") and title and not set(line) <= set("|-: "):
            cells = [c.strip() for c in line.strip("|").split("|")]
            if cells[0] in ("Solver",):
                out.setdefault(title, {})["__header__"] = cells
            else:
                out.setdefault(title, {})[cells[0]] = cells[1:]
    return out


def wiki_solvers():
    rows = []
    for k in (7, 8, 9, 10, 11):
        m = (lambda n, k=k: bool(ours(n)) and ours(n)[0] == "" and ours(n)[1] == k)
        rows.append(("cube-opt h%d" % k, m, OURS_MB[k], None))
    for c in ["nxopt31", "nxopt22", "vcube112", "vcube208", "vcube308", "h48h7", "h48h8", "h48h9", "h48h10",
              "h48h11", "cubeopt30", "cube48opt10", "c48rsopt7", "c48rsopt8", "c48rsopt10"]:
        tb = next((r["tbytes"] for r in runs if r["name"] == c and r["tbytes"]), None)
        rows.append((c, (lambda n, c=c: n == c), table_mb(c, tb), None))
    return rows


def wiki_nodes(match, dataset, build=None):
    """Mean nodes per cube over the first 100 cubes of a dataset, from 1T runs (any page type: node
    counts do not depend on it; the chosen build); -> (mean, cubes) or (None, 0)."""
    per = {}
    for r in good:
        if r["threads"] == 1 and match(r["name"]) and r["dataset"] == dataset and \
                (build_ok(r) if build is None else r["build"] == build):
            for i, v in r["p"].get("bynode", {}).items():
                if i < 100:
                    per.setdefault(i, []).append(v)
    if not per:
        return None, 0
    return statistics.mean(statistics.mean(v) for v in per.values()), len(per)


def wiki_section():
    o = []
    ref = wiki_tables(WIKI_HOME)
    sv = wiki_solvers()
    nodes = {(lab, key): wiki_nodes(m, ds) for lab, m, mb, b in sv for key, ds in WIKI_SETS}
    o.append("\n## cube_solver_test wiki metrics (this machine)\n")
    o.append("The wiki's three tables, measured here: first 100 cubes of random_move_15f/16f/17f and of "
             "random_state ('inf'; fewer where marked), nodes from 1-thread runs (`solve -t 1 -c 1` for "
             "cube-opt). **Node definitions differ between solvers** (what one node is: a move generated, an "
             "evaluation, a lookup...), so node counts and space-node products compare search-plus-table "
             "efficiency only roughly; nissy's counts also vary run to run (competitor-bugs.md #7), so its "
             "values are means over the runs of each cube.\n")
    o.append("### Space-node product (table bytes x nodes visited, in 1e15 bytes; less is better)\n")
    o.append("| solver | table MB | 15f | 16f | 17f | inf |")
    o.append("|---|---:|---:|---:|---:|---:|")
    for lab, m, mb, b in sv:
        cells = []
        for key, ds in WIKI_SETS:
            v, n = nodes[(lab, key)]
            cells.append("-" if v is None or not mb else ("%.3f" % (mb * 1e6 * v / 1e15)) + ("" if n >= 100 else " (%d)" % n))
        if any(c != "-" for c in cells):
            o.append("| %s | %s | %s |" % (lab, "%.2f" % mb if mb else "-", " | ".join(cells)))
    o.append("\n### Number of nodes visited (millions per cube; less is better)\n")
    o.append("| solver | 15-move | 16-move | 17-move | inf-move |")
    o.append("|---|---:|---:|---:|---:|")
    for lab, m, mb, b in sv:
        cells = []
        for key, ds in WIKI_SETS:
            v, n = nodes[(lab, key)]
            cells.append("-" if v is None else ("%.3f" % (v / 1e6)) + ("" if n >= 100 else " (%d)" % n))
        if any(c != "-" for c in cells):
            o.append("| %s | %s |" % (lab, " | ".join(cells)))
    o.append("\n### Average time per node (random_state; harness tt, ns/node = sec/solv / nodes per cube of the same run)\n")
    o.append("1 GiB pages, chosen builds; THP where no 1 GiB row exists (marked). 1T timings: the same first 50 cubes, 48T: 200 cubes, "
             "median of 3 repeats. 48T sec/solv is a latency for cube-opt, nissy and cubeopt, a throughput "
             "(one cube per thread) for nxopt and vcube, as in the wiki.\n")
    for t, title in ((1, "Single-thread"), (48, "All-thread (48)")):
        o.append("%s results\n" % title)
        o.append("| solver | ns/node | sec/solv | GB*sec |")
        o.append("|---|---:|---:|---:|")
        for lab, m, mb, b in wiki_solvers():
            c, mark = cell(m, "1g", t, build=b), ""
            if not c:
                c, mark = cell(m, "thp", t, build=b), " (THP)"
            if not c or not c["nodes"]:
                continue
            o.append("| %s%s | %.2f | %.4f | %.3f |" % (lab, mark, c["med"] / c["nodes"] * 1e9, c["med"],
                                                     mb / 1000 * c["med"] if mb else 0))
        o.append("")
    # The wiki's own values (its machine) and the node cross-check (nodes do not depend on the machine).
    o.append("### Reference: the wiki's published values (Intel Core i7-10750H, 6 cores / 12 threads, THP always; NOT this machine)\n")
    sn, nv = ref.get("Space-node product", {}), ref.get("Number of nodes visited", {})
    o.append("| solver | table MB | space-node 15f / 16f / 17f / inf | nodes (M) 15 / 16 / 17 / inf |")
    o.append("|---|---:|---|---|")
    for name in WIKI_REF:
        if name in sn or name in nv:
            a = sn.get(name, ["-"] * 5)
            b = nv.get(name, ["-"] * 4)
            o.append("| %s | %s | %s | %s |" % (name, a[0], " / ".join(x or "-" for x in a[1:5]),
                                              " / ".join(x or "-" for x in b[:4])))
    o.append("\nTime per node on the wiki's machine (ns/node, sec/solv, GB*sec):\n")
    o.append("| solver | 1 thread | 12 threads |")
    o.append("|---|---|---|")
    single, multi = {}, {}
    for title, d in ref.items():
        pass
    # The two "results" tables share a section title; read them in order from the file.
    try:
        txt = open(WIKI_HOME).read()
        parts = re.split(r"\n(?:Single-thread|12-thread) results\n", txt)
        for idx, dest in ((1, single), (2, multi)):
            for line in parts[idx].splitlines():
                if line.startswith("|") and not set(line) <= set("|-: "):
                    cells = [c.strip() for c in line.strip("|").split("|")]
                    if cells[0] != "Solver":
                        dest[cells[0]] = cells[1:]
    except (OSError, IndexError):
        pass
    for name in WIKI_REF:
        if name in single or name in multi:
            o.append("| %s | %s | %s |" % (name, " / ".join(single.get(name, ["-"])), " / ".join(multi.get(name, ["-"]))))
    o.append("\n### Node counts vs the wiki (same datasets, first 100 cubes; nodes do not depend on the machine)\n")
    o.append("| solver | 15-move here / wiki | 16-move | 17-move | inf-move |")
    o.append("|---|---|---|---|---|")
    for lab, m, mb, b in sv:
        if lab not in nv:
            continue
        cells = []
        for j, (key, ds) in enumerate(WIKI_SETS):
            v, n = nodes[(lab, key)]
            w = nv[lab][j] if j < len(nv[lab]) else ""
            cells.append("%s / %s" % ("-" if v is None else "%.3f%s" % (v / 1e6, "" if n >= 100 else " (%d)" % n), w or "-"))
        o.append("| %s | %s |" % (lab, " | ".join(cells)))
    return o


# ---- the report ---------------------------------------------------------------------------------
out = []
commit_file = os.path.join(OUT, "cube-opt-commit")
commit = open(commit_file).read().strip() if os.path.exists(commit_file) else "(no cube-opt rows yet)"
out.append("# cube-opt vs nxopt, vcube, nissy-core H48, cubeopt30, cube48opt10 and cube48opt-rust\n")
out.append("cube-opt: %s. Machine: AMD EPYC 9V45, 48 cores (Zen 5), 94 GiB; THP enabled=madvise, "
           "defrag=madvise. Harness: cube_solver_test runtest.sh (extended), driven by final-compare.sh.\n" % commit)
out.append("Cells: seconds per cube (sec/solv); at 48T the median over repeats [min-max, repeats]; at 1T "
           "(one run of 50 cubes, or two 25-cube chunks) ± the 95% interval of the mean over the cubes (this is "
           "the cube-to-cube spread; every solver gets the same cubes, see 'per cube' in the headline). "
           "1G = 1 GiB hugetlb pages, THP = 2 MiB transparent huge pages: every solver is measured on both, "
           "and a speedup is only stated between equal page types, equal semantics and the same size group. "
           "'size' is the table size relative to the cube-opt table it is compared with. GB·s = table GB x "
           "sec/solv at 48T. nodes/cube is each solver's own count (not comparable across solvers).\n")


if os.path.exists(os.path.join(OUT, "no-panic-inventory.md")):
    out.append("No-panic integration: in first-party sources (dependencies excluded), the same source scanner found 127 forbidden explicit calls at "
               "`1f5eb70` and zero at `01e7683` (including shipped backends, binaries and build scripts; "
               "tests and compile-time contexts excluded). The full backend checks enforce the policy. "
               "[Source inventory and remaining implicit-check diagnostics](no-panic-inventory.md).\n")


validation_path = os.path.join(OUT, "final-build-validation.md")
if os.path.exists(validation_path):
    out.append("\n" + open(validation_path).read())

miri_path = os.path.join(OUT, "miri-validation.md")
if os.path.exists(miri_path):
    out.append("\n" + open(miri_path).read())


def group_ours(ks, mb):
    """The cube-opt k of the group nearest in size to a table of mb MB."""
    return min(ks, key=lambda k: abs(OURS_MB[k] - mb)) if ks else None


headline = []
for sem in ("latency", "throughput"):
    out.append("\n## %s\n" % ("Latency: one cube at a time on all threads (cube-opt solve -c 1 vs nissy H48, "
                                "cubeopt30, cube48opt10)" if sem == "latency" else
                                "Throughput: many cubes in flight (cube-opt solve -c 0 / bench stream vs nxopt "
                                "-t, vcube -w, which solve one cube per thread)"))
    for gtitle, comps, ks in GROUPS:
        rows = []
        names = []
        for k in ks:
            if sem == "latency":
                m = lambda n, k=k: ours(n) and ours(n)[0] == "" and ours(n)[1] == k
                names.append(("cube-opt h%d" % k, m, OURS_MB[k], k, None))
            else:
                m = lambda n, k=k: ours(n) and ours(n)[0] == "batch" and ours(n)[1] == k
                names.append(("cube-opt h%d batch" % k, m, OURS_MB[k], k, None))
                names.append(("cube-opt h%d stream" % k, lambda n, k=k: ours(n) and ours(n)[0] == "stream" and ours(n)[1] == k,
                              OURS_MB[k], k, None))
        for c in comps:
            if semantics(c) == sem:
                names.append((c, lambda n, c=c: n == c, None, None, None))
        for label, match, mb, k, bld in names:
            cells = {(pg, t): cell(match, pg, t, build=bld) for pg in ("1g", "thp") for t in (1, 48)}
            if not any(cells.values()):
                continue
            if mb is None:
                tb = next((c["tbytes"] for c in cells.values() if c and c["tbytes"]), None)
                mb = table_mb(label, tb)
            ref_k = k if k is not None else group_ours(ks, mb)
            size = mb / OURS_MB[ref_k] if ref_k is not None and mb else None
            gbs = {pg: (mb / 1000 * cells[(pg, 48)]["med"]) if cells[(pg, 48)] and mb else None for pg in ("1g", "thp")}
            fam = family(label.split()[0]) if not label.startswith("cube-opt") else "cube-opt"
            build = chosen.get(fam, "shipped") if fam in BUILD_CHOICE else (
                "prebuilt (no PGO possible)" if fam in ("cubeopt", "cube48opt") else
                "SIMD cube")
            if fam == "nxopt":
                build = "%s (-march=native)" % build
            nodes = next((c["nodes"] for key in [("1g", 48), ("thp", 48), ("1g", 1), ("thp", 1)]
                          for c in [cells[key]] if c), None)
            # cube-opt's advantage, same pages, same semantics, nearest cube-opt table, 48T and 1T.
            adv = []
            if not label.startswith("cube-opt") and ref_k is not None:
                mode = "" if sem == "latency" else "batch"
                for pg in ("1g", "thp"):
                    for t in (48, 1):
                        theirs = cells[(pg, t)]
                        mine = cell(lambda n: ours(n) and ours(n)[0] == mode and ours(n)[1] == ref_k, pg, t)
                        if theirs and mine:
                            adv.append("%s %dT %.1fx" % (pg.upper(), t, theirs["med"] / mine["med"]))
                            headline.append((sem, gtitle, label, pg, t, theirs["med"] / mine["med"], ref_k, size,
                                             paired(theirs, mine)))
            rows.append("| %s | %s | %s | %s | %s | %s | %s | %s / %s | %s | %s | %s |" % (
                label, build, "%.2f (%.2fx)" % (mb / 1000, size) if size else ("%.2f" % (mb / 1000) if mb else "-"),
                fmt_cell(cells[("1g", 1)], 3), fmt_cell(cells[("thp", 1)], 3),
                fmt_cell(cells[("1g", 48)]), fmt_cell(cells[("thp", 48)]),
                "%.3f" % gbs["1g"] if gbs["1g"] else "-", "%.3f" % gbs["thp"] if gbs["thp"] else "-",
                "%.2fM" % (nodes / 1e6) if nodes else "-",
                " / ".join(fmt_place(cells[(pg, t)]) for pg in ("1g", "thp") for t in (48,) if cells[(pg, t)]) or "-",
                ", ".join(adv) or "-"))
        if rows:
            out.append("\n### %s (%s)\n" % (gtitle, sem))
            out.append("| solver | build | table GB (size) | 1T 1G | 1T THP | 48T 1G | 48T THP | GB·s 48T 1G / THP "
                       "| nodes/cube | pages measured (1G / THP rows) | cube-opt faster by (same pages) |")
            out.append("|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|")
            out.extend(rows)

if headline:
    out.append("\n## Headline (same page type, same semantics, same size group)\n")
    out.append("cube-opt uses the default SIMD build, compared against the same competitor rows.\n")
    out.append("| semantics | group | competitor | size vs cube-opt | pages | threads | cube-opt faster by (sec/solv) "
               "| per cube |")
    out.append("|---|---|---|---:|---|---:|---:|---|")
    for sem, g, lab, pg, t, ratio, k, size, pc in headline:
        note = " (prebuilt binary: not profile-tunable)" if family(lab) in ("cubeopt", "cube48opt") else ""
        out.append("| %s | %s | %s%s vs cube-opt h%d | %.2fx | %s | %d | %.1fx | %s |" % (
            sem, g, lab, note, k, size or 0, pg.upper(), t, ratio, pc or "-"))

out.extend(wiki_section())


# ---- Phase 3: cube20.org distance-20 positions ------------------------------------------------
D20_URL = "https://cube20.org/distance20s/"
D20_48, D20_1 = "distance20_random100", "distance20_random10"


def d20_cell(name_match, threads, dataset, build=None):
    """Only publish a distance-20 cell once every requested cube is present."""
    result = cell(name_match, "1g", threads, dataset=dataset, build=build)
    expected = 100 if dataset == D20_48 else 10
    if result and result["n"] == expected and result["coverage"] == set(range(expected)):
        return result
    return None


def d20_section():
    o = []
    have = [r for r in good if r["dataset"] in (D20_48, D20_1)]
    if not have:
        return o
    o.append("\n## Distance-20 positions (cube20.org)\n")
    o.append("Positions from the known list of distance-20 positions at %s (list of 25 July 2025; random "
             "samples of 10 and 100, /data/agents/d20, converted to the harness format). Every position is a proven "
             "distance-20 position in the half-turn metric: a solver must exhaust depth 19 before it finds a "
             "20-move solution, so these are the hardest cases. 1 GiB pages only; chosen builds. 48T: the 100-cube "
             "sample, one run per solver (chunked into holds of <= 15 min where needed: the per-cube times and, for "
             "nxopt/vcube, the harness formula sum / cubes / 48 do not depend on the chunking); 1T: the 10-cube "
             "sample for every solver (and the 100-cube sample for %s where cheap). Semantics as above: latency "
             "rows (all threads on one cube) vs throughput rows (one cube per thread).\n" % (D20_URL, LABEL))
    # 1T method: parallel blocks, and the two-way interference check.
    val = []
    for nm, ds in (("cubeoptrsh8", "distance20_val3"), ("h48h7", "distance20_val2")):
        alone = [r for r in good if r["name"] == nm and r["dataset"] == ds and r["meta"].get("par", "0 -").split()[0] == "0"]
        par = [r for r in good if r["name"] == nm and r["dataset"] == ds and r["meta"].get("par", "0 -").split()[0] != "0"]
        if alone and par:
            a_, p_ = alone[-1]["p"], par[-1]["p"]
            per = ["#%d %.2f -> %.2f s" % (i, a_["bytime"][i], p_["bytime"][i]) for i in sorted(a_["bytime"]) if i in p_["bytime"]]
            val.append("%s %+.1f%% (%s; nodes %s)" % (LABEL + " h8" if nm.startswith("cubeopt") else "nissy h48h7",
                       100 * (p_["tt"] / a_["tt"] - 1), ", ".join(per),
                       "identical" if a_["bynode"] == p_["bynode"] else "differ (nissy's counts vary run to run, #7)"))
    o.append("1T method (agreed with the coordinator): to fit ~10 h of 1T work, the 1T rows ran in parallel blocks, "
             "each inside one exclusive machine hold: up to 5 of these 1T rows at once, one per L3 domain (CPUs "
             "8-15, 16-23, 24-31, 32-39, 40-47; CPUs 0-7 left to the harness), every row next to 4 busy neighbours "
             "(idle slots ran an unreported filler, %s h8 on random states). Interference check, same cubes, alone "
             "on an idle machine vs inside a full block: %s. (Threshold: > 2%% would have meant serial 1T rows for "
             "that solver. Failed families: %s; their parallel timings are excluded and replaced by serial "
             "reruns.)\n" % (LABEL, "; ".join(val) if val else "pending",
                              ", ".join(sorted(SERIAL_D20_FAMILIES)) or "none"))
    o.append("A timing cell is shown only when all 10 or 100 requested cubes are present. "
             "Probes and incomplete chunks are withheld from timings and speedup claims.\n")
    # Length check: every solver must return 20 on every cube.
    bad, checked = [], 0
    for r in have:
        for i, l in sorted(r["p"]["lengths"].items()):
            checked += 1
            if l != 20:
                bad.append("%s cube #%d: length %d (%s)" % (r["file"], i, l, "IMPOSSIBLE: shorter than the proven distance"
                                                            if l < 20 else "NOT OPTIMAL: longer than 20"))
        if r["p"]["n"] and len(r["p"]["lengths"]) < r["p"]["n"]:
            bad.append("%s: %d of %d cubes without a solution" % (r["file"], r["p"]["n"] - len(r["p"]["lengths"]), r["p"]["n"]))
    if bad:
        o.append("**LENGTH CHECK FAILED** (%d solutions checked):\n" % checked)
        o.extend("- **%s**" % b for b in bad)
        o.append("")
    else:
        o.append("Length check: all %d solutions returned have length 20.\n" % checked)
    h48_11 = [r for r in have if r["name"] == "h48h11"]
    if h48_11:
        over = [(r["file"], i, l) for r in h48_11 for i, l in r["p"]["lengths"].items() if l > 20]
        o.append("nissy h48h11 (its generator is slightly inadmissible, competitor-bugs.md #1): %s.\n" % (
            "returned > 20 moves on " + ", ".join("%s #%d (%d)" % x for x in over) if over else
            "20 moves on every cube here, so the inadmissible entries did not cost optimality on these positions"))
    for gtitle, comps, ks in GROUPS:
        rows = []
        for sem in ("latency", "throughput"):
            mode = "" if sem == "latency" else "batch"
            names = []
            for k in ks:
                m = lambda n, k=k, mode=mode: ours(n) and ours(n)[0] == mode and ours(n)[1] == k
                names.append(("%s h%d%s" % (LABEL, k, " batch" if mode else ""), m, k, None))
            for c in comps:
                if semantics(c) == sem:
                    names.append((c, lambda n, c=c: n == c, None, None))
            for label, match, k, bld in names:
                c48 = d20_cell(match, 48, D20_48, build=bld)
                c1 = d20_cell(match, 1, D20_1, build=bld)
                c1h = d20_cell(match, 1, D20_48, build=bld) if k is not None else None
                if not (c48 or c1 or c1h):
                    continue
                tb = next((x["tbytes"] for x in (c48, c1, c1h) if x and x["tbytes"]), None)
                mb = OURS_MB[k] if k is not None else table_mb(label, tb)
                ref_k = k if k is not None else group_ours(ks, mb) if mb else None
                adv = []
                if k is None and ref_k is not None:
                    mm = lambda n: ours(n) and ours(n)[0] == mode and ours(n)[1] == ref_k
                    for t, theirs, ds in ((48, c48, D20_48), (1, c1, D20_1)):
                        mine = d20_cell(mm, t, ds)
                        if theirs and mine:
                            adv.append("%dT %.1fx" % (t, theirs["med"] / mine["med"]))
                nodes = next((x["nodes"] for x in (c48, c1, c1h) if x), None)
                rows.append("| %s | %s | %s | %s | %s | %s | %s | %s |" % (
                    label, sem, "%.2f" % (mb / 1000) if mb else "-", fmt_cell(c48, 3), fmt_cell(c1, 2),
                    fmt_cell(c1h, 2) if c1h else "-", "%.1fM" % (nodes / 1e6) if nodes else "-",
                    (", ".join(adv) + " (vs %s h%d)" % (LABEL, ref_k)) if adv else "-"))
        if rows:
            o.append("\n### %s (distance 20)\n" % gtitle)
            o.append("| solver | semantics | table GB | 48T 1G (100 cubes) | 1T 1G (10 cubes) | 1T 1G (100 cubes) "
                     "| nodes/cube | %s faster by (same semantics) |" % LABEL)
            o.append("|---|---|---:|---:|---:|---:|---:|---|")
            o.extend(rows)
    return o


out.extend(d20_section())

if screen_rows:
    out.append("\n## Build screening (shipped flags vs -O3 -march=native and vs PGO)\n")
    selection_label = "Frozen selection" if os.path.exists(selection_path) else "Provisional selection (screening in progress)"
    out.append(selection_label + ": " + ", ".join("%s %s" % (f, chosen[f]) for f in BUILD_CHOICE) +
               " (an alternative build replaces the shipped one only when its runs are > 1% faster). nxopt's "
               "makefile already uses -O3 -march=native. cubeopt30/cube48opt10 are prebuilt closed-source "
               "binaries: they cannot be rebuilt or profile-tuned.\n")
    out.append("| run | shipped | alternative | vs shipped |")
    out.append("|---|---:|---:|---:|")
    out.extend(screen_rows)

# Optimality cross-check.
ref, comp = {}, {}
for r in good:
    tgt = ref if ours(r["name"]) else comp
    for i, l in r["p"]["lengths"].items():
        tgt.setdefault((r["dataset"], i), set()).add(l)
out.append("\n## Solution lengths (optimality cross-check)\n")
split = sorted(k for k, v in comp.items() if len(v) > 1)
out.append("Competitors among themselves: %d cubes, %d with different lengths%s. cube-opt: %d cubes%s.\n" % (
    len(comp), len(split), (": " + ", ".join("%s #%d %s" % (d, i, sorted(comp[(d, i)])) for d, i in split[:10])) if split else "",
    len(ref), "" if not [k for k, v in ref.items() if len(v) > 1] else ", INCONSISTENT"))
out.append("| run | cubes vs cube-opt | equal | different |")
out.append("|---|---:|---:|---:|")
for r in good:
    if ours(r["name"]):
        continue
    same = diff = 0
    bad = []
    for i, l in r["p"]["lengths"].items():
        v = ref.get((r["dataset"], i))
        if not v:
            continue
        if l in v:
            same += 1
        else:
            diff += 1
            bad.append("#%d: %d vs %s" % (i, l, "/".join(map(str, sorted(v)))))
    if same + diff:
        out.append("| %s | %d | %d | %d%s |" % (r["file"], same + diff, same, diff, (" (" + ", ".join(bad[:5]) + ")") if bad else ""))

FAIRNESS = """
## Harness correction (2026-11-05)

Every 48T row measured before 06:45 on 2026-11-05 used a shell page sampler that forked pgrep/awk/cat
every 0.1 s (later 2 s) at the solver's own priority. With all 48 cores busy, each fork preempted a
worker thread. It made cube-opt's 1 GiB latency rows 11-27% slower at 48T: those runs map their table
instantly, so the whole solve fell in the dense sampling window, while the THP rows spent that window
copying the table (found by hp1; sys time 17-31 s against 6 s). This produced the "1 GiB slower than
THP" result in the provisional rows; measured cleanly, 1 GiB is 3-6% faster. Those runs are kept in
old-sampler/ and are not reported. All 48T rows here were re-measured with a fork-free sampler
(tools/src/pagesampler.py, SCHED_IDLE), and the per-run watcher shell also drops to SCHED_IDLE while
the run is going. 1T rows were kept (47 idle cores: the forks rarely landed on the solver); spot
check, same cubes, old sampler (repeat 1) vs new (repeat 2): {spotcheck}.

## Fairness

Equalized:
1. **Page types.** Every solver runs on both 1 GiB hugetlb pages and 2 MiB THP for the small and
   medium tables (2.6-7.8 GB); the large tables (cube-opt h9..h11, vcube308, cube48opt10, h48h9..h11)
   are measured on 1 GiB pages only (user decision: a 58 GiB THP copy does not fit next to the 1 GiB
   pool). The measured page placement of each row is in the tables ("pages measured", % of the table).
   - cube-opt: 1G = its table file on hugetlbfs; THP = `CUBEOPT_LOAD=copy` from an exact copy on /data.
   - vcube: 1G = its own MAP_HUGETLB 1 GiB mapping (free pool pages given); THP = hugeshim madvise on its
     fallback mmap (shipped vcube gets 4K pages when the pool is full).
   - nxopt: 1G = hugeshim arena (its 9,930 row mallocs carved from one 1 GiB-page arena); THP = glibc
     malloc THP + 32 MiB mmap threshold (shipped: 4K).
   - nissy H48: 1G = hugeshim arena (the table bytearray); THP = glibc malloc THP (shipped: 4K).
   - cubeopt30 / cube48opt10: THP = as shipped (they madvise themselves); 1G = hugeshim arena.
2. **Builds.** vcube and nissy also built with -O3 -march=native; the faster build at the screening
   is used (see "Build screening"). nxopt already uses -march=native. cube-opt: -C target-cpu=native.
   **PGO:** since cube-opt may ship a profile-guided build, nxopt, vcube and nissy got one too, with
   their shipped flags otherwise (pgo/pgo-build.sh): gcc `-fprofile-generate/-fprofile-use
   -fprofile-correction` for nxopt and nissy's C core, clang `-fprofile-instr-generate` +
   `llvm-profdata merge` + `-fprofile-instr-use` for vcube, all with `-fprofile-update=atomic`.
   Training (pgo/pgo-train.sh, pgo-train2.sh), disjoint from the evaluated cubes (random_state.txt
   lines 301-400), the same tables as the evaluation, multithreaded: `nxopt31|22-pgogen -t 8` on 24
   cubes each, `vc-optimal-pgogen -c 112|208 -w 2` on 6 cubes each (profiles merged), nissy
   `cmd.py h48h7 8` on 10 cubes and `cmd.py h48h7 1` on 2. (Fewer threads than the evaluation: the
   instrumented binaries share their counters between threads, and at 48 threads with atomic
   counters nxopt31 did not finish one cube in 10 min; vcube was ~15x slower even at 2 threads.)
   At profile-generation time h48h8's table was not available, so nissy's profile uses h48h7 only. The PGO build is used where it
   is faster (Build screening). **cubeopt30 and cube48opt10 are prebuilt closed-source binaries: they cannot
   be profile-tuned or rebuilt with other flags**; their rows are marked so in the headline. If
   cube-opt's rows use a PGO build, they are shown with and without it.
   nissy THREADS=48 (its build.sh default would follow the build's CPU affinity).
3. **Timing.** Every sec/solv excludes table load and generation (each solver times the solve itself;
   cube-opt's latency rows use the mean of its per-cube times, as h48 and cubeopt do).
4. **Semantics.** Latency rows (all threads on one cube) and throughput rows (many cubes in flight) are
   in separate tables, and speedups are only computed within one.
5. **Noise.** The same cubes for every solver (random_state: the first 200 at 48T, the first 50 at 1T,
   1T runs over 15 min split into two chunks of 25); solvers interleaved in the run order; >= 3 repeats
   per 48T cell (median and range); one exclusive machine hold per run behind a quiet gate; the outside
   load during each run is in "Runs".
6. **Table size.** Size groups; each row shows its size relative to the cube-opt table it is compared
   with, and GB·s.

Remaining asymmetries:
- cubeopt30 and cube48opt10 are prebuilt binaries (GCC 9.4, "use sse, bmi2"); they cannot be rebuilt.
- nissy H48 runs through its Python module (as the harness does); the time is taken around each
  `nissy.solve` call, so the Python call overhead (microseconds) is included. nissy's C shell
  (`./run`) loads the table per invocation, so it is not usable for a batch.
- Different table families: cube-opt and nissy use H48 tables (cube-opt's hN keep different EO bits than
  nissy's), cube48opt its own H48 variant, nxopt and vcube their coset tables; same-size groups are
  approximate (see "size").
- Thread semantics: nxopt and vcube cannot put several threads on one cube, so they have no latency
  rows at 48T; their 48T throughput uses the harness formula (sum of per-cube times / 48), which
  leaves out the idle tail, while cube-opt's batch rows are wall / cube including its tail.
- nissy's node counts vary run to run (uninitialized task rank, competitor-bugs.md #7); its h11 table is
  slightly inadmissible (#1).
- cube48opt-rust (github.com/2017YANR02/cuberoot.me solver/experiments/cube48opt-rust at 1effb34, a Rust
  port of cube48opt1-9 plus its own Opt10; audited and set up by xcr, /data/agents/xcr): its latency mode
  is capped at 18 search threads (`MAX_SEARCH_THREADS`), so its 48T latency rows run on 18 threads (its
  cap); its throughput rows use 48 (one cube per thread). It allocates its table with malloc (4 KiB pages
  as shipped): THP = hugeshim thp mode, 1G = hugeshim arena, as nxopt and nissy. Opt7/Opt8 tables were
  made by its own `init` (SHA-256 = upstream's published hashes). **Opt10 reads cube48opt10's raw file
  through a loader-only shim** (xcr): with cube48opt10's unread block minima (15) written as 10 the bytes
  equal its official Opt10 payload (SHA-256 2ce3d57d..., re-verified on every load); 959 of 962 functions
  (every search function) are byte-identical to the official build. Builds screened: shipped, native,
  PGO, and its optional x86 table prefetch (pf / pfnative, off in its official sets).
"""


MEMORY_STEPS = """
## Tables

| table | size | status | generation time |
|---|---:|---|---|
| nxopt31 | 2603 MB | made | 55 s (8 cores) |
| vcube112 | 2603 MB | made | ~75 s (8 cores; 71 s search + write) |
| h48h7 | 3794 MB | made | 17 min of compute on 8 cores (8144 s CPU) + 3.5 min of nissy's progress-log sleep |
| nxopt22 | 7809 MB | made | 77 s at 48 threads (24 s search, 53 s writing) |
| vcube208 | 7809 MB | made | 87 s at 48 threads (23 s search + write) |
| cubeopt30 | 2602 MB | made | 243 s at 48 threads, including solving one cube |
| h48h8 | 7586 MB | made (/data2) | 1642 s at 48 threads (72,000 s CPU, 7.5 GB peak): ~10x h48h7's work (base 10 vs 9); h48h5 and h48h7 both take ~17-20 min on 8 cores |
| vcube308 | 22777 MB | made (/data2) | 316 s at 48 threads (7852 s CPU; vcube's own MAP_HUGETLB, 22 pool pages) |
| cube48opt10 | 31131 MB | made (/data2/competitors/cube48opt10) | 1587 s at 48 threads (63,285 s CPU), including solving one cube; hugeshim 1 GiB arena |
| h48h9 | 15169 MB | made (/data2) | 1784 s at 48 threads (75,000 s CPU, 14.9 GB peak) |
| h48h10 | 30336 MB | made (/data2) | 1720 s at 48 threads (68,528 s CPU; h48gen with the hugeshim 1 GiB arena, 29 pool pages) |
| h48h11 | 60671 MB | made (/data2) | 1332 s at 48 threads (39,876 s CPU; h48gen with the hugeshim 1 GiB arena, 57 pool pages); nissy's h11 generator is slightly inadmissible (x-h48: 11,546 entries one level high), so watch the length cross-check |
| cube-opt h9, h10 | 14.5 / 29.0 GiB | made: /data2/tables/h9, h10 (`gen N auto --parts 4`), `gen compare` against /mnt/huge/h11: same | 250 s / 382 s at 48 threads (4.1 / 7.9 GB peak RAM) |

## Memory and hugepage steps

Table files: the competitors' small tables are on /data/competitors, everything large on /data2
(/data2/competitors/{vcube,cube48opt,h48}, cube-opt's h9/h10 in /data2/tables); the harness's table
folders point there. All tables stay on disk, so nothing is deleted or regenerated between runs.
Page columns need: 1G rows = free 1 GiB pool pages for the duration of a run (one run at a time);
THP rows = ordinary RAM (~18 GiB is free; cube-opt's THP copy of h11 needs 58 GiB).

| table | 1G row needs (free pool pages) | THP row needs (ordinary RAM) |
|---|---:|---:|
| nxopt31, vcube112, cubeopt30 | 3 | 2.6 GB |
| h48h7 | 4 | 3.8 GB |
| nxopt22, vcube208, h48h8 | 8 | 7.8 GB |
| cube-opt h9 / h48h9 | 15 / 15 | 15.6 / 15.2 GB |
| vcube308 | 22 (vcube maps them itself) | 22.8 GB |
| cube-opt h10, cube48opt10, h48h10 | 29, 29 (+8 while generating), 29 | 31.1, 31.1 (+7.8), 30.3 GB |
| cube-opt h11, h48h11 | 58 (the hugetlbfs file), 57 | 62.3, 60.7 GB |

Order:
1. Phase 1 (done for the competitors' 1G rows with the 12 pages freed from /mnt/huge/h7+h8): small and
   medium tables, both columns; then /mnt/huge/h7 and h8 restored from /data/tables/h7.mnt, h8.mnt
   (`hugecp`), and cube-opt's h7/h8 rows (1G from /mnt/huge, THP from the .mnt copies).
2. cube-opt h9/h10 into /data2/tables (done: `gen 9|10 auto /data2/tables/hN --parts 4`, both "same" as
   /mnt/huge/h11 under `gen compare`). Also `gen compare /data/tables/h11
   /mnt/huge/h11` (via the sidecar tables-thp/h11.meta), since cube-opt's THP h11 row copies that file.
3. Phase 2: remove /mnt/huge/h11 (58 free pages), `final-compare.sh --prep` for vcube308, cube48opt10,
   h48h8..h11 (written to /data2), then one large table at a time on 1 GiB pages (hugecp cube-opt's
   h9/h10 to /mnt/huge for its rows; competitors via vcube's own MAP_HUGETLB or the hugeshim arena).
4. (No THP rows for the large tables: 1 GiB only, by decision.)
5. Restore h11 from its verified saved copy: `hugecp /data2/tables/h11.mnt /mnt/huge/h11`, then
   `cmp /data2/tables/h11.mnt /mnt/huge/h11`. The pool remains at 70 pages throughout; no pool
   resizing or regeneration is needed. h7 and h8 remain in /mnt/huge.
"""

screen_txt = "; ".join(r.strip("| ").replace(" | ", ": ", 1).replace(" | ", " shipped vs ", 1).replace(" | ", " native, ", 1)
                       for r in screen_rows)
rejdir = os.path.join(OUT, "rejected")
for f in sorted(os.listdir(rejdir)) if os.path.isdir(rejdir) else []:
    if f.endswith(".meta.txt"):
        m = read_meta(os.path.join(rejdir, f))
        rejected_run = m.get("run", "").split()
        if "_scalar" in f or (len(rejected_run) > 5 and rejected_run[5] == "scalar"):
            continue
        w = m.get("pages", "").split()
        t = m.get("table", "").split()
        tb = int(t[1]) if len(t) > 1 and t[1].isdigit() else 0
        why = "THP %d%%, 1G %d%% of the table" % (100 * int(w[3]) * 1024 / tb, 100 * int(w[4]) * 1024 / tb) if len(w) >= 5 and tb else "?"
        rp = os.path.join(rejdir, f[:-9] + ".reason")
        if os.path.exists(rp) and "outside load" in open(rp).read():
            why = open(rp).read().strip()
        rejected.append(({"file": f[:-9]}, why + "; rerun"))
if rejected:
    out.append("\n## Rejected runs (page placement, outside load, or interference validation)\n")
    out.extend("- %s: %s" % (r["file"], why) for r, why in rejected)
    out.append("")
spots = []
for r2 in good:
    if r2["threads"] == 1 and r2["rep"] == 2 and r2["build"] == "shipped" and r2["offset"] == 0:
        r1 = [r for r in good if (r["name"], r["pages"], r["threads"], r["ntest"], r["build"], r["offset"], r["rep"]) ==
              (r2["name"], r2["pages"], 1, r2["ntest"], "shipped", 0, 1)]
        if r1:
            spots.append("%s %s 1T %d cubes %.3f -> %.3f s (%+.1f%%)" % (r2["name"], r2["pages"].upper(), r2["ntest"],
                         r1[0]["p"]["tt"], r2["p"]["tt"], 100 * (r2["p"]["tt"] / r1[0]["p"]["tt"] - 1)))
FAIRNESS = FAIRNESS.replace("{spotcheck}", "; ".join(spots) or "pending")
out.append(FAIRNESS.replace("is used (see \"Build screening\").",
                            "is used (see \"Build screening\"). Result: -O3 -march=native was not faster for "
                            "either, so vcube and nissy use their shipped flags (%s)." % (screen_txt or "pending")))

# Step 2c: cube48opt10's table vs cube-opt's h10 (n1's recipe; tools/cmp-cube48opt10.sh, tools/bin/tablediff).
cmp_sem = os.path.join(OUT, "prep", "cmp-cube48opt10.txt")
cmp_bytes = os.path.join(OUT, "prep", "cmp-cube48opt10-bytes.txt")
if os.path.exists(cmp_sem) or os.path.exists(cmp_bytes):
    out.append("\n## cube48opt10's table vs cube-opt h10\n")
    out.append("cube48opt10's h48prun31h1.dat (made by its first run in the empty folder /data2/competitors/cube48opt10) "
               "against cube-opt's /data2/tables/h10 (data part; ours has a 4 KiB trailer after it). n1's static "
               "analysis (/data/agents/n1/cube48opt10-table.md) found the same size, base (10) and 2-bit code meaning, with "
               "one expected benign difference: the unread minimum nibble of groups whose entries are all above the base "
               "(15 in cube48opt, 11/12 in ours). `gen compare` reads both as cube-opt h10 tables; the byte compare counts "
               "benign vs real differences and the histograms of both files (equal histograms with many real differences "
               "= the same values in a different index order).\n")
    summ = os.path.join(OUT, "prep", "cmp-cube48opt10-summary.txt")
    if os.path.exists(summ):
        out.append(open(summ).read().strip() + "\n")
    for f in (cmp_sem, cmp_bytes):
        if os.path.exists(f):
            out.append("```\n" + open(f, errors="replace").read().strip() + "\n```\n")

# The report includes the reviewed upstream findings as requested. Keep a
# companion copy too, so the user can share it without the timing tables.
bugs_path = os.path.join(OUT, "competitor-bugs.md")
if os.path.exists(bugs_path):
    bugs_text = open(bugs_path).read()
    bugs_text = re.sub(r"(?<![\w/.-])cube-opt(?![\w/.-])", LABEL, bugs_text)
    open(os.path.join(OUT, "competitor-bugs.md"), "w").write(bugs_text)
    out.append("\n" + re.sub(r"^(#{1,5}) ", r"\1# ", bugs_text, flags=re.M))

pmu_path = os.path.join(OUT, "pmu-phack-analysis.md")
if os.path.exists(pmu_path):
    out.append("\n" + re.sub(r"^(#{1,5}) ", r"\1# ", open(pmu_path).read(), flags=re.M))

# Runs.
out.append("\n## Runs\n")
out.append("| run | status | wall s | outside load (cores) | gate wait s | pages measured |")
out.append("|---|---|---:|---:|---:|---|")
total = 0.0
for r in runs:
    total += r["wall"]
    pl = placement(r["meta"], r["tbytes"])
    out.append("| %s | %s | %.0f | %.1f | %s | %s |" % (r["file"], r["status"], r["wall"], r["ext"],
               r["meta"].get("gate_wait_s", ""), "%s %d%%" % (pl[0], round(min(pl[1], 100))) if pl else "-"))
out.append("\nTotal measured wall time: %.2f h.\n" % (total / 3600))
out.append(MEMORY_STEPS)
text = "\n".join(out) + "\n"
# Rename prose without changing the preserved source paths or run IDs.
text = re.sub(r"(?<![\w/.-])cube-opt(?![\w/.-])", LABEL, text)
if not os.path.exists(os.path.join(OUT, ".final-validation-complete")):
    title, separator, body = text.partition("\n")
    text = title + "\n\n**Draft: benchmark collection and final build validation are in progress.**\n" + body
text = re.sub(r"\bc48rsbatch(opt)(\d+)\b(?!_)", r"cube48opt-rust Opt\2 batch", text)
text = re.sub(r"\bc48rs(opt)(\d+)\b(?!_)", r"cube48opt-rust Opt\2 (latency: 18 threads, its cap)", text)
text = re.sub(r"\bc48rs-opt(\d+)\b(?![-_])", r"cube48opt-rust Opt\1", text)
open(os.path.join(OUT, "results.md"), "w").write(text)
print("wrote %s/results.md (%d runs, %d good)" % (OUT, len(runs), len(good)))
