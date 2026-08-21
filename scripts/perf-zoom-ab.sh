#!/usr/bin/env bash
# A/B the per-tick zoom scene rebroadcast. Alternates on/off runs so thermal
# drift hits both arms equally. Usage: zoom-ab.sh [runsPerArm=3] [profile=slow-zoom] [durationMs=1400]
set -euo pipefail
RUNS=${1:-3}; PROFILE=${2:-slow-zoom}; DUR=${3:-1400}
SECRET=$(jq -r .secret ~/.specular/specular-mcp.json)
H=(-H "x-specular-secret: $SECRET" -H 'Content-Type: application/json')
U=http://localhost:29979
OUT=${ZOOM_AB_OUT:-/tmp/zoom-ab-results.jsonl}; : > "$OUT"

run() { # $1 = true|false
  curl -s -X POST "$U/perf/flags" "${H[@]}" -d "{\"zoomSceneRebroadcast\":$1}" >/dev/null
  sleep 0.5
  curl -s -X POST "$U/perf/pan-zoom/run" "${H[@]}" \
    -d "{\"summarize\":true,\"profiles\":[\"$PROFILE\"],\"durationMs\":$DUR}" \
  | jq -c --arg mode "$1" '{
      mode: $mode,
      build_mean: .buildStats.mean, build_p95: .buildStats.p95,
      renderer_ms: ([.summary.threads[]? | select(.thread=="CrRendererMain") | .busyMs] | add // 0),
      browser_ms:  ([.summary.threads[]? | select(.thread=="CrBrowserMain")  | .busyMs] | add // 0),
      gpu_ms:      ([.summary.threads[]? | select(.thread=="CrGpuMain")      | .busyMs] | add // 0),
      raster_n:    ([.summary.markers[]? | select(.label=="Raster tasks") | .count] | add // 0),
      layout_n:    ([.summary.markers[]? | select(.label=="Layout / style recalc") | .count] | add // 0),
      commits_n:   ([.summary.markers[]? | select(.label=="Compositor commits") | .count] | add // 0),
      bridge_ms:   ([.summary.topEvents[]? | select(.name|test("ContextBridge")) | .totalMs] | add // 0),
      summarized:  (.summary != null)
    }' | tee -a "$OUT"
  sleep 1.5
}

echo "warmup"; run true >/dev/null
for i in $(seq "$RUNS"); do run true; run false; done

# restore default
curl -s -X POST "$U/perf/flags" "${H[@]}" -d '{"zoomSceneRebroadcast":true}' >/dev/null

echo; echo "=== means per arm (on = rebroadcast every tick, off = CSS transform until settle) ==="
jq -s -r '
  group_by(.mode) | map({
    mode: .[0].mode, n: length,
    build_mean: (map(.build_mean)|add/length),
    renderer_ms: (map(.renderer_ms)|add/length),
    browser_ms: (map(.browser_ms)|add/length),
    gpu_ms: (map(.gpu_ms)|add/length),
    raster_n: (map(.raster_n)|add/length),
    layout_n: (map(.layout_n)|add/length),
    commits_n: (map(.commits_n)|add/length),
    bridge_ms: (map(.bridge_ms)|add/length)
  }) | (["mode","n","build_mean","renderer_ms","browser_ms","gpu_ms","raster_n","layout_n","commits_n","bridge_ms"]|@tsv),
       (.[] | [.mode,.n,(.build_mean*100|round/100),(.renderer_ms|round),(.browser_ms|round),(.gpu_ms|round),(.raster_n|round),(.layout_n|round),(.commits_n|round),(.bridge_ms|round)]|@tsv)
' "$OUT" | column -t
