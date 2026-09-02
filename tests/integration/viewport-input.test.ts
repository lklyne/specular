/**
 * PR B: viewport input applies on arrival — no 16ms input bucket.
 *
 * `applyViewportInputDelta` used to be reachable only through
 * `enqueueViewportInputDelta`, which accumulated deltas and flushed them
 * from a `setTimeout(..., 16)`. The IPC handlers now call
 * `applyViewportInputDelta` directly, and it updates the runtime camera
 * before returning — no timer in between.
 *
 * The harness's fake window reports `isDestroyed()`, so `layoutAllViews()`
 * stays dormant here (see harness.ts) and view bounds aren't observable at
 * this tier. What IS observable is the camera state `applyViewportInputDelta`
 * owns directly: `zoom` changes synchronously, before any `await`.
 *
 * Mutation-verified by reintroducing the deleted bucket (queuing the delta
 * and flushing via `setTimeout(..., 16)` instead of applying it inline) —
 * the synchronous assertion fails because `zoom` hasn't moved yet when the
 * call returns.
 */
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { applyViewportInputDelta } from '../../src/main/runtime/viewport-input'
import { setViewportCamera } from '../../src/main/runtime/viewport-control'
import { pan, zoom } from '../../src/main/runtime/runtime-context'
import { clearAllDirty, isDirty } from '../../src/main/runtime/layout-dirty'
import { layoutCache } from '../../src/main/runtime/layout-cache'
import { ipcChannels } from '../../src/shared/ipc-contract'
import type { RuntimePatchBatch } from '../../src/shared/runtime-patch'
import { aboveView, bgView } from '../../src/main/runtime/view-refs'

let harness: WorkspaceHarness

describe('viewport input applies on arrival', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  afterAll(() => harness?.dispose())

  it('updates zoom synchronously, with no 16ms bucket timer scheduled', () => {
    const setTimeoutSpy = vi.spyOn(global, 'setTimeout')
    const zoomBefore = zoom

    applyViewportInputDelta({ zoomDeltaY: -100, mouseX: 200, mouseY: 200 })

    // No await between the call and this assertion: if the delta were still
    // bucketed behind a timer, zoom would be unchanged here.
    expect(zoom).not.toBe(zoomBefore)
    expect(setTimeoutSpy).not.toHaveBeenCalledWith(expect.any(Function), 16)
  })
})

/**
 * A camera move is a camera patch, and nothing else (camera-local projection,
 * phase 2).
 *
 * Zoom used to `markDirty('canvas')`, so the same pass that positioned the
 * native views also rebuilt, diffed and fanned out the whole scene — once per
 * wheel event. Renderers project canvas-space geometry through the `camera`
 * slice, so the move is that slice: no entity changed, and the pass has no
 * scene to rebuild.
 *
 * Mutation-verified by:
 * - restoring `markDirty('toolbar', 'canvas')` in `setViewportCamera` — the
 *   zoom case's "canvas stays clean" assertion fails;
 * - dropping the `broadcastCamera()` call — the patch assertions fail;
 * - dropping `markDirty('toolbar')` — the zoom-readout assertion fails.
 */
describe('a camera move broadcasts one camera patch', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  /** Fixture loading and reset legitimately dirty the canvas and arm the
   *  debounce; reset them by hand so the camera move is the only thing
   *  observable. */
  function armLayoutWatch(): void {
    harness.clearBroadcasts()
    clearAllDirty()
    if (layoutCache.layoutTimer) {
      clearTimeout(layoutCache.layoutTimer)
      layoutCache.layoutTimer = null
    }
  }

  function cameraPatchesFor(webContentsId: number) {
    return harness.broadcasts
      .filter((b) => b.channel === ipcChannels.runtimePatch && b.webContentsId === webContentsId)
      .flatMap((send) => (send.args[0] as RuntimePatchBatch).patches)
  }

  it('patches the camera slice on a pan and leaves the scene alone', () => {
    armLayoutWatch()

    setViewportCamera(zoom, { x: pan.x + 40, y: pan.y - 25 })

    const expected = {
      kind: 'slice',
      slice: 'camera',
      value: { zoom, pan: { x: pan.x, y: pan.y }, cameraTransitionStartedAt: null },
    }
    expect(cameraPatchesFor(bgView!.webContents.id)).toEqual([expected])
    expect(cameraPatchesFor(aboveView!.webContents.id)).toEqual([expected])
    expect(isDirty('canvas')).toBe(false)
    expect(layoutCache.layoutTimer).toBeNull()
  })

  it('patches the camera slice on a zoom, dirtying only the toolbar readout', () => {
    armLayoutWatch()

    setViewportCamera(zoom * 1.5, pan)

    expect(cameraPatchesFor(bgView!.webContents.id)).toEqual([
      {
        kind: 'slice',
        slice: 'camera',
        value: { zoom, pan: { x: pan.x, y: pan.y }, cameraTransitionStartedAt: null },
      },
    ])
    // The zoom percentage is not on the scene bus, so it still rides the pass.
    expect(isDirty('toolbar')).toBe(true)
    expect(isDirty('canvas')).toBe(false)
  })
})
