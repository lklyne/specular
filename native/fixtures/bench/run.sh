#!/bin/sh
# Runs the pan, zoom and idle profiles on every bench canvas and writes one
# .jsonl a canvas into runs/perf/LABEL/, then prints the table.
#
#   fixtures/bench/run.sh LABEL [headless|window] [synthetic|cef] [winit|kit]
#
# From native/, after a release build. The last argument picks the shell
# whose window is measured: `winit` is target/release/specular-app and `kit`
# is target/release/specular. APP names another binary, and with `cef` it
# must be the bundle's inner binary (see README.md, "Morning run"). A window
# run needs the display awake and the window uncovered, or macOS presents
# nothing.
set -eu
label="$1"
target="${2:-headless}"
source="${3:-synthetic}"
shell="${4:-winit}"
case "$shell" in
  winit) binary=target/release/specular-app ;;
  kit) binary=target/release/specular ;;
  *) echo "unknown shell \`$shell\` (expected winit or kit)" >&2; exit 2 ;;
esac
app="${APP:-$binary}"
out="runs/perf/$label"
mkdir -p "$out"
canvases="${CANVASES:-kitchen-sink bench/stickies-500 bench/stickies-2000 bench/drawings-300 bench/edges-200 bench/documents-50 bench/mixed}"
for canvas in $canvases; do
  name="$(basename "$canvas")"
  file="$out/$name-$target-$source.jsonl"
  caffeinate -d "$app" --source "$source" --bench slow-pan,slow-zoom,idle \
    --bench-target "$target" --window 1600x1000 --snapshot-size 1600x1000 --snapshot-scale 2 \
    --warmup-ms "${WARMUP_MS:-3000}" "fixtures/$canvas.canvas" > "$file" 2>> "$out/run.log"
done
python3 fixtures/bench/table.py "$out"/*-"$target"-"$source".jsonl
