/**
 * Mermaid text → SVG string, via beautiful-mermaid. Pure: no DOM, so it runs
 * in unit tests and could run in the main process.
 *
 * The output is theme-agnostic. beautiful-mermaid derives every color from
 * `--bg` / `--fg` custom properties on the `<svg>`, so instead of baking a
 * palette in at render time the svg reads `--diagram-bg` / `--diagram-fg`
 * from its container (see `diagram-theme.ts`). Flipping the app theme then
 * restyles a diagram without re-laying it out, and the render cache needs
 * no theme in its key.
 */

import { renderMermaidSVG } from 'beautiful-mermaid'
import { DIAGRAM_LIGHT } from './diagram-theme'

export type MermaidRender = { ok: true; svg: string } | { ok: false; error: string }

/** Diagram text uses the app's UI face; the note's typeface is the same. */
const DIAGRAM_FONT = 'system-ui'

/**
 * beautiful-mermaid embeds a Google Fonts `@import` for whatever `font` it was
 * given. The canvas renderer must not fetch fonts from the network, and the
 * face is local anyway.
 */
const REMOTE_FONT_IMPORT = /@import url\('https:\/\/fonts\.googleapis\.com[^']*'\);\s*/g

const INLINE_THEME = /--bg:[^;"]+;--fg:[^;"]+/

export function renderMermaid(source: string): MermaidRender {
  try {
    const svg = renderMermaidSVG(source, {
      ...DIAGRAM_LIGHT,
      transparent: true,
      font: DIAGRAM_FONT,
    })
    return { ok: true, svg: retheme(svg.replace(REMOTE_FONT_IMPORT, '')) }
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) }
  }
}

function retheme(svg: string): string {
  return svg.replace(
    INLINE_THEME,
    `--bg:var(--diagram-bg,${DIAGRAM_LIGHT.bg});--fg:var(--diagram-fg,${DIAGRAM_LIGHT.fg})`,
  )
}

const CACHE_LIMIT = 64
const cache = new Map<string, MermaidRender>()

/** `renderMermaid` memoized by source. Layout runs synchronously on the UI
 *  thread, so a note that re-decorates on every keystroke must not pay for
 *  it twice. */
export function renderMermaidCached(source: string): MermaidRender {
  const hit = cache.get(source)
  if (hit) return hit
  const result = renderMermaid(source)
  if (cache.size >= CACHE_LIMIT) {
    const oldest = cache.keys().next().value
    if (oldest !== undefined) cache.delete(oldest)
  }
  cache.set(source, result)
  return result
}
