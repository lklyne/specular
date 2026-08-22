/**
 * Live bboxes for element-anchored annotation popovers (ADR 0006).
 *
 * Pages resolve the selectors they were subscribed to and report where those
 * elements sit; main holds the answer so it can ride the runtime store like any
 * other slice. Holding it here rather than in the renderer is what lets a
 * popover survive its selector going stale — the last known box stays, flagged,
 * instead of being lost with the report that failed to resolve.
 *
 * The record is rebuilt only when the map changes, so a layout pass that
 * carries it re-sends nothing.
 */

import type { AnnotationBboxReport, AnnotationLiveBbox, AnnotationLiveBboxes } from '../../shared/types'

const liveBboxes = new Map<string, AnnotationLiveBbox>()
let projection: AnnotationLiveBboxes = {}
let projectionStale = false

export function annotationLiveBboxes(): AnnotationLiveBboxes {
  if (!projectionStale) return projection
  projectionStale = false
  projection = Object.fromEntries(liveBboxes)
  return projection
}

function replace(annotationId: string, next: AnnotationLiveBbox): boolean {
  const current = liveBboxes.get(annotationId)
  if (
    current &&
    current.pageId === next.pageId &&
    current.stale === next.stale &&
    boxesEqual(current.boundingBox, next.boundingBox)
  ) {
    return false
  }
  liveBboxes.set(annotationId, next)
  projectionStale = true
  return true
}

function boxesEqual(
  a: AnnotationLiveBbox['boundingBox'],
  b: AnnotationLiveBbox['boundingBox'],
): boolean {
  if (!a || !b) return a === b
  return a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height
}

/** Fold one page's report into the map. Returns true when anything moved. */
export function applyAnnotationBboxReports(
  pageId: string,
  reports: readonly AnnotationBboxReport[],
): boolean {
  let changed = false
  for (const report of reports) {
    if (typeof report?.annotationId !== 'string') continue
    const held = liveBboxes.get(report.annotationId)
    const next: AnnotationLiveBbox = report.boundingBox
      ? { pageId, boundingBox: report.boundingBox, stale: false }
      : { pageId, boundingBox: held?.boundingBox ?? null, stale: true }
    changed = replace(report.annotationId, next) || changed
  }
  return changed
}

/**
 * Drop entries for `pageId` that the renderer no longer subscribes to. The
 * subscription set is the lifecycle signal — a popover that closed stops being
 * subscribed, and an unsubscribed bbox is state nobody can render.
 */
export function retainAnnotationBboxes(pageId: string, subscribedIds: readonly string[]): boolean {
  const keep = new Set(subscribedIds)
  let changed = false
  for (const [annotationId, entry] of liveBboxes) {
    if (entry.pageId !== pageId || keep.has(annotationId)) continue
    liveBboxes.delete(annotationId)
    projectionStale = true
    changed = true
  }
  return changed
}
