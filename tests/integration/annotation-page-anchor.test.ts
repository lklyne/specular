/**
 * Page-anchored annotations (`Annotation.pageAnchor`, shared/page-anchor.ts)
 * against the real runtime, in-process. Regions split at creation with no
 * mode: a marquee that grabbed page content (non-empty regionComponents/
 * regionElements) binds to the grab's page — it travels with page drags and
 * nudges, hides while the page is off its anchor URL, nests under the page
 * in the sidebar, and persists the anchor to disk. A grab-less region is
 * canvas-anchored: it never moves with pages and never hides. Annotations
 * from older files without a `pageAnchor` (even ones carrying the retired
 * `metadata.pageUrl`) load fine and behave as canvas-bound.
 *
 * Mutation-verified by:
 * - removing the `...(pageAnchor ? { pageAnchor } : {})` spread in
 *   `createAnnotationInternal` (workspace-annotations.ts) — the grab-rule,
 *   persistence, travel, and sidebar cases fail;
 * - removing the `translateAnnotationsAnchoredToPage` call in
 *   `applyDragDelta` (document-commands.ts) — the drag-travel and one-undo-
 *   step cases fail;
 * - removing the `translateAnnotationsAnchoredToPage` call in
 *   `nudgeSelection` (document-commands.ts) — the nudge case fails;
 * - moving the drag translate out of the gesture session (calling it after
 *   `dragSession?.finalize()` in `finalizeDrag` with its own autosave) — the
 *   single-transaction and one-undo-step assertions fail;
 * - replacing the `hiddenByPageAnchor` annotations filter in
 *   `buildCanvasLayoutData` (canvas-layout-data.ts) with
 *   `[...workspaceAnnotations]` — the navigation cases fail;
 * - dropping the annotations half of `sidebarPageChildren`
 *   (sidebar-builder.ts) — the sidebar-nesting case fails.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import type { Annotation } from '../../src/shared/types'
import { createAnnotation } from '../../src/main/workspace-annotations'
import {
  applyDragDelta,
  finalizeDrag,
  initializeDrag,
  nudgeSelection,
} from '../../src/main/runtime/document-commands'
import { getCanvasLayoutData, getLeftSidebarData } from '../../src/main/runtime/canvas-layout-data'
import { workspaceAnnotations } from '../../src/main/runtime/workspace-model'
import { pages } from '../../src/main/runtime/runtime-context'
import { selectEntities, selectNone } from '../../src/main/runtime/selection-controller'
import { undo, redo } from '../../src/main/runtime/workspace-undo'

let harness: WorkspaceHarness

const PAGE_ID = 'page-region-host'
const PAGE_URL = 'https://example.com/pricing'

function hostPageNode(): JsonCanvasLinkNode {
  return {
    id: PAGE_ID,
    type: 'link',
    x: 120,
    y: 120,
    width: 375,
    height: 667,
    url: PAGE_URL,
    presetIndex: 0,
  }
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Region anchor host',
    doc: {
      nodes: [hostPageNode()],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

/** A region annotation whose marquee grabbed content from the host page. */
function createGrabbedRegion(): Annotation {
  return createAnnotation({
    anchor: { type: 'region', canvasRect: { x: 140, y: 140, width: 80, height: 60 } },
    text: 'grabbed region',
    metadata: {
      regionComponents: [{ pageId: PAGE_ID, pageName: 'Host', components: [] }],
      regionElements: [{ pageId: PAGE_ID, pageName: 'Host', elements: [{}] }],
    },
  })
}

/** A region annotation whose marquee grabbed nothing (empty canvas). */
function createGrablessRegion(): Annotation {
  return createAnnotation({
    anchor: { type: 'region', canvasRect: { x: 900, y: 900, width: 50, height: 50 } },
    text: 'canvas region',
    metadata: { regionComponents: [], regionElements: [] },
  })
}

function liveAnnotation(id: string): Annotation | undefined {
  return workspaceAnnotations.find((candidate) => candidate.id === id)
}

function regionRect(id: string): { x: number; y: number } {
  const annotation = liveAnnotation(id)
  if (annotation?.anchor.type !== 'region') throw new Error(`not a region: ${id}`)
  return { x: annotation.anchor.canvasRect.x, y: annotation.anchor.canvasRect.y }
}

/** Count Y.Doc afterTransaction events during `fn` + the settled sync. */
async function observeTransactions(fn: () => void | Promise<void>): Promise<number> {
  let count = 0
  const handler = () => {
    count += 1
  }
  harness.doc.on('afterTransaction', handler)
  try {
    await fn()
    await settleSync()
  } finally {
    harness.doc.off('afterTransaction', handler)
  }
  return count
}

