/**
 * Page-paints contract for the comment tool (ADR 0006).
 *
 * While the comment tool is active, main broadcasts a per-page snapshot of
 * the pointer state to each page on the canvas. We paint outlines directly
 * in the page's own DOM so they align pixel-perfectly with content and cost
 * zero IPC per frame:
 *
 *   - `regionRect` set → outline for every visible element whose bbox
 *     intersects the rect (multi-page marquee falls out automatically; each
 *     page paints its own contained elements).
 *   - `regionRect` null + `pointer` set → the inspect-tool overlay (blue
 *     highlight + box-model strips + label) for the deepest element under
 *     the pointer. Routed through `updateDomInspectionOverlay` so the
 *     comment tool's hover affordance stays visually identical to inspect.
 *   - `active === false` → clear all overlays.
 */

import {
  ensureDomInspectionOverlay,
  hideDomInspectionOverlay,
  updateDomInspectionOverlay,
} from './dom-inspection'
import {
  inspectionPayload,
  isVisibleForSnapshot,
  pickContentElementAtPoint,
  rectFullyContainedInRegion,
  rectIntersectsRegion,
} from './dom-element-utils'
import { isPageOverlayTarget } from './gesture-forwarding'
import type { CommentToolPagePreviewState } from '../shared/types'
import { REGION_SELECT_FULL_CONTAINMENT } from '../shared/featureFlags'

const OVERLAY_LAYER_ID = '__canvas-comment-preview-layer'
const OUTLINE_CLASS = '__canvas-comment-preview-outline'

const REGION_ELEMENT_LIMIT = 200

let overlayLayerEl: HTMLDivElement | null = null
let lastState: CommentToolPagePreviewState = {
  active: false,
  pointer: null,
  regionRect: null,
}
let pendingRefresh = 0

function ensureOverlayLayer(): HTMLDivElement {
  if (overlayLayerEl && overlayLayerEl.isConnected) return overlayLayerEl
  const layer = document.createElement('div')
  layer.id = OVERLAY_LAYER_ID
  Object.assign(layer.style, {
    position: 'fixed',
    inset: '0',
    pointerEvents: 'none',
    // Above the page-content `__canvas-blocking-overlay` (z-index
    // 2147483646) so our marquee item outlines aren't painted underneath it
    // while the comment tool keeps the page non-interactive.
    zIndex: '2147483647',
  })
  document.documentElement.appendChild(layer)
  overlayLayerEl = layer
  return layer
}

function clearMarqueeLayer(): void {
  if (!overlayLayerEl) return
  overlayLayerEl.replaceChildren()
}

function buildOutline(rect: DOMRect | { left: number; top: number; width: number; height: number }): HTMLDivElement {
  const outline = document.createElement('div')
  outline.className = OUTLINE_CLASS
  Object.assign(outline.style, {
    position: 'fixed',
    left: `${Math.round(rect.left)}px`,
    top: `${Math.round(rect.top)}px`,
    width: `${Math.max(1, Math.round(rect.width))}px`,
    height: `${Math.max(1, Math.round(rect.height))}px`,
    // Match the inspect-tool hover highlight (see updateDomInspectionOverlay)
    // so every item in the marquee reads like a hovered item.
    border: '1px dashed rgba(59, 130, 246, 0.95)',
    background: 'rgba(59, 130, 246, 0.14)',
    boxShadow: '0 0 0 1px rgba(255, 255, 255, 0.22) inset',
    pointerEvents: 'none',
    boxSizing: 'border-box',
  })
  return outline
}

function paintPointerElement(x: number, y: number, labelScale: number): void {
  clearMarqueeLayer()
  const target = pickContentElementAtPoint(x, y)
  if (!target) {
    hideDomInspectionOverlay()
    return
  }
  ensureDomInspectionOverlay()
  updateDomInspectionOverlay(target, inspectionPayload(target), { labelScale })
}

