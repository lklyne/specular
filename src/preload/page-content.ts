import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import {
  CAPTURE_SUPPRESSION_CSS,
  CAPTURE_SUPPRESSION_STYLE_ID,
} from './capture-suppression'
import type {
  AnnotationBboxSubscription,
  CommentToolPagePreviewState,
  ScrollSyncData,
} from '../shared/types'
import { PRESENCE_SCROLL_ANIMATION_MS } from '../shared/presence-timing'
import { REGION_SELECT_FULL_CONTAINMENT } from '../shared/featureFlags'

// The page-content preload still consumes the legacy `set-annotate-mode`
// channel from main: it carries an `enabled` flag plus a coarse mode
// discriminator. ADR 0006 retired the `region_select` variant — the comment
// tool now captures pointerdown in the aboveView overlay for both clicks
// and region drags, and the page no longer paints a region-select overlay
// itself. The mode remains exposed for `draw` (legacy) and as `off`.
type AnnotateOverlayMode = 'off' | 'comment' | 'draw'

import {
  getInspectableElementByNodeId,
  initComponentInspector,
} from './component-inspector'
import {
  applyCommentHoverOverlay,
  clearCommentHoverOverlay,
  queueRefreshCommentHoverOverlay,
} from './comment-hover-overlay'
import {
  queueRecomputeAnnotationBboxes,
  setAnnotationBboxSubscriptions,
} from './annotation-bbox-tracker'
import {
  buildElementPath,
  buildStructuredDomSnapshot,
  compactText,
  inspectionPayload,
  isInteractiveForSnapshot,
  isVisibleForSnapshot,
  pickContentElementAtPoint,
  rectFullyContainedInRegion,
  rectIntersectsRegion,
} from './dom-element-utils'
import { captureElementAtDocumentPoint } from './element-attachment-capture'
import {
  applyDomInspectionState,
  handleInspectFocusNode,
  hideDomInspectionOverlay,
  isDomInspectionEnabled,
  getDomInspectionLastTarget,
  emitHoveredElement,
  queueRefreshDomInspectionOverlay,
  setDomInspectionEnabled,
} from './dom-inspection'
import {
  forwardMiddleDragPan,
  forwardViewportWheel,
} from './gesture-forwarding'
import {
  applyIncomingLinkedScroll,
  clearScrollSuppression,
  queueScrollSyncBroadcast,
  seedScrollSyncBaseline,
  stopFollowerAnimation,
} from './scroll-sync-handler'

let interactive = false
let multiSelected = false
let canvasZoom = 1
let annotateEnabled = false
let captureSuppressionStyleEl: HTMLStyleElement | null = null
let cleanupBlockingOverlayListeners: (() => void) | null = null
const SELECTION_DEBUG = process.env.CANVAS_DEBUG_SELECTION === '1'
let lastReportedTextEditing = false

function setCaptureSuppression(active: boolean): void {
  if (!active) {
    captureSuppressionStyleEl?.remove()
    captureSuppressionStyleEl = null
    queueRefreshCommentHoverOverlay()
    queueRefreshDomInspectionOverlay()
    return
  }
  if (captureSuppressionStyleEl?.isConnected) return
  const style = document.createElement('style')
  style.id = CAPTURE_SUPPRESSION_STYLE_ID
  style.textContent = CAPTURE_SUPPRESSION_CSS
  document.documentElement.appendChild(style)
  captureSuppressionStyleEl = style
}

function selectionDebug(event: string, details?: Record<string, unknown>): void {
  if (!SELECTION_DEBUG) return
  console.log('[selection-debug:page-content]', {
    ts: Date.now(),
    event,
    interactive,
    domInspectionEnabled: isDomInspectionEnabled(),
    annotateEnabled,
    ...details,
  })
}

// --- Debug log forwarding ---

function serializeDebugArg(value: unknown): unknown {
  if (value instanceof Error) {
    return {
      name: value.name,
      message: value.message,
      stack: value.stack,
    }
  }
  if (
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    value === null
  ) {
    return value
  }
  try {
    return JSON.parse(JSON.stringify(value))
  } catch {
    return String(value)
  }
}

function debugLog(level: 'log' | 'warn' | 'error', ...args: unknown[]): void {
  ipcRenderer.send(ipcChannels.debugLog, {
    source: 'page-content',
    level,
    args: args.map(serializeDebugArg),
  })
}

