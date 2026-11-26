#!/usr/bin/env python3
"""One S6 bench case on one shell, folded into one JSON line.

    python3 tools/s6_run.py OUT.json -- APP ARGS...

APP is the Kit's or the spike's bundle binary, with its `--bench` arguments.
This runs it under `caffeinate -d` with `SPECULAR_RESIZE_TRACE` set, and
measures from outside what both shells can be measured by alike:

- the main thread's CPU time (`ps -M`, the first thread) and the CEF
  children's CPU (`ps`), from the line that says the profile starts to the
  bench line on stdout (both shells print it after the 250 ms gap);
- the ledger over the profile's window: CEF pumps (`Pump`), the main run
  loop's busy turns (`Loop`) and frames. The spike marks the window with
  `step` entries; for the Kit it is cut back from the last frame (it draws
  none once it exits) by the gap and the line's `durationMs`;
- with RSS_AT=SECONDS, the process tree's memory at that time after start
  (`specular-bench rss`, BENCH names the binary).
"""

import json
import os
import subprocess
import sys
import threading
import time


def cpu_seconds(text):
    days, _, rest = text.rpartition("-")
    seconds = 0.0
    for part in rest.split(":"):
        seconds = seconds * 60 + float(part)
    return seconds + (int(days) * 86400 if days else 0)


def main_thread_cpu(pid):
    """User plus system seconds of the process's first thread."""
    rows = subprocess.run(["ps", "-M", "-p", str(pid)], capture_output=True, text=True).stdout.split("\n")
    if len(rows) < 2:
        return None
    cells = rows[1].split()
    # USER PID TT %CPU STAT PRI STIME UTIME COMMAND...
    return cpu_seconds(cells[6]) + cpu_seconds(cells[7])


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


def pct(values, q):
    if not values:
        return None
    ordered = sorted(values)
    return ordered[round((len(ordered) - 1) * q)]


def ledger_window(path, line):
    entries = []
    with open(path) as handle:
        for raw in handle:
            try:
                entries.append(json.loads(raw))
            except ValueError:
                pass
    steps = [e["t"] for e in entries if e.get("e") == "step"]
    if len(steps) >= 2:
        start, end, how = steps[-2], steps[-1], "step marks"
    else:
        frames = [e["t"] for e in entries if e.get("e") == "frame"]
        if not frames:
            return {}
        end = frames[-1] - 250_000
        # An idle profile draws only what pages paint, so its `durationMs`
        # (first to last frame) is not its length; the bench ran it 30 s.
        length = 30_000 if line.get("phase") == "idle" else line.get("durationMs", 0)
        start = end - length * 1000
        how = "last frame less the gap"
    inside = [e for e in entries if start <= e["t"] <= end]
    wall = max(end - start, 1) / 1e6
    pumps = [e["dur"] / 1e3 for e in inside if e.get("e") == "pump"]
    short = sum(e.get("short", 0) for e in inside if e.get("e") == "pump")
    loops = [e["busy"] / 1e3 for e in inside if e.get("e") == "loop"]
    frames = sorted(e["t"] for e in inside if e.get("e") == "frame")
    # Pump time between consecutive frames: what ADR 0053 item 5 asks of a frame.
    per_frame = []
    pump_at = sorted((e["t"], e["dur"] / 1e3) for e in inside if e.get("e") == "pump")
    k = 0
    for a, b in zip(frames, frames[1:]):
        total = 0.0
        while k < len(pump_at) and pump_at[k][0] < b:
            if pump_at[k][0] >= a:
                total += pump_at[k][1]
            k += 1
        per_frame.append(total)
    every = pumps + [0.025] * short  # a short pump is under 50 us; counted as 25
    return {
        "window": how,
        "windowS": round(wall, 3),
        "pumpsPerS": round((len(pumps) + short) / wall, 1),
        "pumpLoggedMeanMs": round(sum(pumps) / len(pumps), 3) if pumps else 0,
        "pumpMeanMs": round(sum(every) / len(every), 3) if every else 0,
        "pumpP95Ms": round(pct(every, 0.95) or 0, 3),
        "pumpMaxMs": round(max(pumps), 3) if pumps else 0,
        "pumpMsPerS": round(sum(every) / wall, 2),
        "pumpPerFrameP95Ms": round(pct(per_frame, 0.95) or 0, 3),
        "loopTurnsPerS": round(len(loops) / wall, 1),
        "loopBusyShare": round(sum(loops) / 1e3 / wall, 4),
        "loopBusyMsPerFrame": round(sum(loops) / max(len(frames), 1), 3),
        "ledgerFrames": len(frames),
    }


