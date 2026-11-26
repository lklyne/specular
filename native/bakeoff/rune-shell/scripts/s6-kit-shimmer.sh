#!/usr/bin/env bash
# S6 case 2 on the Kit: a pan with an agent run's bar animating in the chat
# panel, against the same pan with the panel open and no run. A Kit bench
# run has agent runs off, so this is a scripted window run instead
# (SPECULAR_SHELL_SCRIPT_FILE, the `pan` step: a scroll every refresh), read
# from the ledger. Usage: s6-kit-shimmer.sh OUT_DIR
set -euo pipefail
out="$1"
reps="${REPS:-3}"
spike="$(cd "$(dirname "$0")/.." && pwd)"
native="$(cd "$spike/../.." && pwd)"
KIT="$native/target/release/specular.app/Contents/MacOS/specular"
for i in $(seq "$reps"); do
  for mode in shimmer noshimmer; do
    space="$(mktemp -d)"
    cp "$native/fixtures/static-20.canvas" "$space/"
    if [[ "$mode" == shimmer ]]; then
      sed 's/^@S@//' "$spike/scripts/s6/kit-pan.txt" > "$space/pan.txt"
    else
      sed '/^@S@/d' "$spike/scripts/s6/kit-pan.txt" > "$space/pan.txt"
    fi
    SPECULAR_NATIVE_CONFIG_DIR="$space/config" \
    SPECULAR_AGENT_SCRIPT="$spike/scripts/s6/agent-long.txt" \
    SPECULAR_SHELL_SCRIPT_FILE="$space/pan.txt" SPECULAR_FLOAT_WINDOW=1 \
      python3 "$spike/tools/s6_run.py" "$out/kitpan-$mode-kit-$i.json" -- \
        "$KIT" --window 1600x1000 --paint-policy full-rate "$space/static-20.canvas" || true
    rm -rf "$space"
  done
done