window.addEventListener('error', (event) => {
  debugLog('error', event.message, event.filename, event.lineno, event.colno)
})

window.addEventListener('unhandledrejection', (event) => {
  debugLog('error', 'unhandledrejection', event.reason)
})

function isTypingTarget(element: Element | null): boolean {
  if (!element) return false
  const tag = element.tagName.toLowerCase()
  if (tag === 'textarea') return true
  if (tag === 'input') {
    const input = element as HTMLInputElement
    const type = input.type.toLowerCase()
    return ![
      'button',
      'checkbox',
      'color',
      'file',
      'hidden',
      'image',
      'radio',
      'range',
      'reset',
      'submit',
    ].includes(type)
  }
  return element instanceof HTMLElement && element.isContentEditable
}

function reportTextEditing(active: boolean): void {
  if (lastReportedTextEditing === active) return
  lastReportedTextEditing = active
  ipcRenderer.send(ipcChannels.canvasSetTextEditing, { active })
}

function reportCurrentTextEditing(): void {
  try {
    reportTextEditing(isTypingTarget(document.activeElement))
  } catch {
    reportTextEditing(false)
  }
}

window.addEventListener('focusin', reportCurrentTextEditing, true)
window.addEventListener('focusout', reportCurrentTextEditing, true)
window.addEventListener('focus', reportCurrentTextEditing, true)
window.addEventListener('DOMContentLoaded', reportCurrentTextEditing, true)
window.addEventListener('load', reportCurrentTextEditing, true)
window.addEventListener('blur', () => reportTextEditing(false), true)
queueMicrotask(reportCurrentTextEditing)
window.setTimeout(reportCurrentTextEditing, 50)

const originalConsole = {
  log: console.log.bind(console),
  warn: console.warn.bind(console),
  error: console.error.bind(console),
}

console.log = (...args: unknown[]) => {
  originalConsole.log(...args)
  debugLog('log', ...args)
}

console.warn = (...args: unknown[]) => {
  originalConsole.warn(...args)
  debugLog('warn', ...args)
}

console.error = (...args: unknown[]) => {
  originalConsole.error(...args)
  debugLog('error', ...args)
}

// --- Annotate mode ---
//
// ADR 0006 retired the page-side annotate-click / hover handlers. The
// comment tool now captures pointerdown in the aboveView overlay; the
// resulting element resolution comes from `query-element-at-point`
// invoked from main on pointerup-without-drag. Hover preview is painted
// by the page in response to `comment-tool-pointer-state` broadcasts (see
// below), not by the page's own mousemove listener.

function applyAnnotateState(): void {
  // Kept as a no-op so legacy call sites don't change shape; if a future
  // tool resurrects in-page annotation hover, restore the listeners here.
  if (!annotateEnabled) {
    hideDomInspectionOverlay()
  }
}

// Intercept canvas-level wheel events on page content views.
// Cmd/Ctrl + wheel (or trackpad pinch-to-zoom) should zoom the canvas, not the page.
window.addEventListener(
  'wheel',
  (e: WheelEvent) => {
    if (!e.metaKey && !e.ctrlKey) return
    e.preventDefault()
    forwardViewportWheel(e, canvasZoom)
  },
  { passive: false, capture: true }
)

// --- Selection overlay ---
// When the page is not interactive, inject an overlay that blocks native page
// input and forwards only native/page-neutral viewport affordances. Canvas
// selection, drag, resize, marquee, placement, and edge gestures are owned by
// aboveView's canvas pointer router.

