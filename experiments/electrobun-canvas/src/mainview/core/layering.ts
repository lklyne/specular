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

/** Every canvas item's shell carries `data-item-id`; this matches it. */
export const itemSelector = (id: string): string => `[data-item-id="${id}"]`;

/**
 * The mask selectors a page's webview should expose: every item stacked above it
 * in the shared z-order. (The page's own chrome sits in a host-DOM strip the
 * webview doesn't cover, so it needs no mask.)
 */
export const pageMaskSelectors = (
  page: PageEntity,
  stickies: readonly StickyEntity[],
): string[] =>
  stickies.filter((s) => s.z > page.z).map((s) => itemSelector(s.id));

/**
 * Reconcile a webview element's live mask set to `desired`, touching only what
 * changed. Kept here (pure-ish, element-in) so PageBody stays declarative.
 */
export const syncMasks = (
  element: {
    maskSelectors: Set<string>;
    addMaskSelector(s: string): void;
    removeMaskSelector(s: string): void;
  },
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
