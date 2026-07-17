# Element attachment — implementation plan

Decision record: [ADR 0030](../adr/0030-element-attachment.md). Read it first;
this plan is the build order, not the rationale.

Goal: page-anchored items (shapes, stickies, drawings, page-anchored regions)
track the DOM element they were placed over, so they survive page reflow
(focus-mode preset/fill changes, viewport resizes, dynamic content). Ships as
PR-sized steps on one feature branch (`claude/element-attachment-design`),
then folds into the scroll-tracking PR — the feature doesn't ship until this
architecture is in.

## Constraints (from the ADR — violating any of these is a design change)

- Canvas coords (and `docRect`) stay the stored truth; the attachment is a
  render-time correction plus a fold-on-reanchor, mirroring the scroll shift.
- Attachment is derived metadata: persisted, **outside undo scope**, re-derived
  on position change. Never a visibility gate.
- Re-resolution on reflow events only (resize / load / debounced mutations) —
  no per-scroll-frame work; scroll stays on the overlay band fast path.
- One element per item, translation-only tracking.
- Smallest change from the current system: extend the existing bbox tracker
  and the existing shift/rebase seams; no parallel pipeline.

## Steps

### 1. Anchor shape + capture rule (pure/preload groundwork)

- Add `element?: { selector, docX, docY }` to `PageAnchor`
  (`src/shared/page-anchor.ts`); flows through persisted-field projections for
  text/drawing/shape and the annotation record (persistence only — no undo
  exclusion yet, no consumer reads it).
- Preload capture function in `src/preload/` next to `dom-element-utils.ts`:
  document point → `{ selector, docX, docY }` via the ADR's rule (center hit →
  walk up to nearest meaningful element → nearest-at-Y → `body`). Reuses
  `elementSelectorParts` / `buildElementPath`.
- Unit tests: capture fallback chain (jsdom), persisted-field drift tests.

### 2. Capture wiring (fire-and-forget enrichment)

- IPC: main asks a page "capture element at document point" (new channel next
  to `queryDomElements`).
- `reanchorEntityById` stays sync; after it writes an anchor, main fires the
  capture query and stamps `anchor.element` when it resolves (entity may have
  moved again — stamp only if the anchor is unchanged since the query fired).
- Region annotations: capture once at creation (`workspace-annotations.ts`),
  from the region's document-space center.
- Undo exclusion: the enrichment write must not create an undo step. Follow
  the zoom/pan precedent (untracked transaction origin); integration test:
  draw → wait for enrichment → one ⌘Z removes the stroke.
- Integration tests: create-on-page stamps attachment; drag-end re-captures;
  drag-off clears it with the anchor.

### 3. Reflow pipeline (tracker extension)

- Extend `src/preload/annotation-bbox-tracker.ts`: second subscription kind
  (attachment selectors → *document* positions), re-run on viewport resize,
  document load, and a debounced (~150ms) `MutationObserver` on `body`; one
  batched broadcast per page.
- Main: subscription source is the set of attached items on visible pages
  (main-side state, not renderer popover state); store live element positions
  on the runtime page, mark layout dirty on change.
- Integration tests: broadcast round-trip updates the stored positions; zero
  subscriptions → observer not installed.

### 4. Render correction + fold

- `pageAnchorElementShift(anchor)` next to `pageAnchorScrollShift`: live
  element document position − `anchor.element.docX/docY`, zero when
  unresolved/absent.
- Scene builders (shape/text/drawing) add the term alongside the scroll
  shift; drawing applies it to stroke points. `regionCanvasRect` /
  `annotationMath` apply it to `docRect` regions.
- Fold into stored coords on reanchor, extending `rebaseAnchorScroll` (update
  `docX/docY` reference after folding, like the scroll reference).
- Integration tests mirror the scroll-follow suite: scene shifts when the
  element moves, stored coords unchanged; rebase folds and undoes cleanly;
  unresolvable selector → zero shift (never hides).

### 5. Manual smoke + ship

- One manual smoke on the branch (per workflow convention): annotate a live
  page, switch device preset → fill in focus mode, confirm ink/regions/
  stickies track the content; kill the dev server mid-session and confirm
  items hold their last geometry, no hiding.
- Merge into the scroll-tracking branch/PR; changelog entry framed as
  "annotations and ink stay glued to page content when the page reflows."

## Explicitly out (v1 ceilings, named in the ADR)

- Ink re-wrapping / multi-point attachment (translation-only).
- Per-scroll-frame element tracking.
- `file` entities as anchor consumers.
- Stale-attachment UI (no badge; geometry fallback is silent by design).