function injectBlockingOverlay(): void {
  const overlayMode: 'default' = 'default'
  const existingOverlay = document.getElementById('__canvas-blocking-overlay')
  if (
    existingOverlay instanceof HTMLDivElement &&
    existingOverlay.dataset.overlayKind === 'blocking' &&
    existingOverlay.dataset.overlayMode === overlayMode
  ) {
    return
  }
  removeBlockingOverlay()
  selectionDebug('inject-blocking-overlay')

  const overlay = document.createElement('div')
  overlay.id = '__canvas-blocking-overlay'
  overlay.dataset.overlayKind = 'blocking'
  overlay.dataset.overlayMode = overlayMode
  Object.assign(overlay.style, {
    position: 'fixed',
    top: '0',
    left: '0',
    width: '100%',
    height: '100%',
    zIndex: '2147483646',
    background: 'transparent',
    cursor: 'default',
  })

  overlay.addEventListener('mousedown', (e: MouseEvent) => {
    if (e.button === 0) {
      selectionDebug('overlay-mousedown-left-suppressed', {
        clientX: e.clientX,
        clientY: e.clientY,
      })
      e.preventDefault()
      e.stopPropagation()
    }
  })

  // Middle-click pan forwarding
  let middleDrag: { screenX: number; screenY: number } | null = null

  overlay.addEventListener('mousedown', (e: MouseEvent) => {
    if (e.button !== 1) return
    e.preventDefault()
    e.stopPropagation()
    middleDrag = { screenX: e.screenX, screenY: e.screenY }
  })

  overlay.addEventListener('mousemove', (e: MouseEvent) => {
    if (!middleDrag) return
    e.preventDefault()
    e.stopPropagation()
    middleDrag = forwardMiddleDragPan(middleDrag, e)
  })

  const handleWindowMouseUp = (e: MouseEvent) => {
    if (e.button !== 1) return
    middleDrag = null
  }
  window.addEventListener('mouseup', handleWindowMouseUp)

  overlay.addEventListener('mouseleave', () => {
    middleDrag = null
  })

  // Forward wheel events to canvas operations
  overlay.addEventListener(
    'wheel',
    (e: WheelEvent) => {
      e.preventDefault()
      e.stopPropagation()
      forwardViewportWheel(e, canvasZoom)
    },
    { passive: false }
  )

  document.body.appendChild(overlay)
  cleanupBlockingOverlayListeners = () => {
    middleDrag = null
    window.removeEventListener('mouseup', handleWindowMouseUp)
  }
}

function removeBlockingOverlay(): void {
  if (cleanupBlockingOverlayListeners) {
    cleanupBlockingOverlayListeners()
    cleanupBlockingOverlayListeners = null
  }
  const overlay = document.getElementById('__canvas-blocking-overlay')
  if (overlay) {
    selectionDebug('remove-blocking-overlay')
    overlay.remove()
  }
}

function applyInteractiveState(): void {
  selectionDebug('applyInteractiveState')
  if (isDomInspectionEnabled() || annotateEnabled) {
    removeBlockingOverlay()
  } else if (interactive) {
    removeBlockingOverlay()
  } else if (multiSelected) {
    injectBlockingOverlay()
  } else {
    injectBlockingOverlay()
  }
}

// --- IPC handlers ---

ipcRenderer.on(ipcChannels.setInteractive, (_event, value: boolean) => {
  const wasInteractive = interactive
  selectionDebug('ipc:set-interactive', { value, wasInteractive })
  interactive = value
  if (interactive && !wasInteractive) {
    stopFollowerAnimation()
    clearScrollSuppression()
    seedScrollSyncBaseline()
  }
  applyInteractiveState()
})

ipcRenderer.on(ipcChannels.setCanvasZoom, (_event, value: number) => {
  canvasZoom = value
})

ipcRenderer.on(ipcChannels.setMultiSelected, (_event, value: boolean) => {
  selectionDebug('ipc:set-multi-selected', { value })
  multiSelected = value
  applyInteractiveState()
})

ipcRenderer.on(ipcChannels.setAnnotateMode, (_event, payload: { enabled?: boolean; mode?: AnnotateOverlayMode } | undefined) => {
  selectionDebug('ipc:set-annotate-mode', {
    enabled: Boolean(payload?.enabled),
    mode: payload?.mode ?? 'off',
  })
  annotateEnabled = Boolean(payload?.enabled)
  applyAnnotateState()
  applyInteractiveState()
})

ipcRenderer.on(ipcChannels.annotateClearHover, () => {
  if (!annotateEnabled) return
  hideDomInspectionOverlay()
})

ipcRenderer.on(ipcChannels.captureMode, (_event, active: boolean) => {
  setCaptureSuppression(Boolean(active))
})

// ADR 0006 — page-paints contract for the unified comment tool. Main fans
// out the latest pointer state (per-page coords; region rect intersected
// with this page's viewport) to every page on the canvas. The page paints
// outlines directly in its own DOM so they align pixel-perfectly with
// content and cost no IPC per frame. `active === false` clears.
ipcRenderer.on(
  ipcChannels.commentToolPagePreview,
  (_event, payload: CommentToolPagePreviewState | null | undefined) => {
    if (!payload || !payload.active) {
      clearCommentHoverOverlay()
      return
    }
    applyCommentHoverOverlay(payload)
  },
)

