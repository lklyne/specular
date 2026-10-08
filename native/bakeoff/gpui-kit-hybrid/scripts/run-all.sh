#!/usr/bin/env bash
# Runs every scenario into out/ and copies the evidence the ADR cites into
# shots/. Each run opens one window for a few seconds and closes it.
#
#   cargo build && scripts/run-all.sh
#   CEF_PATH=~/.local/share/cef CARGO_TARGET_DIR=target/cef cargo build --features cef \
#     && CEF_PATH=~/.local/share/cef scripts/bundle.sh && scripts/run-all.sh
set -uo pipefail
cd "$(dirname "$0")/.."
bin=target/debug/gpui-kit-hybrid
cef=target/cef/debug/gpui-kit-hybrid.app/Contents/MacOS/gpui-kit-hybrid
rm -rf out && mkdir -p out shots
caffeinate -d -u -t 600 &
awake=$!

run() { # binary args..
  local binary="$1"; shift
  "$binary" --out "$PWD/out" "$@" >/dev/null 2>&1 &
  local pid=$!
  ( sleep 90; kill "$pid" 2>/dev/null ) &
  local guard=$!
  wait "$pid"; kill "$guard" 2>/dev/null
}

for layer in below above above-through; do run $bin --layer $layer --scenario overlays; done
for layer in below above; do run $bin --layer $layer --scenario input; done
for layer in below above; do for drive in link gpui; do for anim in off on; do
  run $bin --layer $layer --drive $drive --gpui-anim $anim --scenario pacing --tag pacing-$layer-$drive-anim$anim
done; done; done
run $bin --layer below --scenario resize
run $bin --layer above --scenario resize --tag resize-above-grab-on-divider
SPIKE_GRAB_GPUI_SIDE=1 run $bin --layer above --scenario resize
SPIKE_GRAB_GPUI_SIDE=1 run $bin --layer above --drive gpui --scenario resize --tag resize-above-gpuidrive
run $bin --layer below --scenario timer
run $bin --layer below --scenario model
if [[ -x $cef ]]; then
  page="$PWD/fixtures/cef.canvas"
  run $cef --layer below --source cef --canvas "$page" --scenario cef --tag cef-below
  run $cef --layer below --source cef --canvas "$page" --scenario cef --pump gpui --tag cef-below-gpuipump
  run $cef --layer below --source cef --canvas "$page" --scenario pacing --gpui-anim on --tag cef-below-animon
  run $cef --layer below --source cef --canvas "$page" --scenario pacing --drive gpui --gpui-anim on --tag cef-below-gpuidrive-animon
fi
kill $awake 2>/dev/null

for tag in resize-below resize-above resize-above-gpuidrive; do
  python3 -I scripts/lockstep-scan.py out/$tag-drag-*.png out/$tag-window-*.png > out/$tag-scan.txt
done
cp out/*.txt shots/
for name in overlays-below-dropdown overlays-below-popover overlays-below-context-menu \
  overlays-below-dialog overlays-above-dropdown overlays-above-dialog \
  overlays-above-through-context-menu model-below-tool-armed model-below-row-selected \
  resize-below-drag-21 resize-above-drag-21 resize-above-drag-63 \
  cef-below-composing cef-below-typed; do
  [[ -f out/$name.png ]] && cp out/$name.png shots/
done
ls shots | wc -l
