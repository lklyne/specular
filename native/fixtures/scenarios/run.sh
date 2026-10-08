#!/bin/sh
# Runs the scenario scripts headless and leaves their PNGs (and any saved
# .canvas) in native/runs/qa/<scenario>/. With no arguments, runs them all.
#
#   fixtures/scenarios/run.sh            # every scenario
#   fixtures/scenarios/run.sh a-first-session d-text-edge-cases
#
# A script's first line names its canvas, relative to this folder, or with
# `runs:` relative to the output folder (a canvas an earlier scenario saved):
#   # canvas: ../kitchen-sink.canvas
#   # canvas: runs:a-first-session/08-arranged.canvas
# check.py then compares the .canvas files the scripts saved.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
native=$(cd "$here/../.." && pwd)
out=${QA_OUT:-$native/runs/qa}

(cd "$native" && cargo build -q -p specular-app)
app=${CARGO_TARGET_DIR:-$native/target}/debug/specular-app

[ $# -gt 0 ] || set -- $(cd "$here" && ls *.txt | sed 's/\.txt$//')
for name in "$@"; do
    script=$here/$name.txt
    canvas=$(sed -n '1s/^# canvas: *//p' "$script")
    case $canvas in
        runs:*) canvas=$out/${canvas#runs:} ;;
        *) canvas=$here/$canvas ;;
    esac
    rm -rf "$out/$name"
    mkdir -p "$out/$name"
    echo "== $name"
    (cd "$out/$name" && "$app" --script "$script" "$canvas" 2>"$out/$name/stderr.log") || {
        cat "$out/$name/stderr.log"
        exit 1
    }
    grep -E "WARN|ERROR|panicked" "$out/$name/stderr.log" || true
done
python3 "$here/check.py" "$out" "$@"
