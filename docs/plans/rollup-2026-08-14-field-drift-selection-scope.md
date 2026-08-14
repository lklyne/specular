# Rollup — field drift + selection scope (2026-08-14)

Session summary for picking up remaining bugs in a fresh context.

## Branches (neither pushed, no PRs yet)

- `worktree/field-drift-net` — PR 1, code-complete, awaiting manual smoke
- `worktree/selection-scope` — stacked on it, code-complete, awaiting the same smoke

## What landed

**PR 1 — entity field drift** (plan: [entity-field-drift.md](./entity-field-drift.md))
- `8ab30f5b` Step A: restore / tab-switch / snapshot rebuild entities through one
  registry intake door (`space-restore.ts`, `space-tabs.ts`; group gained registry
  `persist()`/`restore()`)
- `8de31c14` Step B: copy/paste/duplicate = persist → re-id → offset → restore
  (`entity-clone.ts` new; clipboard payload is now the persisted record + delta)
- `acb44313` Step C: registry `update` forwards whole patch; patchable set derived
  from each kind's declared field list (`patchableFields` in `apply-patch.ts`).
  Dead-on-update fields fixed: text/shape/drawing `label`/`parentGroupId`/`pageAnchor`/
  `textStyle`, file `parentGroupId`
- Net: `tests/integration/entity-field-roundtrip.test.ts` + fixtures now cover
  Y.Doc round trip, save/reload, tab switch, copy/paste, duplicate, update

**Selection scope** (ADR: [0034-one-selection-scope-resolver.md](../adr/0034-one-selection-scope-resolver.md), CONTEXT.md "Selection scope")
- `46a386a8` `resolveSelectionScope(anchorId?)` → `{ memberIds, operandIds, bounds }`
  (`src/main/runtime/selection-scope.ts`); drag, multi-resize, bbox overlay consume it.
  Fixes: drag-by-grouped-member moves whole mixed selection; bbox renders with groups
  (old `!hasSelectedGroup` guard deleted); resize scales group descendants.
  Deleted: `selectedDragEntityIds`, `expandDraggedGroupIds`, `resolveDraggedSelection`
- `6db3f17c` copy/paste carries groups (persist → re-id → restore + old→new id remap of
  `parentGroupId`, nested groups included). `duplicateGroup` left on its own path
- Net: `tests/integration/selection-scope.test.ts`, `clipboard-groups.test.ts`

## Verified decisions (don't re-research)

- ADR 0004's `specular: {}` namespacing confirmed against ecosystem (glTF/OCIF/
  Excalidraw/tldraw all use one owner-namespaced bag). Survey + caveats folded into
  [json-canvas-spec-compliance.md](./json-canvas-spec-compliance.md)
- Nothing outside the repo reads `.canvas` files — no external release-note concern
- OCIF = separate interchange spec (canvasprotocol.org), track as export target, don't switch

## Open / next

- **Manual smoke before PRs** (run dev app on `worktree/selection-scope`): styled-shape
  copy/paste identical; mixed selection drag-by-grouped-member + bbox + resize;
  mixed copy/paste keeps group; group-duplicate excludes straddling non-member;
  quit/reopen + tab switch keeps everything; undo sane
- **PR 2 — spec compliance** (plan: [json-canvas-spec-compliance.md](./json-canvas-spec-compliance.md)):
  fresh branch after PR 1 merges (same serializer). Includes doc-level `specular`
  consolidation (`appState`, `annotations`, `entityOrder`), `groupId`→`parentGroupId`
  collapse, ADR 0004 amendment, skill-file + file-formats.md updates
- **[#386](https://github.com/lklyne/specular/issues/386)** — delete doesn't consume the
  scope: page-anchored items must survive page delete (needs resolver variant for
  destructive verbs); pre-existing: group delete only cascades page children
- Deferred from Step A: page-snapshot shape duplication across `space-persistence.ts`/
  `space-tabs.ts` (two parallel page types); `pageIds`/`entityIds` on groups —
  derived vs persisted still undecided (also flagged in PR 2 plan)
- Flagged in Step C: `page`'s registry update is still a hand-subset
  (`name`/`parentGroupId`/`metadata` not patchable)
