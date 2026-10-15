#!/usr/bin/env bash
# Full ground-truth capture of the Prism scene. Needs: lyle-klyne dev server on :4321,
# capture Chrome on :9333 (see report), upload-server.py on :8765 writing to reference/_raw,
# and the temporary `window.__prismRenderer = renderer` line in lyle-klyne's webgpu-canvas.tsx.
set -euo pipefail
cd "$(dirname "$0")"
URL='http://localhost:4321/builds/2026-10-11-prism/prism?tab=Prism'
ev() { node cdp.mjs eval -e "$1"; }
dump() { # tag snap canvasOnly
  node cdp.mjs eval uniform-labels.js >/dev/null
  ev "const TAG='$1'; const SNAP=$2; const CANVAS_ONLY=$3; $(cat dump-frame.js)" | tr -d '\n '; echo " $1"
  ev "const s = await (async () => { $(cat three-uniforms.js) })(); await __gpu.post('frames/$1/three-uniforms.json', s); return 'ok'" >/dev/null
}
size() { ev "const SIZE=$1; $(cat setsize.js)" | tr -d '\n '; echo; }

PRE='window.__prismStartPaused=true; window.__prismTraceFromStart=true' node cdp.mjs load "$URL" gpu-hook.js
size 1024
ev 'await __gpu.waitReal(180); window.__initTrace = __gpu.trace; __gpu.trace = null; return __initTrace.length'
ev "$(cat dump-init.js)"
dump f0000 true false
ev 'await __gpu.advance(240); return __gpu.frameIndex' >/dev/null
dump f0240 true false
# dense run: 10 consecutive frames for motion matching
for i in $(seq 241 250); do
  ev 'await __gpu.advance(1); return __gpu.frameIndex' >/dev/null
  dump "$(printf 'f%04d' $i)" true true
done
ev 'await __gpu.advance(20); return __gpu.frameIndex' >/dev/null
dump f0270 true true
for i in 2 3 4 5 6 7 8; do
  ev 'await __gpu.advance(30); return __gpu.frameIndex' >/dev/null
  dump "$(printf 'f%04d' $((240 + 30 * i)))" true true
done
for s in 512 256 128 64; do size $s; dump "f0480-s$s" true false; done
size 1024
ev "$(cat dump-textures.js)"
node cdp.mjs eval dump-static.js >/dev/null
