# Scroll tracking — page-anchored regions follow page content

Follow-up to `docs/plans/page-anchoring-refactor.md` (ADR 0029's named follow-up).
The refactor was the prerequisite; this is the feature.

## The gap

Page-anchored regions (the marquee grabbed page content) are specified to
"translate with page drags/nudges, hide while the page is off its anchor URL,
nest under the page in the sidebar, **and scroll-follow when scroll tracking
lands**" (page-anchoring-refactor.md:120). Everything in that list ships today
except the last clause.

The reason is that a region anchor stores only `canvasRect` — a rect in canvas
coordinates (`types.ts:1706`). Canvas coordinates say where the region sits on
the *surface*; they say nothing about where in the *document* it was drawn. A
page-anchored region therefore has no coordinate that scrolling could move.

Element anchors do not have this problem: they re-resolve a selector against
the live DOM on every scroll frame and stream the resulting viewport bbox back
(`annotation-bbox-tracker.ts` → `register-comment-hover-ipc.ts` → `useLiveAnnotationBboxes.ts`).
That round trip is proof the plumbing works; a region has no selector to
resolve, so it needs the page's scroll offset instead.

## What we are building

One new number per page in the layout broadcast — the page's absolute scroll
offset — and one new region anchor variant that stores document coordinates
instead of canvas coordinates. The existing `pageViewportToScreen` transform
(`shared/page-space.ts`) grows a scroll term, and every overlay that positions
chrome over page content inherits scroll-following for free.

### Non-goals

- **Element and page anchors.** They already track scroll via the live-bbox
  round trip. Leave them alone; do not migrate them onto the scroll offset.
- **Canvas-anchored regions.** The grab-less half of the split marks canvas
  space and must never move with page content. That is the design.
- **Anchored entities (stickies, drawings).** Decided: they stay in canvas space.
  See "Resolved" below.
- **Reusing the linked-scroll broadcast.** See the trap below.

## Trap: the existing scroll broadcast is not the one we need

`pageScrollChanged` already exists, and it is tempting to hang this off it. It
is the wrong shape in three separate ways, all of which must be fixed rather
than worked around:

1. **It carries progress fractions, not pixels.** `createScrollSyncData`
   (`shared/scroll-sync.ts:24`) sends `xProgress`/`yProgress` in 0..1, derived
   by dividing by max scroll. That is exactly right for linked scroll (two
   pages of different heights staying in sympathy) and useless for anchoring —
   a document that grows taller (lazy loading, "Load More") changes the
   denominator, so the same fraction maps to a different pixel offset. Anchors
   need `window.scrollX`/`scrollY` in raw CSS pixels.
2. **It is gated on `interactive`.** `queueScrollSyncBroadcast(interactive)`
   drops the broadcast unless the page is in interactive mode
   (`scroll-sync-handler.ts:112`). A comment must follow scroll on a page the
   user is merely scrolling past, not only one they have entered.
3. **Main drops it unless the page is linked.** `ipcMain.on(pageScrollChanged)`
   returns early on `!page.syncId` (`register-page-chrome-ipc.ts:39`). Most
   pages have no `syncId`.

It also debounces on a 1%-of-progress threshold, which is coarse enough to make
a comment visibly lag its content.

Conclusion: add a **separate, always-on, absolute-pixel** scroll broadcast.
Do not widen the linked-scroll one — the two have genuinely different
requirements and fusing them re-creates the hybrid the last refactor just
finished pulling apart.

## Phases

### Phase 1 — Broadcast the page's absolute scroll offset

**Preload.** In the existing `window.addEventListener('scroll', …, {capture: true})`
handler in `page-content.ts:709` (which already drives
`queueRecomputeAnnotationBboxes`), add a scroll-offset broadcast: rAF-coalesced,
sends `{ scrollX: window.scrollX, scrollY: window.scrollY }` when either
changed. Not gated on `interactive`. Also emit once on `did-finish-load` so a
page that restores its scroll position starts correct, and on `resize` (layout
reflow moves content under a fixed offset).

**Main.** New IPC channel, no `syncId` gate. Store on the runtime page as
`page.scrollX` / `page.scrollY` (ephemeral runtime state — this is a view
property, never persisted, never in the Y.Doc). Mark the canvas layer dirty and
let the existing layout broadcast carry it.

**Layout.** `CanvasScenePageEntity` gains `scrollX` / `scrollY`, defaulting to 0.

**Caution — the scroll container is not always the document.** v0.app scrolls an
inner `overflow: auto` div: `document.scrollingElement.scrollTop` stays 0 while
content moves. `page-content.ts:620-637` already has the logic to find the real
scroll target — it walks up for an ancestor with `overflow: (auto|scroll|overlay)`
and a `scrollHeight > clientHeight`, falling back to `document.scrollingElement`.
It is inline inside the `dispatchScroll` handler, not a named helper: **extract
it** (`resolveScrollTarget()`) and use it from both places. Report the offset of
*that* element. `capture: true` on the window listener
already catches inner-container scroll events, which is why the element-anchor
path works on v0 today. Get this wrong and scroll tracking will silently no-op
on exactly the class of app (Next.js shells) our users point at most.

**Test.** Integration: set `page.scrollY`, assert the broadcast layout carries
it. Mutation: drop the field from the scene entity → test fails.

### Phase 2 — The transform learns about scroll

`pageViewportToScreen(rect, page, layout, frame)` (`shared/page-space.ts:53`)
maps a page-*viewport* rect to screen. Add a sibling for the other coordinate
space:

```
pageDocumentToScreen(rect, page, layout)  // rect in document coords
  = pageViewportToScreen({...rect, x: rect.x - page.scrollX,
                                   y: rect.y - page.scrollY}, page, layout)
```

Document coords minus scroll offset *are* viewport coords, so this is one
subtraction composed with the transform we already own. Keep both functions:
callers holding a live DOM rect (element anchors, inspect popovers, hover
overlay) are in viewport space and must not subtract scroll; callers holding a
stored document anchor are in document space and must.

This is the payoff from phase 3 of the last refactor — there is exactly one
place to add the scroll term. Do not let a second copy appear.

**Test.** Unit, against `tests/unit/page-space.test.ts`: a document rect with
the page scrolled down by N renders N pixels higher.

### Phase 3 — The document-anchored region variant

`AnnotationAnchor`'s region arm (`types.ts:1706`) becomes:

```
| { type: 'region'; canvasRect: WorkspaceBounds }                 // canvas-anchored (grab-less)
| { type: 'region'; docRect: WorkspaceBounds; pageAnchor: … }     // page-anchored (grabbed)
```

Concretely: page-anchored regions store `docRect` in the page's document CSS
pixels, captured at creation by converting the marquee's canvas rect into the
page's viewport space and adding the page's scroll offset. Canvas-anchored
regions keep `canvasRect` unchanged. The grab rule that decides which
(`page-anchoring-refactor.md:120`, already shipped in `cfe0faaa`) is untouched —
this only changes what the page-anchored half *stores*.

**Decision (2026-07-13).** Existing page-anchored regions in `.canvas` files
carry `canvasRect` and become **canvas-anchored** — a region without a `docRect`
is canvas-anchored, full stop. No conversion pass, no dual-shape reader, one
read path (the no-backwards-compatibility stance the last refactor took).

The cost is real and worth naming: a region a user already drew over page
content stops tracking its page — it stays where it is on the canvas and stops
hiding on navigation. It does not disappear or move; it demotes. Re-drawing it
restores the behavior. This is the honest trade for not carrying a migration
path for a shape that has existed for days.

**This deletes work, not just adds it.** `translateAnnotationsAnchoredToPage`
(`page-anchor-state.ts`) exists to translate a page-anchored region's
`canvasRect` when its page is dragged. A `docRect` is relative to the document,
so it needs no translation on page drag — it moves because the transform moves.
That function, and its call sites in the drag tick and keyboard nudge, go away
for regions.

**Render.** `AnnotationsLayer.tsx:15` maps region annotations through
`canvasRectToScreenRect`. Page-anchored regions route through
`pageDocumentToScreen` instead; canvas-anchored ones keep the canvas transform.
Clip page-anchored regions to the page's content frame and fade them at the
edges the way `badgeOpacity` does for badges (`CommentBadgesLayer.tsx:199`) —
a region scrolled out of view must not paint over the page's chrome or float
across the canvas.

**Test.** Integration: create a page-anchored region → scroll the page → its
screen rect moves by the scroll delta; drag the page → it moves with the page
and round-trips in one undo step; navigate away → dropped from the broadcast.
Grab-less region → unaffected by both. Mutation-verify each per `tests/README.md`.

### Phase 4 — Clicking a comment scrolls its page to it

Clicking any comment in the right details panel (`CommentsPane.tsx`) should
smooth-scroll its page until the commented content is in view. Today the click
calls `rightDetailsPanelApi.openAnnotationThread(annotation.id)` and nothing
moves — on a long page the thread opens pointing at content the user cannot
see, which is the same class of "the comment and its content are decoupled"
complaint this whole plan exists to fix.

**Reuse the ramp we already have.** `page-content.ts:611-703` (the
`dispatchScroll` handler) already implements an eased, rAF-driven scroll ramp
with in-flight supersession, against the *real* scroll target (the same
container phase 1 extracts as `resolveScrollTarget()`). It is what the CLI
`scroll` verb rides. Do not write a second smooth-scroll — send this through the
same command.

**The one new thing is the target offset**, and each anchor type answers it
differently:

| Anchor | Target scroll offset |
| --- | --- |
| element | Re-resolve the selector, read its rect, add the page's current scroll offset → document position. Falls back to the stored bbox if the selector is stale. |
| region, page-anchored | `docRect.y` — this is precisely what phase 3 gives us, and why phase 4 comes after it. |
| page | `offsetY × document height` — the anchor is already a fraction of the page. |
| region, canvas-anchored / canvas point | Nothing to scroll to. These mark canvas space, not page content. |

Scroll so the anchor lands roughly a third down the viewport rather than flush
at the top — content pinned to the very top edge reads as cut off, and a sticky
site header will often cover it outright.

**Decision (2026-07-13): canvas-anchored and canvas-point comments scroll
nothing.** Clicking one opens its thread and leaves every page alone — they mark
canvas space, so there is no page position to reveal and scrolling some page
would be inventing a destination. Panning the *canvas* to the comment is a
reasonable feature, but it is canvas navigation, not scroll tracking; it gets
its own plan.

**Test.** Integration: click a comment whose anchor is below the fold → the
page receives a scroll command targeting the anchor's document offset. Element
anchor with a stale selector → falls back to the stored bbox, does not throw.
Canvas-anchored region → no scroll command is sent. Mutation-verify each.

## Resolved — entities stay in canvas space

**Decision (2026-07-13).** Page-anchored entities (stickies, drawings) do **not**
scroll-follow. They keep canvas coordinates and keep travelling with page drags,
exactly as they do today. `docRect` is an annotation concept only.

The line this draws: entity anchoring is about *ownership* — which page owns
this item, which document is it bound to (ADR 0029). Annotations are the things
that point *at content*. A sticky beside a page is furniture in canvas space,
not a note on a paragraph, and sliding it under the fold on scroll would be
surprising. Do not let phase 3's shape quietly extend to `AnchorableEntity`.

## Sequencing

Phases are ordered and each is independently shippable: 1 (offset in the
broadcast, nothing consumes it) → 2 (transform, nothing calls it) → 3 (the
anchor variant, which turns the feature on) → 4 (scroll-to-comment). Phase 3 is
the only one with a file-format change, so it is the only one that needs the
ADR amendment.

Phase 4 depends on phase 1 (it needs the page's current scroll offset to
convert a viewport rect into a document position) and on phase 3 for regions
(`docRect` *is* its answer), so it goes last. All four ship together on this
branch — the ordering is for building, not for release.

## Working notes (how to verify this on a real page)

Hard-won during the element-anchor bug that preceded this plan (fixed in
`buildUniqueSelector`, `dom-element-utils.ts`) — all of it applies to building
and testing scroll tracking:

- **`pnpm dev:restart` does not kill the dev app.** Its `pkill` matches
  `Electron.app.*Specular`, but in dev the binary is
  `node_modules/electron/dist/Electron.app`. The old process survives, the
  health check then passes *instantly against the stale app*, and you test the
  build you thought you replaced. Until the script is fixed:
  `pkill -9 -f "telescope/node_modules/electron/dist/Electron.app"` first.
- **`CDP=1 pnpm dev:restart`** exposes CDP on `127.0.0.1:9333`.
  `curl -s http://127.0.0.1:9333/json/list` lists every renderer *and* every
  page; attach to a page's `webSocketDebuggerUrl` to run `Runtime.evaluate`
  inside it. This is the only way to see what a page's DOM actually reports.
- **Pages report `document.visibilityState === 'hidden'` when the app window is
  occluded**, which throttles their rAF to nothing. Any rAF-coalesced work in
  the page preload — including the scroll broadcast phase 1 adds — silently
  stops. Bring the app to the front before concluding a scroll path is dead.
- **Verify against a page whose scroll container is not the document.**
  `https://v0.app/templates` is the standing example: `window.scrollY` stays 0
  while content moves. If scroll tracking works there, it works.

**Docs:** amend ADR 0029 (scroll tracking lands; the "will scroll-follow when
scroll tracking lands" caveat resolves), CONTEXT.md §Annotations (the region
split now differs in coordinate space, not just in behavior), and
`docs/file-formats.md` (the `docRect` anchor shape).
