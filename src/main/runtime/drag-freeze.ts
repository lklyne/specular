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
 * Captures the dragged pages into a bitmap, hands it to aboveView, and
 * parks the native views hidden once aboveView has decoded and acked it.
 * aboveView sits above every page in the native stack, so the bitmap
 * covers the live view before the park lands and there is no gap between
 * "view hidden" and "raster drawn".
 *
 * No-op behind the flag, and a no-op if nothing captures. A drag that can't
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
    console.warn('[drag-freeze] aboveView never acked; staying live')
    // aboveView never acked. Stay live rather than park pages behind a
    // bitmap it can't draw.
    publish(FREEZE_TARGET, { revision, target: FREEZE_TARGET, active: false, frames: [] })
    return
  }

  console.info(`[drag-freeze] parking ${frames.length} page(s) (revision ${revision})`)
  registerFreeze(FREEZE_ID, {
    target: FREEZE_TARGET,
    pageIds: frames.map((frame) => frame.pageId),
    parking: 'hidden',
  })
  active = true
  // The bitmap already covers the live view, so the park landing a
  // debounce tick later causes no visible change.
  requestLayout()
}

/**
 * Unparks the native views, lays out synchronously so they land at their
 * final bounds, then tells aboveView to drop the bitmap. Publishing first
 * would remove the raster while the native view is still parked or at the
 * last drag tick's position, one debounce window of blank or stale
 * content. `layoutAllViews` normally stays behind `requestLayout` (I1);
 * `settleZoomGesture` makes the same exception for the same reason.
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
