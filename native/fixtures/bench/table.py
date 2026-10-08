#!/usr/bin/env python3
"""Prints bench lines as a markdown table: one row a canvas and profile,
mean milliseconds a frame in each step.

    python3 fixtures/bench/table.py runs/perf/LABEL/*.jsonl
"""

import json
import sys

STEPS = ["update", "view", "cull", "shaping", "batching", "tessellation", "build", "glyphs", "upload", "submit"]


def main():
    print("| canvas | shell | profile | drawn | skipped | cpu mean | cpu p95 | " + " | ".join(STEPS) + " | gpu | interval p95 | items | batches | draws | glyphs | triangles | process cpu |")
    print("|" + "---|" * (len(STEPS) + 16))
    for path in sys.argv[1:]:
        for text in open(path):
            line = json.loads(text)
            work = line.get("work")
            if not work:
                continue
            cells = [
                (line.get("canvas") or "").removesuffix(".canvas"),
                line.get("shell") or line.get("target") or "",
                line["phase"],
                work["framesDrawn"],
                work["framesSkipped"],
                f'{work["cpu"]["mean"]:.2f}',
                f'{work["cpu"]["p95"]:.2f}',
                *(f'{work[step]["mean"]:.2f}' for step in STEPS),
                f'{work["gpu"]["mean"]:.2f}',
                f'{line.get("p95FrameMs", 0):.1f}',
                work["items"],
                work["batches"],
                work["drawCalls"],
                work["glyphCount"],
                work["triangles"],
                # Cores the whole process used; window runs only.
                f'{work["processCpu"]:.3f}' if "processCpu" in work else "",
            ]
            print("| " + " | ".join(str(cell) for cell in cells) + " |")


if __name__ == "__main__":
    main()
