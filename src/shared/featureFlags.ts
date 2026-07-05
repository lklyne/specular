/**
 * Region marquee selection mode.
 *
 * `true`  — only elements *fully contained* in the marquee are highlighted
 *           and grabbed (default; matches Figma / most design tools).
 * `false` — any element whose bbox *intersects* the marquee qualifies
 *           (legacy behavior).
 *
 * Internal A/B switch — not user-facing. Read by the page-paints region
 * preview overlay (preload/comment-hover-overlay.ts) and the commit-time
 * element extractor (preload/page-content.ts → `query-elements-in-rect`).
 */
export const REGION_SELECT_FULL_CONTAINMENT = true
