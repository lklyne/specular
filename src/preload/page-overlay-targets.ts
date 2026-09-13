const PAGE_OVERLAY_ROOT_SELECTORS = [
  '[data-overlay-ui]',
  '#__canvas-comment-preview-layer',
  '#__canvas-blocking-overlay',
  '#__canvas-resize-handle',
  '#__canvas-select-fallback',
  // `elementsFromPoint` returns elements regardless of `pointer-events: none`,
  // so the comment tool's click resolver would otherwise land on these
  // Specular-painted overlays instead of the page element underneath.
  '[id^="__canvas-dom-inspection-"]',
]

export function isPageOverlayTarget(target: Element | null): boolean {
  if (!target) return false
  return PAGE_OVERLAY_ROOT_SELECTORS.some((selector) => target.closest(selector))
}
