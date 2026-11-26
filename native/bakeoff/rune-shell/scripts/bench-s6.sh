#!/usr/bin/env bash
# Step S6 of the Rune spike: the Kit shell against the Rune spike, on real
# CEF pages, in windows, release builds. Rerun on another Mac with:
#
#   native/bakeoff/rune-shell/scripts/bench-s6.sh OUT_DIR [CASES]
#
# CASES is a space-separated subset of `build pan idle shimmer popover gpu
# latency` (default: all; `latency` last, since it rebuilds the spike with
# Rune's tooling for `--window-script` and then without it again). Results are one JSON file a run in OUT_DIR, and
# `python3 tools/s6_table.py OUT_DIR` prints the tables.
#
# Before running: power connected, display awake, nothing else building, and
# do not touch the machine. Each bench window floats above the others; a
# covered window, a locked screen or a sleeping display presents nothing.
# The fixtures load live sites, so the network must be up.
set -euo pipefail

out="${1:?usage: bench-s6.sh OUT_DIR [CASES]}"
cases="${2:-build pan idle shimmer popover gpu latency}"
reps="${REPS:-3}"
spike="$(cd "$(dirname "$0")/.." && pwd)"
native="$(cd "$spike/../.." && pwd)"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
export CEF_PATH="${CEF_PATH:-$HOME/.local/share/cef}"

KIT="$native/target/release/specular.app/Contents/MacOS/specular"
RUNE="$spike/target/release/rune-shell.app/Contents/MacOS/rune-shell"
export BENCH="$native/target/release/specular-bench"
RUN="python3 $spike/tools/s6_run.py"
WINDOW=1600x1000
WARMUP="${WARMUP_MS:-20000}"
PAN_MS="${PAN_MS:-6000}"

has() { [[ " $cases " == *" $1 "* ]]; }

if has build; then
  (cd "$native" && cargo build --release -p specular-shell --features cef \
    && cargo build --release -p specular-bench \
    && crates/specular-cef/scripts/bundle-macos.sh release)
  (cd "$spike" && cargo build --release --features cef \
    && CARGO_TARGET_DIR="$spike/target" "$native/crates/specular-cef/scripts/bundle-macos.sh" release rune-shell)
fi

# What else is running, for the record.
ps -Ao pid,pcpu,pmem,comm -r > "$out/ps-before.txt"
{ sysctl -n machdep.cpu.brand_string; sw_vers -productVersion; pmset -g batt; uptime; } > "$out/machine.txt"

kit() { # NAME ARGS...
  local name="$1"; shift
  $RUN "$out/$name.json" -- "$KIT" --window "$WINDOW" --paint-policy full-rate "$@"
}
rune() { # NAME ARGS...
  local name="$1"; shift
  $RUN "$out/$name.json" -- "$RUNE" --source cef --size "$WINDOW" "$@"
}

cd "$native"

# 1. Pan over 9, 20 and 40 pages: the Kit's slow-pan, stretched, both shells
# interleaved so drift in the machine lands on both.
if has pan; then
  for pages in 9 20 40; do
    for i in $(seq "$reps"); do
      kit "pan-$pages-kit-$i" --bench slow-pan --bench-duration-ms "$PAN_MS" \
        --warmup-ms "$WARMUP" "fixtures/static-$pages.canvas"
      rune "pan-$pages-rune-$i" "fixtures/static-$pages.canvas" --bench slow-pan \
        --bench-duration-ms "$PAN_MS" --warmup-ms "$WARMUP"
    done
  done
fi

# 3 and 5. Idle for 30 s at 20 pages, with the tree's memory sampled 25 s in
# (settled: pages loaded at least 5 s before).
if has idle; then
  for i in $(seq "$reps"); do
    RSS_AT=25 kit "idle-20-kit-$i" --bench idle --bench-duration-ms 30000 \
      --warmup-ms "$WARMUP" fixtures/static-20.canvas
    RSS_AT=25 rune "idle-20-rune-$i" fixtures/static-20.canvas --bench idle \
      --bench-duration-ms 30000 --warmup-ms "$WARMUP"
  done
fi