// ADR 0006 — live-bbox subscriptions for element-anchored annotation
// popovers. The renderer pushes the full per-page subscription set whenever
// it changes; the page resolves selectors against the live DOM and reports
// bboxes back via `annotation-bbox-update`.
ipcRenderer.on(
  ipcChannels.annotationBboxSubscriptions,
  (_event, payload: { subscriptions?: AnnotationBboxSubscription[] } | undefined) => {
    setAnnotationBboxSubscriptions(payload?.subscriptions ?? [])
  },
)

ipcRenderer.on(ipcChannels.setInspectionMode, (_event, payload: { enabled?: boolean } | undefined) => {
  selectionDebug('ipc:set-inspection-mode', { enabled: Boolean(payload?.enabled) })
  setDomInspectionEnabled(Boolean(payload?.enabled))
  applyInteractiveState()
  if (!isDomInspectionEnabled()) {
    ipcRenderer.send(ipcChannels.inspectNodeHover, null)
  } else if (getDomInspectionLastTarget()) {
    emitHoveredElement(getDomInspectionLastTarget())
  }
})

ipcRenderer.on(
  ipcChannels.inspectFocusNode,
  (_event, payload: { nodeId?: string | null; pin?: boolean; fromPanel?: boolean } | undefined) => {
    handleInspectFocusNode(payload, getInspectableElementByNodeId)
  },
)

ipcRenderer.on(ipcChannels.applyLinkedScroll, (_event, data: ScrollSyncData) => {
  applyIncomingLinkedScroll(data)
})

// --- MCP page inspection handlers ---

ipcRenderer.on(ipcChannels.takeDomSnapshot, (_event, payload: { requestId: string; maxDepth?: number; structured?: boolean }) => {
  const maxDepth = payload.maxDepth ?? 10

  function walkDom(element: Element, depth: number, indent: string): string {
    if (depth > maxDepth) return ''
    const rect = element.getBoundingClientRect()
    if (rect.width === 0 && rect.height === 0) return ''
    const styles = window.getComputedStyle(element)
    if (styles.display === 'none' || styles.visibility === 'hidden') return ''

    const role = element.getAttribute('role') ?? undefined
    const tagName = element.tagName.toLowerCase()
    const text = compactText(element.textContent, 80)
    const path = buildElementPath(element, 4)
    const box = `[${Math.round(rect.left)},${Math.round(rect.top)} ${Math.round(rect.width)}x${Math.round(rect.height)}]`

    let label = tagName
    if (role) label += ` role="${role}"`
    if (text) label += ` "${text}"`

    let result = `${indent}${label} ${box} path=${path}\n`

    for (const child of element.children) {
      result += walkDom(child, depth + 1, indent + '  ')
    }
    return result
  }

  const snapshot = payload.structured
    ? buildStructuredDomSnapshot(maxDepth)
    : walkDom(document.body, 0, '')
  ipcRenderer.send(ipcChannels.takeDomSnapshotResponse, { requestId: payload.requestId, data: snapshot })
})

ipcRenderer.on(
  ipcChannels.queryElementAtPoint,
  (_event, payload: { requestId: string; x: number; y: number }) => {
    // ADR 0006 — comment tool's click-vs-element resolver. See
    // pickContentElementAtPoint for the overlay-skip rationale.
    const target = pickContentElementAtPoint(payload.x, payload.y)
    if (!target) {
      ipcRenderer.send(ipcChannels.queryElementAtPointResponse, {
        requestId: payload.requestId,
        data: null,
      })
      return
    }
    ipcRenderer.send(ipcChannels.queryElementAtPointResponse, {
      requestId: payload.requestId,
      data: inspectionPayload(target),
    })
  },
)

ipcRenderer.on(
  ipcChannels.captureElementAtPoint,
  (_event, payload: { requestId: string; docX: number; docY: number }) => {
    // ADR 0030 — element attachment. Find the reference element under a
    // document point so an anchored item can track it through page reflow.
    ipcRenderer.send(ipcChannels.captureElementAtPointResponse, {
      requestId: payload.requestId,
      data: captureElementAtDocumentPoint(payload.docX, payload.docY),
    })
  },
)

