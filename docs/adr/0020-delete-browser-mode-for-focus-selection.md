# ADR 0020 — Delete Browser Mode for Focus Selection

Status: Accepted

Date: 2026-06-24

## Context

Browser mode was added as a second presentation of the same canvas data: pages
could appear as browser-like tabs while canvas mode kept the freeform spatial
layout. Over time that split made the system harder to reason about. Layout,
input gates, toolbar controls, page visibility, and persistence all had to ask
which mode was active, even though the user's underlying work was still a
single canvas document.

The useful behavior was not the mode itself. Users needed a quick way to bring
one page or canvas item into attention, then return to the broader spatial
context.

## Decision

Delete Browser mode as an active runtime state and keep Specular in the canvas
view at all times.

Replace the mode switch with `Focus selection`, an ephemeral camera command
that:

- centers and zooms to fit the selected item or multi-selection with padding;
- caps zoom at 100%;
- preserves each node's persisted width and height;
- on exit (Escape / X / dimmed-canvas click) keeps the current camera position
  and zooms out a touch (`FOCUS_EXIT_ZOOM_OUT`, anchored on the viewport
  center) rather than restoring the pre-focus camera — easy in-focus
  navigation made the original position usually irrelevant.

(The original design restored a stored pre-focus camera; [ADR 0021](./0021-focus-session-as-first-class-concept.md)'s
session rework dropped that in favor of the zoom-out-in-place exit above. `returnCamera`
was never added to the final `FocusSession` struct. Focus exit always zooms out by
`FOCUS_EXIT_ZOOM_OUT`; no return camera is captured or read.)

Legacy `viewMode` and `browserTabMode` fields remain readable for old files.
On restore, Browser-mode metadata selects the formerly active page and then the
workspace proceeds as a normal canvas document.

## Consequences

- Layout and input no longer branch on Canvas vs Browser mode.
- Pages are never temporarily resized to fill the app viewport.
- Toolbar chrome is simpler: one focus button instead of a Canvas/Browser
  segmented control and tab strip.
- The phrase "part browser" now means live page nodes on a spatial canvas, not
  a dedicated browser view.
- Old files continue to open, but new saves stop writing Browser-mode metadata.
