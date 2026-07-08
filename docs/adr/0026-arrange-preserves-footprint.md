# ADR 0026 — Arrange tidies in place; distribute folds in

**Status:** Accepted
**Date:** 2026-07-06
**Related:** [ADR 0015 — Auto-layout groups](./0015-auto-layout-groups.md) (introduced Distribute as D7), [ADR 0019 — Canvas as document CLI](./0019-canvas-as-document-cli.md) (the `arrange` verb).
**Supersedes:** the standalone Distribute button / `distribute` verb / `POST /selection/distribute` / `src/shared/distribute-row.ts` from ADR 0015 D7.

## Context

The multi-select popup carried four spacing actions: **row**, **column**, **grid** (arrange, ADR 0019) and **distribute** (even the gaps in place, ADR 0015 D7). They split along a hidden seam:

- Arrange **repacked**: sort into reading order, collapse to a fixed gap from the cluster's top-left. It threw away the layout the user had built and locked them into a gap they never chose and couldn't see.
- Distribute **preserved the footprint**: pin the first and last item, even the gaps between — but left the cross-axis untouched, so it wasn't "a row," just "x-gaps evened."

Four concepts, two of which overlapped confusingly. Users reaching for "column" on a wide, unevenly-spaced row got a tight stack, not the even wide row they meant. FigJam's "tidy up" — verified against a sparse/odd grid — does the opposite: it keeps the footprint and holes and only regularizes spacing. That is the behavior users expect from a tidy gesture.

## Decision

**Row / column / grid preserve the footprint by default; distribute is deleted, its behavior absorbed.**

1. **Tidy in place is the default.** With no explicit gap, arrange keeps the cluster's current extent and only evens the spacing inside it. Row/column pin the outer items, even the gaps along one axis, and align the other (to the leading edge — tops for a row, left edges for a column). Grid keeps the existing 2-D structure — rows and columns are read off current positions by overlap-clustering, holes are preserved — and evens gaps on both axes. Grid is not a special ragged-packing problem; it is the same gap-evening applied per-axis over bands.

2. **Packing to a fixed gap is the exception, not a peer.** The old collapse-to-a-gap behavior survives only behind the CLI's `--gap` flag (`arrange row --gap m`), where an agent is explicitly asking for a chosen gap. The toolbar never reaches it. The create/placement path (`applyLayoutDirective` from `upsertEntities`) is unchanged — new entities have no existing footprint to preserve, so they still pack.

3. **Distribute is gone, not relocated.** Its "even the gaps, keep the endpoints" logic now lives inside every arrange mode, so the standalone button, verb, route, and kernel are deleted rather than kept as a fourth concept. Three buttons, one mental model: "regularize spacing, keep the footprint."

One kernel implements all three modes: `src/shared/span-arrange.ts` (`arrangeInSpan`). `arrangeEntities` branches on whether a gap was passed — absent → span-preserve, present → the extracted `packEntities` (the former behavior).

## Consequences

- The popup drops from four spacing actions to three; the hidden arrange-vs-distribute seam disappears.
- Grid band-clustering uses overlap runs, no tuned threshold. A diagonal staircase merges into one band and touching-but-not-overlapping items split — marked `ponytail:` in the kernel with a center-tolerance upgrade path if real grids band wrong.
- Cross-axis alignment is leading-edge (tops/left), chosen for consistency with the existing top-left anchor. Switching to centers is a one-line change in `arrangeInSpan` if it reads better.
- Coverage: `arrangeInSpan` is unit-tested per mode (`tests/unit/span-arrange.test.ts`); the commit path (drawing-stroke travel, group-descendant carry, single-undo batching, span-vs-`--gap` branch) is integration-tested (`tests/integration/arrange-selection.test.ts`).
- The CONTEXT.md **Distribute** glossary entry is replaced by **Arrange**; `distribute` is dropped from the verb surface in `src/main/entities/CLAUDE.md`.
