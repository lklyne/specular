#!/bin/sh
# Runs the scenario scripts in the GPUI Kit window (the `specular` binary)
# and checks the canvases they save with check.py, as run.sh does for the
# headless runner. Each scenario opens a window on a scratch copy of its
# canvas, replays the script with real pointer and key events and quits.
#
#   fixtures/scenarios/run-kit.sh                    # every scenario
#   fixtures/scenarios/run-kit.sh j-toolbar-and-popups
#   KIT_WRAP="lockf /tmp/w.lock" fixtures/scenarios/run-kit.sh   # a wrapper per window
#
# Output goes to native/runs/qa-kit/<scenario>/ (QA_OUT moves it). A
# scenario that fails prints the step that failed and the rest still run;
# only the ones that finished are checked. The run needs the display: a
# covered window draws nothing, so the window floats above the others.
set -u
here=$(cd "$(dirname "$0")" && pwd)
native=$(cd "$here/../.." && pwd)
out=${QA_OUT:-$native/runs/qa-kit}

(cd "$native" && cargo build -q -p specular-shell) || exit 1
app=${CARGO_TARGET_DIR:-$native/target}/debug/specular

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
    # The window saves into the canvas it opens and keeps its state beside
    # it, so it opens a copy, with the files the canvas names.
    space=$out/$name/space
    mkdir -p "$space" "$out/$name/config"
    from=$(dirname "$canvas")
    cp "$canvas" "$space/"
    for beside in "$from"/*.md "$from"/assets; do
        [ -e "$beside" ] && cp -R "$beside" "$space/"
    done
    echo "== $name"
    if (cd "$out/$name" && SPECULAR_SHELL_SCRIPT_FILE="$script" \
        SPECULAR_FLOAT_WINDOW=1 \
        SPECULAR_NATIVE_CONFIG_DIR="$out/$name/config" \
        SPECULAR_PORT=0 SPECULAR_DISCOVERY_FILE="$out/$name/discovery.json" \
        ${KIT_WRAP:-} perl -e 'alarm 300; exec @ARGV' \
        "$app" "$space/$(basename "$canvas")" 2>"$out/$name/stderr.log"); then
        passed="$passed $name"
    else
        failed="$failed $name"
        grep -E "^specular:|panicked" "$out/$name/stderr.log" || tail -3 "$out/$name/stderr.log"
    fi
    grep -E "WARN|ERROR" "$out/$name/stderr.log" | grep -v "synthetic CPU frames" || true
done
status=0
[ -z "$passed" ] || python3 "$here/check.py" "$out" $passed || status=1
[ -z "$failed" ] || { echo "did not finish:$failed"; status=1; }
exit $status
