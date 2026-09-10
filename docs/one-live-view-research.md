# One live view, textures everywhere else — assessment against what was tried

*Research note, 2026-09-10. Answers the open questions in the September 2026
"Infinite Canvas + Live Web Views" research summary by checking them against
Electron 43 source and docs, and against the perf work already recorded in
this repo. Sibling to [offscreen-rendering-research.md](offscreen-rendering-research.md)
(the original OSR survey) and [pan-zoom-perf-unknowns.md](pan-zoom-perf-unknowns.md)
(§2.B is the `sharedTexture` path this note builds on). Not a recommendation
to build; a record of what the idea runs into.*

---

## 1. The load-bearing answer: offscreen cannot be toggled

Open Question 1 asked whether `offscreen` can be flipped on an existing
webContents at runtime, so one webContents could have "two display modes".

**No.** Read from `shell/browser/api/electron_api_web_contents.cc` at v43.2.0:

- `offscreen_` is read once from the web preferences in
  `InitWithSessionAndOptions` (`offscreen_ = web_preferences->IsOffscreen()`,
  line ~1136) and `type_ = Type::kOffScreen` is set in the constructor.
- The `OffScreenWebContentsView` (the thing that turns a compositor frame
  into a `paint` event instead of a native surface) is constructed at the
  same time, from the same options. Nothing later replaces it.
- There is no setter. `webContents.getType()` reports `'offscreen'` for the
  life of the object.

So "same web contents, two display modes" does not exist. What does exist is
one of three shapes:

| Shape | What the interactive page is | State continuity |
|---|---|---|
| **A. Two webContents per page** — offscreen at rest, a fresh live `WebContentsView` on enter | A different webContents | Lost: DOM, JS, scroll, form state. Only partition-level session/cookies survive. This is the failure the research doc's §4 rules out. |
| **B. Every page offscreen, always** — the "live" page is the one receiving forwarded input | The same offscreen webContents | Kept by construction. No swap at all. |
| **C. Every page a live `WebContentsView`, always** — textures only during gestures and for parked pages | The live view | Kept. **This is the current architecture.** |

The research doc's proposal, read literally, is A. Its stated goals (one
source of truth, only ever pay for one live view) are only met by B. C is
what the last three months of perf work built, and its results say a lot
about what A and B would cost.

## 2. What has already been tried (and what it says about this)

Every item below is in `docs/` or git history; read the linked record before
re-deriving it.

**Textures during gestures already exist (shape C).** `zoom-snapshot-freeze.ts`
parks every visible page behind a `capturePage` JPEG for the duration of a
zoom, and the settle re-captures at up to a 2048px long edge over CDP so a
zoom-in stays crisp ([perf-zoom-pan-log.md](perf-zoom-pan-log.md) Exp A–E).
`drag-freeze.ts` does the same for the dragged page on aboveView. The
renderer side (`useFrozenPageBitmaps.ts` → `chromeItemDraw.ts`
`drawItemSnapshot`) already draws a bitmap into the camera-projected rect.
A GPU-texture tier would replace the JPEG in that pipeline, not invent a new one.

**Parking a live view is not free — ADR 0037.** The pan freeze was built,
measured, and reverted: a parked page drops its tiles and rebuilds them on
unpark, ~300ms of image decode after every 160ms flick on a 15-page canvas.
This is the single most relevant prior result for the new proposal. In
shape A, "swap the texture for the live view" *is* an unpark of a view that
never composited — worse than the pan freeze, because a brand-new webContents
has no tiles at all. In shape B there is no unpark, because nothing is ever
parked: the page composites to a texture pool continuously and we keep our
own copy. That is the strongest argument for B over A.

**Pan is not a page-count problem any more.** After ADR 0036 and
camera-local projection, a pan sends one ~40-byte camera patch and one
`setBounds` per visible page. The dominant pan cost turned out to be
canvas-bg's own dot grid (Exp G, 44× fix), not the pages. Zoom's cost was the
per-tick device-emulation re-raster storm, which the quantized emulation
(Exp A, −80% raster ms) and the zoom freeze already removed. A texture
canvas would make pan/zoom *cheaper still* (no `setBounds` loop, no
emulation at all), but the "pan and zoom are expensive because of live
views" premise in the research doc's §1 is largely stale. **Trace the
current build before assuming the pages are the cost** — Exp G and the two
measurement traps at the end of the perf log exist because that assumption
was wrong once already.

