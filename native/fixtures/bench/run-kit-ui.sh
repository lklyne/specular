#!/bin/sh
# Runs the UI-busy profiles (crates/specular-shell/src/ui_bench.rs) in the
# GPUI Kit window, one window a profile on a scratch copy of the canvas,
# writes one .jsonl a canvas into runs/perf/LABEL/ and prints the table.
#
#   fixtures/bench/run-kit-ui.sh LABEL [synthetic|cef]
#
# After a release build of specular-shell. APP names another binary, and
# with `cef` it must be the bundle's inner binary. CANVASES and PROFILES
# narrow the run, and CHECK=1 fails it when a part of the UI is built more
# often than kit-ui-table.py allows. The screen must be on and unlocked: a
# profile run without it prints `invalid` and the table says so.
set -eu
label="$1"
source="${2:-synthetic}"
native=$(cd "$(dirname "$0")/../.." && pwd)
app="${APP:-${CARGO_TARGET_DIR:-$native/target}/release/specular}"
out="$native/runs/perf/$label"
canvases="${CANVASES:-kitchen-sink bench/stickies-2000}"
profiles="${PROFILES:-quiet chat-open chat-run sidebar-churn popup-dropdown typing resize}"
mkdir -p "$out"
python3 "$native/fixtures/bench/agent-stream.py" > "$out/agent-stream.txt"
for canvas in $canvases; do
  name=$(basename "$canvas")
  from="$native/fixtures/$(dirname "$canvas")"
  : > "$out/$name-$source.jsonl"
  for profile in $profiles; do
    # The window saves into the canvas it opens and keeps its threads
    # beside it, so it opens a copy, with the files the canvas names.
    run="$out/$name-$source-$profile"
    rm -rf "$run"
    mkdir -p "$run/space" "$run/config"
    cp "$native/fixtures/$canvas.canvas" "$run/space/"
    for beside in "$from"/*.md "$from"/assets "$from"/notes; do
      if [ -e "$beside" ]; then cp -R "$beside" "$run/space/"; fi
    done
    (cd "$run" && SPECULAR_UI_BENCH="$profile" \
      SPECULAR_AGENT_SCRIPT="$out/agent-stream.txt" \
      SPECULAR_FLOAT_WINDOW=1 \
      SPECULAR_NATIVE_CONFIG_DIR="$run/config" \
      SPECULAR_PORT=0 SPECULAR_DISCOVERY_FILE="$run/discovery.json" \
      caffeinate -d perl -e 'alarm 180; exec @ARGV' \
      "$app" --source "$source" --window 1600x1000 "space/$name.canvas" \
      >> "$out/$name-$source.jsonl" 2> "$run/stderr.log") \
      || echo "$name $profile: the window exited with an error, see $run/stderr.log" >&2
  done
done
python3 "$native/fixtures/bench/kit-ui-table.py" ${CHECK:+--check} "$out"/*-"$source".jsonl
