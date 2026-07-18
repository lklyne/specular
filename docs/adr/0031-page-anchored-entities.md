# ADR 0031 — Page-anchored entities (the "hook to a page" utility)

**Status:** Accepted
**Date:** 2026-07-13
**Amended:** 2026-07-13 — annotations now consume the utility (see Amendment below).
**Related:** [ADR 0006 — Unified comment tool](./0006-unified-comment-tool.md) (annotations pioneered the URL-gating this generalizes); [ADR 0024 — Entity-kind registry](./0024-entity-kind-registry-spans-runtime-and-persistence.md) (persisted-field plumbing the anchor rides on).

## Context

Canvas items and page content live in different worlds: a sticky or a drawing
placed over a page is spatially *near* the page but semantically *about* it —
about the specific document the page showed at the time. Three consequences
users feel:

1. Moving the page leaves its commentary behind.
2. Navigating the page to a different URL leaves stale ink/notes floating
   over unrelated content.
3. The sidebar shows no relationship between a page and the items on it.

Annotations already solved (2) for themselves by recording
`metadata.pageUrl` at creation and hiding visuals when the page's current URL
diverges. That mechanism deserved to be a **generic utility** any canvas
entity can use.

## Decision

A canvas entity is either **free-form** (default) or carries a
**`PageAnchor { pageId, pageUrl? }`** (`src/shared/page-anchor.ts`). Anchored
kinds today: `text`, `drawing`. Opting a kind in = adding the `pageAnchor`
persisted field and listing it in `anchorableEntities()`
(`src/main/runtime/page-anchor-state.ts`).

**Positions stay in canvas coordinates.** The anchor stores no offset — the
placement relative to the page is implicit in the existing coordinates and
preserved because anchored entities move *with* their page: drag/nudge id
sets expand to include entities anchored to any moved page.

**Placement decides anchoring; there is no mode.** An entity whose center
lands inside a page's body (topmost page wins) anchors on creation and on
drag end; dragging it off clears the anchor. Grouped entities never
auto-anchor — group membership already owns their movement. Deleting a page
frees its anchored entities in place.

**Anchored means document-bound.** While the page shows a different URL than
`pageAnchor.pageUrl` (hash-insensitive, same predicate annotations use), the
entity leaves the scene payload entirely — not rendered, not hit-testable —
and returns when the page navigates back. The sidebar keeps it discoverable:
anchored entities render as child rows of their page (the page acts as a
folder), dimmed while off-URL.

**Persistence is transparent.** `pageAnchor` rides the JSON Canvas node: in
the `specular` extension block for `text` nodes, as a direct field on the
app-specific `drawing` node. Y.Doc sync and undo flow through the existing
persisted-field projection (ADR 0024 §5) — an anchor change made inside a
drag gesture lands in that gesture's single undo step.

## Alternatives considered

- **Store page-relative offsets in the anchor.** Requires syncing offsets on
  every entity/page move; positions-in-canvas + drag-set expansion preserves
  relative placement with zero bookkeeping. Rejected.
- **Explicit anchor/unanchor command (popup toggle).** More discoverable
  control, but adds UI surface and a mode; placement-decides matches how
  users already think ("I put it on the page"). Revisit if silent anchoring
  surprises people; the sidebar nesting makes the state visible.
- **Derive anchoring purely from overlap at render time (no stored field).**
  Loses the URL binding — the whole point is remembering *which document*
  the item was about. Rejected.
- **Dim instead of hide when off-URL.** Stale ink over the wrong document
  reads as commentary on that document; hiding matches the annotation
  precedent. The dimmed sidebar row is the recovery affordance.

## Out of scope (follow-ups)

- ~~**Scroll tracking.**~~ Landed for **region annotations** (the
  `anchor.docRect` variant described below) and
  subsequently for **all anchored entities** (shapes, stickies, drawings):
  the anchor stamps a scroll reference (`scrollX/scrollY`) and scene builders
  shift by the live delta. The original "entities stay pinned to the page
  frame" resolution is superseded — see
  [ADR 0032](./0032-element-attachment.md), which also adds reflow tracking
  via a derived element reference and narrows this ADR's "no stored offset"
  clause to positional authority only.
