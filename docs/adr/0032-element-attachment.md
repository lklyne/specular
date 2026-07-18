# ADR 0032 — Element attachment (anchored items track page content through reflow)

**Status:** Accepted
**Date:** 2026-07-17
**Related:** [ADR 0031 — Page-anchored entities](./0031-page-anchored-entities.md) (supersedes its "no stored offset" clause and its scroll-tracking follow-up); [ADR 0006 — Unified comment tool](./0006-unified-comment-tool.md) (element anchors pioneered the selector + live-requery pattern this generalizes).

## Context

Page-anchored items (shapes, stickies, drawings, page-anchored region
annotations) position themselves geometrically: canvas coordinates plus a
scroll reference, or a rect in the page's document space (`docRect`). Geometry
tracks the page frame and its scroll perfectly — but not its **content**.
When the page reflows (focus-mode preset → fill, viewport resize, dynamic
content, a dev-server reload), elements move to different document
coordinates and the ink stays where the content *used* to be.

Element-anchored comments don't have this problem: they store a DOM selector
and re-query the live element's position. That principle — "attached to what's
on the page, not to where it was" — matches the user's mental model for
*everything* anchored to a page.

ADR 0031 decided "positions stay in canvas coordinates; the anchor stores no
offset." That keeps canvas coordinates the universal currency every subsystem
speaks (hit-testing, drag, undo, `.canvas` files, agent tooling), and this ADR
does not reverse it. But its implicit corollary — that geometry alone is
enough to keep an item glued to content — does not survive reflow.

## Decision

A `PageAnchor` may carry an **element attachment**:

```ts
PageAnchor {
  pageId, pageUrl?,
  scrollX?, scrollY?,           // scroll reference (existing)
  element?: {
    selector: string            // DOM selector for the reference element
    docX: number, docY: number  // element's document position at capture
    viewportPositioned?: true   // inside a fixed/sticky containing rail
  }
}
```

**Canvas coordinates stay authoritative; the attachment is a render-time
correction.** Stored truth remains `canvasX/Y` (and `docRect` for regions).
Scene builders apply *(element's live document position − recorded
`docX/docY`)* as a shift, exactly like the existing scroll-follow shift, and
the correction folds into stored coordinates on reanchor (the same rebase
pattern). An unresolvable selector means zero correction — the item renders at
its stored geometry, never hides. Attachment is **never a visibility gate**;
the URL gate (ADR 0031) is untouched and decides *whether* an item shows,
while the element decides *where*.

**Capture is derived, not chosen.** At creation and drag-end, a fire-and-forget
preload query finds the item's reference element and stamps the attachment
onto the anchor; anchoring itself stays synchronous and geometric. The capture
rule: hit-test the item's center, walk up past trivial wrappers to the nearest
meaningful element (id/role/classes, reasonable size); if the center hits
nothing, take the horizontally-nearest element at that document Y; final
fallback is `body` — which means "track document top" and degrades to plain
scroll-follow, so every item always captures something. **One element per
item, translation-only tracking**: a stroke or region moves as a unit with its
element; it never stretches or re-wraps with internal reflow.

**Re-resolution is event-driven, not per-frame.** Document positions don't
change on scroll (the overlay band already handles scroll), so the page
preload re-resolves subscribed selectors only on real reflow events: viewport
resize, document (re)load, and a debounced `MutationObserver` on `body`. It
broadcasts document positions per page in one message; main applies them as
corrections and marks layout dirty. This extends the existing annotation
bbox tracker into one tracker with two consumers (popover bboxes, attachment
corrections) rather than adding a parallel pipeline.

**Fixed and sticky containing rails opt out of the generic fast scroll
transform.** Capture walks the selected element's ancestors and stamps
`viewportPositioned` when any establishes `position: fixed` or
`position: sticky`. Their authoritative document-position correction already
accounts for scroll; applying the overlay band's fast document-scroll residual
as well would briefly move the item twice before reconciliation. These items
remain clipped to their page but follow only the authoritative element
projection. Ordinary document-flow attachments retain the fast path.

**The attachment is outside undo scope.** The user never chose the element —
placement chose it — so the machine's memo of "what was under this item" does
not occupy a slot in undo history (same reasoning as viewport zoom/pan).
Undo restores position; re-capture re-derives the attachment from the
restored position. The attachment *is* persisted to `.canvas` files (it must
survive restarts).

