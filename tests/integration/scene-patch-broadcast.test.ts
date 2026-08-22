/**
 * The scene bus (diffed-runtime-store, phase 2): a layout pass sends the cells
 * that changed, not the scene.
 *
 * Two claims, and the second is what makes the first safe to ship. A structural
 * edit after a snapshot goes out as a patch batch naming the entity that
 * appeared — no `layoutUpdate`, no re-serialized scene. And a renderer store
 * that has drifted, for whatever reason, is put back exactly right by the next
 * snapshot: patches are allowed to be lossy only because that reconcile is
 * unconditional.
 *
 * Mutation-verified by:
 * - having `broadcastSceneUpdate` always call `broadcastSceneSnapshot` — the
 *   "patches, not a snapshot" assertions fail;
 * - dropping the entity loop from `diffRuntimeStores` — the new-entity patch
 *   assertion fails;
 * - making the renderer store's `applySnapshot` keep what it holds instead of
 *   replacing — the reconcile test fails;
 * - dropping `baseline = applyRuntimePatch(...)` from `broadcastRuntimePatch` —
 *   the "hover already delivered is not re-sent" assertion fails;
 * - returning `data` unfiltered from `filterSceneSnapshot` — the routing
 *   assertions fail (canvas-bg and above-view see the inspect tree again).
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type { RuntimePatchBatch } from '../../src/shared/runtime-patch'
import type { LayoutUpdateData } from '../../src/shared/types'
import { snapshotToStore } from '../../src/shared/runtime-store'
import { filterSceneSnapshot } from '../../src/shared/runtime-store-filter'
import { createRuntimeStore } from '../../src/renderer/shared/runtime-store'
import { aboveView, bgView } from '../../src/main/runtime/view-refs'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import {
  broadcastSceneSnapshot,
  broadcastSceneUpdate,
} from '../../src/main/runtime/runtime-patch-broadcast'
import { setHoveredPage } from '../../src/main/runtime/runtime-core'
import { createTextEntity } from '../../src/main/runtime/text-entity-state'

let harness: WorkspaceHarness

/** What one renderer received, in order. Each is routed its own slices, so a
 *  target has to be named rather than sampled. */
function sends(channel: string, webContentsId: number) {
  return harness.broadcasts.filter(
    (b) => b.channel === channel && b.webContentsId === webContentsId,
  )
}

const aboveViewId = () => aboveView!.webContents.id
const canvasBgId = () => bgView!.webContents.id

function batches(webContentsId = aboveViewId()): RuntimePatchBatch[] {
  return sends(ipcChannels.runtimePatch, webContentsId).map(
    (send) => send.args[0] as RuntimePatchBatch,
  )
}

function snapshots(webContentsId = aboveViewId()): LayoutUpdateData[] {
  return sends(ipcChannels.layoutUpdate, webContentsId).map(
    (send) => send.args[0] as LayoutUpdateData,
  )
}

/** Seat a baseline the way a connecting renderer does, then watch what the
 *  next pass sends. */
function seatBaseline(): LayoutUpdateData {
  const payload = getCanvasLayoutData()
  broadcastSceneSnapshot(payload)
  harness.clearBroadcasts()
  return payload
}

describe('scene patch broadcast', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('sends the whole scene on connect', () => {
    broadcastSceneSnapshot(getCanvasLayoutData())

    expect(snapshots()).toHaveLength(1)
    expect(batches()).toHaveLength(0)
  })

  it('turns a structural edit into a patch batch, not a snapshot', () => {
    seatBaseline()

    const text = createTextEntity({ canvasX: 20, canvasY: 40, text: 'patched in' })
    broadcastSceneUpdate(getCanvasLayoutData())

    expect(snapshots()).toHaveLength(0)
    expect(batches()).toHaveLength(1)

    const [batch] = batches()
    expect(batch.patches).toContainEqual(
      expect.objectContaining({ kind: 'entity', id: text.id }),
    )
    // The scene ordering admitted it; nothing else about the scene moved.
    expect(
      batch.patches.filter((patch) => patch.kind === 'slice').map((patch) => patch.slice),
    ).toEqual(['scene'])
  })

  it('says nothing when a pass changed nothing', () => {
    seatBaseline()

    broadcastSceneUpdate(getCanvasLayoutData())

    expect(harness.broadcasts).toHaveLength(0)
  })

  it('does not re-send a hover the patch channel already delivered', () => {
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'hoverable' })
    const text = createTextEntity({ canvasX: 200, canvasY: 0, text: 'other' })
    seatBaseline()

    setHoveredPage(null)
    setHoveredPage(text.id)
    harness.clearBroadcasts()
    broadcastSceneUpdate(getCanvasLayoutData())

    expect(batches()).toHaveLength(0)
  })

  it('routes the inspect tree away from the renderers that never draw it', () => {
    broadcastSceneSnapshot(getCanvasLayoutData())

    expect(snapshots(aboveViewId())[0]).not.toHaveProperty('inspect')
    expect(snapshots(canvasBgId())[0]).not.toHaveProperty('inspect')
    // The slices each one does draw still arrive.
    expect(snapshots(aboveViewId())[0]).toHaveProperty('edges')
    expect(snapshots(canvasBgId())[0]).toHaveProperty('annotations')
  })

  it('reconciles a drifted renderer store from the next snapshot', () => {
    const initial = filterSceneSnapshot(seatBaseline(), 'above-view')
    const store = createRuntimeStore(initial)

    createTextEntity({ canvasX: 20, canvasY: 40, text: 'patched in' })
    broadcastSceneUpdate(getCanvasLayoutData())

    // Drop the batch, the way a renderer that missed a send would, so the
    // store is now behind main by a whole entity.
    const missed = batches()[0]
    expect(missed.patches.length).toBeGreaterThan(0)
    harness.clearBroadcasts()

    const truth = filterSceneSnapshot(getCanvasLayoutData(), 'above-view')
    expect(store.read()).not.toEqual(snapshotToStore(truth))

    broadcastSceneSnapshot(getCanvasLayoutData())
    store.applySnapshot(snapshots()[0])

    expect(store.read()).toEqual(snapshotToStore(truth))
  })
})
