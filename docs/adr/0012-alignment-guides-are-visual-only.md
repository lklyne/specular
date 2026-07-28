# ADR 0012 — Alignment guides are visual-only; grid-snap remains the sole magnet

**Status:** Accepted
**Date:** 2026-05-14

## Context

Specular's drag flow has always pulled entities to a 20 px grid via `snapToGrid` inside `applyDragDelta` (`src/main/runtime/document-commands.ts`). Adding FigJam-style alignment guides — top / bottom / left / right / horizontal-center / vertical-center lines that appear when the dragged entity aligns with neighbors — raises the question of whether those guides should be magnetic (pull the entity onto them) or purely visual (render only when alignment already happens to be true).

## Decision

**Guides are visual-only. Grid-snap is the only magnetic pull during drag.** A guide renders iff the dragged entity's edge or center is within 0.5 px of a snap candidate's edge or center *after* `snapToGrid` has run. The guide list is broadcast from main to `aboveView` for paint; no second projection runs in main.

Distribution guides (`==` marks for equal spacing) follow the same contract — detection only, never pull.

## Considered options

- **Magnetic guides (FigJam parity).** Guides override grid-snap on engaged axes; grid is the per-axis fallback. Better feel, but introduces a second magnetic system that fights the grid on disengaged axes and changes the meaning of every coordinate `applyDragDelta` produces.
- **Drop grid-snap entirely.** Cleanest, matches FigJam exactly. Rejected because the 20 px grid is part of Specular's current placement feel and dropping it is a much larger product change than adding guides.
- **Magnetic with a shift-to-suppress modifier.** Conflicts with the axis-lock claim on `Shift` (see CONTEXT → Drag affordances → Axis lock); would need a different modifier and a parallel concept of "snap mode."

## Consequences

- Center alignment will rarely engage. An entity's center sits at `x + width/2`; unless `width` is a multiple of 40, the center is off-grid, so the post-grid-snap position will almost never satisfy the 0.5 px equality with another center. Edge alignment (top / left between two grid-placed entities) will engage often.
- The guide is honest by construction: if the line is on screen, the entity is actually aligned. There is no flicker-and-bounce from the grid pulling the entity off a guide it briefly hit.
- The implementation stays inside `applyDragDelta`'s existing seam: snap to grid → compute 6 reference points → diff against the drag-begin snapshot of viewport-visible candidates → broadcast guide list. No new interaction mode, no second projection site, no reconciler.
- Pivot path: if center alignment turns out to be the feature users actually want, the cheapest move is to drop grid-snap (or reduce it to 1 px) rather than introduce magnetic guides on top of the grid.