def ledger_pan(path, duration_ms):
    """A scripted window run (no bench line): frame intervals from the
    ledger's frames over the `pan` step, the second-to-last step marked,
    less its first 300 ms."""
    entries = []
    with open(path) as handle:
        for raw in handle:
            try:
                entries.append(json.loads(raw))
            except ValueError:
                pass
    steps = [e["t"] for e in entries if e.get("e") == "step"]
    if len(steps) < 2:
        return {}
    start = steps[-2] + 300_000
    end = steps[-2] + duration_ms * 1000
    frames = sorted(e["t"] for e in entries if e.get("e") == "frame" and start <= e["t"] <= end)
    gaps = [(b - a) / 1e3 for a, b in zip(frames, frames[1:])]
    if not gaps:
        return {}
    return {
        "draws": len(gaps),
        "drawFps": round(len(gaps) / (sum(gaps) / 1e3), 1),
        "p50FrameMs": round(pct(gaps, 0.5), 3),
        "p95FrameMs": round(pct(gaps, 0.95), 3),
        "p99FrameMs": round(pct(gaps, 0.99), 3),
        "maxFrameMs": round(max(gaps), 3),
        "longFrames": sum(1 for g in gaps if g > 12.5),
    }


def main():
    if sys.argv[1] == "--reledger":
        # Reads a finished run's ledger again, after a change to this file.
        for out in sys.argv[2:]:
            with open(out) as handle:
                result = json.load(handle)
            if result.get("bench") and os.path.exists(out + ".ledger") and "inputLatency" not in result["bench"]:
                result["ledger"] = ledger_window(out + ".ledger", result["bench"])
            with open(out, "w") as handle:
                json.dump(result, handle)
                handle.write("\n")
        return
    out = sys.argv[1]
    command = sys.argv[sys.argv.index("--") + 1 :]
    ledger = out + ".ledger"
    if os.path.exists(ledger):
        os.remove(ledger)
    env = dict(os.environ, SPECULAR_RESIZE_TRACE=ledger)
    started = time.time()
    run = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
    awake = subprocess.Popen(["caffeinate", "-d", "-w", str(run.pid)])
    marks = {}
    log = open(out + ".err", "w")

    def read_err():
        for raw in run.stderr:
            log.write(raw)
            if "profile starts" in raw or "bench profile" in raw:
                marks.setdefault("start", (time.time(), main_thread_cpu(run.pid), tree_cpu(run.pid)))
            if "display link creation failed" in raw:
                marks["display_link_failed"] = raw.strip()

    reader = threading.Thread(target=read_err, daemon=True)
    reader.start()
    memory = None
    rss_at = os.environ.get("RSS_AT")
    if rss_at:
        def sample():
            nonlocal memory
            time.sleep(float(rss_at))
            bench = os.environ.get("BENCH", "specular-bench")
            got = subprocess.run([bench, "rss", "--pid", str(run.pid), "--per-process", "true"],
                                 capture_output=True, text=True)
            try:
                memory = json.loads(got.stdout)
            except ValueError:
                memory = {"error": got.stderr.strip()[:300]}
        threading.Thread(target=sample, daemon=True).start()
    lines = []
    for raw in run.stdout:
        raw = raw.strip()
        if raw.startswith("{"):
            if "end" not in marks:
                marks["end"] = (time.time(), main_thread_cpu(run.pid), tree_cpu(run.pid))
            lines.append(json.loads(raw))
    try:
        run.wait(timeout=60)
    except subprocess.TimeoutExpired:
        run.kill()
    awake.wait()
    reader.join(timeout=5)
    log.close()
    result = {"command": command[1:], "exit": run.returncode, "seconds": round(time.time() - started, 1)}
    if lines:
        line = lines[-1]
        result["bench"] = line
        if "start" in marks and "end" in marks:
            (t0, m0, (a0, c0)), (t1, m1, (a1, c1)) = marks["start"], marks["end"]
            wall = t1 - t0
            frames = max(line.get("draws", 0), 1)
            if m0 is not None and m1 is not None:
                result["mainThreadCpu"] = round((m1 - m0) / wall, 4)
                result["mainThreadMsPerFrame"] = round((m1 - m0) * 1e3 / frames, 3)
            result["appCpu"] = round((a1 - a0) / wall, 4)
            result["childrenCpu"] = round((c1 - c0) / wall, 4)
            result["outsideWindowS"] = round(wall, 3)
        if os.path.exists(ledger):
            result["ledger"] = ledger_window(ledger, line)
    if not lines and os.path.exists(ledger):
        result["bench"] = ledger_pan(ledger, float(os.environ.get("PAN_MS", "6000")))
        result["ledger"] = {}
    if memory is not None:
        result["memory"] = memory
    if "display_link_failed" in marks:
        result["displayLinkFailed"] = marks["display_link_failed"]
    with open(out, "w") as handle:
        json.dump(result, handle)
        handle.write("\n")
    print(json.dumps({k: result.get(k) for k in ("exit", "mainThreadCpu", "appCpu", "childrenCpu")}))


if __name__ == "__main__":
    main()
