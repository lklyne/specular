const G = __gpu, t = window.__initTrace
await G.post('init/trace.json', G.json(t))
await G.post('init/objects.json', G.json([...G.objs.values()].filter((o) => !(o.kind === 'texture' && o.canvas) && !(o.kind === 'textureView' && G.objs.get(o.texture)?.canvas))))
const meta = []
for (const [id, list] of G.texData) for (let i = 0; i < list.length; i++) {
  const e = list[i]
  meta.push({ tex: id, i, ...e, bytes: e.bytes ? e.bytes.byteLength : undefined, label: G.objs.get(id).label, texSize: G.objs.get(id).size, format: G.objs.get(id).format })
  if (e.bytes) await G.post(`init/texdata/tex${id}-${i}.bin`, e.bytes)
}
await G.post('init/texdata.json', G.json(meta))
return 'init ops ' + t.length
