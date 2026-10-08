#!/usr/bin/env bash
# The installed `specular` CLI driving two live pages in the Rust app: every
# browse verb, the page and region screenshots, and the two file verbs.
#
# It starts its own web server and its own app on a scratch space, with a
# discovery file and a port of its own, so it never touches the user's space
# or a Specular that is already running, and it stops only what it started.
#
# Usage: cli-pages.sh [app-binary] [out-dir]
#   app-binary  the inner binary of a debug CEF bundle
#               (default target/debug/specular-app.app/Contents/MacOS/specular-app)
#   out-dir     where transcript.txt and the PNGs go (default runs/a3-cli-pages)
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
native="$(cd "$here/../../.." && pwd)"
app="${1:-$native/target/debug/specular-app.app/Contents/MacOS/specular-app}"
out="${2:-$native/runs/a3-cli-pages}"
[[ -x "$app" ]] || { echo "missing $app; build with --features cef and bundle it" >&2; exit 2; }
command -v specular >/dev/null || { echo "the specular CLI is not installed" >&2; exit 2; }

work="$(mktemp -d)"
mkdir -p "$out" "$work/space" "$work/config" "$work/outside"
out="$(cd "$out" && pwd)"
log="$out/transcript.txt"
: > "$log"
pids=()
cleanup() {
  for pid in "${pids[@]:-}"; do [[ -n "$pid" ]] && kill "$pid" 2>/dev/null; done
  wait 2>/dev/null
  rm -rf "$work"
}
trap cleanup EXIT

port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
(cd "$here/site" && exec python3 -m http.server "$port" --bind 127.0.0.1 >/dev/null 2>&1) &
pids+=($!)
site="http://127.0.0.1:$port"
cat > "$work/space/Pages.canvas" <<JSON
{"nodes":[
{"id":"pa","type":"link","x":0,"y":0,"width":800,"height":500,"url":"$site/a.html"},
{"id":"pb","type":"link","x":900,"y":0,"width":800,"height":500,"url":"$site/b.html"}
],"edges":[]}
JSON
python3 - "$work/outside/dot.png" <<'PY'
import struct, sys, zlib
def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
width, height = 320, 180
rows = b"".join(b"\x00" + b"\x30\x80\xd0" * width for _ in range(height))
png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")
open(sys.argv[1], "wb").write(png)
PY

export SPECULAR_DISCOVERY_FILE="$work/discovery.json"
SPECULAR_NATIVE_CONFIG_DIR="$work/config" SPECULAR_PORT=0 RUST_LOG=info \
  "$app" --source cef "$work/space/Pages.canvas" > "$out/app.log" 2>&1 &
pids+=($!)
for _ in $(seq 1 100); do [[ -s "$SPECULAR_DISCOVERY_FILE" ]] && break; sleep 0.2; done
[[ -s "$SPECULAR_DISCOVERY_FILE" ]] || { echo "the app wrote no discovery file; see $out/app.log" >&2; exit 2; }
api="http://127.0.0.1:$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$SPECULAR_DISCOVERY_FILE")"
secret="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["secret"])' "$SPECULAR_DISCOVERY_FILE")"

passed=0
failed=()
# check <name> <text the output must hold> <command...>
check() {
  local name="$1" want="$2"; shift 2
  local output status
  output="$("$@" 2>&1)"; status=$?
  { echo "\$ ${*//$secret/SECRET}"; echo "$output" | cut -c1-400 | head -30; } >> "$log"
  if [[ $status -eq 0 && "$output" == *"$want"* ]]; then
    echo "PASS $name" | tee -a "$log"; passed=$((passed + 1))
  else
    echo "FAIL $name (exit $status, wanted: $want)" | tee -a "$log"; failed+=("$name")
  fi
  echo >> "$log"
}
post() { curl -s -X POST -H "x-specular-secret: $secret" -H 'content-type: application/json' "$api$1" -d "$2"; }
# Saves the PNG a JSON answer holds under `key` and prints its size.
png() {
  python3 -c '
import base64, json, struct, sys
body = json.load(sys.stdin)
for key in sys.argv[2].split("."):
    body = body.get(key) or {}
data = base64.b64decode(body) if isinstance(body, str) else b""
if data[:8] != b"\x89PNG\r\n\x1a\n":
    sys.exit("no PNG in the answer")
open(sys.argv[1], "wb").write(data)
print("png %dx%d" % struct.unpack(">II", data[16:24]))' "$1" "$2"
}

