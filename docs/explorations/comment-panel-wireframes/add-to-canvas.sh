#!/usr/bin/env bash
# Place the comment-panel wireframe docs on the live Specular canvas.
# Run from anywhere with the Specular app open: bash add-to-canvas.sh
# Uses `specular find-placement` implicitly by anchoring the grid at --at
# coordinates; adjust ORIGIN_X/Y to taste.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

ORIGIN_X="${ORIGIN_X:-0}"
ORIGIN_Y="${ORIGIN_Y:-0}"
W=1240
H=860
GAP=100

add() { # file, col, row, width, height
  local file="$1" col="$2" row="$3" w="$4" h="$5"
  local x=$((ORIGIN_X + col * (W + GAP)))
  local y=$((ORIGIN_Y + row * (H + GAP)))
  local out id
  out=$(specular add file "$DIR/$file" --at "$x,$y")
  echo "$out"
  id=$(echo "$out" | grep -oE 'file_[A-Za-z0-9_-]+' | head -1 || true)
  if [ -n "$id" ]; then
    specular update "$id" --size "$w,$h" >/dev/null
  fi
}

# Row 0: the two structural decisions
add 01-panel-architecture.html    0 0 $W $H
add 02-list-thread-navigation.html 1 0 $W $H

# Row 1: the chat interface itself
add 03-thread-anatomy.html        0 1 780 $H
add 04-agent-run-states.html      1 1 1500 720

# Row 2: interaction details
add 05-composer-variants.html     0 2 $W 640
add 06-canvas-choreography.html   1 2 1400 620

specular add note "Comment panel wireframes — read 01 and 02 first (structural), then 03-06 (chat detail). Amber boxes are the decisions to react to; annotate directly on these docs." \
  --at "$((ORIGIN_X - 260)),$ORIGIN_Y" --color 3

echo "Done. 6 wireframe docs + overview note placed."