**ADR 0023 (rejected).** Moving the camera to the renderer and scaling
views with `setBounds` gave no felt improvement and rested on a false
assumption (content is pinned to `viewSize × scale`; `setBounds` does not
scale it). Its postmortem's conclusion — the live-page substrate needs a
snapshot-freeze, not a transform trick — is what led to shape C.

**The `<webview>`/OOPIF single-compositor canvas** was built on 2026-08-23
and does not work; the failure mode was not written down
([pan-zoom-perf-unknowns.md](pan-zoom-perf-unknowns.md) §2.A). Nothing on
any remote branch records it — `fix/oopif-pointer-dispatch` is unrelated (it
routes forwarded clicks through CDP so cross-origin iframes inside a page
receive them). Do not reopen it without first recovering why.

**Input forwarding is already how the entered page gets pointer and wheel
input.** Since ADR 0022, exactly one page is interactive at a time
(`interactivePageId`), and because `aboveView` sits above every page, its
pointer and wheel events are forwarded into the entered page through
`page-input-forwarding.ts` (`sendInputEvent`, with a CDP
`Input.dispatchMouseEvent` variant for cross-origin iframes). The May 2026
PoC validated hover, cursor styling (via `page-cursor-bridge.ts`), click,
drag, scroll with trackpad inertia, right-click and form input
([aboveview-interactive-layer.md](plans/aboveview-interactive-layer.md) §9).
Recorded carve-outs: drag-out of the page, IME, DevTools-while-focused.
Keyboard is the one input that is *not* forwarded — it rides
`webContents.focus()` (`focus-reconciler-runtime.ts`). So "one interactive
view at a time with forwarded input" is not a new model for this app; the
question is only whether the target can be an offscreen webContents.

**ADR 0035.** Idle pages are frozen with `Page.setWebLifecycleState`, never
hidden, because `capturePage` is how agents see. OSR pages are never
background-throttled by Chromium, but the lifecycle freeze still applies to
them, and under shape B the agent screenshot path could read our retained
texture copy instead of `capturePage`, which removes the "hidden view returns
a blank frame" constraint that shaped that ADR.

## 3. What Electron 43 actually provides for shape B

Verified against the v43.2.0 docs and source (electronjs.org is unreachable
from this environment; GitHub raw docs were used).

- **`offscreen: { useSharedTexture: true, deviceScaleFactor }`** delivers each
  frame as an IOSurface-backed texture on the `paint` event, from a pool of
  10 per page, with dirty rects, no frame-rate cap, and no frames at all when
  the page is idle. `deviceScaleFactor` is also creation-time; setting it to
  2 gives a crisp texture up to 2× the CSS size, which matches what the
  current hi-res settle capture targets.
- **`sharedTexture` module** (experimental): `importSharedTexture` in main,
  `sendSharedTexture` to a renderer frame (1000ms timeout), and
  `getVideoFrame()` in the renderer yields a `VideoFrame` usable with
  Canvas2D `drawImage`, WebGL or WebGPU. No native addon. Release is
  sync-token aware. **The Electron spec for this module runs only on macOS
  arm64** (`spec/api-shared-texture-spec.ts` line 14) — the platform question
  (Open Question 8) is answered: this is a macOS-first path and untested by
  upstream elsewhere.
- **The frame must be copied.** Chromium's OSR README is explicit: the pool
  texture is reused the moment it is released, so consumers draw it into a
  texture they own and release immediately. That copy is what makes shape B
  immune to ADR 0037's problem — the app always holds the last frame of
  every page, regardless of what Chromium's `FrameEvictionManager` does.
