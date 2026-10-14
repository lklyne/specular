// Map every TSL uniform node reachable from React component state to a readable path,
// e.g. "GradientCard.gradient.uniforms.distortion" or "PrismScene.paint.uniforms.strokeWidth".
const labels = new Map()
const seen = new WeakSet()
const isNode = (v) => v && typeof v === 'object' && v.isNode
const visit = (v, path, depth) => {
  if (!v || typeof v !== 'object' || depth > 4) return
  if (v.isUniformNode) { if (!labels.has(v)) labels.set(v, path); return }
  if (isNode(v) || seen.has(v)) return
  if (v.isObject3D || v.isTexture || v.isBufferGeometry || v.$$typeof || v instanceof Node || v instanceof Window) return
  seen.add(v)
  if (v.isMaterial) {
    for (const k of Object.keys(v)) if (v[k] && v[k].isUniformNode) labels.has(v[k]) || labels.set(v[k], path + '.' + k)
    return
  }
  const keys = Array.isArray(v) ? v.keys() : Object.keys(v)
  for (const k of keys) {
    let x
    try { x = v[k] } catch { continue }
    visit(x, path + '.' + k, depth + 1)
  }
}
const walkFiber = (f) => {
  const stack = [f]
  while (stack.length) {
    const fiber = stack.pop()
    if (!fiber) continue
    const name = typeof fiber.type === 'function' ? fiber.type.displayName || fiber.type.name : null
    if (name) {
      let h = fiber.memoizedState, i = 0
      while (h && typeof h === 'object' && 'next' in h) {
        const s = h.memoizedState
        // useMemo stores [value, deps]; useRef stores { current }
        const val = Array.isArray(s) && s.length === 2 && (Array.isArray(s[1]) || s[1] === null) ? s[0] : s && typeof s === 'object' && 'current' in s && Object.keys(s).length === 1 ? s.current : s
        visit(val, `${name}.hook${i}`, 0)
        h = h.next; i++
      }
      visit(fiber.memoizedProps, `${name}.props`, 0)
    }
    if (fiber.sibling) stack.push(fiber.sibling)
    if (fiber.child) stack.push(fiber.child)
  }
}
for (const root of window.__prismFiberRoots || []) walkFiber(root.current)
window.__prismUniformLabels = labels
return labels.size
