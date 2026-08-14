# ADR 0034 — One selection-scope resolver for every gesture and verb

**Status:** Accepted
**Date:** 2026-08-14
**Related:** [ADR 0024 — Entity-kind registry spans runtime and persistence](./0024-entity-kind-registry-spans-runtime-and-persistence.md) (same disease, different organ), [ADR 0025 — Single workspace mutation seam](./0025-single-workspace-mutation-seam.md), [ADR 0031 — Page-anchored entities](./0031-page-anchored-entities.md), CONTEXT.md "Group-aware marquee selection".
**Origin:** User-reported bug — a mixed selection (ungrouped items + a group) does not move as a unit when dragged by a grouped member, and the selection bounding box disappears when groups are involved.

## Context

Selection *construction* already has a single shared resolver: the marquee's
live outline and its committed selection both go through
`resolveMarqueeSelectionIds`, and `normalizeEntitySelection` stores groups as
first-class members of a multi-selection.

Selection *consumption* has no such seam. Every consumer answers "what does
this selection mean for my operation" with its own hand-written partial
resolution:

- **Drag** — `selectedDragEntityIds` (selection-controller) has three ad-hoc
  branches. Pressing on a member of a *selected group* inside a
  multi-selection falls through all three (the child's id is not in
  `selectedIds`; `selectedGroupId` only returns a value for single-entity
  group selections) and resolves to `[entityId]` — one item drags out of the
  selection. The drag IPC layer then layers its own group expansion
  (`expandDraggedGroupIds`) and page-anchor attachment on top.
- **Copy/paste** — `copyableEntityPayload` filters to pages + the four
  map-backed kinds; groups in the selection are silently dropped, descendants
  are never expanded.
- **Resize** — the multi-resize path computes its own member set and bounds.
- **Bounding box** — the selection overlay does its own arithmetic over its
  own idea of the members, and shows nothing for group-involving selections.

Four consumers, four disagreeing interpretations of one concept. This is
structurally the field-drift bug class (ADR 0024 §5) in the selection layer:
parallel hand-written resolutions that fall out of sync, where the fix is one
declaration every path derives from — not patching whichever consumer a bug
report happens to name.

## Decision

One resolver module, `resolveSelectionScope`, owns the meaning of a
selection. Every gesture and verb derives from its result instead of holding
its own opinion.

```
resolveSelectionScope(anchorId?) → {
  memberIds:  string[]   // top-level selected items; groups as members, unexpanded
  operandIds: string[]   // flat gesture set: groups → descendants, page-anchored items attached
  bounds:     Rect | null // union over operands' bounds
}
```

1. **`memberIds`** is what membership operations act on: reparent-on-drop,
   group/ungroup, arrange slots. A group is one member.
2. **`operandIds`** is what gestures move/clone/delete: groups expanded to all
   descendants (via `descendantEntityIdsForGroup`), page-anchored entities
   attached (ADR 0031, `withPageAnchoredEntityIds`). Expansion lives here and
   nowhere else.
3. **Anchor resolution**: when `anchorId` is given (the item under the
   pointer), pressing any *operand* of the selection — including a descendant
   of a selected group — resolves to the whole selection. Pressing an item
   outside the selection resolves to that item alone (existing
   click-replaces-selection semantics are unchanged; this only governs what a
   drag that *preserves* selection operates on).
4. **`bounds`** is computed once here and broadcast; the renderer overlay
   draws it rather than re-deriving membership.

Consumers converted to derive from the resolver — deleting their bespoke
resolutions rather than wrapping them:

- Drag IPC (`resolveDraggedSelection` + `expandDraggedGroupIds` → resolver);
  `selectedDragEntityIds`'s branches collapse into the anchor rule.
- Resize begin/multi-resize member set and bounds.
- `copyableEntityPayload` (clipboard copy and duplicate-selection) — takes
  `operandIds`, and additionally learns to carry groups (below).
- Delete-selection and arrange, where they re-derive membership today.
- The selection bounding-box overlay (consumes the broadcast bounds).

**Groups become copyable.** The clipboard's historical blocker was having no
way to serialize a group; the registry now has `persist()`/`restore()` for
group (ADR 0024, field-drift Step A). A group in the payload is its persisted
record plus its descendants' records; paste re-ids all of them and remaps
`parentGroupId` through the id map — the same persist → re-id → restore shape
copy/paste already uses for map-backed kinds. A selection therefore behaves
identically for drag, resize, copy/paste, and duplicate: the operand set is
the same set.

## Alternatives considered

**A. Patch `selectedDragEntityIds` to handle the missing branch.** Fixes the
reported drag symptom, leaves copy/resize/bbox each with their own
still-disagreeing resolution — the next bug is already latent. Rejected; this
is the "fix today's field" answer ADR 0024 rejected for persistence.

**B. Expand groups at selection time** (store descendants in the selection).
Destroys the group-as-member semantics that reparent, ungroup, and the
sidebar rely on, and bloats every selection broadcast. Construction and
consumption need different shapes; conflating them is how the current
divergence started. Rejected.

**C. Resolver as methods on the selection state object.** Same logic, but
selection-controller.ts is already the file with the most opinions in it; a
separate module keeps the seam visible and unit-testable. Chosen shape: own
module (`selection-scope.ts`), controller and gesture IPC both import it.

## Consequences

- A mixed selection (grouped + ungrouped) drags, resizes, copies, duplicates,
  and deletes as one unit, grabbed by any of its operands; the bounding box
  always renders.
- "What a selection means" cannot drift between gestures — there is one
  resolution to be wrong, and one test surface: resolve a mixed selection,
  run each verb, assert every operand was affected.
- Copy/paste round-trips groups, closing a capability gap that was previously
  invisible behind the resolver drift.
- The resolver is a new dependency for hot paths (drag start, overlay
  broadcast). It is pure derivation over runtime arrays — no Y.Doc access —
  and runs at gesture boundaries, not per move tick.

**Out of scope:** marquee construction (`resolveMarqueeSelectionIds` already
shared, unchanged); selection *storage* shape in ui-state; edge selection
semantics; page-anchor re-targeting on paste (placement logic, ADR 0031).