- ~~**Other kinds** (`shape`)~~ — landed with scroll tracking. `file` remains
  mechanical once wanted.
- ~~**Region annotations as anchor consumers** (ADR 0006 alternative F).~~
  Resolved by the amendment below.
- **Reveal affordance** for a hidden anchored entity's sidebar row (e.g.
  navigate the page back to the anchor URL on click).

## Amendment (2026-07-13) — annotations consume the utility

Annotations no longer predate this mechanism; they use it. `Annotation`
carries the same `pageAnchor?: PageAnchor`, written **once at creation** and
never re-resolved (an annotation's binding is part of what it says —
placement-decides applies to entities only):

- **Element/page anchors** bind to their anchor page and the URL it shows.
- **Region anchors split**: a region whose marquee grabbed page content
  (`regionComponents`/`regionElements` non-empty; first group wins when
  several pages intersect) is page-anchored — it stores its rect in the
  page's *document* space (`anchor.docRect`, page CSS px), so it travels with
  page drags/nudges **and scroll-follows** for free (the transform moves it;
  no anchor translation), hides while the page is off its URL, and nests
  under the page in the sidebar. A grab-less region is canvas-anchored: it
  stores `anchor.canvasRect`, marks canvas space, never moves with pages,
  never hides. This resolves the "region annotations as anchor consumers"
  follow-up (ADR 0006 alt F's spirit, without a new anchor variant).
- **Canvas points** never bind.

`pageAnchor` is the **only** page-binding read — the legacy
`metadata.pageUrl` stamp is no longer written or read, with **no compat
shim**: annotations in existing files without a `pageAnchor` load fine and
behave as canvas-bound (they lose the URL gate until recreated; accepted
deliberately, noted in the changelog). One gate accessor
(`hiddenByPageAnchor`, document-binding.ts) and one sidebar child-row
builder (`sidebarPageChildren`) now serve entities and annotations alike.

## Amendment (2026-07-17) — scroll tracking and reveal

Page scroll is ephemeral view state. Every live page reports its absolute
scroll offset in raw CSS pixels through a dedicated, always-on broadcast;
main stores `scrollX`, `scrollY`, and `scrollHeight` on the runtime `Page` and
includes the offsets in layout data. This transport is intentionally separate
from linked-scroll synchronization:

- anchoring needs absolute pixels, while linked scroll needs progress fractions
  across documents of different heights;
- anchoring must work whether or not the page is interactive or belongs to a
  sync set;
- anchor movement must not inherit linked scroll's coarse progress threshold.

The preload resolves the actual scroll container—walking from the event target
to a scrollable ancestor and falling back to the document scrolling element—so
application shells with an inner `overflow: auto` container behave the same as
document-scrolling pages. Scroll broadcasts are rAF-coalesced and integer
rounded to avoid redundant IPC.

Two coordinate transforms remain distinct:

- `pageViewportToScreen` consumes live DOM rectangles already expressed in
  viewport space.
- `pageDocumentToScreen` consumes stored document rectangles and subtracts the
  page's live scroll offset before delegating to the viewport transform.

Page-anchored regions store `docRect` in document CSS pixels. Grab-less regions
store `canvasRect`. There is no dual interpretation or migration heuristic: a
region without `docRect` is canvas-bound. A `docRect` moves with its page
through projection, so page drag/nudge never rewrites it.

Shapes, text, and drawings keep authoritative canvas coordinates and stamp the
page's scroll offset when anchored. Scene projection applies the live scroll
delta; reanchor folds that delta back into stored coordinates. Drawing stroke
points fold with their entity bounds. ADR 0032 extends the same projection with
derived DOM-element correction.

Opening a page-bound comment also reveals its content through the page's
existing smooth-scroll command:

| Annotation anchor | Document target |
| --- | --- |
| Element | Live selector document position; stored bbox fallback |
| Page-anchored region | `docRect.y` |
| Page offset | `offsetY × scrollHeight` |
| Canvas point or canvas-bound region | No page scroll |

The target is placed roughly one third down the viewport to leave context and
avoid sticky site headers. The same scroll-container resolution is used for
both reporting and commanded scrolling.
