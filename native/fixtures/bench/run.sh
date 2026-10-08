#!/bin/sh
# Runs the pan, zoom and idle profiles on every bench canvas and writes one
# .jsonl a canvas into runs/perf/LABEL/, then prints the table.
#
#   fixtures/bench/run.sh LABEL [headless|window] [synthetic|cef]
#
# From native/, after a release build. With `cef`, APP must be the bundle's
# inner binary (see README.md, "Morning run"). A window run needs the
# display awake and the window uncovered, or macOS presents nothing.
set -eu
label="$1"
target="${2:-headless}"
source="${3:-synthetic}"
app="${APP:-target/release/specular-app}"
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
