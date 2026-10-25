#!/bin/sh
# Runs the scenario scripts and prints one pass or fail line each. Exits
# non-zero when any fails.
#
#   fixtures/scenarios/run.sh                  # every scenario, headless
#   fixtures/scenarios/run.sh first-session    # one, by name
#   fixtures/scenarios/run.sh --kit [name..]   # in the Kit window
#   fixtures/scenarios/run.sh --cef [name..]   # the real-page suite, cef/
#   fixtures/scenarios/run.sh --cli [case..]   # the CLI end to end, cli/
#
# headless  `specular --script` with the synthetic pages and no built-in
#           panels. Steps only a window can do are skipped and listed.
#           `cargo test --workspace` runs the same scenarios
#           (crates/specular-app/tests/scenarios.rs).
# --kit     the `specular` binary opens a window on a scratch copy of the
#           scenario's canvas and replays it with real events. It needs the
#           display: a covered window or a locked screen draws nothing, so
#           the window floats above the others. KIT_APP names another build
#           (a bundle's inner binary), KIT_ARGS adds arguments
#           (`--source cef`), KIT_WRAP wraps each run (`lockf /tmp/w.lock`).
# --cef     cef/*.txt on real pages, then cef/cli-pages.sh when the
#           `specular` CLI is installed. It needs a bundled debug CEF build
#           (see native/README.md); CEF_APP names its inner binary. `{site}`
#           in a script is the address cef/ is served at.
# --cli     cli/run.sh: every `specular` CLI verb against the Kit app on a
#           scratch space, one case a line (cli/cases.py).
#
# Output goes to native/runs/scenarios/<mode>/<scenario>/: what the run
# saved and drew, and stderr.log. SCENARIO_OUT moves it.
set -u
here=$(cd "$(dirname "$0")" && pwd)
native=$(cd "$here/../.." && pwd)
target=${CARGO_TARGET_DIR:-$native/target}

mode=headless
case ${1:-} in
    --kit) mode=kit; shift ;;
    --cef) mode=cef; shift ;;
    --cli) shift; OUT=${SCENARIO_OUT:-$native/runs/scenarios/cli} exec "$here/cli/run.sh" "$@" ;;
    --headless) shift ;;
    -*) echo "unknown mode $1 (expected --kit, --cef or --cli)" >&2; exit 2 ;;
esac
out=${SCENARIO_OUT:-$native/runs/scenarios/$mode}
folder=$here
[ "$mode" = cef ] && folder=$here/cef

case $mode in
    headless)
        (cd "$native" && cargo build -q -p specular-shell) || exit 1
        app=$target/debug/specular ;;
    kit)
        if [ -n "${KIT_APP:-}" ]; then
            app=$(cd "$(dirname "$KIT_APP")" && pwd)/$(basename "$KIT_APP")
        else
            (cd "$native" && cargo build -q -p specular-shell) || exit 1
            app=$target/debug/specular
        fi ;;
    cef)
        app=${CEF_APP:-$target/debug/specular.app/Contents/MacOS/specular}
        [ -x "$app" ] || {
            echo "missing $app; build with --features cef and bundle it (native/README.md)" >&2
            exit 2
        }
        port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
        (cd "$folder" && exec python3 -m http.server "$port" --bind 127.0.0.1 >/dev/null 2>&1) &
        server=$!
        trap 'kill "$server" 2>/dev/null || true' EXIT
        site=http://127.0.0.1:$port ;;
esac

[ $# -gt 0 ] || set -- $(cd "$folder" && ls *.txt | sed 's/\.txt$//')

# The canvas a script names on its first line, or the empty canvas.
canvas_of() {
    named=$(sed -n '1s/^# canvas: *//p' "$1")
    if [ -n "$named" ]; then
        echo "$(dirname "$1")/$named"
    else
        echo "$here/empty.canvas"
    fi
}

passed=0
failed=""
for name in "$@"; do
    script=$folder/$name.txt
    run=$out/$name
    rm -rf "$run"
    mkdir -p "$run"
    log=$run/stderr.log
    case $mode in
        headless)
            (cd "$run" && "$app" --script "$script" --script-panels off 2>"$log") ;;
        cef)
            sed "s#{site}#$site#g" "$script" > "$run/$name.txt"
            canvas=$(canvas_of "$script")
            (cd "$run" && "$app" --source cef --snapshot-size 1400x800 --script-panels off \
                --script "$run/$name.txt" "$canvas" 2>"$log") ;;
        kit)
            # The window saves into the canvas it opens and keeps its state
            # beside it, so it opens a copy, with the files the canvas names.
            canvas=$(canvas_of "$script")
            space=$run/space
            mkdir -p "$space" "$run/config"
            cp "$canvas" "$space/"
            for beside in "$(dirname "$canvas")"/*.md "$(dirname "$canvas")"/assets; do
                [ -e "$beside" ] && cp -R "$beside" "$space/"
            done
            (cd "$run" && SPECULAR_SHELL_SCRIPT_FILE="$script" \
                SPECULAR_FLOAT_WINDOW=1 \
                SPECULAR_NATIVE_CONFIG_DIR="$run/config" \
                SPECULAR_PORT=0 SPECULAR_DISCOVERY_FILE="$run/discovery.json" \
                ${KIT_WRAP:-} perl -e 'alarm 300; exec @ARGV' \
                "$app" ${KIT_ARGS:-} "$space/$(basename "$canvas")" 2>"$log") ;;
    esac
    status=$?
    if [ $status -eq 0 ]; then
        passed=$((passed + 1))
        skipped=$(sed -n 's/^skipped \([0-9]*\) window-only steps:$/\1/p' "$log")
        echo "pass $name${skipped:+ (skipped $skipped window steps)}"
    else
        failed="$failed $name"
        reason=$(grep -E "^Error|^specular:|panicked" -A4 "$log" | grep -v "^$" | head -6)
        [ -n "$reason" ] || reason=$(tail -3 "$log")
        echo "FAIL $name"
        echo "$reason" | sed 's/^/    /'
    fi
done

if [ "$mode" = cef ] && command -v specular >/dev/null; then
    if "$here/cef/cli-pages.sh" "$app" "$out/cli-pages" >"$out/cli-pages.log" 2>&1; then
        passed=$((passed + 1))
        echo "pass cli-pages"
    else
        failed="$failed cli-pages"
        echo "FAIL cli-pages (see $out/cli-pages.log)"
    fi
fi

echo "$passed passed${failed:+, failed:$failed}"
[ -z "$failed" ]
