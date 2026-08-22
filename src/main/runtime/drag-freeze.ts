import type { FrozenPageFrame } from '../../shared/types'
import { DRAG_FREEZE } from './runtime-constants'
import { findPageById } from './runtime-context'
import type { Page } from './runtime-entities'
import {
  capturePageFrame,
  nextRevision,
  publish,
  registerFreeze,
  releaseFreeze,
  waitForRendererReady,
} from './page-freeze'
import { layoutAllViews } from './layout-engine'
import { requestLayout } from './viewport-control'

const FREEZE_TARGET = 'above'
const FREEZE_ID = 'drag'

/**
 * Bumped on every begin/end. `beginDragFreeze`'s capture is async; if the
 * drag already ended (or a new one began) by the time it resolves, the
 * generation it captured is stale and it must not register a freeze for a
 * gesture nobody is running anymore.
 */
let generation = 0
let active = false

/**
 * Captures the dragged page(s) into a bitmap, hands it to aboveView, and —
 * once aboveView has decoded and acked it — parks the native views hidden.
 * The bitmap is on screen (occluding the still-visible live view, since
 * aboveView sits above every page in the native stack) before the park
 * lands, so there is no gap between "view hidden" and "raster drawn".
 *
 * No-op behind the flag, and a no-op if nothing captures: a drag that can't
 * be pictured stays live rather than parking pages behind a blank raster.
 */
export async function beginDragFreeze(pageIds: string[]): Promise<void> {
  if (!DRAG_FREEZE) return
  const gen = ++generation

  const pages = pageIds
    .map((id) => findPageById(id))
    .filter((page): page is Page => page !== undefined)
  const frames = (
    await Promise.all(pages.map((page) => capturePageFrame(page)))
  ).filter((frame): frame is FrozenPageFrame => frame !== null)
  if (frames.length === 0) return
  if (gen !== generation) return // the drag ended (or a new one began) mid-capture

  const revision = nextRevision(FREEZE_TARGET)
  publish(FREEZE_TARGET, { revision, target: FREEZE_TARGET, active: true, frames })
  const ready = await waitForRendererReady(FREEZE_TARGET, revision)
  if (gen !== generation) return

  if (!ready) {
    // aboveView never acked — stay live rather than park pages behind a
    // bitmap it can't draw.
    publish(FREEZE_TARGET, { revision, target: FREEZE_TARGET, active: false, frames: [] })
    return
  }

  registerFreeze(FREEZE_ID, {
    target: FREEZE_TARGET,
    pageIds: frames.map((frame) => frame.pageId),
    parking: 'hidden',
  })
  active = true
  // Debounced is fine here: the bitmap already occludes the live view, so
  // the native park landing a tick later than this call causes no visible
  // change.
  requestLayout()
}

/**
 * Ends the drag freeze: unparks the native views, lays out synchronously so
 * they land at their final bounds, then tells aboveView to drop the bitmap.
 *
 * Order matters. Publishing the empty state first would remove the raster
 * that's occluding the page while its native view is still mid-flight to
 * its final bounds (parked, or wherever the last drag tick left it) —
 * one 16ms debounce window of blank/incorrect content. Running the layout
 * pass synchronously first, then publishing, keeps the raster up until the
 * native view is already where it needs to be.
 *
 * `layoutAllViews` normally stays behind the debounced `requestLayout`
 * (I1); this follows the existing exception in `settleZoomGesture`
 * (viewport-control.ts), which needs the identical synchronous-bounds-
 * before-reveal ordering for the zoom settle reveal.
 */
export function endDragFreeze(): void {
  generation += 1 // invalidate any beginDragFreeze capture still in flight
  if (!active) return
  active = false
  releaseFreeze(FREEZE_ID)
  layoutAllViews()
  const revision = nextRevision(FREEZE_TARGET)
  publish(FREEZE_TARGET, { revision, target: FREEZE_TARGET, active: false, frames: [] })
}
