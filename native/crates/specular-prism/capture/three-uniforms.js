const r = window.__prismRenderer
const G = window.__gpu
const val = (v) => {
  if (v == null) return v
  if (typeof v === 'number' || typeof v === 'boolean' || typeof v === 'string') return v
  if (v.isVector2) return [v.x, v.y]
  if (v.isVector3) return [v.x, v.y, v.z]
  if (v.isVector4) return [v.x, v.y, v.z, v.w]
  if (v.isColor) return { color: [v.r, v.g, v.b] }
  if (v.isMatrix3 || v.isMatrix4) return Array.from(v.elements)
  if (v.isTexture) return { texture: v.name || v.uuid, isRT: !!v.isRenderTargetTexture, w: v.image?.width, h: v.image?.height }
  if (Array.isArray(v)) return v.map(val)
  return String(v)
}
const desc = (node) => {
  if (!node) return null
  const d = { type: node.type || node.constructor?.name, name: node.name || undefined, label: node.label || undefined }
  for (const k of ['property', 'uniformType', 'scope', 'groupNode']) if (node[k] !== undefined) d[k] = typeof node[k] === 'object' ? node[k]?.name ?? String(node[k]) : node[k]
  if (node.referenceNode) d.referenceProperty = node.referenceNode.property
  if (refName.has(node)) d.ref = refName.get(node)
  const L = window.__prismUniformLabels
  if (L && L.has(node)) d.label = L.get(node)
  return d
}
const refName = new Map()
for (const ro of r._objects._renderObjects) {
  const st = ro.getNodeBuilderState()
  for (const set of [st.updateNodes, st.updateBeforeNodes, st.updateAfterNodes]) for (const n of set || []) {
    if (n.node && n.property) refName.set(n.node, (n.isMaterialReferenceNode ? 'material.' : 'ref.') + n.property)
    if (n.uniformNode && n.scope) refName.set(n.uniformNode, (n.type || n.constructor.name) + '.' + n.scope)
  }
}
const out = []
for (const ro of r._objects._renderObjects) {
  const e = {
    object: { type: ro.object?.type, name: ro.object?.name, id: ro.object?.id, renderOrder: ro.object?.renderOrder, scale: ro.object?.scale && val(ro.object.scale) },
    material: { type: ro.material?.type, name: ro.material?.name, transparent: ro.material?.transparent, side: ro.material?.side },
    pipeline: null,
    groups: [],
  }
  try {
    const bd = r.backend.get(ro.pipeline)
    e.pipelineGpuId = bd?.pipeline ? G.idOf(bd.pipeline) : null
  } catch (err) { e.pipelineErr = String(err) }
  try {
    for (const bg of ro.getBindings()) {
      const g = { name: bg.name, index: bg.index, bindings: [] }
      for (const b of bg.bindings) {
        const bb = { name: b.name, type: b.constructor?.name }
        if (b.uniforms) bb.uniforms = b.uniforms.map((u) => ({ name: u.name, type: u.type ?? u.constructor?.name, offset: u.offset, node: desc(u.nodeUniform?.node), value: val(u.getValue ? u.getValue() : u.value) }))
        if (b.texture) bb.texture = val(b.texture)
        if (b.textureNode) bb.textureNode = desc(b.textureNode)
        g.bindings.push(bb)
      }
      e.groups.push(g)
    }
  } catch (err) { e.bindErr = String(err) }
  out.push(e)
}
return G.json(out)
