/**
 * Shared DOM element tracker for annotation popovers and element attachments.
 *
 * The renderer subscribes a set of `{ annotationId, selector }` pairs to
 * each page that owns currently-visible element popovers. The page resolves
 * the selectors against its live DOM and broadcasts the resulting viewport
 * bboxes back to main → above-view, which uses them to position popovers
 * that track page scroll.
 *
 * Popover bboxes re-run on:
 *  - subscription churn (popover open/close, selection change)
 *  - page scroll (already-bound listener in `page-content.ts`)
 *  - page resize (ditto)
 *
 * Attachment document positions reuse the same selector-resolution seam, but
 * run only for attachment subscriptions on resize/load/debounced mutation
 * (plus scroll for fixed/sticky rails whose document position changes).
 */

import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import type {
  AnnotationBboxSubscription,
  AnnotationLiveBboxUpdate,
  ResolvedElementAttachmentPosition,
} from '../shared/types'

let activeSubscriptions: AnnotationBboxSubscription[] = []
let lastBoxes: Map<string, AnnotationLiveBboxUpdate['boundingBox']> = new Map()
let pendingFlush = 0

const MUTATION_DEBOUNCE_MS = 150
let activeElementSelectors: string[] = []
let lastElementPositions = new Map<string, string>()
let pendingElementFlush = 0
let mutationObserver: MutationObserver | null = null
let mutationDebounceTimer = 0

function resolveElement(selector: string): Element | null {
  if (!selector) return null
  try {
    const element = document.querySelector(selector)
    if (!element) return null
    const rect = element.getBoundingClientRect()
    return rect.width === 0 && rect.height === 0 ? null : element
  } catch {
    return null
  }
}

function resolveBbox(selector: string): AnnotationLiveBboxUpdate['boundingBox'] {
  const element = resolveElement(selector)
  if (!element) return null
  const rect = element.getBoundingClientRect()
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

function isViewportPositioned(element: Element): boolean {
  let current: Element | null = element
  while (current && current !== document.body) {
    const position = window.getComputedStyle(current).position
    if (position === 'fixed' || position === 'sticky') return true
    current = current.parentElement
  }
  return false
}

function resolveDocumentPosition(selector: string): ResolvedElementAttachmentPosition | null {
  const element = resolveElement(selector)
  if (!element) return null
  const rect = element.getBoundingClientRect()
  return {
    selector,
    docX: Math.round(rect.left + window.scrollX),
    docY: Math.round(rect.top + window.scrollY),
    ...(isViewportPositioned(element) ? { viewportPositioned: true } : {}),
  }
}

function elementPositionKey(position: ResolvedElementAttachmentPosition): string {
  return `${position.docX}:${position.docY}:${position.viewportPositioned === true}`
}

function flushElementPositions(): void {
  pendingElementFlush = 0
  const nextPositions = new Map<string, string>()
  const updates: Array<
    ResolvedElementAttachmentPosition | { selector: string; resolved: false }
  > = []
  for (const selector of activeElementSelectors) {
    const resolved = resolveDocumentPosition(selector)
    if (!resolved) {
      if (lastElementPositions.has(selector)) updates.push({ selector, resolved: false })
      continue
    }
    const key = elementPositionKey(resolved)
    nextPositions.set(selector, key)
    if (lastElementPositions.get(selector) !== key) updates.push(resolved)
  }
  lastElementPositions = nextPositions
  if (updates.length) {
    ipcRenderer.send(ipcChannels.elementAttachmentPositions, { positions: updates })
  }
}

export function queueRecomputeElementPositions(): void {
  if (!activeElementSelectors.length || pendingElementFlush) return
  pendingElementFlush = window.requestAnimationFrame(flushElementPositions)
}

function handleElementMutations(): void {
  if (mutationDebounceTimer) return
  mutationDebounceTimer = window.setTimeout(() => {
    mutationDebounceTimer = 0
    queueRecomputeElementPositions()
  }, MUTATION_DEBOUNCE_MS)
}

function installElementMutationObserver(): void {
  if (mutationObserver || !document.body) return
  mutationObserver = new MutationObserver(handleElementMutations)
  mutationObserver.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
  })
}

function disconnectElementMutationObserver(): void {
  mutationObserver?.disconnect()
  mutationObserver = null
  if (mutationDebounceTimer) {
    window.clearTimeout(mutationDebounceTimer)
    mutationDebounceTimer = 0
  }
}

export function setElementAttachmentSubscriptions(next: string[]): void {
  activeElementSelectors = Array.isArray(next) ? [...new Set(next)] : []
  if (!activeElementSelectors.length) {
    disconnectElementMutationObserver()
    lastElementPositions = new Map()
    if (pendingElementFlush) window.cancelAnimationFrame(pendingElementFlush)
    pendingElementFlush = 0
    return
  }
  installElementMutationObserver()
  lastElementPositions = new Map()
  if (!pendingElementFlush) {
    pendingElementFlush = window.requestAnimationFrame(flushElementPositions)
  }
}

export function refreshElementAttachmentObserver(): void {
  if (!activeElementSelectors.length) return
  disconnectElementMutationObserver()
  installElementMutationObserver()
}

export function isElementAttachmentObserverInstalled(): boolean {
  return mutationObserver !== null
}
