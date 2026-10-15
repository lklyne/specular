const G = __gpu
const trace = await G.recordNext(1)
const written = new Set(), bound = new Map()
const viewTex = (v) => G.objs.get(v)?.texture
for (const t of trace) {
  if (t.op === 'beginRenderPass') t.colors.forEach((c) => c && written.add(viewTex(c.view)))
  if (t.op === 'copyTextureToTexture') written.add(t.dst)
  if (t.op === 'setBindGroup') for (const e of G.objs.get(t.group).entries) if (e.textureView) bound.set(viewTex(e.textureView), G.objs.get(t.group).id)
}
const out = []
for (const [tid] of bound) {
  if (written.has(tid)) continue
  const o = G.objs.get(tid)
  if (['DFG_LUT', 'PMREM.cubeUv', 'BrushNormal'].includes(o.label)) continue
  const r = await G.readTexture(tid, 0)
  await G.post(`textures/static-tex${tid}-mip0.bin`, r.data)
  out.push({ name: `static-tex${tid}`, id: tid, mip: 0, w: r.w, h: r.h, bytesPerRow: r.bytesPerRow, format: r.format })
}
await G.post('textures/static-manifest.json', JSON.stringify(out, null, 1))
// also record which bind slot each static texture fills, for labelling
await G.post('textures/static-trace.json', G.json({ trace, objs: [...new Set([...bound.values()])].map((id) => G.objs.get(id)) }))
return out
