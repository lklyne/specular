#!/usr/bin/env python3
"""The tables from a bench.sh run folder: median [min to max] of each case
for each label (before, after).

    table.py OUT_DIR"""
import glob, json, os, statistics, subprocess, sys

out = sys.argv[1]
here = os.path.dirname(os.path.abspath(__file__))
labels = ("before", "after")


def cell(values):
    values = [v for v in values if v is not None]
    if not values:
        return "-"
    return f"{statistics.median(values):.2f} [{min(values):.2f} to {max(values):.2f}]"


def runs(case, label):
    return [json.load(open(p)) for p in sorted(glob.glob(f"{out}/{case}-{label}-*.json"))]


def row(name, case, get):
    print(f"| {name} | " + " | ".join(cell([get(r) for r in runs(case, l)]) for l in labels) + " |")


print("| measure | before | after |\n|---|---|---|")
lat = lambda key: lambda r: r["bench"]["inputLatency"][key]
for key in ("p50Ms", "p95Ms", "maxMs", "unresolved"):
    row(f"flip: key to present {key}", "flip", lat(key))
for label in labels:
    ledgers = sorted(glob.glob(f"{out}/flip-{label}-*.json.ledger"))
    if ledgers:
        last = subprocess.run(["python3", f"{here}/p2p.py", "--skip", "0", *ledgers], capture_output=True, text=True).stdout.strip().split("\n")[-1]
        print(f"| flip: paint to present ({label}) | {last} | |")
for pages in (9, 20, 40):
    case = f"pan-{pages}"
    row(f"{case}: p99 ms", case, lambda r: r["bench"]["p99FrameMs"])
    row(f"{case}: long frames", case, lambda r: r["bench"]["longFrames"])
    row(f"{case}: main thread ms/frame", case, lambda r: r.get("mainThreadMsPerFrame"))
    row(f"{case}: pump p95 ms", case, lambda r: r["ledger"]["pumpP95Ms"])
    row(f"{case}: dropped for pool pressure", case, lambda r: r["bench"]["textures"]["framesDroppedForPoolPressure"])
row("idle: turns/s", "idlestill-20", lambda r: r["ledger"]["loopTurnsPerS"])
row("idle: pumps/s", "idlestill-20", lambda r: r["ledger"]["pumpsPerS"])
row("idle: frames", "idlestill-20", lambda r: r["bench"]["work"]["framesDrawn"])
row("idle: browser CPU %", "idlestill-20", lambda r: 100 * r["appCpu"])
row("idle: main thread CPU %", "idlestill-20", lambda r: 100 * r["mainThreadCpu"])

def rates(case, args):
    for label in labels:
        ledgers = sorted(glob.glob(f"{out}/{case}-{label}-*.ledger"))
        if ledgers:
            last = subprocess.run(["python3", f"{here}/rates.py", *args, *ledgers], capture_output=True, text=True).stdout.strip().split("\n")[-1]
            print(f"| {case} {' '.join(args)} ({label}) | {last} | |")

rates("entered", ["--step", "-1", "-4.5", "-0.3"])
for case in ("occluded", "occluded-full"):
    rates(case, ["--step", "0", "-5.5", "-0.5"])
    rates(case, ["--step", "0", "1", "8"])
