/**
 * Live bboxes for element-anchored annotation popovers (ADR 0006) on the patch
 * bus. Main, not the renderer, holds where a subscribed element sits — which is
 * what makes the two behaviours below possible at all: a bbox whose selector
 * stopped resolving keeps its last known position and is flagged stale rather
 * than lost, and an unsubscribed popover's bbox is dropped instead of leaking
 * for the session.
 *
 * The harness has no real page preload, so the page → main report is delivered
 * by emitting the IPC on the page's fake webContents — the same event the real
 * `ipcMain.on` handler sees.
 *
 * Mutation-verified by:
 * - dropping the `broadcastRuntimePatch` call from `broadcastAnnotationBboxes`
 *   (register-comment-hover-ipc.ts) — every patch assertion fails;
 * - making `applyAnnotationBboxReports` write `boundingBox: null` for an
 *   unresolved selector instead of holding `held?.boundingBox` — the
 *   stale-holds-its-position assertion fails;
 * - dropping the `retainAnnotationBboxes` call from the subscriptions handler —
 *   the unsubscribe case fails (the entry lingers).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { ipcMain } from 'electron'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import type { AnnotationLiveBboxes } from '../../src/shared/types'
import type { RuntimePatchBatch } from '../../src/shared/runtime-patch'
import { ipcChannels } from '../../src/shared/ipc-contract'
import { pages } from '../../src/main/runtime/runtime-context'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import { registerCommentHoverIpc } from '../../src/main/ipc/register-comment-hover-ipc'

let harness: WorkspaceHarness
let commentHoverIpcRegistered = false

const PAGE_ID = 'bbox-host'
const ANNOTATION_ID = 'annotation-1'
const BOX = { x: 10, y: 20, width: 100, height: 40 }

function hostPageNode(): JsonCanvasLinkNode {
  return {
    id: PAGE_ID,
    type: 'link',
    x: 0,
    y: 0,
    width: 375,
    height: 667,
    url: 'https://example.com/bbox',
    presetIndex: 0,
  }
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Bbox host',
    doc: { nodes: [hostPageNode()], edges: [], appState: { zoom: 1, pan: { x: 0, y: 0 } } },
  })
}

function subscribe(annotationIds: string[]): void {
  ipcMain.emit(
    ipcChannels.commentToolBboxSubscriptions,
    {},
    {
      pageId: PAGE_ID,
      subscriptions: annotationIds.map((annotationId) => ({
        annotationId,
        selector: `#${annotationId}`,
      })),
    },
  )
}

function report(boundingBox: typeof BOX | null): void {
  const page = pages.find((candidate) => candidate.id === PAGE_ID)!
  ipcMain.emit(
    ipcChannels.annotationBboxUpdate,
    { sender: page.pageView.webContents },
    { updates: [{ annotationId: ANNOTATION_ID, boundingBox }] },
  )
}

/** The value of the last `annotationBboxes` patch that went out. */
function lastPatchedBboxes(): AnnotationLiveBboxes | undefined {
  const patches = harness.broadcasts
    .filter((b) => b.channel === ipcChannels.runtimePatch)
    .flatMap((b) => (b.args[0] as RuntimePatchBatch).patches)
    .filter((patch) => patch.kind === 'slice' && patch.slice === 'annotationBboxes')
  const last = patches.at(-1)
  return last?.kind === 'slice' ? (last.value as AnnotationLiveBboxes) : undefined
}

describe('annotation live bboxes on the patch bus', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    if (!commentHoverIpcRegistered) {
      registerCommentHoverIpc()
      commentHoverIpcRegistered = true
    }
    harness.reset()
    subscribe([])
    harness.clearBroadcasts()
  })

  afterAll(() => harness?.dispose())

  it('turns a page report into a patch the next snapshot agrees with', () => {
    loadHostPage()
    subscribe([ANNOTATION_ID])
    harness.clearBroadcasts()

    report(BOX)

    expect(lastPatchedBboxes()).toEqual({
      [ANNOTATION_ID]: { pageId: PAGE_ID, boundingBox: BOX, stale: false },
    })
    expect(getCanvasLayoutData().annotationBboxes).toEqual(lastPatchedBboxes())
  })

  it('holds a stale anchor at its last known position', () => {
    loadHostPage()
    subscribe([ANNOTATION_ID])
    report(BOX)
    harness.clearBroadcasts()

    report(null)

    expect(lastPatchedBboxes()).toEqual({
      [ANNOTATION_ID]: { pageId: PAGE_ID, boundingBox: BOX, stale: true },
    })
  })

  it('forgets a bbox once its popover unsubscribes', () => {
    loadHostPage()
    subscribe([ANNOTATION_ID])
    report(BOX)
    harness.clearBroadcasts()

    subscribe([])

    expect(lastPatchedBboxes()).toEqual({})
  })
})
