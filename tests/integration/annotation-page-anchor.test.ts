/**
 * Page-anchored annotations (`Annotation.pageAnchor`, shared/page-anchor.ts)
 * against the real runtime, in-process. Regions split at creation with no
 * mode: a marquee that grabbed page content (non-empty regionComponents/
 * regionElements) binds to the grab's page and stores its rect in that page's
 * *document* space (`anchor.docRect`) — so it scroll-follows and travels with
 * page drags for free (the transform moves; nothing translates the anchor),
 * hides while the page is off its anchor URL, nests under the page in the
 * sidebar, and persists the anchor to disk. A grab-less region is
 * canvas-anchored: it stores `canvasRect`, never moves with pages, never
 * hides. Annotations from older files without a `pageAnchor` (even ones
 * carrying the retired `metadata.pageUrl`) load fine and behave as
 * canvas-bound, keeping their `canvasRect`.
 *
 * Mutation-verified by:
 * - making `anchoredRequestAnchor` return the incoming anchor unchanged
 *   (skip the canvasRect→docRect conversion) in `createAnnotationInternal`
 *   (workspace-annotations.ts) — the docRect-on-creation, scroll-follow, and
 *   drag-tracking cases fail (the region stays a `canvasRect` variant);
 * - dropping the `- (page.scrollY ?? 0)` term in `regionCanvasRect`
 *   (page-anchor-state.ts) — the scroll-follow case fails;
 * - freezing `regionCanvasRect`'s `body.x` to the creation origin instead of
 *   the live `pageBodyCanvasBounds(page)` — the drag-tracking case fails;
 * - removing the `...(pageAnchor ? { pageAnchor } : {})` spread in
 *   `createAnnotationInternal` — the grab-rule, persistence, and sidebar
 *   cases fail;
 * - replacing the `hiddenByPageAnchor` annotations filter in
 *   `buildCanvasLayoutData` (canvas-layout-data.ts) with
 *   `[...workspaceAnnotations]` — the navigation cases fail;
 * - dropping the annotations half of `sidebarPageChildren`
 *   (sidebar-builder.ts) — the sidebar-nesting case fails.
 *
 * `translateAnnotationsAnchoredToPage` is gone: a docRect is page-relative, so
 * page drag/nudge no longer touches the anchor. The single-undo-step guarantee
 * now rides the page move alone (gesture batching).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import type { Annotation, WorkspaceBounds } from '../../src/shared/types'
import { createAnnotation } from '../../src/main/workspace-annotations'
import {
  applyDragDelta,
  finalizeDrag,
  initializeDrag,
  nudgeSelection,
} from '../../src/main/runtime/document-commands'
import { regionCanvasRect } from '../../src/main/runtime/page-anchor-state'
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

/**
 * A region annotation whose marquee grabbed content from the host page. The
 * marquee is at canvas x140/y140 w80/h60; the page body sits at canvas
 * x120/y120 with scroll 0, so the stored docRect is x20/y20/w80/h60.
 */
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

/** The page-relative document rect of a page-anchored region. Throws for a
 *  canvas-anchored region — the split is the whole point. */
function regionDocRect(id: string): WorkspaceBounds {
  const annotation = liveAnnotation(id)
  if (annotation?.anchor.type !== 'region' || !('docRect' in annotation.anchor)) {
    throw new Error(`not a page-anchored region: ${id}`)
  }
  return annotation.anchor.docRect
}

/** Where a region sits on the canvas *now* — canvasRect as-is for
 *  canvas-anchored, the docRect inverted through the live page for
 *  page-anchored. Mirrors what main-side consumers read. */