ipcRenderer.on(ipcChannels.queryDomElements, (_event, payload: { requestId: string; selector: string; maxResults?: number }) => {
  const maxResults = payload.maxResults ?? 20
  let elements: Element[]
  try {
    elements = [...document.querySelectorAll(payload.selector)].slice(0, maxResults)
  } catch {
    ipcRenderer.send(ipcChannels.queryDomElementsResponse, {
      requestId: payload.requestId,
      data: { error: `Invalid selector: ${payload.selector}` },
    })
    return
  }
  const results = elements.map((el) => inspectionPayload(el))
  ipcRenderer.send(ipcChannels.queryDomElementsResponse, { requestId: payload.requestId, data: results })
})

ipcRenderer.on(
  ipcChannels.queryElementsInRect,
  (_event, payload: { requestId: string; rect: { x: number; y: number; width: number; height: number }; maxResults?: number }) => {
    const maxResults = payload.maxResults ?? 15
    const region = payload.rect
    const seen = new Set<Element>()
    const results: ReturnType<typeof inspectionPayload>[] = []

    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, {
      acceptNode(node) {
        const el = node as Element
        // FILTER_SKIP (not REJECT): a `display: contents` wrapper has a 0×0
        // bbox but its children render normally; REJECT would prune the
        // whole subtree the moment we hit such a wrapper. See the matching
        // walker in comment-hover-overlay.ts.
        if (!isVisibleForSnapshot(el)) return NodeFilter.FILTER_SKIP
        const box = el.getBoundingClientRect()
        if (!rectIntersectsRegion(box, region)) return NodeFilter.FILTER_SKIP
        if (REGION_SELECT_FULL_CONTAINMENT && !rectFullyContainedInRegion(box, region)) {
          return NodeFilter.FILTER_SKIP
        }
        if (isInteractiveForSnapshot(el)) return NodeFilter.FILTER_ACCEPT
        return NodeFilter.FILTER_SKIP
      },
    })

    let node: Node | null
    while ((node = walker.nextNode()) && results.length < maxResults) {
      const el = node as Element
      if (seen.has(el)) continue
      seen.add(el)
      results.push(inspectionPayload(el))
    }

    ipcRenderer.send(ipcChannels.queryElementsInRectResponse, { requestId: payload.requestId, data: results })
  },
)

// --- IPC handlers for main-process queries (replacing executeJavaScript) ---

ipcRenderer.on(ipcChannels.queryFavicon, () => {
  const el =
    document.querySelector('link[rel~="icon"]') ||
    document.querySelector('link[rel="shortcut icon"]')
  const href = el instanceof HTMLLinkElement ? el.href : null
  ipcRenderer.send(ipcChannels.queryFaviconResult, href)
})

ipcRenderer.on(
  ipcChannels.queryActiveElementRect,
  (_event, payload: { requestId: string }) => {
    const el = document.activeElement
    if (
      !(el instanceof HTMLElement) ||
      el === document.body ||
      el === document.documentElement
    ) {
      ipcRenderer.send(ipcChannels.queryActiveElementRectResult, {
        requestId: payload.requestId,
        data: null,
      })
      return
    }
    const rect = el.getBoundingClientRect()
    const labelText =
      el.getAttribute('aria-label') ||
      ('placeholder' in el ? el.getAttribute('placeholder') : null) ||
      ('name' in el ? el.getAttribute('name') : null) ||
      el.id ||
      (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement
        ? el.type
        : null)
    ipcRenderer.send(ipcChannels.queryActiveElementRectResult, {
      requestId: payload.requestId,
      data: {
        x: rect.left,
        y: rect.top,
        width: rect.width,
        height: rect.height,
        name: labelText || null,
      },
    })
  },
)


// Find the real scroll container under a viewport point. Next.js/v0 shells
// scroll an inner `overflow: auto` div while `document.scrollingElement`
// stays at 0, so both the scroll-command ramp and the offset broadcast must
// probe from a concrete point and walk up to the nearest scrollable ancestor.
// Defaults to the viewport center, which resolves the primary scroll
// container when there is no pointer (the offset broadcast has none).
function resolveScrollTarget(
  x: number = window.innerWidth / 2,
  y: number = window.innerHeight / 2,
): Element {
  const isScrollable = (el: Element): boolean => {
    const style = window.getComputedStyle(el)
    const overflowY = style.overflowY
    const overflowX = style.overflowX
    const canScrollY =
      /(auto|scroll|overlay)/.test(overflowY) &&
      el.scrollHeight > el.clientHeight
    const canScrollX =
      /(auto|scroll|overlay)/.test(overflowX) &&
      el.scrollWidth > el.clientWidth
    return canScrollY || canScrollX
  }
  let node: Element | null = document.elementFromPoint(x, y)
  while (node && !isScrollable(node)) node = node.parentElement
  return node || document.scrollingElement || document.documentElement
}

