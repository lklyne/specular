#!/usr/bin/env python3
"""The footprint of the app's process tree over a run, by kind of process.

    python3 fixtures/bench/memory.py APP BENCH [--at 6,15,30] -- APP ARGS..

Starts the app with the arguments after `--` plus a long idle bench, and at
each of the times (seconds after launch) prints the tree's footprint in MiB
split into the browser (the app itself), the GPU process, the renderers
and the rest. Needs `specular-bench` built (`BENCH`).
"""

import collections
import json
import subprocess
import sys
import time


def main():
    app, bench, *rest = sys.argv[1:]
    times = [6, 15, 30]
    if rest and rest[0] == "--at":
        times = [float(part) for part in rest[1].split(",")]
        rest = rest[2:]
    arguments = rest[1:] if rest and rest[0] == "--" else rest
    length = int(max(times) * 1000) + 4000
    command = [app, *arguments, "--bench", "idle", "--bench-duration-ms", str(length), "--warmup-ms", "0", "--window", "1600x1000"]
    run = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.Popen(["caffeinate", "-d", "-w", str(run.pid)])
    started = time.time()
    for at in times:
        time.sleep(max(0.0, started + at - time.time()))
        rows = json.loads(subprocess.run([bench, "rss", "--pid", str(run.pid), "--per-process", "true"], capture_output=True, text=True).stdout)
        kinds = collections.defaultdict(float)
        for row in rows:
            kind = row["kind"] if row["kind"] in ("browser", "gpu-process", "renderer") else "other"
            kinds[kind] += row["footprintMb"] or 0.0
        renderers = sum(1 for row in rows if row["kind"] == "renderer")
        print(json.dumps({"atSeconds": at, "totalMb": round(sum(kinds.values())), "renderers": renderers, **{kind: round(value) for kind, value in sorted(kinds.items())}}))
    run.terminate()
    run.wait()


if __name__ == "__main__":
    main()
