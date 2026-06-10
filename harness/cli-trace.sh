#!/usr/bin/env bash
#
# Faithful CLI tracer for the discovery phase (general purpose — knows no domain).
#
# Runs the real specular CLI against whatever app the canonical discovery file
# points at (the live app — this wrapper never overrides SPECULAR_DISCOVERY_FILE),
# appends one JSON record per call to $SPECULAR_TRACE_FILE, then passes stdout,
# stderr, and the exit code straight through unchanged.
#
# The discovery judge reads ONLY this trace, so the doer must route every CLI call
# through this wrapper. The record is mechanical (args array, exit code, raw
# stdout/stderr, duration) — not the agent's self-report — which is what makes the
# judge's grade independent of the doer.
#
# Usage:  SPECULAR_TRACE_FILE=/path/to/trace.jsonl harness/cli-trace.sh <verb> [args...]
set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
CLI="$REPO_ROOT/out/main/cli.js"
TRACE="${SPECULAR_TRACE_FILE:-$REPO_ROOT/harness/.traces/trace.jsonl}"
mkdir -p "$(dirname "$TRACE")"

out="$(mktemp)"
err="$(mktemp)"
start="$(python3 -c 'import time;print(int(time.time()*1000))' 2>/dev/null || echo 0)"
node "$CLI" "$@" >"$out" 2>"$err"
code=$?
end="$(python3 -c 'import time;print(int(time.time()*1000))' 2>/dev/null || echo 0)"

# Append a JSONL record. python3 handles escaping and keeps args as a real array.
python3 - "$code" "$start" "$end" "$out" "$err" "$@" >>"$TRACE" 2>/dev/null <<'PY' || true
import json, sys
code, start, end, outf, errf, *args = sys.argv[1:]
rec = {
    "args": args,
    "exit": int(code),
    "ms": int(end) - int(start),
    "stdout": open(outf, encoding="utf-8", errors="replace").read(),
    "stderr": open(errf, encoding="utf-8", errors="replace").read(),
}
print(json.dumps(rec))
PY

# Pass real output through so the doer can react like a normal CLI user.
cat "$out"
cat "$err" >&2
rm -f "$out" "$err"
exit "$code"