function paintRegionIntersection(region: { x: number; y: number; width: number; height: number }): void {
  hideDomInspectionOverlay()
  const layer = ensureOverlayLayer()
  layer.replaceChildren()

  // Cache the bbox computed during the walk so we don't call
  // getBoundingClientRect again when building outlines.
  const rectByElement = new WeakMap<Element, DOMRect>()
  const accepted: Element[] = []
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, {
    acceptNode(node) {
      const el = node as Element
      // Skip Specular's own page-injected UI: the blocking overlay (full
      // viewport, would dominate the marquee), comment badges, the inspect
      // overlay we paint, and our own preview layer/outlines.
      if (isPageOverlayTarget(el)) return NodeFilter.FILTER_REJECT
      // FILTER_SKIP (not REJECT) for invisibility: a `display: contents`
      // wrapper has a 0×0 bbox while its children render normally, so we
      // need to keep walking into the subtree. REJECT here would prune the
      // entire subtree the moment we hit such a wrapper — which is what
      // happened to the page body of modern React/Astro sites.
      if (!isVisibleForSnapshot(el)) return NodeFilter.FILTER_SKIP
      const box = el.getBoundingClientRect()
      if (!rectIntersectsRegion(box, region)) return NodeFilter.FILTER_SKIP
      if (REGION_SELECT_FULL_CONTAINMENT && !rectFullyContainedInRegion(box, region)) {
        return NodeFilter.FILTER_SKIP
      }
      rectByElement.set(el, box)
      return NodeFilter.FILTER_ACCEPT
    },
  })

  let node: Node | null
  while ((node = walker.nextNode()) && accepted.length < REGION_ELEMENT_LIMIT) {
    accepted.push(node as Element)
  }

  for (const el of accepted) {
    const rect = rectByElement.get(el)
    if (!rect || rect.width <= 0 || rect.height <= 0) continue
    layer.appendChild(buildOutline(rect))
  }
}

function refresh(state: CommentToolPagePreviewState): void {
  if (!state.active) {
    clearMarqueeLayer()
    hideDomInspectionOverlay()
    return
  }
  if (state.regionRect) {
    paintRegionIntersection(state.regionRect)
    return
  }
  if (state.pointer) {
    paintPointerElement(state.pointer.x, state.pointer.y, state.displayScale ?? 1)
    return
  }
  clearMarqueeLayer()
  hideDomInspectionOverlay()
}

function statesEqual(a: CommentToolPagePreviewState, b: CommentToolPagePreviewState): boolean {
  if (a.active !== b.active) return false
  if ((a.displayScale ?? 1) !== (b.displayScale ?? 1)) return false
  if ((a.pointer == null) !== (b.pointer == null)) return false
  if (a.pointer && b.pointer && (a.pointer.x !== b.pointer.x || a.pointer.y !== b.pointer.y)) {
    return false
  }
  if ((a.regionRect == null) !== (b.regionRect == null)) return false
  if (a.regionRect && b.regionRect) {
    if (
      a.regionRect.x !== b.regionRect.x ||
      a.regionRect.y !== b.regionRect.y ||
      a.regionRect.width !== b.regionRect.width ||
      a.regionRect.height !== b.regionRect.height
    ) {
      return false
    }
  }
  return true
}

/**
 * Apply the latest broadcast state. The overlay is repainted synchronously
 * — pointer events arrive at native input rate and the work per frame is
 * cheap (one elementFromPoint or one TreeWalker pass + a few DOM nodes).
 */
export function applyCommentHoverOverlay(state: CommentToolPagePreviewState): void {
  if (statesEqual(lastState, state)) return
  lastState = state
  refresh(state)
}

/**
 * Re-run the most recent paint without changing inputs. Page scroll / resize
 * shifts every element's viewport bbox, so the outline needs to follow even
 * when the broadcast hasn't changed.
 */
export function queueRefreshCommentHoverOverlay(): void {
  if (!lastState.active) return
  if (pendingRefresh) return
  pendingRefresh = window.requestAnimationFrame(() => {
    pendingRefresh = 0
    refresh(lastState)
  })
}

export function clearCommentHoverOverlay(): void {
  if (pendingRefresh) {
    window.cancelAnimationFrame(pendingRefresh)
    pendingRefresh = 0
  }
  lastState = { active: false, pointer: null, regionRect: null }
  if (overlayLayerEl) {
    overlayLayerEl.remove()
    overlayLayerEl = null
  }
  hideDomInspectionOverlay()
}
