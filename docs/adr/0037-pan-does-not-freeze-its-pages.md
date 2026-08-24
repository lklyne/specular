# ADR 0037 — A pan does not freeze its pages

**Status:** Accepted
**Date:** 2026-08-23
**Related:** [ADR 0023](./0023-renderer-owned-camera-gpu-panzoom.md) §"Endgame B" (freeze-to-bitmap as the general answer to gesture drift), [docs/pan-zoom-perf-unknowns.md](../pan-zoom-perf-unknowns.md) §Endgames, [docs/perf-zoom-pan-log.md](../perf-zoom-pan-log.md) Exp F–H (the measurements)

## Context

The zoom snapshot freeze parks every visible page at zero bounds behind a
bitmap for the duration of a zoom gesture, and zoom got dramatically smoother
for it. Pan never had one: it moved live `WebContentsView`s through the window
server, one `setBounds` per page per input tick.

Once zoom was smooth, pan became the worst-feeling gesture on the canvas, and
the obvious move was to give it what zoom got. Both the ADR 0023 postmortem and
the unknowns survey point that way — they treat freeze-to-bitmap as the general
cure for gesture cost and two-substrate drift, with zoom merely the first
gesture to get it.

That generalization is wrong, and the reason is worth writing down, because the
pan freeze is small to build, plausible on its face, and measurably harmful.

## Decision

**The snapshot freeze is for gestures that change scale.** Zoom freezes. Drag
freezes (a different freeze, on `aboveView`). **Pan keeps its pages live.**

## Why the premise does not carry over

The freeze exists to stop re-raster, not to stop movement.

A zoom changes `enableDeviceEmulation({ scale })` on every tick. Blink responds
with `SetNeedsRecalculateRasterScales()` — every `PictureLayerImpl` recomputes
its ideal raster scale and re-rasters all tilings, with none of the throttling a
real pinch gesture gets. Parking the page stops that storm outright, and the
storm is large enough that the price of parking is worth paying.

A pan changes no scale. The pages are already composited, and moving a
composited layer is close to free. So the freeze has almost nothing to save —
and it still charges the price:

**A parked page drops its tiles.** Parking means zero bounds, and when the pan
ends and the views unpark, every page rebuilds what parking threw away. On a
15-page canvas that is ~40 JPEG decodes and ~300ms of image decode arriving
right as the gesture finishes — precisely when the next flick starts. Zoom pays
this too, but zoom's settle re-captures everything anyway and the raster storm it
avoids is far larger.

For pan the trade is simply backwards: it pays a re-raster to avoid work that
was never expensive.

## Measurements

15 pages, zoom 0.21, 1600×1000 CSS at dpr 2, 120Hz display. "Burst" is 20 pan
ticks ~8ms apart (~160ms) then idle; the settle cost lands in the idle tail.

| 160ms pan burst + settle | browser main | JPEG decodes |
|---|---|---|
| pages live (shipped) | 61–64ms | **0** |
| pages frozen | 61–81ms | **40–41 (~300–341ms)** |
| after reverting the freeze | 67–68ms | **0** |

Browser-main work is unchanged across all three rows. That is the load-bearing
observation: no snapshot was re-captured, so the decodes are the pages
rebuilding their own tiles, not the freeze re-encoding frames. A net-zero pan
(out and back, revealing no new page) reproduced it identically at 41 decodes,
which rules out "the pan simply brought new pages on screen".

Against that, freezing does help *during* the pan — over a continuous 1.1s pan,
three runs each side:

| continuous pan | freeze off | freeze on |
|---|---|---|
| browser main | 18% of wall | 9% |
| GPU main | 36% | 25% |
| canvas-bg renderer main | 40% | 41% |
| renderer raster pool | 13% | 18% |

Over a 160ms burst that saving is worth roughly 18ms, against ~300ms of decode
after it. The continuous-pan figures above were what first argued for shipping
the freeze; they are real but incomplete, because those traces stop when the pan
stops — before the re-raster the freeze itself caused.

## What actually made pan faster

Neither fix involved the freeze, and both were larger than it:

1. **The dot grid was redrawn as one path fill per dot** — 21,879 `beginPath`/
   `arc`/`fill` calls per frame at this viewport, 6.433ms of an 8.3ms frame
   budget at 120Hz. Drawing one cached tile and letting a repeating pattern do
   the work costs 0.147ms, a 44× reduction, and takes GPU-main from 92% busy to
   24% over a continuous pan.
2. **Every pan discarded the prepared snapshot set**, because the set was keyed
   on each page's view bounds and a pan moves every page. Fifty milliseconds
   after each pan stopped, main re-captured and re-encoded every page to arrive
   back at the frames it already held. Keying on content and resolution instead
   of position removed it.

The grid is the reason the original live-versus-frozen A/B looked so flat on
raster cost: that work was in canvas-bg and identical either way. It also
explains why the freeze looked more promising than it was — it was being
credited for a frame budget the grid was eating.

## Consequences

- `zoom-snapshot-freeze.ts` keeps its name. It is not a general gesture freeze.
- Pan's remaining cost is one `setBounds` per visible page per tick, on the
  browser process. That is real (~18% of wall during a continuous pan) but it is
  compositor-cheap work, and it is not what pan jank was made of.
- Page and chrome still move on two unsynchronized commits during a pan (native
  views on the window server, chrome on the renderer's compositor). Freezing
  would have collapsed that to one commit. Whether that drift is perceptible was
  never measured, and remains the one open argument for revisiting this.

## What would change the answer

A parked page that keeps its tiles. The whole case against the pan freeze is the
re-raster on unpark; remove that and the trade inverts. The `warm` parking mode
that the zoom settle already uses (full size, off-screen, still compositing) is
the shape of it, but it keeps the pages compositing during the gesture, which is
the cost the freeze was trying to avoid in the first place. Nothing here should
be re-attempted until that tension has an answer.

Do not re-attempt this by reasoning from the zoom result. Measure the burst
case — flick, pause, flick — because that is how people pan, and it is where the
freeze loses.
