#!/usr/bin/env bash
# Runs the resize scenarios against a bundled release build with real pages,
# three times each, with the resize ledger on, and prints the report table
# (docs/plans/page-resize-jank.md).
#
#   fixtures/scenarios/resize/run.sh LABEL [scenario ...]
#
# Ledgers go to native/runs/resize/LABEL/<scenario>-<n>.jsonl, each run's
# space and stderr to runs/resize/LABEL/<scenario>-<n>/. RESIZE_APP names
# another bundle's inner binary, RESIZE_RUNS another number of runs, and
# RESIZE_BENCH another specular-bench.
#
# The window floats above the others: a covered window gets no drawables.
# Numbers from a busy machine, a sleeping display or a debug build are not
# worth keeping; the load average is printed before and after.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
native="$(cd "$here/../../.." && pwd)"
label="${1:?usage: run.sh LABEL [scenario ...]}"
shift
app="${RESIZE_APP:-$native/target/release/specular.app/Contents/MacOS/specular}"
bench="${RESIZE_BENCH:-$native/target/release/specular-bench}"
runs="${RESIZE_RUNS:-3}"
out="$native/runs/resize/$label"
abs() { echo "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"; }
[[ -x "$app" ]] || { echo "missing $app; build with --features cef and bundle it" >&2; exit 2; }
[[ -x "$bench" ]] || { echo "missing $bench; cargo build --release -p specular-bench" >&2; exit 2; }
app="$(abs "$app")"
bench="$(abs "$bench")"
[[ $# -gt 0 ]] || set -- canvas-only fill-drag tab-switch

mkdir -p "$out"
echo "load before: $(uptime | sed 's/.*load averages*: //')"
ledgers=()
for name in "$@"; do
    script="$here/$name.txt"
    for n in $(seq 1 "$runs"); do
        run="$out/$name-$n"
        rm -rf "$run" "$run.jsonl"
        mkdir -p "$run/space" "$run/config"
        # The window saves into the canvas it opens, so it opens a copy.
        if grep -q '^# page$' "$script"; then
            cat > "$run/space/resize.canvas" <<JSON
{"nodes":[
{"id":"probe","type":"link","x":0,"y":0,"width":800,"height":500,"url":"file://$native/fixtures/probe/resize.html"}
],"edges":[]}
JSON
        else
            echo '{"nodes":[],"edges":[]}' > "$run/space/resize.canvas"
        fi
        echo "== $name-$n"
        (cd "$run" && SPECULAR_SHELL_SCRIPT_FILE="$script" \
            SPECULAR_FLOAT_WINDOW=1 \
            SPECULAR_RESIZE_TRACE="$run.jsonl" \
            SPECULAR_NATIVE_CONFIG_DIR="$run/config" \
            SPECULAR_PORT=0 SPECULAR_DISCOVERY_FILE="$run/discovery.json" \
            perl -e 'alarm 120; exec @ARGV' \
            "$app" --source cef --window 1400x900 "$run/space/resize.canvas" \
            2>"$run/stderr.log") || {
            echo "did not finish: $name-$n"
            grep -E "^specular:|panicked" "$run/stderr.log" || tail -3 "$run/stderr.log"
        }
        if [[ -s "$run.jsonl" ]]; then ledgers+=("$run.jsonl"); fi
    done
done
echo "load after: $(uptime | sed 's/.*load averages*: //')"
[[ ${#ledgers[@]} -gt 0 ]] || { echo "no ledgers were written" >&2; exit 1; }
"$bench" resize-report "${ledgers[@]}" | tee "$out/report.txt"
