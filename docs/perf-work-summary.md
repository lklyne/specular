# Canvas zoom and pan performance handoff

Branch: `perf/zoom-emulation-pan-dedirty`

## Goal

Make canvas pan and zoom feel compositor-smooth with several live web pages.
It is acceptable for pages to become static and non-interactive during a zoom
gesture.

## What has been implemented

### 1. Repeatable performance measurement

- Added isolated pan, zoom, and combined-motion profiles to the existing perf
  runner.
- Added a macOS native-window frame capture path because Electron renderer
  capture does not include the composed `WebContentsView` result.
- Added `/perf/pan-zoom/visual-run` to run a profile, collect the native frame
  sequence, and optionally compare live pages with frozen snapshots.
- The detailed measurements and earlier experiments are in
  `docs/perf-zoom-pan-log.md`.

### 2. Reduced live-page raster work

- Quantized Chromium device-emulation scale during zoom instead of changing it
  on every input tick.
- Kept the final exact scale update when zoom settles.
- This substantially reduced raster work, but live `WebContentsView` resizing
  can still visibly stutter whenever the snapshot path is unavailable.

### 3. De-dirtied camera movement

- Pan and zoom publish a lightweight viewport nudge rather than rebuilding and
  broadcasting the complete scene on every input tick.
- Renderer scene layers use one live `translate3d + scale` transform and
  reconcile to a fresh layout after motion settles.
- Corrected the transform math to include each renderer's fixed scene origin.
- The grid derives the exact live camera rather than waiting for the settled
  layout.
- Zoom and its anchor-correcting pan now land as one camera update, avoiding a
  transient half-updated camera.

This eliminated the earlier large positional drift. The latest observed border
problem is not border/page drift.

### 4. Experimental frozen-page compositor

- Captures each visible page with `webContents.capturePage()` while idle.
- Sends the encoded page images to `canvas-bg`, decodes them in advance, and
  waits for a renderer-ready acknowledgement.
- During a snapshot-backed zoom, the layout engine parks the native page and
  frame views at hidden bounds while the decoded images move with the renderer
  scene.
- Snapshot validity includes page identity, logical viewport size, navigation
  generation, and native scroll offset.
- A capture lease rejects results if zoom or page state changes while
  `capturePage()` is in flight.
- The latest prepared bitmap can be reused across zoom levels and refreshed
  later for sharpness.
- The zoom-motion lease was increased from 120 ms to 300 ms to bridge pauses
  within a fast macOS trackpad gesture.

In the controlled frozen A/B profile this cut browser-renderer main-thread work
by roughly 66%, Viz work by roughly 75%, and layout/style activity by roughly
73%. Slow zooms now look nearly correct in human testing.

## What is actually composited

Only the page content raster is captured.

The following remain separately rendered DOM/SVG scene layers:

- the outer and inner page borders;
- device shells and their strokes;
- group chrome and other canvas decorations.

`PageBorderLayer` currently draws two `1px solid` absolutely positioned DOM
borders. During live zoom, their last settled geometry is scaled by the scene
ancestor transform. The borders and the page raster share the same positional
transform, so they can remain aligned, but their strokes are still independently
rasterized and may become soft or low-quality until a later layout/render
baseline replaces them.

## Current human-observed failures

These are two separate issues:

### A. Frozen page raster disappears during fast zoom

The prepared page image frequently drops out during a fast gesture. Once that
happens, the app can fall back to live `WebContentsView` pages for the rest of
the motion. Those live pages visibly stutter as their native bounds and
emulation scale change.

The 300 ms settle lease, decode acknowledgement, capture invalidation, and
cache-reuse changes reduce known races but have not eliminated this failure.
Do not treat the current snapshot lifecycle as hardened.

### B. Page border appears low-quality after zoom

After some zooms, a page border appears to be a low-quality scaled version and
can take a noticeable time to settle. It is not visibly drifting away from the
page. The leading hypothesis is resampling of the separately rendered 1 px
DOM/SVG border from an old scene baseline, but this has not yet been
instrumented or proven.

## Native page scrolling

Native page scrolling is still intentional and does not inherently conflict
with the compositor. Wheel input over an eligible selected page is forwarded
to that page; its scroll offset participates in snapshot cache validity so a
pre-scroll capture should not be reused after the page scrolls.

An older Browser-mode-specific scroll rule was removed when Browser mode itself
was deleted. That history is not the cause of the current zoom failures.

## Recommended next investigation

### First: make the snapshot transition a strict state machine

Model the path explicitly, for example:

`live -> preparing -> ready -> frozen -> restoring -> live`

For one physical gesture, choose the rendering substrate once:

- If every visible page has a decoded, valid raster at gesture start, remain
  frozen until restoration completes.
- Otherwise remain live for the entire gesture.
- Never replace a complete prepared set with a partial capture.
- Never switch from frozen to live in the middle of a gesture.
- Restore native views only after their final bounds/emulation update is ready,
  and keep the raster visible through a confirmed compositor frame.

Instrument every state transition with gesture generation, snapshot revision,
expected/captured page IDs, content signature, renderer-ready revision, native
view visibility, and the reason for fallback. The native visual-run harness can
then correlate a dropout frame with the exact transition.

### Second: treat page content and chrome as one zoom presentation

Because temporary non-interactivity is acceptable, the strongest direction is
one per-page zoom presentation containing both content and border/device chrome.
Options include:

1. Rasterize content plus chrome into a single padded image for the gesture.
2. Keep the content bitmap and chrome in one per-page compositor subtree, but
   render strokes at the current display scale instead of scaling a stale 1 px
   border.

The first option is less elegant but gives the cleanest prototype proof: one
texture, one transform, and no independent border settling during zoom.

### Third: stop touching native views while frozen

Verify with instrumentation that a successful frozen gesture performs no
per-tick `setBounds`, emulation-scale update, visibility toggle, or capture on
the live views. Apply their final geometry once behind the still-visible raster,
then swap back atomically.

## Verification status

Before this handoff:

- `pnpm typecheck` passed.
- `pnpm test:unit` passed.
- `pnpm test:integration` passed: 214 tests.
- The fast visual profile reproduced a missing-raster frame followed by live
  page recovery.
- Latest human testing still reproduces both failures described above.

The snapshot approach is promising and measurably faster, but it is still an
exploratory prototype rather than a complete compositor contract.
