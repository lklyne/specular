import { describe, expect, it, vi } from 'vitest'

// prompt-builder reads a region's current canvas rect via regionCanvasRect,
// which lives in the electron-touching runtime. Stub it so this pure
// prompt-shaping test stays Electron-free; region coordinates are covered by
// the integration suite.
vi.mock('../../src/main/runtime/page-anchor-state', () => ({
  regionCanvasRect: vi.fn((annotation: { anchor?: { canvasRect?: unknown } }) =>
    annotation.anchor?.canvasRect ?? null,
  ),
}))

import { buildFixPrompt } from '../../src/main/agent-fix/prompt-builder'
import type { Annotation } from '../../src/shared/types'

function baseAnnotation(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: 'ann-1',
    anchor: { type: 'page', pageId: 'page-a', offsetX: 0, offsetY: 0 },
    author: 'user',
    text: 'Header padding is too big',
    status: 'pending',
    replies: [],
    createdAt: new Date().toISOString(),
    pageAnchor: { pageId: 'page-a', pageUrl: 'http://localhost:4321/garden' },
    metadata: {
      pageName: 'Desktop 1280×800',
    },
    ...overrides,
  }
}

describe('buildFixPrompt', () => {
  it('includes page URL, page name, and the thread', () => {
    const prompt = buildFixPrompt(baseAnnotation())
    expect(prompt).toContain('http://localhost:4321/garden')
    expect(prompt).toContain('Desktop 1280×800')
    expect(prompt).toContain('[User] Header padding is too big')
  })

  it('includes source location and react components when available', () => {
    const prompt = buildFixPrompt(
      baseAnnotation({
        anchor: {
          type: 'element',
          pageId: 'page-a',
          selector: 'header.site-header',
          elementPath: 'body > header',
        },
        metadata: {
          inspectContext: {
            id: 'node-1',
            nodeId: 'node-1',
            timestamp: 0,
            tagName: 'HEADER',
            name: 'SiteHeader',
            elementPath: 'body > header',
            fullPath: 'body > header.site-header',
            cssClasses: ['site-header'],
            nearbyElements: [],
            accessibility: [],
            attributes: [],
            computedStyles: [],
            reactComponents: ['SiteHeader', 'Layout'],
            sourceLocation: { file: 'src/components/SiteHeader.tsx', line: 42 },
          },
        },
      }),
    )
    expect(prompt).toContain('Element source: src/components/SiteHeader.tsx:42')
    expect(prompt).toContain('React components (inner to outer): SiteHeader > Layout')
    expect(prompt).toContain('Element name: SiteHeader')
  })

  it('renders a multi-turn thread in order', () => {
    const prompt = buildFixPrompt(
      baseAnnotation({
        replies: [
          { author: 'agent', text: 'Reduced to 12px.', timestamp: '' },
          { author: 'user', text: 'Make it 8px instead.', timestamp: '' },
        ],
      }),
    )
    const userIdx = prompt.indexOf('[User] Header padding is too big')
    const agentIdx = prompt.indexOf('[Agent] Reduced to 12px.')
    const followupIdx = prompt.indexOf('[User] Make it 8px instead.')
    expect(userIdx).toBeGreaterThanOrEqual(0)
    expect(agentIdx).toBeGreaterThan(userIdx)
    expect(followupIdx).toBeGreaterThan(agentIdx)
  })

  it('ends with the reply-format instruction and marker guidance', () => {
    const prompt = buildFixPrompt(baseAnnotation())
    expect(prompt).toContain('<<RESOLVE>>')
    expect(prompt).toContain('<<WAITING>>')
    expect(prompt).toMatch(/Reply format — REQUIRED/)
  })
})
