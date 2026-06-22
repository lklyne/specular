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

## Recommendation

For this spike's purpose — proving the layering model — **option 1** is enough
and ships with no upstream risk. **Option 2** is the clean follow-up if page
restacking needs to be first-class with pages staying live. **Option 3** stays
on the table for true page-over-page compositing and is independent of the
framework choice.
