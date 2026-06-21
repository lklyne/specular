import type { PageEntity, StickyEntity } from "./scene";

// ── The heart of the spike ───────────────────────────────────────────────────
//
// An <electrobun-webview> is a native layer painted ABOVE the host DOM — host
// `z-index` can never draw over it ("the OOPIF always wins"). Mask selectors are
// the escape hatch: for each page webview we hand it a set of CSS selectors, and
// Electrobun punches a hole through that webview wherever a matching host-DOM
// element lands — every frame. The host paints (and receives clicks) through the
// hole.
//
// So "is this sticky in front of this page?" reduces to "is the sticky's
// selector in this page's mask set?" — and because mask sets are PER PAGE, the
// same sticky can punch through page A (in front of it) while page B paints over
// it (behind it). That is cross-surface, per-pair stacking from one shared
// z-order. No native z-index API required; no aboveView plane; no IPC.

export const stickySelector = (id: string): string => `[data-sticky-id="${id}"]`;
export const pageChromeSelector = (id: string): string => `[data-page-chrome="${id}"]`;

/**
 * The mask selectors a given page's webview should currently expose:
 *  - its own chrome header (host-DOM title/drag bar that floats over the page), and
 *  - every sticky stacked above it in the shared z-order.
 */
export const pageMaskSelectors = (
  page: PageEntity,
  stickies: readonly StickyEntity[],
): string[] => [
  pageChromeSelector(page.id),
  ...stickies.filter((s) => s.z > page.z).map((s) => stickySelector(s.id)),
];

/**
 * Reconcile a webview element's live mask set to `desired`, touching only what
 * changed. Kept here (pure-ish, element-in) so PageLayer stays declarative.
 */
export const syncMasks = (
  element: { maskSelectors: Set<string>; addMaskSelector(s: string): void; removeMaskSelector(s: string): void },
  desired: readonly string[],
): void => {
  const want = new Set(desired);
  for (const current of element.maskSelectors) {
    if (!want.has(current)) element.removeMaskSelector(current);
  }
  for (const selector of want) {
    if (!element.maskSelectors.has(selector)) element.addMaskSelector(selector);
  }
};
