/**
 * Element-attachment reflow tracker (ADR 0030).
 *
 * The page subscribes a set of DOM selectors — declared by main from the items
 * anchored to this page — and reports each resolved element's *document*
 * position back to main. Document positions are scroll-invariant, so, unlike
 * the popover bbox tracker (which reports viewport rects for scroll-follow),
 * this feeds a render-time correction for *reflow*: device-preset changes,
 * resizes, and dynamic content that move content to new document coordinates.
 *
 * Discipline mirrors annotation-bbox-tracker: recompute on flush, diff against
 * the last-sent position, and emit one batched message per flush. A selector
 * that fails to resolve is omitted — main keeps its last-known position; there
 * is no hide signal.
 *
 * This module also owns the MutationObserver that watches `document.body` for
 * layout-affecting changes. It is installed only while there are subscriptions
 * and disconnected the moment the set goes empty, so an idle page (no anchored
 * items) pays nothing.
 */

import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import type { ElementAttachmentPosition } from '../shared/types'
import { isViewportPositionedElement } from './element-attachment-capture'

// Continuous layout animation is chased at mutation-debounce granularity
// rather than per-frame (ADR 0030 accepted ceiling).
const MUTATION_DEBOUNCE_MS = 150

let activeSelectors: string[] = []
// selector -> "docX:docY" of the last position sent, for send-on-change.
let lastPositions = new Map<string, string>()
let pendingFlush = 0
let mutationObserver: MutationObserver | null = null
let mutationDebounceTimer = 0

function resolveDocumentPosition(selector: string): ElementAttachmentPosition | null {
  if (!selector) return null
  let element: Element | null = null
  try {
    element = document.querySelector(selector)
  } catch {
    return null
  }
  if (!element) return null
  const rect = element.getBoundingClientRect()
  // A collapsed (display:none / detached) element reports 0×0 — treat it as
  // unresolved so main keeps the last real position instead of snapping to 0.
  if (rect.width === 0 && rect.height === 0) return null
  // Same convention as the capture (element-attachment-capture.ts): viewport
  // rect plus document scroll = document-space top-left.
  return {
    selector,
    docX: Math.round(rect.left + window.scrollX),
    docY: Math.round(rect.top + window.scrollY),
    ...(isViewportPositionedElement(element) ? { viewportPositioned: true } : {}),
  }
}

function positionKey(position: ElementAttachmentPosition): string {
  return `${position.docX}:${position.docY}:${position.viewportPositioned === true ? 'viewport' : 'document'}`
}

function flush(): void {
  pendingFlush = 0
  const nextPositions = new Map<string, string>()
  const updates: ElementAttachmentPosition[] = []
  for (const selector of activeSelectors) {
    const resolved = resolveDocumentPosition(selector)
    if (!resolved) continue
    const key = positionKey(resolved)
    nextPositions.set(selector, key)
    if (lastPositions.get(selector) !== key) updates.push(resolved)
  }
  // Forget selectors that dropped off or stopped resolving, so a reappearance
  // re-emits its position.
  lastPositions = nextPositions
  if (!updates.length) return
  ipcRenderer.send(ipcChannels.elementAttachmentPositions, { positions: updates })
}

/** Coalesce a recompute onto the next frame. No-op with no subscriptions —
 *  document positions are scroll-invariant, so the scroll-triggered call
 *  naturally diffs to nothing when nothing reflowed. */
export function queueRecomputeElementPositions(): void {
  if (!activeSelectors.length) return
  if (pendingFlush) return
  pendingFlush = window.requestAnimationFrame(flush)
}

function handleMutations(): void {
  if (mutationDebounceTimer) return
  mutationDebounceTimer = window.setTimeout(() => {
    mutationDebounceTimer = 0
    queueRecomputeElementPositions()
  }, MUTATION_DEBOUNCE_MS)
}

function installMutationObserver(): void {
  if (mutationObserver) return
  // Body may not exist yet on early load; onDomReady reinstalls via
  // refreshElementAttachmentObserver() once it does.
  if (!document.body) return
  mutationObserver = new MutationObserver(handleMutations)
  mutationObserver.observe(document.body, {
    childList: true,
    subtree: true,
    attributes: true,
  })
}

function disconnectMutationObserver(): void {
  if (mutationObserver) {
    mutationObserver.disconnect()
    mutationObserver = null
  }
  if (mutationDebounceTimer) {
    window.clearTimeout(mutationDebounceTimer)
    mutationDebounceTimer = 0
  }
}

export function setElementAttachmentSubscriptions(next: string[]): void {
  activeSelectors = Array.isArray(next) ? [...new Set(next)] : []
  if (!activeSelectors.length) {
    disconnectMutationObserver()
    lastPositions = new Map()
    if (pendingFlush) {
      window.cancelAnimationFrame(pendingFlush)
      pendingFlush = 0
    }
    return
  }
  installMutationObserver()
  // Force a full re-emit on subscription churn so every selector posts its
  // current position even if unchanged since a prior subscription set.
  lastPositions = new Map()
  if (!pendingFlush) pendingFlush = window.requestAnimationFrame(flush)
}

/** Reinstall the observer after a document (re)load, when `document.body` may
 *  have been replaced out from under the old observer. No-op with no
 *  subscriptions. */
export function refreshElementAttachmentObserver(): void {
  if (!activeSelectors.length) return
  disconnectMutationObserver()
  installMutationObserver()
}

/** Test/inspection hook: whether the MutationObserver is currently installed. */
export function isElementAttachmentObserverInstalled(): boolean {
  return mutationObserver !== null
}
