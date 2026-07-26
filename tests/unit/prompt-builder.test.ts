/**
 * Selection-annotation cases mutation-verified by flipping the horizontal
 * thirds in positionInRegion (item positions then read mirrored), by dropping
 * the target line from selectionLines, and by dropping the canvas-surfacing
 * line from whatToSurfaceLines — each broke the cases below.
 */

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
import type { FixPromptContext } from '../../src/main/agent-fix/prompt-builder'
import type { Annotation, WorkspaceBounds } from '../../src/shared/types'

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

  it('says nothing about a selection or an editing policy for a plain page comment', () => {
    const prompt = buildFixPrompt(baseAnnotation())
    expect(prompt).not.toContain('Selected items:')
    expect(prompt).not.toContain('What the user sees:')
  })
})

// Region 0,0 → 400×400, so a member's third of the region is readable at a
// glance: 0..133 top/left, 134..266 middle/center, 267..400 bottom/right.
const REGION: WorkspaceBounds = { x: 0, y: 0, width: 400, height: 400 }

function bounds(x: number, y: number, width = 40, height = 40): WorkspaceBounds {
  return { x, y, width, height }
}

function selectionAnnotation(overrides: Partial<Annotation> = {}): Annotation {
  return baseAnnotation({
    anchor: { type: 'region', canvasRect: REGION },
    text: 'Fix everything the stickies call out',
    pageAnchor: undefined,
    metadata: {
      selectionEntityIds: ['text-1', 'draw-1', 'shape-1', 'page-a'],
      selectionTarget: {
        entityId: 'page-a',
        kind: 'page',
        url: 'http://localhost:4321/garden',
      },
    },
    ...overrides,
  })
}

const selectionContext: FixPromptContext = {
  selection: {
    members: [
      {
        id: 'text-1',
        kind: 'text',
        textStyle: 'sticky',
        text: 'this button is too small',
        bounds: bounds(10, 10),
      },
      { id: 'draw-1', kind: 'drawing', bounds: bounds(280, 20, 100, 60) },
      { id: 'shape-1', kind: 'shape', shapeKind: 'rectangle', text: 'Nav', bounds: bounds(10, 360) },
      {
        id: 'page-a',
        kind: 'page',
        url: 'http://localhost:4321/garden',
        pageName: 'Garden',
        bounds: bounds(150, 150, 100, 100),
      },
    ],
    priorFeedback: [
      { text: 'Nav is misaligned', status: 'pending', element: 'header.site-header' },
    ],
  },
  target: {
    kind: 'repo',
    cwd: '/Users/x/dev/site',
    origin: 'http://localhost:4321',
    autoFix: false,
  },
}

describe('buildFixPrompt — selection annotations', () => {
  it('names the target artifact and every selected item with its place in the region', () => {
    const prompt = buildFixPrompt(selectionAnnotation(), selectionContext)
    expect(prompt).toContain(
      'The artifact this request is about: the page http://localhost:4321/garden',
    )
    expect(prompt).toContain('sticky at the top-left of the region: "this button is too small"')
    expect(prompt).toContain('freehand drawing overlays the top-right of the region')
    expect(prompt).toContain('rectangle shape at the bottom-left of the region: "Nav"')
    expect(prompt).toContain('page at the center of the region: http://localhost:4321/garden — Garden')
  })

  it('lists unresolved comments already on the selected items as prior feedback', () => {
    const prompt = buildFixPrompt(selectionAnnotation(), selectionContext)
    expect(prompt).toContain('Prior feedback in scope')
    expect(prompt).toContain('[pending] "Nav is misaligned" (on header.site-header)')
  })

  it('states where a repo-bound artifact lives without prescribing how to change it', () => {
    const prompt = buildFixPrompt(selectionAnnotation(), selectionContext)
    expect(prompt).toContain(
      'http://localhost:4321 is served from the repo at /Users/x/dev/site (your working directory), under version control.',
    )
    expect(prompt).toContain('Target page: http://localhost:4321/garden')
    // Duplicating a prototype to iterate on it is a workflow, not a mistake to
    // guard against — the comment decides, so the prompt bans neither path.
    expect(prompt).not.toMatch(/Do not (duplicate|copy)/)
    expect(prompt).not.toContain('Edit the source in place')
  })

  // Regression: a repo-bound fix that created a new route used to be told
  // "do not duplicate anything on the canvas", so the new page landed on disk
  // only and the run looked like a no-op to the user.
  it('says new artifacts only reach the user via the canvas', () => {
    const prompt = buildFixPrompt(selectionAnnotation(), selectionContext)
    expect(prompt).toContain('What the user sees:')
    expect(prompt).toContain('Pages already on the canvas reload themselves from source.')
    expect(prompt).toContain('Anything new you create reaches the user only once it is on the canvas')
    expect(prompt).toContain('specular add page <full url> --at x,y')
  })

  it('states a space-folder target lacks version control and how a new file reaches the canvas', () => {
    const annotation = selectionAnnotation({
      metadata: {
        selectionEntityIds: ['file-1'],
        selectionTarget: {
          entityId: 'file-1',
          kind: 'file',
          filePath: '/Users/x/space/hero.png',
        },
      },
    })
    const prompt = buildFixPrompt(annotation, {
      selection: {
        members: [{ id: 'file-1', kind: 'file', filePath: '/Users/x/space/hero.png' }],
        priorFeedback: [],
      },
      target: {
        kind: 'space-folder',
        cwd: '/Users/x/space',
        filePath: '/Users/x/space/hero.png',
      },
    })
    expect(prompt).toContain('Target file: /Users/x/space/hero.png')
    expect(prompt).toContain("space folder (/Users/x/space, your working directory)")
    // The fact that motivated the old "do not overwrite" rule, stated as a
    // fact so the model can weigh it against what the comment actually asks.
    expect(prompt).toContain('no repo and no version control behind it')
    expect(prompt).not.toContain('Do not overwrite the original')
    expect(prompt).toContain('specular add file <path> --at x,y')
    // No page is in play, so the live-page inspection block is dead weight.
    expect(prompt).not.toContain('Inspecting the live page')
  })

  it('says the artifact is undecided when the selection names none', () => {
    const annotation = selectionAnnotation({
      metadata: { selectionEntityIds: ['page-a', 'page-b'] },
    })
    const prompt = buildFixPrompt(annotation, { selection: null, target: null })
    expect(prompt).toContain('The selection names no single artifact')
  })
})
