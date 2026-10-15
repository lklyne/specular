// expects TAG (string) in scope; records one frame with pass snapshots and uploads everything
const G = window.__gpu
const trace = await G.recordNext(1, { snapshot: SNAP })
const snaps = SNAP ? await G.readSnaps() : []
const dir = 'frames/' + TAG + '/'
// buffers referenced by this frame's bind groups / vertex buffers
const used = new Set()
for (const t of trace) {
  if (t.op === 'setBindGroup') { const g = G.objs.get(t.group); g && g.entries.forEach((e) => e.buffer && used.add(e.buffer)) }
  if (t.op === 'setVertexBuffer' || t.op === 'setIndexBuffer') used.add(t.buffer)
}
const bufMeta = {}
for (const id of used) {
  const sh = G.shadow.get(id)
  if (!sh) { bufMeta[id] = { missing: true }; continue }
  bufMeta[id] = { size: sh.byteLength }
  await G.post(dir + 'buffers/' + id + '.bin', sh)
}
const snapMeta = []
for (const s of snaps) {
  if (typeof CANVAS_ONLY !== 'undefined' && CANVAS_ONLY && !s.canvas) continue
  const name = `pass${String(s.passIndex).padStart(2, '0')}-slot${s.slot}-tex${s.texId}.bin`
  await G.post(dir + 'snaps/' + name, s.data)
  snapMeta.push({ ...s, data: undefined, file: name })
}
await G.post(dir + 'trace.json', G.json({ frameIndex: G.frameIndex, vt: G.vt, trace, buffers: bufMeta, snaps: snapMeta }))
const isCanvasTex = (id) => G.objs.get(id)?.canvas
const usedViews = new Set()
for (const t of trace) if (t.op === 'beginRenderPass') t.colors.forEach((c) => c && usedViews.add(c.view))
await G.post(dir + 'objects.json', G.json([...G.objs.values()].filter((o) => usedViews.has(o.id) || !((o.kind === 'texture' && o.canvas) || (o.kind === 'textureView' && isCanvasTex(o.texture))))))
return { ops: trace.length, passes: trace.filter((t) => t.op === 'beginRenderPass').length, draws: trace.filter((t) => t.op.startsWith('draw')).length, snaps: snapMeta.length, frame: G.frameIndex }
