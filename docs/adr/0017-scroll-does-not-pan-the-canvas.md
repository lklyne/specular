# ADR 0017 — Scroll does not pan the canvas

**Status:** Accepted
**Date:** 2026-06-01
**Related:** [docs/interaction-layer.md](../interaction-layer.md) §4 (input authority), [docs/input-authority-audit.md](../input-authority-audit.md).

## Context

Since the aboveView interactive-layer migration, a plain wheel / two-finger
trackpad scroll over the canvas mapped to `canvasPan` — the `'pan'` branch of
`classifyViewportWheel`. This matched the Figma-style "two-finger scroll pans
the viewport" convention.

In practice it fights the product's other half. Specular is a browser *and* a
canvas: pages and notes hold scrollable content, and the dominant intent when
scrolling is "scroll the thing under my cursor," not "pan the world." With a
page selected, moving the cursor just outside its frame and scrolling slid the
whole canvas (dots + page card) — surprising, since nothing scrollable was
under the cursor and the user read the gesture as a no-op zone.

We already route wheel to the right target by cursor position:

- Over a selected/edited **page** → forwarded into the page's webContents
  (`routeWheel` → `forwardWheelToPage`).
- Over a selected/edited **note** with overflow → yielded to native DOM scroll
  (`yieldWheelToNativeScroll`, the markdown-note scroll fix).

The only remaining wheel consumer was "everything else → pan."

## Decision

**A plain wheel / two-finger scroll never pans the canvas.** The `'pan'` branch
of both wheel handlers (`useViewportWheelAndMiddlePan` in aboveView,
`useCanvasViewportGestures` in bgView) is a no-op. Wheel still:

- **Zooms** on `Cmd`/`Ctrl`+wheel and trackpad pinch (the `'zoom'` branch,
  unchanged).
- **Scrolls** a hovered page or note (routing unchanged).
- **Does nothing** over empty canvas or a non-scrollable entity.

Panning remains available through explicit gestures, all unchanged:

- **Hand tool** (toolbar) — drag to pan.
- **Space-drag** — hold `Space` and drag (temporary hand tool).
- **Middle-button drag** — drag to pan.

## Consequences

- Reading a page/note no longer drifts the canvas underneath it.
- Users lose trackpad-scroll panning; discoverability of the hand tool /
  space-drag / middle-drag matters more. Acceptable: those are standard and
  already present.
- Cursor-position routing is now the single rule for what a wheel event means,
  which is simpler to reason about than "scroll = pan unless over a target."
- Reversible: re-enabling is a one-line restore of the `canvasPan` call in each
  handler.
