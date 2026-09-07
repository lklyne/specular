import { useEffect, useState } from 'react'
import { loadMermaidRenderer, renderMermaidIfLoaded } from './mermaid-loader'
import type { MermaidRender } from './render-mermaid'

/** Render `source` to SVG, loading the diagram chunk on first use. Null
 *  while the chunk loads or when there is no source yet. */
export function useMermaidRender(source: string | null): MermaidRender | null {
  const [result, setResult] = useState<MermaidRender | null>(() =>
    source === null ? null : renderMermaidIfLoaded(source),
  )
  useEffect(() => {
    if (source === null) {
      setResult(null)
      return
    }
    const sync = renderMermaidIfLoaded(source)
    if (sync) {
      setResult(sync)
      return
    }
    let cancelled = false
    loadMermaidRenderer().then((mod) => {
      if (!cancelled) setResult(mod.renderMermaidCached(source))
    })
    return () => {
      cancelled = true
    }
  }, [source])
  return result
}
