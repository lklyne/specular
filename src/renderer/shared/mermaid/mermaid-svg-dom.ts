/** Turn a rendered SVG string into an element. beautiful-mermaid emits only
 *  its own shapes and text, but the source is user- or agent-authored, so
 *  scripts and event handlers are dropped anyway before the node is adopted. */
export function svgElementFromString(svg: string): SVGSVGElement | null {
  const parsed = new DOMParser().parseFromString(svg, 'image/svg+xml')
  const root = parsed.documentElement
  if (root.nodeName !== 'svg') return null
  for (const script of Array.from(root.querySelectorAll('script'))) script.remove()
  for (const el of Array.from(root.querySelectorAll('*'))) {
    for (const attr of Array.from(el.attributes)) {
      if (/^on/i.test(attr.name)) el.removeAttribute(attr.name)
    }
  }
  return document.adoptNode(root) as unknown as SVGSVGElement
}