function regionCanvasXY(id: string): { x: number; y: number } {
  const annotation = liveAnnotation(id)
  if (!annotation) throw new Error(`missing annotation: ${id}`)
  const rect = regionCanvasRect(annotation)
  if (!rect) throw new Error(`no canvas rect for region: ${id}`)
  return { x: rect.x, y: rect.y }
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

  it('stores a grabbed region as a page-relative docRect and binds it to the grab page; persists both to doc and disk', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    // The canvas marquee is converted to the page's document space at creation.
    expect('docRect' in region.anchor).toBe(true)
    expect(regionDocRect(region.id)).toEqual({ x: 20, y: 20, width: 80, height: 60 })
    expect(region.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL })
    // The retired metadata binding is not written.
    expect(region.metadata?.pageUrl).toBeUndefined()
    await settleSync()

    const docRecord = harness.doc.getMap('annotations').get(region.id) as
      | { toJSON(): Record<string, unknown> }
      | undefined
    const docJson = docRecord?.toJSON()
    expect(docJson?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL })
    expect(docJson?.anchor).toEqual({
      type: 'region',
      docRect: { x: 20, y: 20, width: 80, height: 60 },
    })

    const diskRecord = (harness.diskDoc()?.annotations as Annotation[] | undefined)?.find(
      (candidate) => candidate.id === region.id,
    )
    expect(diskRecord?.pageAnchor).toEqual({ pageId: PAGE_ID, pageUrl: PAGE_URL })
    expect(diskRecord?.anchor).toEqual({
      type: 'region',
      docRect: { x: 20, y: 20, width: 80, height: 60 },
    })
  })

  it('does not bind a region that intersected a page but grabbed nothing', () => {
    // The region select emits a group per intersecting page even when its
    // inner list is empty — overlap alone is not a grab.
    // Mutation-verified: reverting annotationAnchorPageId's region branch to
    // `regionComponents?.[0]?.pageId ?? regionElements?.[0]?.pageId` fails this.
    loadHostPage()
    const region = createAnnotation({
      anchor: { type: 'region', canvasRect: { x: 140, y: 140, width: 80, height: 60 } },
      text: 'overlapped but empty-handed',
      metadata: {
        regionComponents: [{ pageId: PAGE_ID, pageName: 'Host', components: [] }],
        regionElements: [{ pageId: PAGE_ID, pageName: 'Host', elements: [] }],
      },
    })
    expect(region.pageAnchor).toBeUndefined()
    // No grab → keeps its canvas rect, no docRect.
    expect(region.anchor.type === 'region' && 'canvasRect' in region.anchor).toBe(true)
  })

  it('scroll-follows: the region tracks page scroll while its stored docRect is unchanged', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    await settleSync()

    // Before scroll the region sits at its marquee canvas position.
    expect(regionCanvasXY(region.id)).toEqual({ x: 140, y: 140 })

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.scrollY = 200
    // The document rect is scroll-independent; the canvas position moves up by
    // the scroll delta (transform-side), so nothing rewrote the anchor.
    expect(regionDocRect(region.id)).toEqual({ x: 20, y: 20, width: 80, height: 60 })
    expect(regionCanvasXY(region.id)).toEqual({ x: 140, y: 140 - 200 })

    page.scrollY = 0
    expect(regionCanvasXY(region.id)).toEqual({ x: 140, y: 140 })
  })

  it('travels with its page drag without rewriting docRect: one Y.Doc transaction, one undo step', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    await settleSync()

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    const pageStartX = page.canvasX
    const docBefore = regionDocRect(region.id)
    const canvasBefore = regionCanvasXY(region.id)

    const transactions = await observeTransactions(() => {
      initializeDrag([PAGE_ID])
      applyDragDelta([PAGE_ID], 140, 60)
      applyDragDelta([PAGE_ID], 100, -60)
      finalizeDrag()
    })
    // Only the page moves — the docRect is page-relative, so the whole gesture
    // is one forward-sync transaction (gesture batching), hence one undo step.
    expect(transactions).toBe(1)

    expect(page.canvasX - pageStartX).toBe(240)
    // Anchor untouched; canvas position tracks the page.
    expect(regionDocRect(region.id)).toEqual(docBefore)
    expect(regionCanvasXY(region.id)).toEqual({ x: canvasBefore.x + 240, y: canvasBefore.y })

    undo()
    expect(pages.find((candidate) => candidate.id === PAGE_ID)!.canvasX).toBe(pageStartX)
    expect(regionDocRect(region.id)).toEqual(docBefore)
    expect(regionCanvasXY(region.id)).toEqual(canvasBefore)

    redo()
    expect(pages.find((candidate) => candidate.id === PAGE_ID)!.canvasX).toBe(pageStartX + 240)
    expect(regionCanvasXY(region.id)).toEqual({ x: canvasBefore.x + 240, y: canvasBefore.y })
  })

  it('travels with a keyboard nudge of its page without rewriting docRect', async () => {
    loadHostPage()
    const region = createGrabbedRegion()
    await settleSync()
    const docBefore = regionDocRect(region.id)
    const canvasBefore = regionCanvasXY(region.id)
    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    const pageStartX = page.canvasX

    selectEntities([PAGE_ID])
    nudgeSelection(5, -7)
    await settleSync()

    expect(page.canvasX).toBe(pageStartX + 5)
    expect(regionDocRect(region.id)).toEqual(docBefore)
    expect(regionCanvasXY(region.id)).toEqual({ x: canvasBefore.x + 5, y: canvasBefore.y - 7 })

    // The nudge round-trips as one undo step (the page move alone).
    undo()
    expect(page.canvasX).toBe(pageStartX)
    expect(regionCanvasXY(region.id)).toEqual(canvasBefore)
  })

  it('leaves a grab-less region canvas-anchored: keeps canvasRect, ignores scroll and page drag', async () => {
    loadHostPage()
    const region = createGrablessRegion()
    expect(region.pageAnchor).toBeUndefined()
    expect(region.anchor.type === 'region' && 'canvasRect' in region.anchor).toBe(true)
    await settleSync()

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.scrollY = 300
    expect(regionCanvasXY(region.id)).toEqual({ x: 900, y: 900 })
    page.scrollY = 0

    initializeDrag([PAGE_ID])
    applyDragDelta([PAGE_ID], 240, 0)
    finalizeDrag()
    await settleSync()
    expect(regionCanvasXY(region.id)).toEqual({ x: 900, y: 900 })

    page.url = 'https://example.com/elsewhere'
    expect(getCanvasLayoutData().annotations.map((a) => a.id)).toContain(region.id)
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

  it('nests a grabbed region under its page in the sidebar; canvas-anchored ones stay out', async () => {
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
    // Loads with its canvasRect intact — no migration to docRect.
    expect(loaded?.anchor.type === 'region' && 'canvasRect' in loaded.anchor).toBe(true)

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
    expect(regionCanvasXY('ann-legacy')).toEqual({ x: 150, y: 150 })
  })
})
