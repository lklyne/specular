/**
 * Element-attachment reflow pipeline (ADR 0030, plan step 3) against the real
 * runtime, in-process. Two legs:
 *
 * - Broadcast round-trip: a page → main `element-attachment-positions` message
 *   updates `page.elementPositions` (keyed by selector) and marks the canvas
 *   dirty; a no-op repeat changes nothing.
 * - Subscription push: stamping an element attachment pushes the selector to
 *   the page over `element-attachment-subscriptions`; deleting the item drops
 *   it from the pushed set.
 *
 * The harness has no real page preload, so — as in element-attachment.test.ts —
 * the capture response is fed synthetically through `handlePageIpcResponse`,
 * and the page → main position message is delivered by emitting the IPC on the
 * page's fake webContents (the same event the real `ipcMain.on` handler sees).
 *
 * Mutation-verified by:
 * - dropping the `page.elementPositions = map` assignment in the
 *   `element-attachment-positions` handler (register-page-chrome-ipc.ts) — the
 *   round-trip case fails (positions never land);
 * - removing `requestAttachmentSubscriptionRefresh()` from `stampEntityElement`
 *   (element-attachment-capture.ts) — the push case fails (no subscription
 *   broadcast carries the stamped selector);
 * - dropping the `mutateWorkspace` refresh call — the delete-drops-it case
 *   fails (the selector lingers in the pushed set).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { ipcMain } from 'electron'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import {
  createTextEntity,
  deleteTextEntity,
} from '../../src/main/runtime/document-commands'
import { textEntities } from '../../src/main/runtime/text-entity-state'
import { pages } from '../../src/main/runtime/runtime-context'
import { handlePageIpcResponse } from '../../src/main/runtime/page-ipc'
import { registerPageChromeIpc } from '../../src/main/ipc/register-page-chrome-ipc'
import { isDirty, consumeDirty } from '../../src/main/runtime/layout-dirty'
import { selectNone } from '../../src/main/runtime/selection-controller'

let pageChromeIpcRegistered = false

let harness: WorkspaceHarness

const PAGE_ID = 'reflow-page'
const PAGE_URL = 'https://example.com/reflow'
const ON_PAGE = { canvasX: 150, canvasY: 150, width: 100, height: 100 }

function pageNode(): JsonCanvasLinkNode {
  return { id: PAGE_ID, type: 'link', x: 120, y: 120, width: 375, height: 667, url: PAGE_URL, presetIndex: 0 }
}

function loadPage(): void {
  harness.loadFixture({
    name: 'Reflow host',
    doc: { nodes: [pageNode()], edges: [], appState: { zoom: 1, pan: { x: 0, y: 0 } } },
  })
}

function livePage() {
  const page = pages.find((candidate) => candidate.id === PAGE_ID)
  if (!page) throw new Error(`no page: ${PAGE_ID}`)
  return page
}

interface CaptureRequest {
  requestId: string
}

/** The capture request main fired at placement, read off the send seam. */
function captureRequest(): CaptureRequest {
  const record = harness.broadcasts.find((b) => b.channel === 'capture-element-at-point')
  if (!record) throw new Error('no capture request was sent')
  return record.args[0] as CaptureRequest
}

/** Selector sets pushed to the page, in order. */
function pushedSelectorSets(): string[][] {
  return harness.broadcasts
    .filter((b) => b.channel === 'element-attachment-subscriptions')
    .map((b) => (b.args[0] as { selectors: string[] }).selectors)
}

/** Deliver a page → main reflow position message, exactly as `ipcRenderer.send`
 *  from the page would surface it on `ipcMain` (event.sender = the page view). */
function emitPositions(positions: Array<{ selector: string; docX: number; docY: number }>): void {
  const page = livePage()
  ipcMain.emit(
    'element-attachment-positions',
    { sender: page.pageView.webContents },
    { positions },
  )
}

/** Create an anchored sticky and stamp it with a captured element selector. */
async function createAnchoredSticky(selector: string): Promise<string> {
  const sticky = createTextEntity({ ...ON_PAGE, text: 'anchored' })
  await settleSync()
  handlePageIpcResponse({
    requestId: captureRequest().requestId,
    data: { selector, docX: 12, docY: 34 },
  })
  await settleSync()
  return sticky.id
}

describe('element-attachment reflow pipeline', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    // Register the page-chrome IPC handlers once against the shared ipcMain
    // stub so the page → main position message routes to the real handler.
    if (!pageChromeIpcRegistered) {
      registerPageChromeIpc()
      pageChromeIpcRegistered = true
    }
    harness.reset()
    selectNone()
  })

  afterAll(() => harness?.dispose())

  it('stores broadcast document positions on the runtime page and marks canvas dirty', async () => {
    loadPage()
    consumeDirty('canvas')

    emitPositions([
      { selector: '#hero', docX: 40, docY: 800 },
      { selector: '#footer', docX: 40, docY: 3200 },
    ])

    const page = livePage()
    expect(page.elementPositions?.get('#hero')).toEqual({ docX: 40, docY: 800 })
    expect(page.elementPositions?.get('#footer')).toEqual({ docX: 40, docY: 3200 })
    expect(isDirty('canvas')).toBe(true)

    // A repeat of the same positions is a no-op — nothing to re-render.
    consumeDirty('canvas')
    emitPositions([{ selector: '#hero', docX: 40, docY: 800 }])
    expect(isDirty('canvas')).toBe(false)

    // A moved element updates in place and re-dirties.
    emitPositions([{ selector: '#hero', docX: 40, docY: 900 }])
    expect(page.elementPositions?.get('#hero')).toEqual({ docX: 40, docY: 900 })
    expect(isDirty('canvas')).toBe(true)
  })

  it('pushes a stamped selector to the page and drops it when the item is deleted', async () => {
    loadPage()
    harness.clearBroadcasts()

    const stickyId = await createAnchoredSticky('#hero')

    const pushes = pushedSelectorSets()
    expect(pushes.length).toBeGreaterThan(0)
    expect(pushes.at(-1)).toEqual(['#hero'])

    harness.clearBroadcasts()
    deleteTextEntity(stickyId)
    await settleSync()
    expect(textEntities.find((entity) => entity.id === stickyId)).toBeUndefined()

    const afterDelete = pushedSelectorSets()
    expect(afterDelete.length).toBeGreaterThan(0)
    expect(afterDelete.at(-1)).toEqual([])
  })
})
