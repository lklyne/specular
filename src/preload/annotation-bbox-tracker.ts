/**
 * Live-bbox round-trip for element-anchored annotation popovers (ADR 0006).
 *
 * The renderer subscribes a set of `{ annotationId, selector }` pairs to
 * each page that owns currently-visible element popovers. The page resolves
 * the selectors against its live DOM and broadcasts the resulting viewport
 * bboxes back to main → above-view, which uses them to position popovers
 * that track page scroll.
 *
 * The tracker re-runs on:
 *  - subscription churn (popover open/close, selection change)
 *  - page scroll (already-bound listener in `page-content.ts`)
 *  - page resize (ditto)
 */

import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import type { AnnotationBboxSubscription, AnnotationLiveBboxUpdate } from '../shared/types'

let activeSubscriptions: AnnotationBboxSubscription[] = []
let lastBoxes: Map<string, AnnotationLiveBboxUpdate['boundingBox']> = new Map()
let pendingFlush = 0

function resolveBbox(selector: string): AnnotationLiveBboxUpdate['boundingBox'] {
  if (!selector) return null
  let element: Element | null = null
  try {
    element = document.querySelector(selector)
  } catch {
    return null
  }
  if (!element) return null
  const rect = element.getBoundingClientRect()
  if (rect.width === 0 && rect.height === 0) return null
  return {
    x: Math.round(rect.left),
    y: Math.round(rect.top),
    width: Math.round(rect.width),
    height: Math.round(rect.height),
  }
}

function bboxKey(box: AnnotationLiveBboxUpdate['boundingBox']): string {
  if (!box) return 'null'
  return `${box.x}:${box.y}:${box.width}:${box.height}`
}

function flush(): void {
  pendingFlush = 0
  const nextBoxes = new Map<string, AnnotationLiveBboxUpdate['boundingBox']>()
  const updates: Array<{ annotationId: string; boundingBox: AnnotationLiveBboxUpdate['boundingBox'] }> = []
  for (const sub of activeSubscriptions) {
    const next = resolveBbox(sub.selector)
    nextBoxes.set(sub.annotationId, next)
    const prev = lastBoxes.has(sub.annotationId) ? lastBoxes.get(sub.annotationId) ?? null : null
    if (bboxKey(prev) !== bboxKey(next)) {
      updates.push({ annotationId: sub.annotationId, boundingBox: next })
    }
  }
  // Forget bboxes for ids that are no longer subscribed.
  lastBoxes = nextBoxes
  if (!updates.length) return
  ipcRenderer.send(ipcChannels.annotationBboxUpdate, { updates })
}

export function queueRecomputeAnnotationBboxes(): void {
  if (!activeSubscriptions.length) return
  if (pendingFlush) return
  pendingFlush = window.requestAnimationFrame(flush)
}

export function setAnnotationBboxSubscriptions(subscriptions: AnnotationBboxSubscription[]): void {
  activeSubscriptions = Array.isArray(subscriptions) ? subscriptions : []
  // Drop any cached bboxes whose ids fell off so the next flush re-emits them
  // if they reappear later.
  if (!activeSubscriptions.length) {
    lastBoxes = new Map()
    if (pendingFlush) {
      window.cancelAnimationFrame(pendingFlush)
      pendingFlush = 0
    }
    return
  }
  // Force a full emit on subscription churn — clear cached bbox keys so every
  // active subscription posts its current bbox even if it hasn't changed.
  lastBoxes = new Map()
  if (!pendingFlush) {
    pendingFlush = window.requestAnimationFrame(flush)
  }
}
