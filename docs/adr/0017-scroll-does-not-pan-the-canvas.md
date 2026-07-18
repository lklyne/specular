# ADR 0017 — Scroll does not pan the canvas in browser mode

**Status:** Accepted
**Date:** 2026-06-01
**Related:** [docs/interaction-layer.md](../interaction-layer.md) §4 (input authority), [docs/input-authority-audit.md](../input-authority-audit.md).

## Context

A plain wheel / two-finger trackpad scroll maps to `canvasPan` — the `'pan'`
branch of `classifyViewportWheel`. In **canvas mode** that is the primary
navigation gesture and works well; it is unchanged.

In **browser mode** the same gesture is wrong. Browser mode presents a single
page below the URL bar; there is no spatial canvas to navigate. But the canvas
surface (grid dots + the page's backing card) still sits behind the page, and a
scroll over the empty area dragged it around — the dots and the card slid while
the page stayed put. The user reads browser mode as "a browser," where scroll
should affect page content (or nothing), never pan a world they can't see.

Two wheel handlers exist:

- `useViewportWheelAndMiddlePan` (aboveView) — the canvas-mode wheel authority.
  In browser mode aboveView is hidden (gate closed), so this handler does not
  fire for empty-area scrolls.
- `useCanvasViewportGestures` (bgView) — the fallback. In browser mode this is
  the handler that receives an empty-area scroll, and it was panning.

## Decision

**Wheel-pan is canvas-mode only.** The bgView wheel handler pans only when
`viewMode === 'canvas'`. In browser mode a plain wheel/two-finger scroll over
the empty area does nothing. Both modes keep:

- **Zoom** on `Cmd`/`Ctrl`+wheel and trackpad pinch.
- **Scroll** of a hovered page (forwarded into its webContents) or note
  (native DOM scroll).

Canvas mode is untouched — wheel-pan, hand tool, space-drag, and middle-button
drag all behave exactly as before.

## Consequences

- Browser mode no longer drags the canvas behind the page on scroll.
- Canvas-mode navigation is unchanged (this ADR deliberately does **not** remove
  scroll-to-pan there — an earlier draft did, and that was wrong).
- The split lives in the bgView handler's `viewMode` check; the aboveView
  authority is unchanged.

## Implementation note

The `viewMode === 'canvas'` guard described in the Decision was the fix as originally coded, but it became redundant and was removed when ADR 0020 deleted browser mode entirely. `useCanvasViewportGestures.ts` now pans unconditionally — the structural removal of browser mode is the guard. Wheel-pan over empty canvas still only reaches bgView when aboveView does not intercept it first, which matches the original logic.

## Note

This does not address a separate, pre-existing browser-mode bug: the page's
bgView backing card is positioned at canvas coordinates while the live page is
re-centered/fit to the browser viewport, so the two diverge (visibly at
non-100% zoom). That is tracked separately.
