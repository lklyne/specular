#!/usr/bin/env python3
"""Prints UI bench lines as a markdown table: one row a canvas and profile.

    python3 fixtures/bench/kit-ui-table.py [--check] runs/perf/LABEL/*.jsonl

A line taken with the screen locked or the window covered says `invalid`
and has no numbers; the row says why and the exit status is 1.
"""

import json
import sys

PARTS = ["root", "toolbar", "popup", "sidebar", "chatHeader", "transcript", "runBar", "composer"]


def main():
    paths = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
    bad = 0
    print("| canvas | pages | profile | canvas fps | p99 ms | long frames | GPUI fps | GPUI ms a frame, mean / p95 / max | model reads/s | " + " | ".join(PARTS) + " |")
    print("|" + "---|" * (9 + len(PARTS)))
    for path in paths:
        for text in open(path):
            if not text.startswith("{"):
                continue
            line = json.loads(text)
            if "invalid" in line:
                bad += 1
                print(f'| {path.rsplit("/", 1)[-1]} | | {line["uiBench"]} | not measured: {line["invalid"]} |')
                continue
            built = line["builtPerSecond"]
            cells = [
                line["canvas"],
                line["source"],
                line["uiBench"],
                f'{line["canvasFps"]:.1f}',
                f'{line["canvasP99Ms"]:.1f}',
                line["longFrames"],
                f'{line["gpuiFps"]:.1f}',
                f'{line["gpuiFrameMs"]:.2f} / {line["gpuiFrameP95Ms"]:.2f} / {line["gpuiFrameMaxMs"]:.1f}',
                f'{line["modelReadsPerSecond"]:.1f}',
            ] + [f"{built[part]:.0f}" for part in PARTS]
            print("| " + " | ".join(str(cell) for cell in cells) + " |")
    sys.exit(1 if bad else 0)


main()
