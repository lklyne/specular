/**
 * Ephemeral mutators patch a slice; they do not rebuild the scene
 * (camera-local projection, phase 3).
 *
 * Selecting an entity, switching tools, and beginning or refining a gesture
 * all used to `markDirty('canvas')`, so each one rebuilt every scene entity,
 * diffed the whole scene, and fanned it out — to deliver one cell that no
 * entity's geometry depends on. Each ships as a `runtimePatch` on the slice it
 * owns.
 *
 * The layout pass is a separate question from the scene rebuild. A mutator
 * keeps `requestLayout()` when something outside the runtime store reads what
 * it changed and is only computed inside `layoutAllViews` — `reconcileFocus`
 * and viewport culling for the interaction kind, `shouldGateBeOpen` and the
 * cursor-overlay window for the active tool. A mutator that reaches none of
 * those arms no pass at all, which is what the last case pins.
 *
 * Mutation-verified by:
 * - restoring `markDirty('canvas')` in `ui-state.setSelection` and in
 *   `beginDraggingEntities` — those cases' "canvas stays clean" assertions
 *   fail;
 * - dropping `broadcastSelectionChange()` from `commitSelection`, and
 *   `broadcastToolChange()` from `applyToolSideEffects` — the patch assertions
 *   fail for those cases;
 * - restoring `requestLayout()` in `updateEdgeDragTarget` — the live-gesture
 *   case's "no pass armed" assertion fails.
 *
 * `setSelectionOverlayRect` also stopped requesting a pass, but its body sits
 * behind a `win.isDestroyed()` guard the harness's fake window trips, so there
 * is nothing to observe at this tier.
 */
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { createPage } from '../../src/main/runtime/page-runtime'
import { selectNone, selectPageById } from '../../src/main/runtime/selection-controller'
import { setActiveTool } from '../../src/main/runtime/tool-mode'
import {
  beginDraggingEntities,
  beginEdgeDrag,
  clearInteractionState,
  updateEdgeDragTarget,
} from '../../src/main/runtime/interaction-state'
import { getCanvasLayoutData } from '../../src/main/runtime/canvas-layout-data'
import { broadcastSceneSnapshot } from '../../src/main/runtime/runtime-patch-broadcast'
import { clearAllDirty, isDirty } from '../../src/main/runtime/layout-dirty'
import { layoutCache } from '../../src/main/runtime/layout-cache'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type { RuntimePatch, RuntimePatchBatch } from '../../src/shared/runtime-patch'
import { aboveView, bgView } from '../../src/main/runtime/view-refs'

let harness: WorkspaceHarness

describe('ephemeral mutators patch a slice instead of rebuilding the scene', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    selectNone()
    clearInteractionState()
    setActiveTool({ kind: 'select' })
  })

  afterAll(() => harness?.dispose())

  /**
   * Seat the patch bus's baseline on current truth, then clear everything the
   * setup dirtied, so the mutation under test is the only thing observable.
   */
  function armWatch(): void {
    broadcastSceneSnapshot(getCanvasLayoutData())
    harness.clearBroadcasts()
    clearAllDirty()
    if (layoutCache.layoutTimer) {
      clearTimeout(layoutCache.layoutTimer)
      layoutCache.layoutTimer = null
    }
  }

  /** Slice routing is per target, so each case reads the renderer that is
   *  actually routed the slice it asserts on (`runtime-store-filter.ts`). */
  function patchesTo(webContentsId: number): RuntimePatch[] {
    return harness.broadcasts
      .filter((b) => b.channel === ipcChannels.runtimePatch && b.webContentsId === webContentsId)
      .flatMap((send) => (send.args[0] as RuntimePatchBatch).patches)
  }

  function slicesPatched(webContentsId: number): string[] {
    return patchesTo(webContentsId).map((patch) =>
      patch.kind === 'slice' ? patch.slice : `entity:${patch.id}`,
    )
  }

  const canvasBg = (): number => bgView!.webContents.id
  const overlay = (): number => aboveView!.webContents.id

  it('ships a selection as a selection patch, leaving the scene alone', () => {
    const page = createPage('https://example.com/a')
    armWatch()

    selectPageById(page.id)

    expect(slicesPatched(canvasBg())).toEqual(['selection'])
    const patch = patchesTo(canvasBg())[0]
    expect(patch.kind === 'slice' && patch.value).toMatchObject({
      selectedEntityIds: [page.id],
    })
    expect(isDirty('canvas')).toBe(false)
    // The sidebar and toolbar are not on the scene bus; they still ride the pass.
    expect(isDirty('sidebar')).toBe(true)
    expect(isDirty('toolbar')).toBe(true)
  })

  it('ships a tool switch as a tool patch, leaving the scene alone', () => {
    armWatch()

    setActiveTool({ kind: 'hand' })

    expect(slicesPatched(canvasBg())).toEqual(['tool'])
    expect(isDirty('canvas')).toBe(false)
    expect(isDirty('toolbar')).toBe(true)
  })

  it('ships a gesture beginning as an interaction patch, and keeps the pass', () => {
    const page = createPage('https://example.com/b')
    armWatch()

    beginDraggingEntities([page.id])

    expect(slicesPatched(overlay())).toEqual(['interaction'])
    expect(isDirty('canvas')).toBe(false)
    // `reconcileFocus` and viewport culling read the gesture kind, and both
    // are decided inside the pass.
    expect(layoutCache.layoutTimer).not.toBeNull()
  })

  it('refines a live gesture with an interaction patch and arms no pass', () => {
    const page = createPage('https://example.com/c')
    beginEdgeDrag({ id: page.id, kind: 'page' }, 'right')
    armWatch()

    updateEdgeDragTarget({ id: page.id, kind: 'page' }, 'left')

    expect(slicesPatched(overlay())).toEqual(['interaction'])
    expect(isDirty('canvas')).toBe(false)
    expect(layoutCache.layoutTimer).toBeNull()
  })
})
