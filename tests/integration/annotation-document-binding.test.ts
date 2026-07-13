/**
 * Document-binding gate at the layout-broadcast seam, against the real
 * runtime in-process: annotations whose context page shows a different URL
 * than the one they were created on are dropped from the layout payload
 * main-side, the same way anchored entities are — renderers never re-derive
 * visibility from URLs. The harness's layout engine is dormant (the fake
 * window reports destroyed), so tests call `getCanvasLayoutData()` directly —
 * the exact builder whose output every `layoutUpdate` broadcast carries.
 *
 * Mutation-verified by: replacing the `hiddenByPageAnchor` annotations filter
 * in `buildCanvasLayoutData` (canvas-layout-data.ts) with
 * `[...workspaceAnnotations]` — the "drops page-bound annotations" case fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import { createAnnotation } from '../../src/main/workspace-annotations'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import { pages } from '../../src/main/runtime/runtime-context'

let harness: WorkspaceHarness

const PAGE_ID = 'page-doc-host'
const PAGE_URL = 'https://example.com/pricing'

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Document binding host',
    doc: {
      nodes: [
        {
          id: PAGE_ID,
          type: 'link',
          x: 120,
          y: 120,
          width: 375,
          height: 667,
          url: PAGE_URL,
          presetIndex: 0,
        } satisfies JsonCanvasLinkNode,
      ],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

function payloadAnnotationIds(): string[] {
  return getCanvasLayoutData().annotations.map((annotation) => annotation.id)
}

describe('annotation document binding in the layout payload', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('drops page-bound annotations while the page shows another URL and restores them on return', async () => {
    loadHostPage()
    const element = createAnnotation({
      anchor: {
        type: 'element',
        pageId: PAGE_ID,
        selector: 'main > section.hero',
        boundingBox: { x: 10, y: 10, width: 100, height: 40 },
      },
      text: 'element note',
    })
    const pageAnchored = createAnnotation({
      anchor: { type: 'page', pageId: PAGE_ID, offsetX: 0.5, offsetY: 0.4 },
      text: 'page note',
    })
    const region = createAnnotation({
      anchor: { type: 'region', canvasRect: { x: 140, y: 140, width: 80, height: 60 } },
      metadata: {
        regionComponents: [{ pageId: PAGE_ID, pageName: 'Host', components: [] }],
      },
      text: 'region note',
    })
    const canvasNote = createAnnotation({
      anchor: { type: 'canvas', canvasX: 2000, canvasY: 2000 },
      text: 'canvas note',
    })
    await settleSync()

    const pageBound = [element.id, pageAnchored.id, region.id]
    expect(payloadAnnotationIds()).toEqual(
      expect.arrayContaining([...pageBound, canvasNote.id]),
    )

    // did-navigate updates page.url in place (page-factory.ts) and requests a
    // fresh layout broadcast; the rebuilt payload must omit the page-bound
    // annotations but keep the canvas-bound one.
    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.url = 'https://example.com/elsewhere'
    const afterNavigation = payloadAnnotationIds()
    for (const id of pageBound) expect(afterNavigation).not.toContain(id)
    expect(afterNavigation).toContain(canvasNote.id)

    // Navigating back restores them — the gate reads live page state.
    page.url = PAGE_URL
    expect(payloadAnnotationIds()).toEqual(
      expect.arrayContaining([...pageBound, canvasNote.id]),
    )
  })
})
