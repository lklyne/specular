/**
 * Element attachment (ADR 0032) capture wiring against the real runtime,
 * in-process. Placement fires a fire-and-forget preload query; the response
 * stamps `pageAnchor.element` onto the item. Coverage:
 *
 * - create-on-page stamps the element onto the runtime entity, the Y.Doc, and
 *   the .canvas file;
 * - the stamp is outside undo scope (one undo removes the whole entity, not
 *   just the element);
 * - a response for a superseded/reanchored capture is dropped (stale guard);
 * - dragging off a page drops the anchor and the attachment with it;
 * - a page-anchored region annotation captures once at creation.
 *
 * The harness has no real page preload, so the capture response never arrives
 * on its own: tests read the outgoing request's requestId from the recorded
 * `webContents.send` broadcasts and feed a synthetic response through
 * `handlePageIpcResponse`, exactly as the response IPC listener would.
 *
 * Mutation-verified by:
 * - swapping `ANCHOR_ELEMENT_CAPTURE_ORIGIN` for `'user'` in
 *   `writeAnchorElementToDoc` (element-attachment-capture.ts) — the
 *   "not an undo step" case fails (the first undo only removes the element);
 * - dropping the token/anchor guards in `stampEntityElement` — the stale-guard
 *   case fails (the superseded response stamps stale geometry);
 * - removing the `captureElementForEntity` call in `reanchorEntityById`
 *   (page-anchor-state.ts) — the create-stamps and region cases fail (no
 *   request is ever sent).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type {
  Annotation,
  JsonCanvasLinkNode,
  JsonCanvasTextNode,
} from '../../src/shared/json-canvas-types'
import {
  applyDragDelta,
  createTextEntity,
  finalizeDrag,
  initializeDrag,
} from '../../src/main/runtime/document-commands'
import { createAnnotation } from '../../src/main/workspace-annotations'
import { textEntities } from '../../src/main/runtime/text-entity-state'
import { workspaceAnnotations } from '../../src/main/runtime/space-model'
import { handlePageIpcResponse } from '../../src/main/runtime/page-ipc'
import { undo } from '../../src/main/runtime/space-undo'
import { selectNone } from '../../src/main/runtime/selection-controller'

let harness: WorkspaceHarness

const PAGE_A = 'attach-page-a'
const PAGE_A_URL = 'https://example.com/a'
const PAGE_B = 'attach-page-b'
const PAGE_B_URL = 'https://example.com/b'

const ON_PAGE_A = { canvasX: 150, canvasY: 150, width: 100, height: 100 }
const ON_PAGE_B = { canvasX: 2050, canvasY: 150, width: 100, height: 100 }

function pageNode(id: string, url: string, x: number): JsonCanvasLinkNode {
  return { id, type: 'link', x, y: 120, width: 375, height: 667, url, presetIndex: 0 }
}

function loadPages(): void {
  harness.loadFixture({
    name: 'Attachment host',
    doc: {
      nodes: [pageNode(PAGE_A, PAGE_A_URL, 120), pageNode(PAGE_B, PAGE_B_URL, 2000)],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

interface CaptureRequest {
  requestId: string
  docX: number
  docY: number
}

/** Outgoing capture requests recorded at the webContents.send seam. */
function captureRequests(): CaptureRequest[] {
  return harness.broadcasts
    .filter((b) => b.channel === 'capture-element-at-point')
    .map((b) => b.args[0] as CaptureRequest)
}

/** Feed a synthetic capture response, as the response IPC listener would. */
function respond(
  request: CaptureRequest,
  element: {
    selector: string
    docX: number
    docY: number
    viewportPositioned?: boolean
  },
): void {
  handlePageIpcResponse({ requestId: request.requestId, data: element })
}

function liveText(id: string): { pageAnchor?: { element?: unknown } } | undefined {
  return textEntities.find((entity) => entity.id === id)
}

function liveAnnotation(id: string): Annotation | undefined {
  return workspaceAnnotations.find((candidate) => candidate.id === id)
}

/** A region annotation whose marquee grabbed content from page A → docRect. */
function createGrabbedRegion(): Annotation {
  return createAnnotation({
    anchor: { type: 'region', canvasRect: { x: 150, y: 150, width: 80, height: 60 } },
    text: 'grabbed region',
    metadata: {
      regionComponents: [{ pageId: PAGE_A, pageName: 'A', components: [] }],
      regionElements: [{ pageId: PAGE_A, pageName: 'A', elements: [{}] }],
    },
  })
}

