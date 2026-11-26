#!/usr/bin/env python3
"""Runs a window case with the ledger on, and stops it if it outlives
LIMIT_S: TERM, then KILL five seconds later. Prints how it ended.

    timed.py LIMIT_S LEDGER -- APP ARGS..   (env passes through)"""
import os, subprocess, sys, time

limit, ledger = float(sys.argv[1]), sys.argv[2]
command = sys.argv[sys.argv.index("--") + 1:]
env = dict(os.environ, SPECULAR_RESIZE_TRACE=ledger)
started = time.time()
with open(ledger + ".err", "w") as err:
    run = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=err, env=env)
    awake = subprocess.Popen(["caffeinate", "-d", "-w", str(run.pid)])
    try:
        code = run.wait(timeout=limit)
        how = f"exit {code}"
    except subprocess.TimeoutExpired:
        run.terminate()
        try:
            run.wait(timeout=5)
            how = f"hung, stopped by TERM after {limit:.0f} s"
        except subprocess.TimeoutExpired:
            run.kill()
            run.wait()
            how = f"hung, killed after {limit:.0f} s"
print(f"{ledger.split('/')[-1]}: {how} in {time.time() - started:.1f} s")