# 3 again with pages that are still: 20 copies of a static local page, so a
# frame or a wakeup is the shell's own and not a site's animation.
if has idle || has idlestill; then
  for i in $(seq "$reps"); do
    kit "idlestill-20-kit-$i" --bench idle --bench-duration-ms 30000 \
      --warmup-ms 10000 "$spike/fixtures/still-20.canvas"
    rune "idlestill-20-rune-$i" "$spike/fixtures/still-20.canvas" --bench idle \
      --bench-duration-ms 30000 --warmup-ms 10000
  done
fi

# 2. A label that animates for ever while panning 20 pages. Rune: `--shimmer`
# beside the same bench. The Kit cannot animate chrome in a bench run (its
# agent runs are off there), so its case is a scripted window run: see
# scripts/s6-kit-shimmer.sh.
if has shimmer; then
  for i in $(seq "$reps"); do
    rune "shimmer-20-rune-$i" fixtures/static-20.canvas --shimmer --bench slow-pan \
      --bench-duration-ms "$PAN_MS" --warmup-ms "$WARMUP"
    rune "noshimmer-20-rune-$i" fixtures/static-20.canvas --bench slow-pan \
      --bench-duration-ms "$PAN_MS" --warmup-ms "$WARMUP"
  done
  if [[ -x "$spike/scripts/s6-kit-shimmer.sh" ]]; then
    "$spike/scripts/s6-kit-shimmer.sh" "$out"
  fi
fi

# Question 9 as written: 40 pages under a pan with a Rune popover (the zoom
# list) open. The Kit has no popover to open in a bench run.
if has popover; then
  for i in $(seq "$reps"); do
    RUNE_SHELL_POPOVER=1 rune "popover-40-rune-$i" fixtures/static-40.canvas --bench slow-pan \
      --bench-duration-ms "$PAN_MS" --warmup-ms "$WARMUP"
  done
fi

# 6. GPU time of the canvas pass, 4x against 1x, at 20 pages, in the spike
# only: its own wait on the GPU after our submit (RUNE_SHELL_GPU_TIME). The
# Kit has no hook for it without a crate change. The same compositor draws
# both shells' canvas, so the multisampling cost is not a shell property.
if has gpu; then
  for i in $(seq "$reps"); do
    for samples in 4 1; do
      RUNE_SHELL_GPU_TIME=1 rune "gpu-$samples-rune-$i" fixtures/static-20.canvas \
        --samples "$samples" --bench slow-pan --bench-duration-ms "$PAN_MS" --warmup-ms "$WARMUP"
    done
  done
fi

# 4. Input to present: enter the colour-flip page and press a key 40 times,
# 250 ms apart. Both shells time it as the Kit's probe does (forwarded input,
# the page's next paint, the frame that drew it). The Kit's pages are 60 fps
# (a crate constant); the spike runs 60 and 120.
if has latency; then
  (cd "$spike" && cargo build --release --features cef,tooling \
    && CARGO_TARGET_DIR="$spike/target" "$native/crates/specular-cef/scripts/bundle-macos.sh" release rune-shell)
  for i in $(seq "$reps"); do
    space="$(mktemp -d)"
    cp "$spike/fixtures/flip.canvas" "$spike/fixtures/flip.html" "$space/"
    SPECULAR_NATIVE_CONFIG_DIR="$space/config" SPECULAR_FLOAT_WINDOW=1 \
    SPECULAR_SHELL_SCRIPT_FILE="$spike/scripts/s6/flip.txt" \
      $RUN "$out/flip-60-kit-$i.json" -- "$KIT" --window "$WINDOW" "$space/flip.canvas" || true
    rm -rf "$space"
    for fps in 60 120; do
      $RUN "$out/flip-$fps-rune-$i.json" -- "$RUNE" --source cef --size "$WINDOW" \
        "$spike/fixtures/flip.canvas" --camera 100,100,1 --page-fps "$fps" --quit-after 14 \
        --window-script "$spike/scripts/s6/flip.json" || true
    done
  done
  (cd "$spike" && cargo build --release --features cef \
    && CARGO_TARGET_DIR="$spike/target" "$native/crates/specular-cef/scripts/bundle-macos.sh" release rune-shell)
fi

ps -Ao pid,pcpu,pmem,comm -r > "$out/ps-after.txt"
python3 "$spike/tools/s6_table.py" "$out"