// Always-on absolute-pixel scroll broadcast (see docs/plans/scroll-tracking.md
// §Trap: separate from the linked-scroll `pageScrollChanged`, which carries
// progress fractions, is gated on `interactive`, and is dropped unless the
// page is linked). rAF-coalesced; sends only when the offset actually changed.
let pendingScrollOffsetFlush = 0
let lastSentScrollX = Number.NaN
let lastSentScrollY = Number.NaN
let lastSentScrollHeight = Number.NaN

// The container whose offset the broadcast reports. The document wins
// whenever it scrolls at all — the center probe exists only for shells where
// the document is pinned and an inner div scrolls. Probing first is fragile:
// whatever sits at viewport center (an open mega-menu, a modal, a hover
// panel) hijacks the offset and shifts every page-anchored region by its
// scrollTop.
function scrollOffsetSource(): Element {
  const doc = document.scrollingElement ?? document.documentElement
  if (doc.scrollHeight > doc.clientHeight) return doc
  return resolveScrollTarget()
}

function flushScrollOffset(): void {
  pendingScrollOffsetFlush = 0
  const target = scrollOffsetSource()
  // Fractional, not rounded — momentum scrolling produces sub-pixel offsets,
  // and rounding makes scroll-following overlays stair-step against the
  // smoothly compositing page.
  const scrollX = target.scrollLeft
  const scrollY = target.scrollTop
  // scrollHeight rides along so main can turn a page anchor's `offsetY`
  // fraction into a document position for scroll-to-comment (phase 4). It is a
  // property of the same container the offset comes from, so it is captured
  // here rather than in a second query.
  const scrollHeight = Math.round(target.scrollHeight)
  if (
    scrollX === lastSentScrollX &&
    scrollY === lastSentScrollY &&
    scrollHeight === lastSentScrollHeight
  ) {
    return
  }
  lastSentScrollX = scrollX
  lastSentScrollY = scrollY
  lastSentScrollHeight = scrollHeight
  ipcRenderer.send(ipcChannels.pageScrollOffset, { scrollX, scrollY, scrollHeight })
}

function queueScrollOffsetBroadcast(): void {
  if (pendingScrollOffsetFlush) return
  pendingScrollOffsetFlush = window.requestAnimationFrame(flushScrollOffset)
}

// Scroll events are the primary trigger, but app shells can unmount or
// replace their scroll container without a final scroll event (client
// navigation, virtualized lists, closing menus) — the last broadcast offset
// then sticks in main and every page-anchored region maps against a dead
// scroll position. A slow heartbeat re-reads the live DOM; the send-on-change
// dedupe above makes the quiet case free.
window.setInterval(queueScrollOffsetBroadcast, 2000)

let activeScrollToken = 0

ipcRenderer.on(
  ipcChannels.dispatchScroll,
  (
    _event,
    payload: {
      requestId: string
      x: number
      y: number
      deltaX: number
      deltaY: number
    },
  ) => {
    const target = resolveScrollTarget(payload.x, payload.y)
    if (!target) {
      ipcRenderer.send(ipcChannels.dispatchScrollResult, {
        requestId: payload.requestId,
        data: { ok: false, reason: 'no-scroll-target' },
      })
      return
    }
    const beforeLeft = target.scrollLeft
    const beforeTop = target.scrollTop

    const finish = () => {
      const afterLeft = target.scrollLeft
      const afterTop = target.scrollTop
      ipcRenderer.send(ipcChannels.dispatchScrollResult, {
        requestId: payload.requestId,
        data: {
          ok: true,
          consumed: beforeLeft !== afterLeft || beforeTop !== afterTop,
          targetTag:
            target instanceof Element
              ? target.tagName.toLowerCase()
              : 'document',
          beforeLeft,
          beforeTop,
          afterLeft,
          afterTop,
        },
      })
    }

    if (payload.deltaX === 0 && payload.deltaY === 0) {
      finish()
      return
    }

    // Supersede any in-flight ramp so back-to-back scrolls don't stack.
    const token = ++activeScrollToken
    const duration = PRESENCE_SCROLL_ANIMATION_MS
    const easeOutCubic = (t: number) => 1 - Math.pow(1 - t, 3)
    let startedAt = 0
    let appliedX = 0
    let appliedY = 0

    const tick = (now: number) => {
      if (token !== activeScrollToken) {
        finish()
        return
      }
      if (startedAt === 0) startedAt = now
      const progress = Math.min(1, (now - startedAt) / duration)
      const eased = easeOutCubic(progress)
      const targetX = payload.deltaX * eased
      const targetY = payload.deltaY * eased
      const stepX = targetX - appliedX
      const stepY = targetY - appliedY
      if (stepX !== 0 || stepY !== 0) target.scrollBy(stepX, stepY)
      appliedX = targetX
      appliedY = targetY
      if (progress < 1) {
        requestAnimationFrame(tick)
      } else {
        finish()
      }
    }
    requestAnimationFrame(tick)
  },
)

