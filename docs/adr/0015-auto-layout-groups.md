# ADR 0015 — Auto-layout groups

**Status:** Proposed
**Date:** 2026-05-29
**Related:** [ADR 0014 — Canvas stack order and the Notes/Pages sidebar](./0014-canvas-stack-order.md), [ADR 0001 — Click to enter page focus](./0001-click-to-enter-page-focus.md).
**Origin:** Built from [`docs/plans/auto-layout-reorder.md`](../plans/auto-layout-reorder.md) (Milestone 1).

## Context

A selected group can be made to *manage* its children's layout: children pack into a row, and dragging a child's center dot reorders it while siblings reflow to make room (Figma auto-layout reorder).

The seeds already existed: `WorkspaceGroupLayoutMode = 'freeform' | 'row' | 'grid'` and `managedLayout: boolean` were persisted and JSON-Canvas round-tripped, and a pages-only `reflowGroupRow` packed multi-breakpoint page rows on creation. What was missing: a generalized reflow engine for any entity kind, a reorder model, the drag gesture, and the dot affordance.

This ADR records the hard-to-reverse decisions, because they make `managedLayout` load-bearing and couple a group's layout sequence to its `entityOrder` run.

## Decision

**D1 — An "auto-layout group" is a group with `managedLayout: true`.** Its children's positions are *derived* (managed), not free. `layoutMode` selects the packing — `'row'` is the only live mode in Milestone 1; `'column'`/`'grid'` follow. `'freeform'` groups are unchanged — purely a selection/bbox container. A managed group is the opt-in *persistent crystallization* of a reorderable arrangement — the upgrade that makes the layout durable — not the entry point for reorder. Reorder is reachable directly on any eligible selection (D7); making a group is how you keep that arrangement.

**D2 — Layout sequence = the group's `entityOrder` run.** A managed group's children are already a contiguous run in `entityOrder` (ADR 0014 group contiguity). The order *within that run* is the left-to-right layout sequence. Reordering a child rewrites that run (via `entity-order-math.moveBlockBefore` / `replaceSubsequence`, constrained to the run, then `enforceGroupContiguity`); reflow reads the run, not `canvasX`. **No new per-group `childOrder` field.** Tradeoff: in a managed row, stack-order and layout-order are the same axis — acceptable because z-order is meaningless for non-overlapping row children.

**D3 — Reflow is the single writer of managed children's positions.** Any change to a managed group — membership, child resize, child reorder — triggers `reflowManagedGroup(groupId)`. Managed children never hold authoritative `canvasX/canvasY`; those are outputs. (They are still persisted so other tools / freeform fallback render correctly.) The pure kernel is `computeRowReflow(children, gap, originX, originY)` in `layout-math.ts`; the runtime writer is `reflowManagedGroup` in `src/main/managed-layout.ts`, which superseded the pages-only `reflowGroupRow`.

**D4 — Two distinct drags on a managed child.** The **center dot** is the reorder handle (a new `reorder-handle` hit-test layer → `reordering-child` gesture). The child **body** keeps today's behavior: dragging the body moves the *whole group* as a unit, same as any group drag. You reorder with the dot, you move with the body. This keeps the managed invariant: you can't free-drag one child out of place.

**D5 — Reflow snaps the row origin, then packs by gutter.** The origin (min child x/y) is grid-snapped; children then pack by `CLUSTER_HORIZONTAL_GUTTER` without per-child snapping. Alignment/distribution guides are suppressed during a reorder drag — the op is managed, so confirming-alignment guides would be noise.

**D6 — Gap & alignment are defaulted in Milestone 1.** Milestone 1 uses the existing `CLUSTER_HORIZONTAL_GUTTER` constant and start-alignment. Persisted `layoutGap?: number` / `layoutAlign?` fields land with the toolbar (Milestone 2) — additive, no migration (absent → default), same pattern as `specular.textStyle`.

**D-O2 — `reordering-child` is its own interaction mode**, not a `dragging-entities` payload flag. Commit semantics genuinely differ (reorder+reflow vs free move) and it owns its own drop-index preview and cancel path. The mode carries `{ groupId, childId, dropIndex }`; focus reconciles to `aboveView` like other gestures.

**D7 — Selection reorder: drag-to-reorder any aligned selection, no group required.** Reorder dots and the reorder gesture appear on any *eligible selection* — a multi-selection that already reads as an evenly-spaced row. The model is geometry-is-truth, position-only, and ephemeral: eligibility is computed from current positions, nothing is persisted but the new positions, and no group / `entityOrder` / `managedLayout` is created. Eligibility = an equal-gap row along the dominant axis (FigJam parity): ≥2 boxes, no overlap, gaps equal within ~1px tolerance. The gesture and dot affordance are *shared* with the managed door — one hit-test layer, one dots layer, one `reordering-row` interaction mode — diverging only at commit: a loose selection commits via `reorderSelection` (position-only multi-write), a managed group's child commits via `reorderManagedChild` (rewrites the `entityOrder` run + reflow). Making an auto-layout group out of a reorderable selection (D1) is the opt-in upgrade that persists the arrangement.

**Undo batching.** A reorder is `entityOrder` move + reflow position writes. `writeEntityOrder` transacts directly while position writes reach the doc through the diff-sync — two transactions, hence two undo steps, under the UndoManager's `captureTimeout: 0`. `commitAsOneTransaction` (workspace-observers) wraps both in one Yjs transaction (nested transactions flatten) so a reorder is exactly one undo step. Same helper backs make-auto-layout.

## Reachability (Milestone 1)

The polished "wrap selection → auto-layout" toolbar button is Milestone 2. To make drag-reorder reachable on arbitrary content now, a headless command marks a selection (or an existing group) as a managed row: `makeAutoLayoutFromSelection()` (binding `make-auto-layout`, Shift+Cmd+A), the `specular auto-layout` CLI verb, and the `POST /groups/auto-layout` route. It seeds the layout sequence to the children's current left-to-right order so nothing jumps on conversion.

## Consequences

- `managedLayout` is now load-bearing: it gates reflow, the reorder hit layer, and dot painting.
- Mixed-surface groups (ADR 0014 split rows) can be managed; reflow operates in canvas space on all children regardless of paint surface. The sidebar split-row representation reads `entityOrder`, which reorder mutates — rows stay paired.
- Milestone 2 (deferred): draggable gap handles (`layoutGap`), direction toggle (column/grid via the existing `layout-math` paths), align (`layoutAlign`), distribute, and the floating multi-select toolbar.

## Glossary additions

- **Auto-layout group** — a group with `managedLayout: true`; children are reflowed, not freely positioned. Layout sequence = the group's `entityOrder` run (D2).
- **Reorder handle (center dot)** — the per-child drag target that reorders a child within its auto-layout group. A `hit-test` layer above `body`, below `anchors`.
