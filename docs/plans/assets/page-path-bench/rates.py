#!/usr/bin/env python3
"""Rates over a stretch of a Kit ledger, timed from the run's first entry,
or with --step K from the K-th scripted step it marks (0 is the first).

    rates.py [--step K] FROM_S TO_S LEDGER..

For each ledger: CEF paints per second (all pages, and the busiest page),
frames drawn per second, main loop turns per second and CEF pumps per
second between FROM_S and TO_S seconds after the ledger's first entry, then
the median of each over the ledgers."""
import collections, json, statistics, sys

args = sys.argv[1:]
step = None
if args[0] == "--step":
    step, args = int(args[1]), args[2:]
lo, hi, paths = float(args[0]), float(args[1]), args[2:]
rows = []
for path in paths:
    entries = []
    for raw in open(path):
        try:
            entries.append(json.loads(raw))
        except ValueError:
            pass
    if step is None:
        t0 = min(e["t"] for e in entries)
    else:
        t0 = sorted(e["t"] for e in entries if e.get("e") == "step")[step]  # -1 is the last
    inside = [e for e in entries if lo * 1e6 <= e["t"] - t0 <= hi * 1e6]
    wall = hi - lo
    count = collections.Counter(e.get("e") for e in inside)
    per_page = collections.Counter(e.get("page") for e in inside if e.get("e") == "cef.paint")
    pumps = count["pump"] + sum(e.get("short", 0) for e in inside if e.get("e") == "pump")
    row = {
        "paints/s": count["cef.paint"] / wall,
        "busiest page paints/s": (max(per_page.values()) if per_page else 0) / wall,
        "frames/s": count["frame"] / wall,
        "turns/s": count["loop"] / wall,
        "pumps/s": pumps / wall,
        "drops": count["cef.drop"],
    }
    rows.append(row)
    print(path.split("/")[-1], " ".join(f"{k} {v:.1f}" for k, v in row.items()))
if rows:
    print("median", " ".join(f"{k} {statistics.median(r[k] for r in rows):.1f}" for k in rows[0]))
