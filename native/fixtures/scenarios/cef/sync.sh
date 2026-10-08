#!/usr/bin/env bash
# A sync set on real pages: two widths of one site, with the wide page
# scrolled, clicked and navigated, and the narrow one following. Read the
# PNGs it writes.
#
# It starts its own web server, since interaction sync skips pages with no
# origin (`file:`, `data:`), and stops only what it started.
#
# Usage: sync.sh [app-binary] [out-dir]
#   app-binary  the inner binary of a debug CEF bundle
#               (default target/debug/specular-app.app/Contents/MacOS/specular-app)
#   out-dir     where the PNGs go (default runs/qa/cef-sync)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
native="$(cd "$here/../../.." && pwd)"
app="${1:-$native/target/debug/specular-app.app/Contents/MacOS/specular-app}"
out="${2:-$native/runs/qa/cef-sync}"
[[ -x "$app" ]] || { echo "missing $app; build with --features cef and bundle it" >&2; exit 2; }

mkdir -p "$out"
out="$(cd "$out" && pwd)"
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
(cd "$here/sync-site" && exec python3 -m http.server "$port" --bind 127.0.0.1 >/dev/null 2>&1) &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
site="http://127.0.0.1:$port"
cat > "$out/sync.canvas" <<JSON
{"nodes":[
{"id":"p-wide","type":"link","x":0,"y":0,"width":800,"height":500,"url":"$site/index.html","syncId":"sync_breakpoints"},
{"id":"p-narrow","type":"link","x":900,"y":0,"width":400,"height":500,"url":"$site/index.html","syncId":"sync_breakpoints"}
],"edges":[]}
JSON
cd "$out"
"$app" --source cef --snapshot-size 1400x800 --snapshot-camera 40,80,0.75 \
  --script "$here/sync.txt" "$out/sync.canvas"
