# Page stacking: the gap, and how to close it

Follow-up finding from the `electrobun-canvas` layering spike. Read the
[experiment README](README.md) for what the spike is, and
[`docs/research/electrobun-assessment.md`](../../docs/research/electrobun-assessment.md)
(repo root, background only — not edited by this note) for the full
Electron-vs-Electrobun comparison and the Problem A / Problem B framing this
extends.

This note is the *narrow* view — closing the page↔page reorder gap inside the
spike. For the broader question (a classic, unified layers system where live pages
are just layers at any depth), explored **within this spike**, see
[`LAYERS.md`](LAYERS.md); the single-live option below is its substrate **S1**.

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

## The fix is a missing binding, not a platform limit

Before ranking workarounds, the key fact: **the native move that would close this
gap already exists on macOS — it just isn't exposed through Electrobun's public
API.** AppKit restacks a live `WKWebView` in place via
`addSubview:positioned:NSWindowAbove/Below relativeTo:`, tearing nothing down —
the same `addSubview:positioned:` family Electrobun's wrapper already calls to
*mount* a webview. So a reload-free page↔page reorder is one FFI export away
(that's option 3, which needs a patched Electrobun). Everything else below closes
the gap **without touching native code**, working only with what the
`<electrobun-webview>` tag exposes today — which is what keeps it inside this
prototype.

## Ways to close it

Ranked from "ship today, renderer-only" to "true compositing." The first two need
**zero upstream changes**; they differ in whether more than one page stays live.

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
- **Snapshot-card refinement:** render each inert page's *last snapshot bitmap*
  (`SnapshotCallback` → data URL, the native path the assessment notes) instead of
  a title/URL placeholder. Reordering stays pure shared-z, the selected page stays
  the only live one, but the stack now *looks* like real pages frozen in place
  rather than cards. Same cost profile as plain single-live; much higher fidelity.
  This is the recommended shape — see the refinements section.

### 2. Recreate-on-reorder — multi-live, no fork, reload cost

The idea raised directly: since the only public lever on native stack order is
**creation order**, keep every page live and, when the shared z changes, **destroy
and recreate the affected webviews in the new creation order.** Newest webview is
inserted frontmost (`addSubview:positioned:Above relativeTo:nil`), so recreating
pages back-to-front in target order yields exactly the desired stack.

- **It does work, deterministically.** To land page X at an arbitrary depth you
  must recreate X *and every page that should sit above it*, in order — a single
  recreate can only go fully to front, never inserted mid-stack. So bring-to-front
  is one recreate; an arbitrary reorder recreates the whole suffix above the
  target; send-to-back recreates everything else.
- **The cost is the killer.** Recreating an `<electrobun-webview>` is a fresh
  native view + a **full page reload**: scroll position, form state, in-memory JS,
  media playback, and live sockets are all lost (cookies survive via the
  `persist:` partition; runtime state does not). A z-tweak should feel instant; a
  reload storm across a suffix of pages does not. And it pays this to avoid the
  in-place move the macOS layer already supports — the one option 3 would bind —
  while *also* carrying the full cost of many simultaneously-live webviews.
- **Mitigations don't rescue it.** Snapshot-then-recreate (show a frozen bitmap
  during the swap) hides the *flash* but not the *state loss*, and at that point
  you've built half of option 4 anyway. A hidden keep-alive pool (`toggleHidden`)
  avoids reloads but doesn't change z — hiding is what the `multitab-browser`
  template uses precisely *because* there's no reorder, and it shows one page at a
  time rather than ordering an overlap.
- **Verdict:** the only **zero-fork** way to keep *multiple* pages live and still
  reorder them — but the reload cost makes it unfit as the interactive reorder
  mechanism. If you accept reloads on reorder, single-live (option 1) is simpler
  and reloads strictly less; if you want reload-free multi-live, that's the
  one-line binding in option 3. Recreate-on-reorder is dominated on both sides;
  keep it only as a last-resort fallback for rare, explicit reorders where a reload
  is acceptable.

### 3. Native reorder fork — small upstream change

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

### 4. Snapshot / bitmap — true page-over-page, framework-agnostic

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

## Building the fix in this prototype

What it actually takes to close the gap inside `electrobun-canvas`, file by file —
design notes, not code. Only the two zero-fork options land here: 3 needs a
vendored Electrobun patch and 4 needs the native snapshot path, so both sit
outside the prototype's reach without new native surface.

**The data model is already done.** `core/scene.ts` gives pages and stickies one
shared `z`, and `stepZ` already walks *any* entity past *any* other — so a page's
▲▼ already reorders it past other pages *in the model*. The gap is purely in the
rendering substrate: two live native webviews can't both win an overlap. So every
in-prototype fix changes how a page *renders*, never the z-order itself.

### Option 1 here

