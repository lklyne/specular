"""Scans resize captures for a frame where GPUI and the canvas view disagree.

GPUI paints a 4 pt pure-green stripe on the sidebar's right edge, and in the
`above` layering a magenta fill under the canvas view. On one pixel row of
each capture this reports the stripe's extent and how much magenta shows: a
canvas view that trails the layout leaves magenta (sidebar shrinking) or
covers the stripe (sidebar growing).

    python3 scripts/lockstep-scan.py shots/resize-above-*.png
"""
import subprocess
import sys

ROW = 56 + 800  # title bar, then 400 pt down the content, in device pixels


def row_of(path):
    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "stream=width,height", "-of", "csv=p=0", path],
        capture_output=True, text=True, check=True)
    width, height = (int(n) for n in probe.stdout.strip().split(","))
    raw = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", path, "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
        capture_output=True, check=True).stdout
    start = min(ROW, height - 1) * width * 3
    return [tuple(raw[start + 3 * x:start + 3 * x + 3]) for x in range(width)]


worst = 0
for path in sys.argv[1:]:
    row = row_of(path)
    green = [x for x, (r, g, b) in enumerate(row) if r < 140 and g > 190 and b < 110]
    magenta = [x for x, (r, g, b) in enumerate(row) if r > 180 and g < 120 and b > 180]
    stripe = f"{green[0]}..{green[-1] + 1} ({len(green)} px)" if green else "hidden"
    fault = len(magenta) + (0 if len(green) == 8 else abs(8 - len(green)))
    worst = max(worst, fault)
    where = f" at {magenta[0]}..{magenta[-1] + 1}" if magenta else ""
    print(f"{path.split('/')[-1]:40} width {len(row):5} stripe {stripe:20} magenta {len(magenta):3} px{where}")
print(f"worst disagreement on the row: {worst} device px")
