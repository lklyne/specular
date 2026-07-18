# ADR 0001 — Click-to-enter frame focus replaces rasterization-dependent gate

**Status:** Accepted
**Date:** 2026-05-04
**Supersedes premise of:** `docs/interaction-layer.md` §4.2 (input gate) and §4.7 (bitmap compositor)

## Context

`docs/interaction-layer.md` specifies a single input authority via the `aboveView` gate. To keep `aboveView` always-visible without breaking native page input, §4.7 prescribed rasterizing inactive frames into `bgView` (Phase 6). Phase 6 is multi-quarter and entangles rasterization fidelity (fonts, scroll peers, accessibility, GPU memory) with the gesture-routing fix.

Issue #41 surfaced a representative bug from the interim state: edge anchor dots' inflated hit ring shadows frame chrome, allowing a URL-bar misclick to drag an edge endpoint off the frame. The class — "layer A's hit area shadows layer B's interactive control, both sit in bgView's DOM, no arbitration between them" — is latent across ~7 layers and 145 ESLint warnings.

## Decision

> **Implementation note:** Several specifics below evolved in later ADRs. `frameFocus` was renamed and restructured by ADR 0021 (`FocusSession`). The "total focus" model (Escape to exit, two clicks to act elsewhere) was replaced by ADR 0022 (select-first/interact-second: `interactivePageId` in runtime-context). The hit-priority table grew `reorder-handle` and `gap-handle` layers (ADR 0015) and lost the `chrome` layer (ADR 0028). For the current model read ADRs 0021 and 0022. The core commitment — WebContentsViews stay live; `aboveView` is the default pointer authority — still holds.

Frames remain live `WebContentsView` instances at all times — they paint, run JS, play media, fire timers. They do **not** receive native pointer input until explicitly focused.

A new runtime variable `frameFocus: { id } | null` lives in main. The input gate predicate becomes:

```ts
shouldGateBeOpen(s) === (s.frameFocus === null)
```

When `frameFocus === null` (canvas mode), `aboveView` is `setVisible(true)` and is the sole input authority. When `frameFocus` is set, `aboveView` is `setVisible(false)` and the focused frame receives native input. Exit detection rides on the focused frame's `webContents` `blur` event — when the user clicks any other view, blur fires and main clears `frameFocus`.

**Focus model — total focus.**
- Click on a frame body → enter focus.
- Click anywhere else (canvas, another frame, sidebar) → blur fires → exit focus. The exit click does **not** double as the next interaction; two clicks to act on another canvas element.
- Escape → exit focus.
- DevTools attach is treated as a companion to focus, not an exit (focus *intent* in main is tracked separately from actual webContents focus; `FocusReconciler` reasserts).

**Hit-region priority table — 5 layers, top wins.**

| # | Layer | Action |
|---|---|---|
| 1 | resize-handles | begin resize |
| 2 | chrome (frame + entity) | drag / select |
| 3 | edge anchors | begin edge drag |
| 4 | body | frame → enterFocus; other entities → select |
| 5 | background | marquee / pan |

Load-bearing constraints encoded:
- Resize handles above chrome — once selected, the next gesture is shaping.
- Chrome above anchors — fixes #41.
- Body kind-dispatches — same priority slot, behavior chosen by entity kind.

Edges visually cross frame bodies; clicking such an overlap goes to the frame body (focus). Clicking inside a group selects the inner entity, not the group. Group bounds folds into the body layer.

## Consequences

**Replaces:**
- `interaction-layer.md` §4.2 visibility predicate `shouldGateBeOpen` becomes a one-liner.
- `interaction-layer.md` §4.7 bitmap compositor becomes optional (a future memory/perf optimization, no longer load-bearing).
- The Path B "renderer-local dispatcher" alternative from issue #41 — superseded; the dispatcher lives in main.

**Enables:**
- Single hit-test authority in main, testable without DOM.
- `no-mouse-events` ESLint rule promotable to project-wide error.
- Bug class from #41 structurally eliminated, not patched.

**Costs:**
- Users lose hover-tooltips on a focused frame's chrome / anchors. To interact with chrome of a focused frame, Escape first.
- Two clicks to act on another canvas element after focus.
- Webhooks reliability: depends on within-window WCV-to-WCV `webContents.blur` firing reliably across DevTools, modals, and programmatic focus moves. Spike validates before commit.

**Out of scope:**
- Bitmap compositor / Phase 6. Still tracked separately if memory/CPU regressions of N live frames demand it.
- Per-page sub-region carve-outs (the "partial focus" alternative considered and rejected).
