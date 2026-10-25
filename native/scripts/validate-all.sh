#!/usr/bin/env bash
# Every check the app has, in one pass, ending in one pass or fail table.
#
#   native/scripts/validate-all.sh                 # everything
#   native/scripts/validate-all.sh --skip release  # all but the named steps
#   native/scripts/validate-all.sh --only kit cef  # just these
#
# The steps, in order:
#   gate      cargo fmt --check, clippy -D warnings, cargo test --workspace
#   headless  every headless scenario (fixtures/scenarios/run.sh)
#   kit       every scenario in the Kit window (run.sh --kit)
#   cef       the real-page suite on a bundled debug CEF build, including the
#             CLI's browse verbs (run.sh --cef)
#   cli       the CLI end to end against a running app (run.sh --cli)
#   release   a release Specular.app, then its first run on a scratch home
#             (crates/specular-shell/scripts/bundle-app.sh, app/first-run.sh)
#
# A failing step does not stop the ones after it. Logs, what each run saved
# and its screenshots go to native/runs/validate/<step>/. The kit, cef and
# release steps open windows and need the display awake and unlocked: a
# locked screen draws nothing, and the table marks those steps when it was.
# CEF_PATH defaults to ~/.local/share/cef, where the first build downloads
# CEF. Nothing here reads or writes the real space or preferences.
set -uo pipefail

native="$(cd "$(dirname "$0")/.." && pwd)"
out="$native/runs/validate"
export CEF_PATH="${CEF_PATH:-$HOME/.local/share/cef}"
target="${CARGO_TARGET_DIR:-$native/target}"

all=(gate headless kit cef cli release)
steps=("${all[@]}")
case "${1:-}" in
    --only) shift; steps=("$@") ;;
    --skip) shift; steps=(); for s in "${all[@]}"; do [[ " $* " == *" $s "* ]] || steps+=("$s"); done ;;
    "") ;;
    *) echo "usage: validate-all.sh [--only STEP..|--skip STEP..]; steps: ${all[*]}" >&2; exit 2 ;;
esac

rm -rf "$out"
mkdir -p "$out"

# The console session carries this key only while the screen is locked.
screen_locked() { ioreg -n Root -d1 | grep -q '"CGSSessionScreenIsLocked"=Yes'; }

# Runs one step's commands with their output in its log, and records the
# result and how long it took.
results=()
run_step() { # name, then a function that does the step
    local name=$1 fn=$2 dir="$out/$1" start rc note=""
    mkdir -p "$dir"
    echo "== $name"
    start=$(date +%s)
    ( cd "$native" && "$fn" "$dir" ) >"$dir/log.txt" 2>&1
    rc=$?
    case $name in kit | cef | release) screen_locked && note=" (screen was locked)" ;; esac
    local took=$(( $(date +%s) - start ))
    if [ $rc -eq 0 ]; then
        results+=("pass|$name|${took}s|$note")
    else
        results+=("FAIL|$name|${took}s|exit $rc, see runs/validate/$name/log.txt$note")
        tail -15 "$dir/log.txt" | sed 's/^/    /'
    fi
}

gate() {
    cargo fmt --all --check &&
        cargo clippy --workspace --all-targets -- -D warnings &&
        cargo test --workspace
}

headless() { SCENARIO_OUT="$1/runs" fixtures/scenarios/run.sh; }

kit() { SCENARIO_OUT="$1/runs" fixtures/scenarios/run.sh --kit; }

# The CEF bundle is the bare layout bundle-macos.sh makes. The CLI goes on
# the PATH as `specular` so run.sh also runs cef/cli-pages.sh.
cef() {
    local bin="$1/bin"
    cargo build -p specular-shell --features cef &&
        cargo build -p specular-cli &&
        crates/specular-cef/scripts/bundle-macos.sh debug specular || return 1
    mkdir -p "$bin" && ln -sf "$target/debug/specular-cli" "$bin/specular"
    PATH="$bin:$PATH" CLI="$target/debug/specular-cli" SCENARIO_OUT="$1/runs" \
        CEF_APP="$target/debug/specular.app/Contents/MacOS/specular" \
        fixtures/scenarios/run.sh --cef
}

cli() { SCENARIO_OUT="$1/runs" fixtures/scenarios/run.sh --cli; }

release() {
    crates/specular-shell/scripts/bundle-app.sh --profile release --out "$1/bundle" &&
        codesign --verify --deep --strict "$1/bundle/Specular.app" &&
        fixtures/scenarios/app/first-run.sh "$1/bundle/Specular.app" "$1/first-run"
}

for step in "${steps[@]}"; do
    case $step in
        gate | headless | kit | cef | cli | release) run_step "$step" "$step" ;;
        *) echo "unknown step $step (steps: ${all[*]})" >&2; exit 2 ;;
    esac
done

echo
printf '%-6s %-9s %7s  %s\n' result step time note
failed=0
for row in "${results[@]}"; do
    IFS='|' read -r res name took note <<<"$row"
    printf '%-6s %-9s %7s  %s\n' "$res" "$name" "$took" "$note"
    [ "$res" = pass ] || failed=1
done
printf '%s\n' "${results[@]}" >"$out/summary.txt"
echo
echo "logs: $out"
exit $failed