- `canvas/bodies/PageBody.tsx` — branch on selection. Selected → today's
  `<EbWebview>` (the single live surface). Unselected → a host-DOM `<PageCard>`
  carrying `data-item-id={page.id}` so it lives in the DOM and is maskable like
  any other item.
- `core/layering.ts` — generalize `pageMaskSelectors`. Today it masks only
  *stickies* above the page; the live page must now mask *every item* above it,
  stickies **and** page cards. The builder stops being sticky-specific and takes
  "all entities with `z` greater than this page." (Honest correction to the
  earlier "lands entirely inside `PageBody`" claim — `layering.ts` has to widen
  its mask source too.)
- `canvas/CanvasItem.tsx` + the ▲▼ chrome — untouched. Stepping a page already
  calls `stepZ`; with cards in the DOM, that step now visibly reorders pages.
- **Net:** one new `PageCard` body + one widened mask query. The
  `live = selected && !panActive` rule and the shell stay as-is.
- **Rough edge:** reselecting a page remounts (reloads) it, since the live webview
  is keyed on selection — addressed next.

### Keep-alive single-visible — the refinement that removes the reload

Plain single-live reloads on every selection switch. Sharpen it: **mount all pages
as live webviews once, but `toggleHidden(true)` every one except the selected
page.** A hidden webview doesn't paint, so it never competes for the overlap —
exactly one *visible* native surface at a time, same as single-live, but no reload
on switch because the others stayed warm.

- A hidden page's live pixels aren't visible, so each still needs a visual
  stand-in at its rect — the same `PageCard` (or a snapshot).
- Trade: N live webview processes in memory vs. 1. Single-live for many cold
  pages; keep-alive for the handful you switch between fast.
- Both variants close the reorder gap identically (reorder = shared-z) and show
  one live page at a time; they differ only on warm-vs-cold selection switching.

### Option 2 here

- All pages stay `<EbWebview>`; on a z-change, force the affected pages to remount
  in new creation order — in React, bump their `key` so the element is destroyed
  and recreated, newest mounting frontmost.
- Driving *creation order* through React reconciliation is fiddly (you must
  remount the whole suffix above the target, in order) and each remount reloads
  the page. In-prototype this mostly serves to *demonstrate the reload jank* —
  which is the only reason to keep it, behind a toggle, if at all.

## Cross-cutting refinements

Two ideas that aren't standalone options — they make the options above cheaper or
sharper, and combine with several of them.

- **Scope the problem to actual overlaps.** The gap only bites where two *live*
  pages overlap; pages with disjoint rects each own their region regardless of
  creation order. So the expensive treatment (single-live, snapshot, or recreate)
  only ever needs to apply within an **overlapping cluster** — typically a small
  subset of a canvas. Resolve each cluster independently: keep its frontmost page
  live, demote the rest. On a canvas of mostly non-overlapping pages this makes
  options 1–3 nearly free in the common case, and shrinks how often any reload or
  snapshot fires.
- **Hybrid mask + snapshot for "page in front" without reorder.** Masks reveal
  *host DOM*, never a natively-behind webview — so you can't punch page A and see
  live page B through the hole. But you *can* punch A over B's region and place
  **B's snapshot bitmap** in the host DOM behind that hole. Result: B appears in
  front of A in their overlap, with no reorder API and without taking B's whole
  surface offscreen — live where it's already frontmost, frozen only where it must
  appear in front. A targeted, per-overlap slice of option 4 that leans on the
  masks the spike already drives.

## Recommendation

The honest ranking for closing the gap **inside the prototype**:

1. **Build option 1 — single-live, in the keep-alive single-visible form.** It's
   the only path that closes the page↔page reorder gap with zero native work, and
   keep-alive removes its one rough edge (the reload-on-reselect). Scope the
   treatment to overlapping clusters (refinement 1); upgrade the `PageCard`
   stand-in to a snapshot bitmap once the native snapshot path is reachable.
2. **Don't build option 2 (recreate-on-reorder) as the mechanism.** It's the only
   zero-fork way to keep *multiple* pages live, but it pays a page reload on every
   order change and is dominated on both sides — simpler if you accept reloads
   (option 1), cheaper if you don't (option 3). Worth a toggle only to *feel* the
   reload cost in the running spike, not as the real fix.
3. **Treat option 3 (native reorder fork) as the graduation step.** When the
   prototype proves out and reload-free multi-live restacking has to be
   first-class, bind the one FFI export that exposes the in-place move the macOS
   layer already supports. Out of scope for a no-native-changes prototype.
4. **Option 4 / hybrid mask+snapshot stays parked** for genuine *blended*
   page-over-page (Problem B) — a different requirement from mere stack order.

**Bottom line for the prototype:** ship option 1 in keep-alive single-visible
form. It closes the gap the spike actually surfaced, stays entirely in renderer
TypeScript, and leaves option 3 as a clean later upgrade rather than a
prerequisite.
