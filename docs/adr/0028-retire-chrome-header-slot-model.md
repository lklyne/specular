# ADR 0028 — Retire the chrome-header slot model

**Status:** Accepted — 2026-07-07
**Supersedes:** [ADR 0002 §1](./0002-canvas-anchored-overlay-ui.md) ("Shape B: entity rect includes its chrome")
**Fixes:** [#312](https://github.com/lklyne/specular/issues/312)

## Context

ADR 0002 §1 proposed **Shape B**: an entity's rect is one layout unit — body plus a
`CHROME_HEADER_HEIGHT` (36px) header band stacked above it — so a `CanvasItemChrome`
component could render a URL bar / nav buttons / file header anchored to the band.

That component was never built. What shipped instead is `CanvasItemPopup`: a
selection-driven popup (`PagePopup`, `FilePopup`) anchored to the **body** rect. The
header slot ended up rendering nothing, yet its machinery stayed:

- `hit-test.ts` emitted a `chrome` hit target for the 36px band above pages/files and
  ranked it above `body`, so an item overlapping that band routed to the page beneath
  it (#312 — the invisible drag-steal band). A `hitSelectedDrawingBody` special case
  existed only to let selected drawings beat that band.
- The 36px extension leaked into `pageVisualBounds`, `computeScreenBoundsForPage`
  (`chrome`/`chromeH`/`chromeY`), placement (`extendUpwardForChrome`,
  `entityDataInsetsById`), and `useAnchoredPosition`'s extend-then-split.
- "Which kinds have chrome" had already forked into three disagreeing definitions
  (`ENTITY_KIND_CAPS.hasChrome` = page/file; `entityChromeSlots` and
  `useAnchoredPosition` = page/file/group).

The band also caused a second bug: **fill/focus mode click offset**. Fill mode pins the
page WCV to `focusFillRegion()` (a fixed rect under the focus bar), but pointer
forwarding subtracted a *camera-derived* origin (`computeScreenBoundsForPage(page).page`)
whose fill camera had centered the **chrome-inflated** `pageVisualBounds`. The two rects
diverged, so forwarded clicks landed offset by `TOOLBAR_HEIGHT − CHROME_HEADER_HEIGHT/2`.

## Decision

**Entity rect == body rect. Delete the chrome-header slot model wholesale.**

- Remove the `chrome` hit layer, payload, collector, `chromeRect`, `entityHasChrome`,
  `hitSelectedDrawingBody`, and `ENTITY_KIND_CAPS.hasChrome`.
- Delete `entity-chrome-slots.ts` (`CHROME_HEADER_HEIGHT`, `entityChromeSlots`,
  `ChromeSlot*`) and its re-exports.
- `pageVisualBounds` hugs the snap rect; drop `computeScreenBoundsForPage`'s chrome
  fields, `frameChromeCanvasBounds`, `extendUpwardForChrome`, and `entityDataInsetsById`.
- `useAnchoredPosition` returns the plain body rect; `AnchorSlot` and `CanvasItemPopup`'s
  `slot` prop collapse away.

**One source of truth for the page's on-screen rect.** Pointer forwarding subtracts the
WCV's *actual placed bounds* (`pageView.getBounds()`) — the same rect the layout pass
set — instead of re-deriving it. Render and forward can no longer diverge in any mode
(normal, fit/device focus, or fill), so the offset is structurally impossible.

## Consequences

- The #312 drag-steal band is gone: pages drag via their body, and an item overlapping a
  page's top edge wins by normal front-to-back z-order — selected or not.
- Camera fit-to-page hugs the body; marquee selects a page by touching the body.
- Group rename is unaffected — it was always driven by `GroupRenameLabel`'s own DOM
  `onDoubleClick`, not the (now-removed) chrome dblclick route. The dead
  `enter-group-rename` router action is removed with it.
- Popup positioning is unchanged — every popup already anchored to `body`.

Any future canvas-anchored persistent UI should anchor to the body rect via
`useAnchoredPosition` and tag itself `data-overlay-ui`; there is no header band to fill.
