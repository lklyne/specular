import type { Entity, PageEntity, StickyEntity } from "./scene";

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

// Selection affordances are host DOM, so — like a sticky body — they only show
// above a page that punches a hole for them. There's only ever one selected
// item, so a class selector matches at most one element each.
//
// The ring sits *outside* the item box, so the owning page's own webview never
// covers it; only OTHER pages stacked above need to mask it. A resize handle
// straddles the item's corner, overlapping the page's own body, so the owning
// page must mask it too — hence the two get different rules below.
export const selectionRingSelector = ".selection-ring";
export const resizeHandleSelector = ".resize-handle";

/**
 * The mask selectors a page's webview should expose: every HOST-DOM thing
 * stacked above it in the shared z-order — sticky bodies and the selected
 * item's selection chrome — so the page paints (and clicks) through to them.
 * A selection below this page is left out, so the page correctly occludes it.
 */
export const pageMaskSelectors = (
  page: PageEntity,
  stickies: readonly StickyEntity[],
  selected: Entity | null,
): string[] => {
  const selectors = stickies
    .filter((s) => s.z > page.z)
    .map((s) => itemSelector(s.id));
  if (selected && selected.z > page.z && selected.id !== page.id) {
    selectors.push(selectionRingSelector);
  }
  // The handle overlaps the page's own body, so the page masks it for its own
  // selection too — not just for items stacked above.
  if (selected && (selected.id === page.id || selected.z > page.z)) {
    selectors.push(resizeHandleSelector);
  }
  return selectors;
};

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
