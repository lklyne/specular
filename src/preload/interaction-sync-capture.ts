import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import { describeElementForLocator } from './dom-element-utils'
import { isPageOverlayTarget } from './gesture-forwarding'
import type { LocatorBundle } from '../shared/locator-kernel'
import type { InteractionSyncEvent } from '../shared/types'

// Hover rebroadcasts only when the hovered element changes or the
// within-element offset fraction moves by more than this (ADR 0030 D7,
// mirrors `hasMeaningfulScrollDelta`'s shape in scroll-sync-handler.ts).
// Small enough to keep a peer's `:hover` menu tracking the cursor, large
// enough that sub-pixel jitter doesn't trigger a resolve round-trip on every
// frame.
const HOVER_OFFSET_EPSILON = 0.015

let captureEnabled = false
let hoverFrameQueued = false
let pendingHoverEvent: MouseEvent | null = null
let hasSentHover = false
let lastHoverElement: Element | null = null
let lastHoverOffsetX = 0
let lastHoverOffsetY = 0
let lastHoverViewportX = 0
let lastHoverViewportY = 0
// Keyed implicitly by `lastHoverElement`: the bundle built the last time the
// hovered element itself changed. Offset-only moves within one element patch
// this instead of re-walking the DOM (innerText forces layout).
let lastHoverBundle: LocatorBundle | null = null

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value))
}

// Shadow-DOM targets live behind `event.target` (retargeted to the host at
// each shadow boundary); `composedPath()[0]` is the true deepest target.
// Specular's own injected chrome (resize handle, comment layers, blocking
// overlay) is never real page content, so it's treated as no target.
function composedTargetElement(event: Event): Element | null {
  const path = event.composedPath()
  const first = path.length > 0 ? path[0] : event.target
  if (!(first instanceof Element)) return null
  return isPageOverlayTarget(first) ? null : first
}

function offsetWithin(
  element: Element,
  clientX: number,
  clientY: number,
): { offsetX: number; offsetY: number } {
  const rect = element.getBoundingClientRect()
  return {
    offsetX: rect.width > 0 ? clamp01((clientX - rect.left) / rect.width) : 0.5,
    offsetY: rect.height > 0 ? clamp01((clientY - rect.top) / rect.height) : 0.5,
  }
}

function viewportFraction(clientX: number, clientY: number): { viewportX: number; viewportY: number } {
  return {
    viewportX: window.innerWidth > 0 ? clamp01(clientX / window.innerWidth) : 0,
    viewportY: window.innerHeight > 0 ? clamp01(clientY / window.innerHeight) : 0,
  }
}

function buildLocatorBundle(element: Element, offsetX: number, offsetY: number): LocatorBundle {
  const descriptor = describeElementForLocator(element)
  return {
    id: descriptor.id ?? undefined,
    testId: descriptor.testId ?? undefined,
    role: descriptor.role ?? undefined,
    name: descriptor.name ?? undefined,
    text: descriptor.text ?? undefined,
    tag: descriptor.tag,
    elementPath: descriptor.elementPath,
    fullPath: descriptor.fullPath,
    offsetX,
    offsetY,
  }
}

function sendInteractionSyncEvent(event: InteractionSyncEvent): void {
  ipcRenderer.send(ipcChannels.interactionSyncEvent, event)
}

function resetHoverState(): void {
  pendingHoverEvent = null
  hasSentHover = false
  lastHoverElement = null
  lastHoverOffsetX = 0
  lastHoverOffsetY = 0
  lastHoverViewportX = 0
  lastHoverViewportY = 0
  lastHoverBundle = null
}

/** Toggle capture (ADR 0030 D1). Disabled is the default; everything is
 *  dropped while disabled, and disabling mid-hover clears the change-gate
 *  baseline so re-enabling never compares against stale state. */
export function setInteractionSyncCaptureEnabled(enabled: boolean): void {
  captureEnabled = enabled
  if (!captureEnabled) resetHoverState()
}

