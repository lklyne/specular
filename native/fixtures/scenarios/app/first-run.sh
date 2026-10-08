#!/usr/bin/env bash
# The first run of the bundled app, end to end, on a throwaway HOME: the
# first-run view, a space made in a scratch folder, a relaunch that reopens
# it, a login that survives the quit, and the prompt when the space is gone.
# Nothing under the real home folder is read or written.
#
# It checks the files and the logs itself and exits non-zero on the first
# thing that is wrong. The PNGs are for reading. With the screen locked or
# the display asleep the window is not drawn, so the captures fail and the
# script says so; the checks still run.
#
# Usage: first-run.sh [app] [out-dir]
#   app      the bundle (default target/release/bundle/Specular Native.app;
#            build it with crates/specular-shell/scripts/bundle-app.sh)
#   out-dir  where the scratch home, the space and the PNGs go
#            (default runs/qa/app-first-run; emptied first)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
native="$(cd "$here/../../.." && pwd)"
app="${1:-$native/target/release/bundle/Specular Native.app}"
out="${2:-$native/runs/qa/app-first-run}"
bin="$app/Contents/MacOS/Specular Native"
[[ -x "$bin" ]] || { echo "missing $bin; run crates/specular-shell/scripts/bundle-app.sh" >&2; exit 2; }

rm -rf "$out"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
home="$out/home"
data="$home/Library/Application Support/Specular Native"
electron="$home/Library/Application Support/Specular"
space="$out/My Space"
mkdir -p "$electron" "$out/electron-space"
printf '{"spacePath":"%s"}\n' "$out/electron-space" > "$electron/preferences.json"
electron_before="$(shasum "$electron/preferences.json")"

fail() { echo "FAIL: $*" >&2; exit 1; }

# A site with a login: the first visit sets a cookie, and every visit logs
# the cookie it came with.
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
python3 - "$port" "$out/visits.log" <<'PY' &
import http.server, sys
port, log = int(sys.argv[1]), sys.argv[2]
class Site(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/":
            with open(log, "a") as visits:
                visits.write((self.headers.get("Cookie") or "none") + "\n")
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Set-Cookie", "session=kept; Max-Age=86400; Path=/")
        self.end_headers()
        self.wfile.write(b"<h1>Signed in</h1>")
    def log_message(self, *_):
        pass
http.server.HTTPServer(("127.0.0.1", port), Site).serve_forever()
PY
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT

launch() { # name script [pick]
  HOME="$home" SPECULAR_FLOAT_WINDOW=1 SPECULAR_SPACE_PICK="${3:-}" \
    SPECULAR_SHELL_SCRIPT="$2" "$bin" --window 1200x800 > "$out/$1.log" 2>&1 \
    || fail "$1 exited with an error; see $out/$1.log"
}

# 1. No space chosen: the first-run view, then a space made in a scratch
#    folder, then the settings dialog over it.
launch 1-first-run "wait 3000; shot $out/1-first-run.png; choose onboarding.create; \
wait 3000; shot $out/2-space-created.png; cmd-key 43 ,; wait 1200; shot $out/3-settings.png; quit" "$space"
grep -q "no space is chosen yet" "$out/1-first-run.log" || fail "the first launch did not ask for a space"
[[ -f "$space/Welcome.canvas" && -f "$space/Welcome.md" ]] || fail "the starter space was not copied into $space"
grep -q "__SPECULAR_SPACE__" "$space/Welcome.canvas" && fail "the starter canvas kept its placeholder"
grep -qF "$space/Welcome.md" "$space/Welcome.canvas" || fail "the starter canvas does not name the note in the space"
python3 - "$data/preferences.json" "$space" <<'PY' || fail "the space was not remembered in this app's preferences"
import json, sys
sys.exit(0 if json.load(open(sys.argv[1])).get("spacePath") == sys.argv[2] else 1)
PY
[[ "$(shasum "$electron/preferences.json")" == "$electron_before" ]] || fail "the Electron app's preferences changed"
[[ -z "$(ls -A "$out/electron-space")" ]] || fail "the Electron app's space was written to"
[[ -d "$data/cef-profile" ]] || fail "no page profile in the data folder"

# 2. A page with a login goes into the space while the app is closed.
python3 - "$space/Welcome.canvas" "http://127.0.0.1:$port/" <<'PY'
import json, sys
path, url = sys.argv[1], sys.argv[2]
canvas = json.load(open(path))
canvas["nodes"].append({"id": "p-login", "type": "link", "url": url,
                        "x": -900, "y": 0, "width": 600, "height": 400})
json.dump(canvas, open(path, "w"), indent=2)
PY

# 3. Relaunch: the same space opens with no question, and the page signs in.
launch 2-relaunch "wait 5000; shot $out/4-relaunch.png; quit"
grep -q "no space is chosen yet" "$out/2-relaunch.log" && fail "the relaunch asked for a space again"
grep -qF "opened space folder=$space" "$out/2-relaunch.log" || fail "the relaunch did not reopen $space"

# 4. Again: the cookie set in the last run comes back, so the login was kept.
launch 3-login-kept "wait 5000; shot $out/5-login-kept.png; quit"
[[ "$(sed -n 1p "$out/visits.log")" == "none" ]] || fail "the first visit already had a cookie"
tail -n 1 "$out/visits.log" | grep -q "session=kept" || fail "the login did not survive the quit: $(cat "$out/visits.log" | tr '\n' ' ')"

# 5. The space goes missing: the app names it, opens nothing and makes nothing.
mv "$space" "$out/My Space (moved)"
launch 4-missing "wait 3000; shot $out/6-missing.png; choose onboarding.quit"
grep -q "the chosen space folder is not there" "$out/4-missing.log" || fail "the missing space was not noticed"
[[ ! -e "$space" ]] || fail "a folder was made where the space went missing"

shots="$(ls "$out"/*.png 2>/dev/null | wc -l | tr -d ' ')"
echo "ok: first run, relaunch, kept login and missing space all check out"
if [[ "$shots" -lt 6 ]]; then
  echo "note: $shots of 6 PNGs were captured; the window is not drawn while the screen is locked or the display is asleep"
else
  echo "read the PNGs in $out"
fi
