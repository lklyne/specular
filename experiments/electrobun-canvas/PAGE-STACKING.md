# Page stacking: the gap, and how to close it

Follow-up finding from the `electrobun-canvas` layering spike. Read the
[experiment README](README.md) for what the spike is, and
[`docs/research/electrobun-assessment.md`](../../docs/research/electrobun-assessment.md)
(repo root, background only — not edited by this note) for the full
Electron-vs-Electrobun comparison and the Problem A / Problem B framing this
extends.

## What works today

The spike settles **Problem A — DOM ↔ page interleaving.** Stickies are host-DOM
elements; pages are `<electrobun-webview>` tags. Per-page **mask selectors** plus
one shared z-order let a sticky punch through (sit in front of) some pages while
staying behind others, simultaneously — see
[`src/mainview/core/layering.ts`](src/mainview/core/layering.ts). A sticky's
`▲▼` walks it across individual pages exactly as intended.

## The gap: pages can't be reordered relative to each other

Trying to extend the same `▲▼` restacking to **page ↔ page** order hits a wall:

- Native webviews stack in **creation order**. Stock Electrobun exposes **no
  z-order / reorder API** on `BrowserView`.
- The per-frame host→native sync pushes only each webview's **rect + mask
  selectors** — never a stacking order.
- So a page's `▲▼` restacks it against *stickies* (via masks), but **never
  against other pages**. In any overlap of two live native pages, exactly one
  surface is frontmost and owns the pixels and input there (the OOPIF-always-wins
  rule from the assessment, §3/§4).

This is a **capability/design finding from API analysis**, not a macOS runtime
observation — masks and passthrough run only on macOS WebKit and this work was
done on Linux, so the spike was not exercised live. The conclusion is about the
absence of a public reorder API, which holds regardless.

It is the concrete, hands-on form of **Problem B** (page-over-page stacking)
that the assessment predicted in the abstract.

## Three ways to close it

Ranked from "ship today, renderer-only" to "true compositing."

### 1. Single-live model — no fork, renderer-only

Only the **selected** page is a live `<electrobun-webview>`; every other page
renders as a host-DOM **card** (a placeholder showing title/URL). Because exactly
one native webview is visible at a time, pages never compete for native stack
order, and the one live page stays z-correct by masking every item — sticky *or*
page card — above it in the shared z.

- **Page reordering becomes ordinary shared-z reordering**: cards reorder via
  `▲▼`/`zIndex` like stickies, and the live page honors the order through its
  mask set. This lands entirely inside the `PageBody` body adapter — the
  `CanvasItem` shell and the `live = selected && !panActive` rule are untouched.
- **Cost / limits:** inert pages show a static card, not live pixels; only one
  page is live at a time; reselecting a page reloads it (mounting keyed on
  selection). Keep-alive (mount + `toggleHidden`) is a later refinement, but it
  re-introduces the multi-live ordering question this model sidesteps.
- **Verdict:** the closest thing to "reorder pages in the stack" achievable with
  **zero upstream changes**. Good enough to demo the model; not true multi-live
  compositing.

### 2. Native reorder fork — small upstream change

Bind a webview-reorder call through Electrobun's FFI so **multiple pages stay
live *and* restack in place**, reload-free.

- AppKit reorders a live `WKWebView` by re-inserting it via
  `addSubview:positioned:NSWindowAbove/Below relativeTo:otherView` — the same
  family of insertion call the native wrapper already uses to mount webviews.
- **Sketch:** `reorderWebview(...)` native export → Zig FFI binding →
  `BrowserView.reorder()` → a `<electrobun-webview>` `moveAbove(id)` /
  `moveBelow(id)`, driven from the shared z-order each frame.
- **To confirm against Electrobun source** (`nativeWrapper.mm`,
  `package/src/bun/core/BrowserView.ts`) — these files aren't vendored in this
  repo, so the exact symbols/signatures must be checked upstream before relying
  on them; no line numbers asserted here.
- **Verdict:** small, focused, reload-free; the natural way to make page
  restacking first-class. Still all-or-nothing per region (one live page wins
  each overlap) — it orders pages, it does not blend them.

### 3. Snapshot / bitmap — true page-over-page, framework-agnostic

Represent inert or stacked pages as **frozen bitmaps** via the native snapshot
path the assessment notes (`SnapshotCallback` → data URL), and composite those
yourself. This is the "Tier-2 offscreen compositing" route to *real*
page-over-page: two opaque pages blended in their overlap, arbitrary z between
them.

- Heaviest path; also the **only** one that yields genuine compositing rather
  than a single frontmost winner per region.
- **Framework-agnostic:** buildable on the current Electron stack too
  (`offscreen: true` + `paint` events), so it does not depend on an Electrobun
  migration — see the assessment's recommendation (§7).
- **Verdict:** the real fix for Problem B, at real cost. Use when blended
  page-over-page is a product requirement, not just stack order.

## The reframe the three above miss

All three ask the same question — *"how do we order N **live native** surfaces
against each other?"* — and answer it at three price points. But the spike's own
interaction rule already makes that the wrong question.

[`core/interactivity.ts`](src/mainview/core/interactivity.ts) gates every item
through `live = selected && !panActive && !dragging`. **At most one item is ever
interactive at a time.** Click-to-interact already serializes input — two pages
are never *input-live* simultaneously, by design.

