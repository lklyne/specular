#!/usr/bin/env bash
# The `specular` CLI end to end: every verb's happy path and its main
# errors, against a running app on a scratch space, asserting on what each
# verb prints, its exit code, the canvas the app reads back and the .canvas
# file it saves. The cases are in cases.py.
#
# It starts its own app with a discovery file, a data folder and a port of
# its own, so it never touches the user's space or a Specular that is
# already running, and it stops only what it started. Pages are synthetic;
# the browse verbs against real pages are fixtures/scenarios/cef/cli-pages.sh.
#
# Usage: run.sh [case ...]
#   APP=path   the app binary (default: build and run target/debug/specular)
#   CLI=path   the CLI binary (default: build target/debug/specular-cli)
#   OUT=dir    where app.log and the transcript go (default runs/cli)
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
native="$(cd "$here/../../.." && pwd)"
target="${CARGO_TARGET_DIR:-$native/target}/debug"
out="${OUT:-$native/runs/cli}"
if [[ -z "${APP:-}" || -z "${CLI:-}" ]]; then
  (cd "$native" && cargo build -q -p specular-cli -p specular-shell) || exit 2
fi
app="${APP:-$target/specular}"
cli="${CLI:-$target/specular-cli}"
[[ -x "$app" ]] || { echo "missing app $app" >&2; exit 2; }
[[ -x "$cli" ]] || { echo "missing CLI $cli" >&2; exit 2; }

work="$(mktemp -d)"
mkdir -p "$out" "$work/space" "$work/config"
out="$(cd "$out" && pwd)"
pid=""
cleanup() {
  [[ -n "$pid" ]] && kill "$pid" 2>/dev/null && wait "$pid" 2>/dev/null
  rm -rf "$work"
}
trap cleanup EXIT

echo '{"nodes":[],"edges":[]}' > "$work/space/Home.canvas"
export SPECULAR_DISCOVERY_FILE="$work/discovery.json"
# The agent-browser the browse verbs would run is never reached here.
export SPECULAR_SESSION_ID="cli-e2e"
SPECULAR_NATIVE_CONFIG_DIR="$work/config" SPECULAR_PORT=0 RUST_LOG=info \
  "$app" "$work/space/Home.canvas" > "$out/app.log" 2>&1 &
pid=$!
for _ in $(seq 1 150); do [[ -s "$SPECULAR_DISCOVERY_FILE" ]] && break; sleep 0.2; done
[[ -s "$SPECULAR_DISCOVERY_FILE" ]] || { echo "the app wrote no discovery file; see $out/app.log" >&2; exit 2; }

python3 "$here/cases.py" "$cli" "$work/space" "$@" | tee "$out/transcript.txt"
status=${PIPESTATUS[0]}
grep -E "panicked" "$out/app.log" && status=1
exit "$status"