- **Only benchmark on record:** 16 pages at 720p ≈ 54fps, at 1080p ≈ 35fps,
  all pages animating, CPU-bound on per-frame IPC
  ([electron#46811](https://github.com/electron/electron/pull/46811)). A
  mostly static canvas is far below that.
- **Open bug:** an offscreen `WebContentsView` paints at the window's size,
  not its own ([electron#45864](https://github.com/electron/electron/issues/45864),
  still open) — use hidden offscreen `BrowserWindow`s per page, as the
  original OSR survey assumed.
- **Input on an offscreen page.** `sendInputEvent` routes to the OSR widget
  (`electron_api_web_contents.cc` ~3797), and CDP `Input.*` works on any
  webContents. `page-input-forwarding.ts` already speaks view-local
  coordinates, so pointer and wheel would carry over. The gaps are in
  `osr_render_widget_host_view.h`: `Focus()` is a no-op, `UpdateCursor` is a
  no-op, and `TextInputStateChanged` / `ImeCancelComposition` are no-ops.
  In practice: keyboard has to go through CDP `Input.dispatchKeyEvent`
  rather than `webContents.focus()`, the cursor bridge probably stops
  receiving `cursor-changed`, and **IME composition is unsupported** —
  typing CJK or using dead keys into a page would break. `<select>` and other
  popup widgets arrive as separate `widgetType: 'popup'` textures that have
  to be composited at their own rect. DevTools, `capturePage`, and the agent
  CDP path are unaffected.

## 4. The open questions, answered as far as they can be from here

1. **Toggle at runtime?** No (§1). The design has to be B or C.
2. **Seamless swap?** Moot under B (no swap). Under A it is the ADR 0037
   unpark cost plus a fresh renderer with no tiles, no scroll, no state.
3. **Real cost of shared textures?** Zero-copy on the GPU; IPC per frame on
   the CPU; ~10 pool textures per page; one retained copy per page in our
   compositor (20 pages at 1280×800 @2× ≈ 330MB of GPU memory if every copy
   is full-res — the perf log already flagged a ~200MB budget as owed for the
   JPEG path). Whether it beats today's JPEG freeze can only be measured; the
   JPEG path costs 17–34ms of synchronous main-thread encode per prepare
   and a decode storm per settle, and shape B has neither.
4. **Frame-rate governance?** `setFrameRate` per page; the lifecycle freeze
   for culled/idle pages (ADR 0035's mechanism, minus its `capturePage`
   constraint); zero frames from static pages for free. Waking a frozen OSR
   page is a lifecycle flip, not an unpark.
5. **Animated/video pages?** They generate frames continuously and the
   author's benchmark is exactly that worst case. Throttle by visibility
   and zoom; a page at 15% zoom does not need 60 frames/s.
6. **Hover/focus across a swap?** Under B there is no swap, but hover and
   cursor styling need the OSR-specific verification above; IME is the
   confirmed casualty.
7. **Layering?** Under B, every page is canvas content and the ADR 0014
   cross-surface constraint disappears entirely: one renderer, one z-order,
   drawings under and over pages freely. Under a hybrid with one live
   `WebContentsView`, the constraint shrinks to that one page — everything
   still has to be split into "below the live page" (bgView) and "above it"
   (aboveView), now dynamically by `entityOrder` relative to whichever page
   is entered. That is a smaller problem than today's, not a solved one.
8. **Platforms?** Upstream tests macOS arm64 only (§3).

## 5. Where this leaves the spike

The cheapest first experiment in the research doc ("one offscreen page,
shared texture, pan/zoom it, then make it live and back") cannot run as
written, because "make it live" is not an operation. Re-scope it to answer
the question that actually decides B:

**Spike 1 — the texture tier (1–2 days).** One hidden offscreen
`BrowserWindow` with `useSharedTexture: true, deviceScaleFactor: 2` →
`importSharedTexture` → `sendSharedTexture` to canvas-bg → `getVideoFrame()`
→ `drawImage` into the existing chrome canvas at the projected rect, replacing
the JPEG `ImageBitmap` in `drawItemSnapshot`. Then 10 and 20 pages. Measure
against the current build with the perf-trace route: frames/s, pool pressure
(`release` latency), GPU memory, browser-main and canvas-bg busy % over the
scripted pan and zoom profiles. This is §2.B experiment 4 of the unknowns
doc, and it is worth running even if B is never adopted, because it would
replace the JPEG freeze substrate in shape C.

**Spike 2 — can the interactive page be offscreen too? (1–2 days, decides B.)**
Point `page-input-forwarding.ts` at the offscreen webContents; keyboard via
CDP `Input.dispatchKeyEvent`. Check, in this order, because each is a
recorded gap: text selection and hover cursor; `<select>` popup compositing;
IME composition (expected to fail — decide whether a hybrid "enter a real
view only for text editing" is acceptable, which reintroduces A for that
case); drag in and out of the page; DevTools attached; an agent driving the
page over CDP while a user pans.

If Spike 2 passes, B is a one-renderer canvas with no `setBounds` loop, no
device emulation, no freeze machinery, and no layering bands — a large
deletion. If it fails on IME or popups, the honest fallback is C plus
Spike 1's texture tier, which keeps today's input model and still removes
the JPEG encode/decode cost. Either way, do not build A.

## 6. The lab: running Spike 1 and Spike 2

Both spikes are built as one window, **View → Open Offscreen Rendering Lab**
(`⌘⌥⇧O`), in `src/main/osr-lab/`, `src/preload/osr-lab.ts` and
`src/renderer/osr-lab/`. It is deliberately independent of the canvas runtime:
its numbers are about Electron's offscreen path, not about Specular's layout
engine, so they compare cleanly against a trace of the real canvas.

What it does:

- Creates one hidden offscreen `BrowserWindow` per URL with
  `offscreen: { useSharedTexture, deviceScaleFactor }`. In shared-texture mode
  each `paint` event's texture is imported in main, transferred to the lab
  renderer with `sendSharedTexture`, copied to an `ImageBitmap` in the preload
  and released; the copy is handed to the page world as a `postMessage`
  transfer. In bitmap mode the same frames go through `toJPEG(80)` on main and
  decode in the renderer — the pipeline the current zoom freeze uses, as the
  baseline arm.
- Draws every page's latest frame at its camera-projected rect on one Canvas2D
  surface. Wheel pans, ⌘/Ctrl+wheel zooms about the pointer, Alt-drag or
  middle-drag pans.
- Click a page to enter it (Esc or a click on empty canvas leaves). The
  entered page gets pointer and wheel events in page-local CSS coordinates,
  keyboard events from a hidden sink input, and IME commits via
  `Input.insertText`. Pointer and keyboard transport are each switchable
  between `sendInputEvent` and CDP so both can be tried. Entering also turns
  on `Emulation.setFocusEmulationEnabled`, because the OSR widget host's
  `Focus()` is a no-op.
- Off-screen pages get `stopPainting()` (toggle), popup-widget textures are
  drawn at the page origin with a red outline (Electron reports no position
  for them), `cursor-changed` events are counted and mirrored onto the canvas
  cursor, `capturePage()` on the entered page shows what an agent would see,
  and DevTools can be opened on it.
- The HUD shows draw rate and draw ms, frames received per second, texture
  copy ms, GPU and page-renderer working sets, per-page frame counts, texture
  size, outstanding-texture high-water mark against the 10-deep pool, release
  latency, JPEG encode ms, cursor events and painting state.
- **Run benchmark** drives the camera through the same four gesture shapes as
  the app's pan/zoom perf profiles (slow pan, slow zoom, fast pan, pan+zoom)
  one rAF at a time and reports draw fps, mean and worst frame interval, long
  frames and frames received per phase as JSON. **Benchmark + Chromium trace**
  wraps it in the same all-process trace the View menu records, so the trace
  can be read with the existing `trace-summary` tooling and compared to the
  perf log's numbers.

### Spike 1 protocol — the texture tier

1. macOS arm64, `pnpm dev`, open the lab. Defaults: 9 pages, 1280×800 at 2×,
   shared-texture mode. Press **Start pages**, then **Fit all**.
2. Confirm the pages paint at all. If the HUD shows "paints without texture",
   the GPU path is unavailable on this machine and everything below is moot.
3. Pan and zoom by hand. Watch draw ms and frames-in per second; a static
   canvas should receive zero frames between gestures.
4. **Run benchmark** at 1, 9 and 20 pages (change Count, Restart pages, Fit
   all). Record the JSON for each. Then switch to bitmap-JPEG mode and repeat
   9 pages: the difference is the JPEG encode/decode cost the current freeze
   pipeline pays.
5. Watch `out/max` per page. A max near 6 means the renderer copy is falling
   behind the pool; pool drops show in the HUD.
6. With 20 pages, compare the GPU working set against the same 20 URLs open in
   the real canvas (Motion Debug window → Processes).
7. **Benchmark + Chromium trace** at 9 pages, then run the app's own pan/zoom
   perf test on 9 live pages and compare `summary.threads` busy % and the top
   events.

### Spike 2 protocol — can the interactive page be offscreen?

Enter a page and try, in order, with `sendInputEvent` and then CDP for each
transport:

1. Hover links and text: does the cursor column count rise and the canvas
   cursor change? (If it never does, `cursor-changed` is dead under OSR.)
2. Click links, scroll with the wheel, drag to select text. Is the selection
   drawn active (blue) rather than inactive (grey)?
3. Type into a text field: ASCII, then a dead key (⌥e then e), then a CJK
   input method. Composition is expected to fail; the commit should still
   land through `Input.insertText`.
4. Open a `<select>`: does a popup frame count appear, and does the popup
   texture arrive as a separate widget?
5. Drag an image out of the page onto the canvas. Expected to fail.
6. **DevTools for entered** while interacting; then **capturePage() entered**
   and check the thumbnail is current.
7. Alt-tab away and back; keep interacting. `sendInputEvent` normally needs
   the owning window focused — check whether the hidden window cares.

Everything that fails in Spike 2 is a cost of shape B. If nothing fails that
matters, B is the design; otherwise the fallback is shape C with Spike 1's
texture tier replacing the JPEG freeze.
