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
#
#   PANELS=off fixtures/scenarios/run.sh
# runs them with the built-in toolbar, popup and sidebar off, so a
# `control NAME` step does what the control does from the models alone, into
# native/runs/qa-panels-off/. A scenario that fails prints the step that
# failed and the rest still run; only the ones that finished are checked.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
native=$(cd "$here/../.." && pwd)
panels=${PANELS:-on}
case $panels in
    on) default_out=$native/runs/qa ;;
    off) default_out=$native/runs/qa-panels-off ;;
    *) echo "PANELS is on or off, not $panels" >&2; exit 2 ;;
esac
out=${QA_OUT:-$default_out}

(cd "$native" && cargo build -q -p specular-app)
app=${CARGO_TARGET_DIR:-$native/target}/debug/specular-app

[ $# -gt 0 ] || set -- $(cd "$here" && ls *.txt | sed 's/\.txt$//')
passed=""
failed=""
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
    if (cd "$out/$name" && "$app" --script "$script" --script-panels "$panels" "$canvas" \
        2>"$out/$name/stderr.log"); then
        passed="$passed $name"
    elif [ "$panels" = on ]; then
        cat "$out/$name/stderr.log"
        exit 1
    else
        failed="$failed $name"
        grep -E "^Error|panicked" -A3 "$out/$name/stderr.log" || tail -3 "$out/$name/stderr.log"
    fi
    grep -E "WARN|ERROR|panicked" "$out/$name/stderr.log" || true
done
status=0
[ -z "$passed" ] || python3 "$here/check.py" "$out" $passed || status=1
[ -z "$failed" ] || { echo "did not finish:$failed"; status=1; }
exit $status
