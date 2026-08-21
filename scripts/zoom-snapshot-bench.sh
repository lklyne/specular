#!/usr/bin/env bash
# Times each link of the zoom-snapshot refresh pipeline in the running app.
# Usage: scripts/zoom-snapshot-bench.sh [repeats] [zoom]   (zoom omitted = fit all pages)
set -euo pipefail
DISC="${SPECULAR_DISCOVERY_FILE:-$HOME/.specular/specular-mcp.json}"
PORT=$(jq -r .port "$DISC"); SECRET=$(jq -r .secret "$DISC")
REPEATS="${1:-3}"
ZOOM_JSON=""; [[ -n "${2:-}" ]] && ZOOM_JSON=",\"zoom\":$2"
curl -s -X POST "http://localhost:$PORT/perf/zoom-snapshot/bench" \
  -H "x-specular-secret: $SECRET" -H 'content-type: application/json' \
  -d "{\"repeats\":$REPEATS,\"fit\":true$ZOOM_JSON}" | jq -r '
  def r: (. * 10 | round) / 10;
  def sum: reduce .[] as $x (0; . + $x);
  def max: reduce .[] as $x (0; if $x > . then $x else . end);
  .runs as $runs
  | "pages: \($runs[0].pageCount)  captured: \($runs[0].capturedCount)  px: \($runs[0].capturedPixels[0] // {} | "\(.width)x\(.height)")",
    "capture wall (ms, per run): \([$runs[].captureWallMs | r] | join("  "))",
    "capture per page (ms, run 1): \([$runs[0].captureMsPerPage[] | r] | join("  "))",
    "",
    "variant          encode-total  encode-max   bytes(MB)   ipc    decode   (medians over \($runs|length) runs)",
    ( [ $runs[0].variants[].variant ] | .[] as $v
      | [ $runs[] | .variants[] | select(.variant == $v) ] as $rs
      | def med(f): [ $rs[] | f ] | sort | .[ (length / 2 | floor) ];
        "\($v | . + "                " | .[0:16])" +
        " \(med(.encodeMsTotal) | r | tostring | . + "        " | .[0:12])" +
        " \(med(.encodeMsPerPage | max) | r | tostring | . + "        " | .[0:11])" +
        " \(med(.bytesTotal) / 1048576 | r | tostring | . + "        " | .[0:10])" +
        " \(med(.ipcMs // -1) | tostring | . + "      " | .[0:6])" +
        " \(med(.decodeMs // -1) | r)"
    )'