describe('element attachment capture', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('stamps the captured element on the runtime entity, the Y.Doc, and disk', async () => {
    loadPages()
    harness.clearBroadcasts()
    const sticky = createTextEntity({ ...ON_PAGE_A, text: 'anchored' })
    await settleSync()

    const requests = captureRequests()
    expect(requests).toHaveLength(1)
    const element = {
      selector: '#hero',
      docX: 12,
      docY: 34,
      viewportPositioned: true,
    }
    respond(requests[0], element)
    await settleSync()

    expect(liveText(sticky.id)?.pageAnchor?.element).toEqual(element)

    const yEntity = harness.doc.getMap('entities').get(sticky.id) as { toJSON(): Record<string, unknown> } | undefined
    expect(yEntity?.toJSON().pageAnchorElement).toEqual(element)

    const node = harness
      .diskDoc()
      ?.nodes.find((candidate) => candidate.id === sticky.id) as JsonCanvasTextNode | undefined
    expect(node?.specular?.pageAnchor?.element).toEqual(element)
  })

  it('does not add an undo step — one undo removes the whole entity', async () => {
    loadPages()
    harness.clearBroadcasts()
    const sticky = createTextEntity({ ...ON_PAGE_A, text: 'x' })
    await settleSync()

    respond(captureRequests()[0], { selector: '#a', docX: 1, docY: 2 })
    await settleSync()
    expect(liveText(sticky.id)?.pageAnchor?.element).toBeDefined()

    undo()
    // The stamp rode an untracked transaction, so the entity's creation is
    // still the top (and only) undo step: one undo removes it entirely.
    expect(liveText(sticky.id)).toBeUndefined()
  })

  it('drops a response for a capture the entity has moved past (stale guard)', async () => {
    loadPages()
    harness.clearBroadcasts()
    const sticky = createTextEntity({ ...ON_PAGE_A, text: 'roamer' })
    await settleSync()
    const staleRequest = captureRequests()[0]

    // Reanchor to page B before the page A capture comes back.
    initializeDrag([sticky.id])
    applyDragDelta([sticky.id], ON_PAGE_B.canvasX - ON_PAGE_A.canvasX, 0)
    finalizeDrag()
    await settleSync()
    expect(liveText(sticky.id)?.pageAnchor).toMatchObject({ pageId: PAGE_B })

    respond(staleRequest, { selector: '#stale', docX: 9, docY: 9 })
    await settleSync()

    // The superseded response never lands; the anchor is B's, unstamped.
    expect(liveText(sticky.id)?.pageAnchor?.element).toBeUndefined()
  })

  it('drops the attachment with the anchor when the entity is dragged off', async () => {
    loadPages()
    harness.clearBroadcasts()
    const sticky = createTextEntity({ ...ON_PAGE_A, text: 'departing' })
    await settleSync()
    respond(captureRequests()[0], { selector: '#a', docX: 1, docY: 2 })
    await settleSync()
    expect(liveText(sticky.id)?.pageAnchor?.element).toBeDefined()

    initializeDrag([sticky.id])
    applyDragDelta([sticky.id], 4000, 4000)
    finalizeDrag()
    await settleSync()

    expect(liveText(sticky.id)?.pageAnchor).toBeUndefined()
  })

  it('re-derives the attachment from restored geometry after undo', async () => {
    loadPages()
    harness.clearBroadcasts()
    const sticky = createTextEntity({ ...ON_PAGE_A, text: 'undo traveller' })
    await settleSync()
    respond(captureRequests()[0], { selector: '#on-a', docX: 10, docY: 20 })
    await settleSync()

    harness.clearBroadcasts()
    initializeDrag([sticky.id])
    applyDragDelta([sticky.id], ON_PAGE_B.canvasX - ON_PAGE_A.canvasX, 0)
    finalizeDrag()
    await settleSync()
    respond(captureRequests()[0], { selector: '#on-b', docX: 30, docY: 40 })
    await settleSync()
    expect(liveText(sticky.id)?.pageAnchor).toMatchObject({
      pageId: PAGE_B,
      element: { selector: '#on-b' },
    })

    harness.clearBroadcasts()
    undo()
    expect(liveText(sticky.id)?.pageAnchor).toMatchObject({ pageId: PAGE_A })
    const restoredCapture = captureRequests()
    expect(restoredCapture).toHaveLength(1)
    respond(restoredCapture[0], { selector: '#on-a', docX: 10, docY: 20 })
    await settleSync()
    expect(liveText(sticky.id)?.pageAnchor).toMatchObject({
      pageId: PAGE_A,
      element: { selector: '#on-a' },
    })
  })

  it('stamps a page-anchored region annotation at creation', async () => {
    loadPages()
    harness.clearBroadcasts()
    const region = createGrabbedRegion()
    expect(region.pageAnchor?.pageId).toBe(PAGE_A)
    await settleSync()

    const requests = captureRequests()
    expect(requests).toHaveLength(1)
    const element = { selector: '#section', docX: 5, docY: 6 }
    respond(requests[0], element)
    await settleSync()

    expect(liveAnnotation(region.id)?.pageAnchor?.element).toEqual(element)

    const yAnn = harness.doc.getMap('annotations').get(region.id) as { toJSON(): Record<string, unknown> } | undefined
    expect(yAnn?.toJSON().pageAnchorElement).toEqual(element)
  })
})
