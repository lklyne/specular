const G = __gpu, out = []
const want = [['DFG_LUT', 'dfg-lut', 1], ['PMREM.cubeUv', 'pmrem-cubeuv', 1], ['BrushNormal', 'brush-normal', 10]]
for (const [label, name, mips] of want) {
  const o = [...G.objs.values()].find((o) => o.kind === 'texture' && o.label === label && !o.destroyed)
  for (let m = 0; m < Math.min(mips, o.mipLevelCount); m++) {
    const r = await G.readTexture(o.id, m)
    await G.post(`textures/${name}-mip${m}.bin`, r.data)
    out.push({ name, id: o.id, mip: m, w: r.w, h: r.h, bytesPerRow: r.bytesPerRow, format: r.format })
  }
}
// the equirect HDR source and its mip chain (PMREM_equirect samples it)
const hdr = [...G.texData.entries()].find(([id, l]) => G.objs.get(id).format === 'rgba16float' && G.objs.get(id).size[0] === 256)
if (hdr) { const o = G.objs.get(hdr[0]); for (let m = 0; m < o.mipLevelCount; m++) { const r = await G.readTexture(o.id, m); await G.post(`textures/env-equirect-mip${m}.bin`, r.data); out.push({ name: 'env-equirect', id: o.id, mip: m, w: r.w, h: r.h, bytesPerRow: r.bytesPerRow, format: r.format }) } }
await G.post('textures/manifest.json', JSON.stringify(out, null, 1))
return 'textures ' + out.length