// --- Global event listeners ---

window.addEventListener(
  'scroll',
  () => {
    queueScrollSyncBroadcast(interactive)
    queueScrollOffsetBroadcast()
    queueRefreshDomInspectionOverlay()
    queueRefreshCommentHoverOverlay()
    queueRecomputeAnnotationBboxes()
  },
  { passive: true, capture: true }
)

window.addEventListener('resize', () => {
  queueScrollOffsetBroadcast()
  queueRefreshDomInspectionOverlay()
  queueRefreshCommentHoverOverlay()
  queueRecomputeAnnotationBboxes()
})

// --- Resize handle ---

function injectResizeHandle(): void {
  if (document.getElementById('__canvas-resize-handle')) return

  const handle = document.createElement('div')
  handle.id = '__canvas-resize-handle'
  Object.assign(handle.style, {
    position: 'fixed',
    bottom: '0',
    right: '0',
    width: '16px',
    height: '16px',
    cursor: 'nwse-resize',
    zIndex: '2147483647',
    background: 'transparent',
    pointerEvents: 'auto',
  })

  let dragState:
    | {
        pointerId: number
        screenX: number
        screenY: number
      }
    | null = null

  handle.addEventListener('pointerdown', (event: PointerEvent) => {
    if (event.pointerType === 'mouse' && event.button !== 0) return
    event.preventDefault()
    event.stopPropagation()
    dragState = {
      pointerId: event.pointerId,
      screenX: event.screenX,
      screenY: event.screenY,
    }
    handle.setPointerCapture(event.pointerId)
    ipcRenderer.send(ipcChannels.peekResizeStart)
  })

  const endResizeDrag = (pointerId?: number) => {
    if (!dragState) return
    if (pointerId !== undefined && dragState.pointerId !== pointerId) return
    if (handle.hasPointerCapture(dragState.pointerId)) {
      handle.releasePointerCapture(dragState.pointerId)
    }
    dragState = null
    ipcRenderer.send(ipcChannels.peekResizeEnd)
  }

  handle.addEventListener('pointermove', (event: PointerEvent) => {
    if (!dragState || dragState.pointerId !== event.pointerId) return
    const dx = event.screenX - dragState.screenX
    const dy = event.screenY - dragState.screenY
    dragState = {
      pointerId: event.pointerId,
      screenX: event.screenX,
      screenY: event.screenY,
    }
    ipcRenderer.send(ipcChannels.peekResizeMove, { dx, dy })
  })

  handle.addEventListener('pointerup', (event: PointerEvent) => {
    endResizeDrag(event.pointerId)
  })

  handle.addEventListener('pointercancel', (event: PointerEvent) => {
    endResizeDrag(event.pointerId)
  })

  handle.addEventListener('lostpointercapture', () => {
    endResizeDrag()
  })

  document.body.appendChild(handle)
}

// Inject elements on every navigation
function onDomReady(): void {
  injectResizeHandle()
  applyInteractiveState()
  applyDomInspectionState()
  applyAnnotateState()
  seedScrollSyncBaseline()
  // Emit once on load so a page that restores its scroll position (bfcache,
  // Next.js scroll restoration) starts correct even without a scroll event.
  queueScrollOffsetBroadcast()
  initComponentInspector()
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', onDomReady)
} else {
  onDomReady()
}