function flushHover(): void {
  hoverFrameQueued = false
  if (!captureEnabled || !pendingHoverEvent) return
  const event = pendingHoverEvent
  const target = composedTargetElement(event)
  const { viewportX, viewportY } = viewportFraction(event.clientX, event.clientY)

  if (!target) {
    // No element under the cursor, but the cursor itself can still be
    // sweeping across the region (e.g. page padding) — keep a peer's
    // proportional glide live instead of freezing it at the last real
    // element's position.
    const viewportMoved =
      Math.abs(viewportX - lastHoverViewportX) > HOVER_OFFSET_EPSILON ||
      Math.abs(viewportY - lastHoverViewportY) > HOVER_OFFSET_EPSILON
    if (hasSentHover && lastHoverElement === null && !viewportMoved) return
    hasSentHover = true
    lastHoverElement = null
    lastHoverBundle = null
    lastHoverViewportX = viewportX
    lastHoverViewportY = viewportY
    sendInteractionSyncEvent({ kind: 'hover', bundle: null, viewportX, viewportY })
    return
  }

  const { offsetX, offsetY } = offsetWithin(target, event.clientX, event.clientY)
  const elementChanged = target !== lastHoverElement
  const offsetMoved =
    Math.abs(offsetX - lastHoverOffsetX) > HOVER_OFFSET_EPSILON ||
    Math.abs(offsetY - lastHoverOffsetY) > HOVER_OFFSET_EPSILON
  if (hasSentHover && !elementChanged && !offsetMoved) return

  hasSentHover = true
  lastHoverElement = target
  lastHoverOffsetX = offsetX
  lastHoverOffsetY = offsetY
  lastHoverViewportX = viewportX
  lastHoverViewportY = viewportY
  const bundle: LocatorBundle =
    !elementChanged && lastHoverBundle
      ? { ...lastHoverBundle, offsetX, offsetY }
      : buildLocatorBundle(target, offsetX, offsetY)
  lastHoverBundle = bundle
  sendInteractionSyncEvent({ kind: 'hover', bundle, viewportX, viewportY })
}

/** rAF-coalesced: multiple mousemoves within a frame collapse to one
 *  evaluation, and the change-gate above decides whether that evaluation
 *  actually broadcasts (D7 — never send per raw mousemove). */
export function handleInteractionSyncPointerMove(event: MouseEvent): void {
  if (!captureEnabled) return
  pendingHoverEvent = event
  if (hoverFrameQueued) return
  hoverFrameQueued = true
  window.requestAnimationFrame(flushHover)
}

/** Only primary-button, trusted clicks ship — main decides everything else
 *  (confidence, origin gating, dispatch). `isTrusted` excludes synthetic
 *  `dispatchEvent` clicks; it does not exclude agent-driven CDP input, which
 *  is filtered upstream by the capture-enable gate (D1) instead. */
export function handleInteractionSyncClick(event: MouseEvent): void {
  if (!captureEnabled) return
  if (!event.isTrusted || event.button !== 0) return
  const target = composedTargetElement(event)
  if (!target) return
  const { offsetX, offsetY } = offsetWithin(target, event.clientX, event.clientY)
  const { viewportX, viewportY } = viewportFraction(event.clientX, event.clientY)
  const bundle = buildLocatorBundle(target, offsetX, offsetY)
  sendInteractionSyncEvent({ kind: 'click', bundle, viewportX, viewportY })

  // A queued rAF hover flush (from a mousemove just before this click) must
  // not ship after the click and re-anchor the peer elsewhere (A3). Drop it
  // and re-prime the change-gate at the click's own position so the next
  // real hover is compared against where the click actually landed.
  pendingHoverEvent = null
  hasSentHover = true
  lastHoverElement = target
  lastHoverOffsetX = offsetX
  lastHoverOffsetY = offsetY
  lastHoverViewportX = viewportX
  lastHoverViewportY = viewportY
  lastHoverBundle = bundle
}
