#!/usr/bin/env python3
"""A 30 second idle run: how much CPU the app's process tree uses and how
many frames it draws while nothing happens.

    python3 fixtures/bench/idle.py APP [--source synthetic|cef] CANVAS

Prints one JSON object: CPU seconds used a second of wall time over the
middle 20 seconds (1.0 is one core), split into the app and its children,
and the idle profile's frame counts.
"""

import json
import subprocess
import sys
import time


def cpu_seconds(text):
    """`ps -o time` prints [[DD-]HH:]MM:SS.ss."""
    days, _, rest = text.rpartition("-")
    parts = [float(part) for part in rest.split(":")]
    seconds = 0.0
    for part in parts:
        seconds = seconds * 60 + part
    return seconds + (int(days) * 86400 if days else 0)


def tree_cpu(root):
    rows = subprocess.run(["ps", "-axo", "pid=,ppid=,time="], capture_output=True, text=True).stdout.split("\n")
    table = {}
    for row in rows:
        cells = row.split()
        if len(cells) == 3:
            table[int(cells[0])] = (int(cells[1]), cpu_seconds(cells[2]))
    children = {}
    for pid, (parent, _) in table.items():
        children.setdefault(parent, []).append(pid)
    below, stack = 0.0, list(children.get(root, []))
    while stack:
        pid = stack.pop()
        below += table[pid][1]
        stack.extend(children.get(pid, []))
    return table.get(root, (0, 0.0))[1], below


def main():
    app, *rest = sys.argv[1:]
    command = [app, *rest, "--bench", "idle", "--bench-duration-ms", "30000", "--warmup-ms", "5000", "--window", "1600x1000"]
    run = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    # Keeps the display awake until the app exits; a sleeping display presents nothing.
    subprocess.Popen(["caffeinate", "-d", "-w", str(run.pid)])
    time.sleep(10)
    root = run.pid
    start, at = tree_cpu(root), time.time()
    time.sleep(20)
    end, wall = tree_cpu(root), time.time() - at
    out, _ = run.communicate()
    line = json.loads(out.strip().split("\n")[-1])
    work = line.get("work", {})
    print(
        json.dumps(
            {
                "canvas": line.get("canvas"),
                "source": line.get("source"),
                "appCpu": round((end[0] - start[0]) / wall, 4),
                "childrenCpu": round((end[1] - start[1]) / wall, 4),
                "framesDrawn": work.get("framesDrawn"),
                "framesSkipped": work.get("framesSkipped"),
            }
        )
    )


if __name__ == "__main__":
    main()
