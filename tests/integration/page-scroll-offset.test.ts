/**
 * ADR 0029 scroll tracking: a page's absolute
 * scroll offset in raw CSS pixels rides the layout broadcast. The page preload
 * reports `{ scrollX, scrollY }` on a dedicated always-on channel, main stores
 * it on the ephemeral runtime page (`page.scrollX` / `page.scrollY`), and the
 * `page` scene entity carries it so downstream phases can scroll-follow.
 *
 * The first block exercises the runtime→scene-entity leg the same way the IPC
 * handler does: set the offset on the runtime page, then read it back off the
 * layout broadcast's page entity.
 *
 * The second covers the wire. A scroll report goes out as a `pageScroll` patch
 * and, when nothing in the scene is bound to that page's document, costs no
 * layout pass at all — the pass would only recompute what the patch already
 * carries. Bind an item to the page and the pass comes back, because it has
 * real work: folding the scroll delta into the anchored item's geometry.
 *
 * Mutation-verified by:
 * - dropping `scrollY: page.scrollY ?? 0` from the `backgroundPageOverlays`
 *   scene-entity literal (canvas-layout-data.ts) — the scrollY assertion fails;
 * - dropping `scrollX: page.scrollX ?? 0` from the same literal — the scrollX
 *   assertion fails;
 * - restoring the unconditional `markDirty('canvas') + requestLayout()` in the
 *   `page-scroll-offset` handler — the "no layout pass" assertion fails;
 * - having `pageScrollMovesScene` always return false — the anchored-item case
 *   fails (the pass that repositions it never runs);
 * - dropping the `broadcastRuntimePatch` call from the same handler — the patch
 *   and convergence assertions fail.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { ipcMain } from 'electron'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import type { RuntimePatchBatch } from '../../src/shared/runtime-patch'
import { snapshotToStore } from '../../src/shared/runtime-store'
import { ipcChannels } from '../../src/shared/ipc-contract'
import { createRuntimeStore } from '../../src/renderer/shared/runtime-store'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import { pages } from '../../src/main/runtime/runtime-context'
import { registerPageChromeIpc } from '../../src/main/ipc/register-page-chrome-ipc'
import { consumeDirty, isDirty } from '../../src/main/runtime/layout-dirty'
import {
  broadcastSceneSnapshot,
  broadcastSceneUpdate,
} from '../../src/main/runtime/runtime-patch-broadcast'
import { createTextEntity } from '../../src/main/runtime/document-commands'

let harness: WorkspaceHarness

const PAGE_ID = 'page-scroll-host'
const PAGE_URL = 'https://example.com/long'

function hostPageNode(): JsonCanvasLinkNode {
  return {
    id: PAGE_ID,
    type: 'link',
    x: 100,
    y: 100,
    width: 375,
    height: 667,
    url: PAGE_URL,
    presetIndex: 0,
  }
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Scroll offset host',
    doc: {
      nodes: [hostPageNode()],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

function pageSceneEntity(id: string): { scrollX: number; scrollY: number } {
  const entity = getCanvasLayoutData().entities.find(
    (candidate) => candidate.kind === 'page' && candidate.id === id,
  )
  if (!entity || entity.kind !== 'page') throw new Error(`no page entity: ${id}`)
  return { scrollX: entity.scrollX, scrollY: entity.scrollY }
}

let pageChromeIpcRegistered = false

/** Deliver a page → main scroll report, exactly as `ipcRenderer.send` from the
 *  page surfaces it on `ipcMain` (event.sender = the page view). */
function emitScroll(scrollX: number, scrollY: number, scrollHeight = 5000): void {
  const page = pages.find((candidate) => candidate.id === PAGE_ID)!
  ipcMain.emit(
    ipcChannels.pageScrollOffset,
    { sender: page.pageView.webContents },
    { scrollX, scrollY, scrollHeight },
  )
}

function patchBatches(): RuntimePatchBatch[] {
  const all = harness.broadcasts.filter((b) => b.channel === ipcChannels.runtimePatch)
  const target = all[0]?.webContentsId
  return all
    .filter((send) => send.webContentsId === target)
    .map((send) => send.args[0] as RuntimePatchBatch)
}

describe('page scroll offset in the layout broadcast', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    if (!pageChromeIpcRegistered) {
      registerPageChromeIpc()
      pageChromeIpcRegistered = true
    }
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('defaults to zero and then carries the runtime page offset', () => {
    loadHostPage()
    expect(pageSceneEntity(PAGE_ID)).toEqual({ scrollX: 0, scrollY: 0 })

    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.scrollX = 42
    page.scrollY = 1337

    expect(pageSceneEntity(PAGE_ID)).toEqual({ scrollX: 42, scrollY: 1337 })
  })

  it('resets the offset when the page navigates to a new document', () => {
    loadHostPage()
    const page = pages.find((candidate) => candidate.id === PAGE_ID)!
    page.scrollX = 42
    page.scrollY = 1337
    page.scrollHeight = 5000

    page.pageView.webContents.emit('did-navigate', {}, 'https://example.com/other')

    expect(pageSceneEntity(PAGE_ID)).toEqual({ scrollX: 0, scrollY: 0 })
    expect(page.scrollHeight).toBe(0)
  })
})

describe('page scroll offset on the patch bus', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    if (!pageChromeIpcRegistered) {
      registerPageChromeIpc()
      pageChromeIpcRegistered = true
    }
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('sends a pageScroll patch and no layout pass when nothing is bound to the page', () => {
    loadHostPage()
    broadcastSceneSnapshot(getCanvasLayoutData())
    consumeDirty('canvas')
    harness.clearBroadcasts()

    emitScroll(0, 900)

    expect(patchBatches()).toHaveLength(1)
    expect(patchBatches()[0].patches).toEqual([
      { kind: 'slice', slice: 'pageScroll', value: { [PAGE_ID]: { scrollX: 0, scrollY: 900 } } },
    ])
    expect(isDirty('canvas')).toBe(false)
  })

  it('keeps the layout pass when an anchored item rides the page', async () => {
    loadHostPage()
    createTextEntity({ canvasX: 150, canvasY: 200, width: 100, height: 60, text: 'anchored' })
    await settleSync()
    broadcastSceneSnapshot(getCanvasLayoutData())
    consumeDirty('canvas')
    harness.clearBroadcasts()

    emitScroll(0, 900)

    expect(isDirty('canvas')).toBe(true)
  })

  it('converges with the next snapshot, and the pass does not re-send it', () => {
    loadHostPage()
    const initial = getCanvasLayoutData()
    broadcastSceneSnapshot(initial)
    const store = createRuntimeStore(initial)
    harness.clearBroadcasts()

    emitScroll(0, 900)
    for (const batch of patchBatches()) store.applyPatches(batch)
    harness.clearBroadcasts()

    const truth = getCanvasLayoutData()
    expect(store.read().slices.pageScroll).toEqual(snapshotToStore(truth).slices.pageScroll)

    broadcastSceneUpdate(truth)
    const resent = patchBatches().flatMap((batch) => batch.patches)
    expect(resent.filter((patch) => patch.kind === 'slice' && patch.slice === 'pageScroll')).toEqual([])
  })
})
