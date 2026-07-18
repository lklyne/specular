/**
 * Sidebar reveal for page-anchored canvas items against the real main-process
 * IPC handler. Clicking an anchored shape in the left sidebar must restore the
 * page document it belongs to and, after that document loads, issue the same
 * smooth-scroll command used when opening a page-bound comment.
 *
 * Observable boundaries:
 * - navigation is captured by the Electron stub's `loadedUrls`;
 * - page scroll is captured at the page WebContents `send` seam.
 *
 * Mutation-verified by removing the page-anchor reveal call from the
 * `canvasRevealEntity` handler: the URL and dispatch-scroll assertions fail.
 */

import { ipcMain } from 'electron'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { registerCanvasEntityIpc } from '../../src/main/ipc/register-canvas-entity-ipc'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type {
  JsonCanvasLinkNode,
  JsonCanvasShapeNode,
} from '../../src/shared/json-canvas-types'
import { pages } from '../../src/main/runtime/runtime-context'
import { handlePageIpcResponse } from '../../src/main/runtime/page-ipc'

let harness: WorkspaceHarness
let registered = false

const PAGE_ID = 'page-host'
const SHAPE_ID = 'anchored-shape'
const ANCHOR_URL = 'https://example.com/original'
const CURRENT_URL = 'https://example.com/elsewhere'

describe('left-sidebar reveal of a page-anchored canvas item', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    if (!registered) {
      registerCanvasEntityIpc()
      registered = true
    }
    harness.loadFixture({
      name: 'Anchored item on another page document',
      doc: {
        nodes: [
          {
            id: PAGE_ID,
            type: 'link',
            x: 100,
            y: 100,
            width: 375,
            height: 667,
            url: CURRENT_URL,
            presetIndex: 0,
          } as JsonCanvasLinkNode,
          {
            id: SHAPE_ID,
            type: 'shape',
            shapeKind: 'rectangle',
            x: 140,
            y: 420,
            width: 120,
            height: 80,
            pageAnchor: {
              pageId: PAGE_ID,
              pageUrl: ANCHOR_URL,
              scrollX: 0,
              scrollY: 900,
            },
          } as JsonCanvasShapeNode,
        ],
        edges: [],
        appState: { zoom: 1, pan: { x: 0, y: 0 } },
      },
    })
    harness.clearBroadcasts()
  })

  afterAll(() => harness?.dispose())

  it('restores the anchor URL and scrolls to the item after load', async () => {
    const page = pages.find((candidate) => candidate.id === PAGE_ID)
    expect(page).toBeDefined()
    page!.pageView.webContents.loadedUrls.length = 0

    ipcMain.emit(
      ipcChannels.canvasRevealEntity,
      {},
      { entityId: SHAPE_ID, entityKind: 'shape' },
    )

    expect(page!.pageView.webContents.loadedUrls).toEqual([ANCHOR_URL])

    page!.pageView.webContents.emit('did-finish-load')
    await Promise.resolve()

    const scroll = harness.broadcasts.find(
      (record) =>
        record.webContentsId === page!.pageView.webContents.id &&
        record.channel === ipcChannels.dispatchScroll,
    )
    expect(scroll).toBeDefined()

    const payload = scroll?.args[0] as
      | { requestId?: string; deltaY?: number }
      | undefined
    // documentY = shape canvasY 420 - page bodyY 100 + anchor scrollY 900
    // = 1220. The common reveal ramp places it 1/3 down the 667px viewport.
    expect(payload?.deltaY).toBeCloseTo(1220 - 667 / 3)
    if (payload?.requestId) {
      handlePageIpcResponse({ requestId: payload.requestId, data: { ok: true } })
    }
  })
})
