import { PAGE_SURFACE_ARM_KEY, parsePageSurfaceArm, type PageSurfaceArm } from '../../../shared/page-surface-spike'

/** Reads the spike arm once. Wrapped in try/catch for the same reason the
 *  preload's read is: this can run in an opaque-origin frame. */
export function readPageSurfaceArm(): PageSurfaceArm {
  try {
    return parsePageSurfaceArm(window.localStorage.getItem(PAGE_SURFACE_ARM_KEY))
  } catch {
    return parsePageSurfaceArm(null)
  }
}
