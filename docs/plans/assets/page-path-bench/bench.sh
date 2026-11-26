#!/usr/bin/env bash
# The page-path cases of the post-Rune follow-ups (phase 3), on two release
# CEF bundles of the Kit, alternated a run at a time so a change in load
# falls on both:
#   flip      S6's flip page under 40 keys: key to present, paint to present
#   entered   an animated page entered: the rate it paints at
#   occluded  four animated pages, the window minimised half way (default
#             paint LOD, then full-rate, where pages are not graded)
#   idle      20 still local pages (full-rate, as the baseline ran it)
#   pan-N     S6's slow pan over N live sites (full-rate)
#
#   ARCHIVE=../telescope-rune-archive bench.sh OUT_DIR BEFORE.app AFTER.app [CASES..]
#
# ARCHIVE is a worktree of the tag archive/rune-spike (S6's runner, tables,
# flip fixture and still-20). Each run waits for the one-minute load to
# drop under LOAD_MAX (6) for up to five minutes and logs `uptime`.
set -euo pipefail
out="${1:?usage}"; before="${2:?usage}"; after="${3:?usage}"; shift 3
cases=("${@:-flip entered occluded idle pan-9 pan-20 pan-40}")
read -r -a cases <<< "${cases[*]}"
reps="${REPS:-3}"
here="$(cd "$(dirname "$0")" && pwd)"
native="$(git -C "$here" rev-parse --show-toplevel)/native"
spike="$(cd "${ARCHIVE:?}" && pwd)/native/bakeoff/rune-shell"
fx="$here/fixtures"
mkdir -p "$out"; out="$(cd "$out" && pwd)"
export BENCH="$native/target/release/specular-bench"
RUN="python3 $spike/tools/s6_run.py"
TIMED="python3 $here/timed.py"
ps -Ao pid,pcpu,pmem,comm -r > "$out/ps-before.txt"
{ sysctl -n machdep.cpu.brand_string; sw_vers -productVersion; pmset -g batt; } > "$out/machine.txt"
export LOAD_MAX="${LOAD_MAX:-6}"
quiet() {
  for _ in $(seq 60); do
    awk '{exit !($1 < ENVIRON["LOAD_MAX"])}' <<< "$(sysctl -n vm.loadavg | tr -d '{}')" && break
    sleep 5
  done
  echo "$1 $(uptime)" >> "$out/machine.txt"
}
one() {
  local case="$1" label="$2" kit="$3" i="$4" space
  space="$(mktemp -d)"
  quiet "$case-$label-$i"
  case "$case" in
    flip)
      cp "$spike/fixtures/flip.canvas" "$spike/fixtures/flip.html" "$space/"
      SPECULAR_NATIVE_CONFIG_DIR="$space/config" SPECULAR_FLOAT_WINDOW=1 \
      SPECULAR_SHELL_SCRIPT_FILE="$spike/scripts/s6/flip.txt" \
        $RUN "$out/flip-$label-$i.json" -- "$kit" --window 1600x1000 "$space/flip.canvas" || true ;;
    entered)
      cp "$fx/anim.canvas" "$fx/anim.html" "$space/"
      SPECULAR_NATIVE_CONFIG_DIR="$space/config" SPECULAR_FLOAT_WINDOW=1 \
      SPECULAR_SHELL_SCRIPT_FILE="$fx/anim-entered.txt" \
        $TIMED 60 "$out/entered-$label-$i.ledger" -- "$kit" --window 1600x1000 "$space/anim.canvas" ;;
    occluded|occluded-full)
      local policy=(); [ "$case" = occluded-full ] && policy=(--paint-policy full-rate)
      cp "$fx/anim-4.canvas" "$fx/anim.html" "$space/"
      SPECULAR_NATIVE_CONFIG_DIR="$space/config" SPECULAR_SHELL_SCRIPT_FILE="$fx/anim-occluded.txt" \
        $TIMED 60 "$out/$case-$label-$i.ledger" -- "$kit" --window 1600x1000 ${policy[@]+"${policy[@]}"} "$space/anim-4.canvas" ;;
    idle)
      $RUN "$out/idlestill-20-$label-$i.json" -- "$kit" --window 1600x1000 --paint-policy full-rate \
        --bench idle --bench-duration-ms 30000 --warmup-ms 10000 "$spike/fixtures/still-20.canvas" ;;
    pan-*)
      $RUN "$out/$case-$label-$i.json" -- "$kit" --window 1600x1000 --paint-policy full-rate \
        --bench slow-pan --bench-duration-ms 6000 --warmup-ms 20000 "$native/fixtures/static-${case#pan-}.canvas" ;;
  esac
  rm -rf "$space"
}
cd "$native"
for case in "${cases[@]}"; do
  for i in $(seq "$reps"); do
    one "$case" before "$before/Contents/MacOS/specular" "$i"
    one "$case" after "$after/Contents/MacOS/specular" "$i"
  done
done
ps -Ao pid,pcpu,pmem,comm -r > "$out/ps-after.txt"
{ pmset -g batt; uptime; } >> "$out/machine.txt"