**Consumers:**

| Item | Position model |
|---|---|
| Shapes, stickies, drawings | canvas coords + scroll stamp + element correction |
| Page-anchored regions (`docRect`) | docRect + element correction (capture at creation only — a region's binding is written once, ADR 0031 amendment; the attachment is tracking, not binding) |
| Element comments | unchanged — already element-attached; tracker unifies underneath |
| Page-offset comment badges (`offsetY`) | unchanged — deliberately positional ("this far down the document"), not content-bound |
| Canvas points, grab-less regions, free entities | unchanged — never page-anchored |

## Sync model: the follower contract

Pages render in their own WebContentsViews, composited by the OS; anchored
overlay ink renders in a separate WCV. Electron cannot atomically commit two
WCVs in one display frame, so every cross-surface sync is an event pipeline
and **the two surfaces can never be frame-locked**. The architectural floor is
a constant one-frame lag on whichever surface is downstream of the input —
zero is not reachable without moving both into one compositor.

Two consequences shape every anchored-position design:

- **Whichever surface is closest to the input moves first; the other chases.**
  On page scroll the page's compositor leads and anchored overlays chase. On
  pan/zoom the overlay renderer originates the gesture and the WCVs chase.
  The lag is a property of being downstream, not of any particular surface.
- **Constant lag reads as attached; oscillating lag reads as jitter.** The
  quality target for a follower pipeline is minimal *variance*, not minimal
  latency — and relative sync between the two surfaces matters more to the
  eye than absolute latency to the input.

The pattern this codebase has converged on — independently for pan (#264) and
for scroll (the overlay band fast path) — is the **follower contract**: the
downstream surface applies a cheap local transform immediately from a live
signal, and reconciles to authoritative geometry when the full (debounced)
layout broadcast lands, resetting the local offset. Stored truth never moves
off main; the local transform is transient and reconciled away on every
broadcast. New anchored-position work should extend this contract, not invent
a parallel sync path. This ADR's element correction rides the same seams: it
shifts stored geometry at scene-build time and folds on reanchor, leaving the
per-frame scroll path untouched.

Pipeline-tightening work (constant-latency scroll path, pan timer collapse
and phase-matching, snapshot-zoom) is tracked in
[#340](https://github.com/lklyne/specular/issues/340), with the companion
CPU-cost work in [#265](https://github.com/lklyne/specular/issues/265). That
work is transport-layer and lands after this refactor; nothing in this ADR
depends on it.

## Alternatives considered

- **Element-authoritative storage** (store only selector + offset, derive
  canvas coords at render). Purer domain statement, but makes the coordinate
  system async for every consumer, and a stale selector leaves the item with
  no position at all — forcing exactly the show/hide behavior this design
  refuses. Rejected.
- **Gate on viewport width** (stamp content width into the anchor, hide items
  while the live width differs — the URL-gate pattern). Cheap and honest, but
  adds another show/hide behavior outside the user's mental model. Rejected
  by product decision.
- **Per-scroll-frame element tracking** (rAF/scroll-driven re-query like open
  popovers). Unnecessary — document positions are scroll-invariant, and the
  band's compositor transform already tracks scroll smoothly. Continuous
  layout animation is chased at mutation-debounce granularity (~150ms)
  instead; accepted ceiling.
- **Multi-point attachment** (per-stroke or per-corner element refs so ink
  re-wraps with content). Large cost for a case users can fix by redrawing.
  Rejected for v1; translation-only is the documented ceiling.

## Consequences

- ADR 0031's "no stored offset" clause is narrowed: the anchor still stores
  no *positional authority* outside canvas space, but it now stores derived
  element metadata. Its scroll-tracking follow-up ("entities stay pinned to
  the page frame") is superseded — all anchored entities scroll-follow and,
  with this ADR, reflow-follow.
- The live-bbox tracker becomes always-on for pages with attached items
  (previously only while a popover was open), bounded by reflow events.
- Selector quality bounds tracking quality: heavily dynamic class names
  degrade to `body`-level tracking. Accepted; the fallback chain makes
  degradation continuous rather than a cliff.
