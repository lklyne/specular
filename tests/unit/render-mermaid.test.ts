/**
 * Mutation-verified by deleting the `REMOTE_FONT_IMPORT` replace in
 * renderMermaid() (the @import assertion fails) and by deleting `retheme()`
 * (the --diagram-fg assertion fails).
 */
import { describe, expect, it } from 'vitest'
import { renderMermaid, renderMermaidCached } from '../../src/renderer/shared/mermaid/render-mermaid'

const FLOW = 'graph TD\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]'

describe('renderMermaid', () => {
  it('renders a flowchart to an svg that fetches nothing and reads its palette from the container', () => {
    const result = renderMermaid(FLOW)
    if (!result.ok) throw new Error(result.error)
    expect(result.svg.startsWith('<svg')).toBe(true)
    expect(result.svg).not.toContain('fonts.googleapis.com')
    expect(result.svg).toContain('--fg:var(--diagram-fg,')
    expect(result.svg).toContain('--bg:var(--diagram-bg,')
    expect(result.svg).toContain('Start')
  })

  it('reports a parse failure instead of throwing', () => {
    const result = renderMermaid('pie title nope\n  "a": 1')
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.error).toMatch(/mermaid/i)
  })

  it('returns the same result object for the same source', () => {
    expect(renderMermaidCached(FLOW)).toBe(renderMermaidCached(FLOW))
  })
})
