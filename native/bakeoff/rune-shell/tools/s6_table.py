#!/usr/bin/env python3
"""Tables from a `bench-s6.sh` folder: median and [min-max] over the runs
of each case and shell.

    python3 tools/s6_table.py OUT_DIR
"""

import glob
import json
import os
import re
import statistics
import sys


def load(folder):
    cases = {}
    for path in sorted(glob.glob(os.path.join(folder, "*.json"))):
        name = os.path.basename(path)[:-5]
        match = re.match(r"(.+)-(kit|rune)-(\d+)$", name)
        if not match:
            continue
        with open(path) as handle:
            try:
                data = json.load(handle)
            except ValueError:
                continue
        cases.setdefault((match.group(1), match.group(2)), []).append(data)
    return cases


def get(data, path):
    value = data
    for key in path.split("."):
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    return value


def frames_drawn(data):
    bench = data.get("bench", {})
    return bench.get("framesDrawn", get(bench, "work.framesDrawn"))


def footprint(data, kinds):
    memory = data.get("memory")
    if not isinstance(memory, list):
        return None
    return sum(row.get("footprintMb", 0) for row in memory if kinds is None or row.get("kind") in kinds)


def cell(values, digits=2):
    values = [v for v in values if v is not None]
    if not values:
        return "-"
    fmt = f"{{:.{digits}f}}"
    if len(values) == 1:
        return fmt.format(values[0])
    return f"{fmt.format(statistics.median(values))} [{fmt.format(min(values))}-{fmt.format(max(values))}]"


PAN = [
    ("p50 ms", lambda d: get(d, "bench.p50FrameMs")),
    ("p95 ms", lambda d: get(d, "bench.p95FrameMs")),
    ("p99 ms", lambda d: get(d, "bench.p99FrameMs")),
    ("long frames", lambda d: get(d, "bench.longFrames")),
    ("fps", lambda d: get(d, "bench.drawFps")),
    ("main thread ms/frame", lambda d: d.get("mainThreadMsPerFrame")),
    ("main thread CPU", lambda d: d.get("mainThreadCpu")),
    ("browser process CPU", lambda d: d.get("appCpu")),
    ("CEF children CPU", lambda d: d.get("childrenCpu")),
    ("pump mean ms", lambda d: get(d, "ledger.pumpMeanMs")),
    ("pump p95 ms", lambda d: get(d, "ledger.pumpP95Ms")),
    ("pump max ms", lambda d: get(d, "ledger.pumpMaxMs")),
    ("pump ms/frame p95", lambda d: get(d, "ledger.pumpPerFrameP95Ms")),
    ("pump ms/s", lambda d: get(d, "ledger.pumpMsPerS")),
    ("rune pacing", lambda d: None),
]

IDLE = [
    ("browser process CPU", lambda d: d.get("appCpu")),
    ("CEF children CPU", lambda d: d.get("childrenCpu")),
    ("main thread CPU", lambda d: d.get("mainThreadCpu")),
    ("main loop turns/s", lambda d: get(d, "ledger.loopTurnsPerS")),
    ("pumps/s", lambda d: get(d, "ledger.pumpsPerS")),
    ("frames drawn", frames_drawn),
    ("footprint browser+GPU MB", lambda d: footprint(d, {"browser", "gpu-process"})),
    ("footprint tree MB", lambda d: footprint(d, None)),
]


FLIP = [
    ("samples", lambda d: get(d, "bench.inputLatency.samples")),
    ("unresolved", lambda d: get(d, "bench.inputLatency.unresolved")),
    ("key to present p50 ms", lambda d: get(d, "bench.inputLatency.p50Ms")),
    ("key to present p95 ms", lambda d: get(d, "bench.inputLatency.p95Ms")),
    ("key to present max ms", lambda d: get(d, "bench.inputLatency.maxMs")),
    ("paint to present p50 ms", lambda d: get(d, "bench.paintToPresentP50Ms")),
    ("paint to present p95 ms", lambda d: get(d, "bench.paintToPresentP95Ms")),
]

GPU = [
    ("GPU after submit mean ms", lambda d: get(d, "bench.gpuMeanMs")),
    ("GPU after submit p95 ms", lambda d: get(d, "bench.gpuP95Ms")),
    ("render CPU mean ms", lambda d: get(d, "bench.renderCpuMeanMs")),
]


def table(cases, case, rows):
    kit, rune = cases.get((case, "kit"), []), cases.get((case, "rune"), [])
    if not kit and not rune:
        return
    print(f"\n## {case}  (runs: kit {len(kit)}, rune {len(rune)})")
    print(f"| | Kit | Rune |\n|---|---|---|")
    for label, read in rows:
        if label == "rune pacing":
            pacing = [json.dumps(get(d, "bench.pacing")) for d in rune]
            print(f"| {label} | - | {'; '.join(pacing)} |")
            continue
        digits = 0 if label in ("long frames", "frames drawn") else 3 if "CPU" in label else 2
        print(f"| {label} | {cell([read(d) for d in kit], digits)} | {cell([read(d) for d in rune], digits)} |")


def main():
    cases = load(sys.argv[1])
    for case in sorted({name for name, _ in cases}):
        rows = IDLE if case.startswith(("idle", "idlestill")) else FLIP if case.startswith("flip") \
            else GPU if case.startswith("gpu") else PAN
        table(cases, case, rows)


if __name__ == "__main__":
    main()