describe('page-anchored region annotations', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('binds a grabbed region to the grab page and persists the anchor to doc and disk', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    expect(region.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL })
    // The retired metadata binding is not written.
    expect(region.metadata?.pageUrl).toBeUndefined()
    await settleSync()

    const docRecord = harness.doc.getMap('annotations').get(region.id) as
      | { toJSON(): Record<string, unknown> }
      | undefined
    expect(docRecord?.toJSON().pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL })

    const diskRecord = (harness.diskDoc()?.annotations as Annotation[] | undefined)?.find(
      (candidate) => candidate.id === region.id,
    )
    expect(diskRecord?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL })
  })

  it('leaves a grab-less region canvas-anchored: no pageAnchor, never hidden, stays put on page drag', async () => {
    loadHostPage()
    const region = createGrablessRegion()
    expect(region.pageAnchor).toBeUndefined()
    await settleSync()

    initializeDrag([PAGE_ID])
    applyDragDelta([PAGE_ID], 240, 0)
    finalizeDrag()
    await settleSync()
    expect(regionRect(region.id)).toEqual({ x: 900, y: 900 })

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.url = 'https://example.com/elsewhere'
    expect(getCanvasLayoutData().annotations.map((a) => a.id)).toContain(region.id)
  })

  it('translates a grabbed region with its page drag, as one Y.Doc transaction and one undo step', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    await settleSync()

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    const pageStartX = page.canvasX
    const rectStart = regionRect(region.id)

    const transactions = await observeTransactions(() => {
      initializeDrag([PAGE_ID])
      applyDragDelta([PAGE_ID], 140, 60)
      applyDragDelta([PAGE_ID], 100, -60)
      finalizeDrag()
    })
    // The whole gesture — page move + region translate — is one forward-sync
    // transaction (gesture batching), hence one undo step.
    expect(transactions).toBe(1)

    const pageDelta = page.canvasX - pageStartX
    expect(pageDelta).toBe(240)
    expect(regionRect(region.id)).toEqual({ x: rectStart.x + 240, y: rectStart.y })

    undo()
    expect(pages.find((candidate) => candidate.id === PAGE_ID)!.canvasX).toBe(pageStartX)
    expect(regionRect(region.id)).toEqual(rectStart)

    redo()
    expect(pages.find((candidate) => candidate.id === PAGE_ID)!.canvasX).toBe(pageStartX + 240)
    expect(regionRect(region.id)).toEqual({ x: rectStart.x + 240, y: rectStart.y })
  })

  it('translates a grabbed region with a keyboard nudge of its page', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    await settleSync()
    const rectStart = regionRect(region.id)
    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    const pageStartX = page.canvasX

    selectEntities([PAGE_ID])
    nudgeSelection(5, -7)
    await settleSync()

    expect(page.canvasX).toBe(pageStartX + 5)
    expect(regionRect(region.id)).toEqual({ x: rectStart.x + 5, y: rectStart.y - 7 })

    // The nudge — page + region — round-trips as one undo step.
    undo()
    expect(page.canvasX).toBe(pageStartX)
    expect(regionRect(region.id)).toEqual(rectStart)
  })

  it('drops a grabbed region from the layout payload while the page is off its URL, restores it on return', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    await settleSync()

    const payloadIds = () => getCanvasLayoutData().annotations.map((a) => a.id)
    expect(payloadIds()).toContain(region.id)

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.url = 'https://example.com/elsewhere'
    expect(payloadIds()).not.toContain(region.id)

    page.url = PAGE_URL
    expect(payloadIds()).toContain(region.id)
  })

  it('nests a grabbed region under its page in the sidebar; canvas-anchored and legacy ones stay out', async () => {
    loadHostPage()
    const grabbed = createGrabbedRegion()
    const grabless = createGrablessRegion()
    await settleSync()

    const pageChildIds = () => {
      const item = getLeftSidebarData().sections.pages.find((entry) => entry.id === PAGE_ID)
      return item && item.kind === 'page' ? (item.children ?? []).map((child) => child.id) : []
    }
    expect(pageChildIds()).toContain(grabbed.id)
    expect(pageChildIds()).not.toContain(grabless.id)
  })

  it('loads a legacy annotation (metadata.pageUrl, no pageAnchor) as canvas-bound: never hides, never travels, never nests', async () => {
    const legacy: Annotation = {
      id: 'ann-legacy',
      anchor: { type: 'region', canvasRect: { x: 150, y: 150, width: 60, height: 40 } },
      author: 'user',
      text: 'legacy region',
      status: 'pending',
      replies: [],
      createdAt: '2026-01-01T00:00:00Z',
      metadata: {
        pageUrl: PAGE_URL,
        regionComponents: [{ pageId: PAGE_ID, pageName: 'Host', components: [] }],
      },
    }
    harness.loadFixture({
      name: 'Legacy annotation host',
      doc: {
        nodes: [hostPageNode()],
        edges: [],
        annotations: [legacy],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })

    const loaded = liveAnnotation('ann-legacy')
    expect(loaded).toBeDefined()
    expect(loaded?.pageAnchor).toBeUndefined()

    // Never hides — the legacy metadata URL is not a binding.
    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.url = 'https://example.com/elsewhere'
    expect(getCanvasLayoutData().annotations.map((a) => a.id)).toContain('ann-legacy')
    page.url = PAGE_URL

    // Never nests.
    const pageItem = getLeftSidebarData().sections.pages.find((entry) => entry.id === PAGE_ID)
    const childIds =
      pageItem && pageItem.kind === 'page' ? (pageItem.children ?? []).map((c) => c.id) : []
    expect(childIds).not.toContain('ann-legacy')

    // Never travels.
    initializeDrag([PAGE_ID])
    applyDragDelta([PAGE_ID], 240, 0)
    finalizeDrag()
    await settleSync()
    expect(regionRect('ann-legacy')).toEqual({ x: 150, y: 150 })
  })
})
