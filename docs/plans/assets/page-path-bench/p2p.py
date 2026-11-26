#!/usr/bin/env python3
"""Paint to present from a Kit ledger: each CEF paint is matched to the first
frame that starts after it; the sample is that frame's end (start + dur, up to
its present returning) less the paint's time. Paints in the first SKIP_S
seconds (load, warmup) are left out.
    p2p.py [--skip S] LEDGER..."""
import json, sys
args = sys.argv[1:]
skip = 12.0
if args and args[0] == '--skip':
    skip = float(args[1]); args = args[2:]
def pct(v, q):
    v = sorted(v); return v[round((len(v) - 1) * q)] if v else float('nan')
allv = []
for path in args:
    paints, frames = [], []
    for raw in open(path):
        try: e = json.loads(raw)
        except ValueError: continue
        if e.get('e') == 'cef.paint': paints.append(e['t'])
        elif e.get('e') == 'frame': frames.append((e['t'], e['t'] + e['dur']))
    frames.sort(); paints.sort()
    t0 = min([p for p in paints] + [f[0] for f in frames] or [0])
    vals, k = [], 0
    for p in paints:
        if p - t0 < skip * 1e6: continue
        while k < len(frames) and frames[k][0] < p: k += 1
        if k == len(frames): break
        vals.append((frames[k][1] - p) / 1e3)
    allv += vals
    print(f"{path.split('/')[-1]}: n={len(vals)} p50={pct(vals,.5):.2f} p95={pct(vals,.95):.2f} max={max(vals) if vals else float('nan'):.2f} ms")
print(f"all: n={len(allv)} p50={pct(allv,.5):.2f} p95={pct(allv,.95):.2f} max={max(allv) if allv else float('nan'):.2f} ms")
