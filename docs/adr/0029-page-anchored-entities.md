# ADR 0029 — Page-anchored entities (the "hook to a page" utility)

**Status:** Accepted
**Date:** 2026-07-13
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

- **Scroll tracking.** Anchored items are pinned to the page frame, not to
  document coordinates — they don't move when the page scrolls. Needs an
  absolute scroll-offset broadcast per page (the current scroll-sync payload
  carries progress ratios only), then a `docX/docY` variant of the anchor.
- **Other kinds** (`shape`, `file`) — mechanical once wanted.
- **Region annotations as anchor consumers** (ADR 0006 alternative F).
- **Reveal affordance** for a hidden anchored entity's sidebar row (e.g.
  navigate the page back to the anchor URL on click).