# Both pages must have loaded before anything is asked of them.
check "wait (pa loads)" "Page A" specular wait --text "Page A" -f pa
check "wait (pb loads)" "Page B" specular wait --text "Page B" -f pb

check "snapshot pa" 'button "Go A"' specular snapshot -i -f pa
check "snapshot pb" 'button "Go B"' specular snapshot -i -f pb
check "fill pb" "Done" specular fill "#name" bee -f pb
check "click pb" "Done" specular click "#go" -f pb
check "get text pb (the click landed on pb)" "clicked B: bee red" specular get text "#out" -f pb
check "type pa" "Done" specular type "#name" ay -f pa
check "select pa" "Done" specular select "#color" green -f pa
check "click pa by ref" "Done" specular click @e2 -f pa
check "get text pa (and pb's click did not reach pa)" "clicked A: ay green" specular get text "#out" -f pa
check "scroll pa" "Done" specular scroll down 300 -f pa
check "eval pa (scrolled)" "300" specular eval "window.scrollY" -f pa
check "eval pb (not scrolled)" "0" specular eval "window.scrollY" -f pb
check "find pa" "a2.html" specular find text "Next A" click -f pa
check "wait --url pa" "a2.html" specular wait --url "**/a2.html" -f pa
check "back pa" "a.html" specular back -f pa
check "forward pa" "a2.html" specular forward -f pa
check "reload pa" "a2.html" specular reload -f pa
check "wait --load pa" "Done" specular wait --load load -f pa
check "get title pa" "Page A two" specular get title -f pa
check "get url pb (never moved)" "b.html" specular get url -f pb
check "console pa" "" specular console -f pa
check "errors pa" "" specular errors -f pa
check "screenshot -f pb" ".png" specular screenshot -f pb
# The page entity followed the navigation the agent drove.
check "canvas (pa's url followed)" "a2.html" specular canvas

check "POST /pages/screenshot" "png 1600x1000" bash -c "$(declare -f post png); api=$api secret=$secret; post /pages/screenshot '{\"pageId\":\"pb\"}' | png '$out/page-pb.png' base64"
check "POST /pages/screenshot-composite" "png " bash -c "$(declare -f post png); api=$api secret=$secret; post /pages/screenshot-composite '{\"pageId\":\"pa\"}' | png '$out/page-pa-composite.png' base64"
made="$(post /annotations '{"text":"look here","anchor":{"type":"region","canvasRect":{"x":600,"y":-40,"width":500,"height":300}}}')"
annotation="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("id",""))' "$made")"
check "GET /annotations/<id> region screenshot" "png 1000x600" bash -c "curl -s -H 'x-specular-secret: $secret' '$api/annotations/$annotation' | { $(declare -f png); png '$out/region.png' metadata.regionScreenshot; }"
check "annotation <id>" "image" specular annotation "$annotation"

long="# Findings"$'\n\n'"A note with a heading is a Document, not a sticky."
check "add note (long)" "file_" specular add note "$long"
check "add note (the .md is in the space)" "A note with a heading" cat "$work/space/Findings.md"
check "add note (short stays a sticky)" "text_" specular add note "short"
check "add file (outside the space)" "file_" specular add file "$work/outside/dot.png"
check "add file (copied into assets, sized from the image)" '"width": 320' bash -c "ls '$work/space/assets/'*.png >/dev/null && specular canvas"

check "print-pdf is refused by name" "print-pdf" bash -c "specular print-pdf --page pa 2>&1; true"
check "record is refused by name" "record" bash -c "specular record start 2>&1; true"

echo "$passed passed, ${#failed[@]} failed" | tee -a "$log"
[[ ${#failed[@]} -eq 0 ]]