So a non-selected page never needs to be a live native input surface. It only
needs to be (a) **visually** live — video still playing, CSS animations still
running — and (b) **correctly z-ordered**. Both are far weaker than "a real
native webview," and the gap between them is where the cheaper, fork-free wins
live. Restate the problem accordingly:

> Keep non-selected pages visually alive and freely stackable, given that only
> the **one selected** page has to be a real native surface.

Every option below is a different answer to *"what represents a non-selected
page?"* Option 1 above already half-found this — it just chose a **frozen card**
for the answer. Replace that card with something live and the model gets
dramatically better at no extra architectural cost.

### 4. Live textures — single native hot seat, the rest stream (recommended)

Option 1's exact skeleton with its one weakness removed. **Only the selected
page is a live `<electrobun-webview>`** (the "hot seat"). Every other page
renders as a host-DOM `<img>`/`<canvas>` **fed by the native stream-image path**
the assessment notes (`SnapshotCallback` can "stream an image of the contents"),
instead of a static placeholder.

What this buys, and why it's strictly better than option 1:

- **Non-selected pages stay alive.** A streamed texture keeps showing video and
  animation at whatever framerate the stream delivers — not a frozen snapshot.
- **Page↔page ordering becomes ordinary DOM z-index.** Streamed pages are plain
  host-DOM elements, so they restack against each other (and against stickies)
  through the same shared z-order the spike already drives — *exactly the way
  Problem A was already solved.* **Problem B dissolves with zero fork**, because
  only one native surface ever exists and it never has to be ordered against
  another native surface.
- **Click-to-interact is untouched.** Selecting a streamed page promotes it into
  the hot seat (mount the live webview, point the texture's anchor at it);
  deselecting demotes it back to a stream. With the stream kept warm there's no
  reload flash — the texture is already painting the live page when the native
  surface swaps in. The `live = selected && !panActive && !dragging` rule and the
  `CanvasItem` shell stay as-is; the swap lives entirely in the
  [`PageBody`](src/mainview/canvas/bodies/PageBody.tsx) body adapter, beside the
  passthrough/mask logic it already owns.

Shape of the change, all renderer-side:

- `PageBody` branches on `selected`: render `<EbWebview>` when selected, render a
  streamed `<canvas>`/`<img>` otherwise. The mask machinery
  ([`core/layering.ts`](src/mainview/core/layering.ts)) is needed **only** for the
  single hot-seat webview — to punch stickies (and now nothing else, since other
  pages are DOM) above it. Every non-selected page is DOM and obeys z-index for
  free.
- A small stream manager keeps a warm image stream per non-selected page and
  tears it down (or drops to a single frozen frame) for pages far off-viewport —
  this is also where a liveness budget lands if page counts get high.

**The one thing to measure before relying on it:** the macOS stream path's
**framerate, latency, and N-concurrent cost.** That single spike decides whether
option 4 is real or whether you fall back to option 2's native-reorder fork.
It's the cheapest, highest-information experiment here and everything hangs on
it — see the new findings-log item in the [README](README.md).

- **Verdict:** the closest thing to a **standard canvas layer system with live
  pages and click-to-interact**, with **no upstream fork** — option 1's
  no-risk profile, but pages stay alive and page reordering is real. Gated on the
  stream-path measurement.

### 5. Texture-everything + one native overlay — the long-term form (sketch)

Option 4 generalized, and where this likely wants to go. Instead of "selected =
native, rest = streamed images into the DOM," composite **every** page as a
GPU/canvas texture in a single host surface — arbitrary z, plus opacity, blend
modes, and transforms the DOM can't give you — and pin **one** native webview
over whichever page is selected for crisp text and real input. One conceptual
model: *everything is a texture; the selected page borrows a native overlay.*

- Same hot-seat input story as option 4, so click-to-interact carries over
  unchanged; option 4 is effectively its MVP (DOM compositor instead of a GPU
  one).
- Unlocks effects option 4 can't: a page at 60% opacity over another, a page
  tilted in 3D, cross-page blends — i.e. genuine page-over-page *compositing*,
  the option-3 payoff, but with input solved by the hot seat rather than by
  synthetic input injection.
- Heavier renderer (a real texture compositor, frame scheduling, color
  management). Build it when page opacity/blend/transform becomes a product
  requirement; until then option 4 delivers the layer model without it.

## Recommendation

The interaction model only ever needs **one** live native surface, so the
cheapest real fix isn't ordering native pages — it's representing the rest as
something live and DOM-stackable.

1. **Spike the macOS stream-image path first** (framerate / latency /
   N-concurrent). One measurement decides the whole tree below it.
2. If streaming is viable → build **option 4** (single native hot seat + live
   streamed textures). It delivers a standard layer system, live pages, and
   click-to-interact with **no fork** — and it's option 1's skeleton, so it's a
   small delta from what's already here.
3. Hold **option 5** as the long-term form once page opacity/blend/transform
   effects are wanted; hold **option 2**'s native-reorder fork as the fallback
   if streaming turns out too slow to keep non-selected pages convincingly live.

Earlier framing, still valid for the live-native branch: **option 1** proves the
model with zero risk, **option 2** makes native page restacking first-class, and
**option 3** remains the framework-agnostic route to blended page-over-page.
