# Deepen the runtime — finish the entity-kind registry + one mutation seam

Track 1 of 3 from the deepen pass (`docs/audit/deepen-3673a35.md`, candidates
1–2). Design is settled: **[ADR 0024](../adr/0024-entity-kind-registry-spans-runtime-and-persistence.md)**
(registry spans runtime + persistence) and **[ADR 0025](../adr/0025-single-workspace-mutation-seam.md)**
(single mutation seam) — read both before coding; they record what to build
*and* the rejected alternatives. Self-contained otherwise. Feature branch off
`main`; one PR per step; integration PR at the end.

## Goal

`EntityKindDefinition` becomes the one answer to "what kinds exist and how
does each do X" for persistence, runtime state, iteration, and delete; a
`mutateWorkspace` seam becomes the only door for mutations. End state: adding
a kind = one handler file + one capability-table row + its React surfaces.

## Ground truth (measured at 3673a35)

- `EntityKindDefinition.serialize` and `.defaultSize` are declared but never
  dispatched (dead). Serialization is a 6-branch if/else at
  `json-canvas-serializer.ts:88-100` (serialize) and `:305-329` (deserialize);
  create-time sizing is `footprintForItem` (`shared/entity-ops.ts:46-72`).
- Kind fan-out `[...textEntities, ...fileEntities, ...]` copy-pasted at:
  `canvas-layout-data.ts:346-359`, `sidebar-builder.ts` ×5 (`:122,141,192,212,219`),
  `workspace-entities.ts` ×4 (`findEntityById:116`, `groupChildIds:177`,
  `selectEntitiesInRect:307`, `getWorkspaceGraph:499`),
  `workspace-doc.ts:219,312`, `entity-order-state.ts` ×5 (`:28,88,99,115,139`),
  `workspace-observers.ts:283-292`, `workspace-restore.ts:237-275`.
- `delete-selection.ts:31-64` hand-buckets 7 id arrays with a 7-case switch —
  does not call `getEntityKind(kind).delete`.
- `syncDocToRuntime` (`workspace-observers.ts:198-308`) hand-lists page
  properties (L264-272) and kinds; reached via a 20-field `RuntimeStateRefs`
  bag (L41). A new persisted field silently fails to restore on undo.
- Six `*-entity-state.ts` modules repeat the same CRUD shape (~150 lines each
  of `if (patch.x !== undefined)` field copies + 1:1 persist/scene
  projections). Real divergence: file's `rendererSceneFields`/`inferRepoRoot`,
  shape's `defaultShapeSize`, text's `defaultWidthMode`, drawing's strokes.
- Renderer per-kind switches: `hit-test.ts:430-440` (`entityHasChrome`,
  `entityHasAnchors`), `useCanvasPointerRouter.ts:1013-1068`
  (`resizeConfigForEntity`, `patchDispatcherForKind`).
- Mutation ritual hand-sequenced in `document-commands.ts` (1,218 lines): 30×
  `scheduleWorkspaceAutosave`, 31× `requestLayout`, 12× `markUndoBoundary`;
  gesture bracket copy-pasted in drag (`initializeDrag`/`applyDragDelta`/
  `finalizeDrag`), `reorderSelection` (L1016), `distributeSelection` (L1060);
  `snapToGrid` prelude ×5 (L669,718,747,815,849); page/file device-preset
  commands duplicated (L1074-1208).

## Constraints

- Two-layer state model is load-bearing: Y.Doc = persisted truth, runtime =
  ephemeral; forward sync on mutation, reverse sync on undo. Preserve the
  diff-sync engine (`syncRuntimeToDoc`/`syncMapFromArray`),
  `commitAsOneTransaction`, `withSuppressedDocSync`, and the
  `workspace-undo.ts` side-effect queue — all confirmed deep.
- Edge does NOT register (ADR 0024 §3). Per-kind React components stay
  per-kind (ADR 0024 "false-abstraction line").
- No descriptor framework: shared helpers + explicit per-kind modules
  (ADR 0024 §6).
- Test contract: every step touching `workspace-*.ts` updates smoke coverage;
  new mutators ship forward/reverse sync coverage (one Y.Doc transaction per
  mutation, clean undo round-trip). Re-read `tests/README.md` first.

## Steps (one PR each)

### 1. Reclaim the dead registry members

`serializeToJsonCanvas` + the deserialize loop dispatch through
`getEntityKind(kind).serialize`/`.deserialize` (add `deserialize` to the
contract); `footprintForItem` dispatches through `defaultSize`. The 6-branch
chains in `json-canvas-serializer.ts` and the kind arms in
`workspace-restore.ts` collapse. Pure relocation — same functions, one owner.

**Done, with two constraints discovered in implementation:** (a) `defaultSize`
dispatch is deferred — `sizeForItem` (the function's real name; `footprintForItem`
never existed) is bundled into the non-Electron CLI/MCP builds where
`registerBuiltInEntityKinds()` never runs, so registry dispatch would throw;
importing the builtins there drags Electron-dependent main code into the lean
bundle. `defaultSize` stays dead until footprint computation moves server-side.
(b) The `workspace-restore.ts` kind chain operates on already-deserialized
persisted entities creating *runtime* state — that is step 3/4's
entity→runtime hydration member, not `deserialize`; left for step 4.

### 2. Kind capability table

`src/shared/entity-kind-caps.ts`: exhaustive
`Record<CanvasEntityKind, { hasChrome, hasAnchors, minSize, aspectMode }>`.
`hit-test.ts` predicates and `resizeConfigForEntity` read it. (Edges aren't in
`CanvasEntityKind`-driven hit paths; unchanged.)

### 3. Registry-driven iteration

The contract gains entity access (methods, not raw arrays — page wraps its
WCV-backed store): per-kind `entities()`, plus registry-level `allEntities()`
/ `forEachKind()`. Collapse the fan-out sites listed in ground truth onto it.
Per-kind scene/persist projections become registry members
(`buildSceneEntity`, `persist`) so `canvas-layout-data` and `workspace-doc`
loop kinds.

### 4. One field list, both directions

Each kind's definition declares its persisted fields once; `persist` and the
reverse-sync rebuild derive from it. `syncDocToRuntime` becomes "for each
registered kind, reconcile its map" — the page-prop allow-list and the
4-kind rebuild branch disappear; `RuntimeStateRefs` shrinks accordingly. Add
the fake-kind round-trip unit test (register a test kind → persist → undo →
assert restore) — this is the step that kills the silent-undo-loss class.

### 5. Unify delete + CRUD helpers

`delete-selection.ts` dispatches through `getEntityKind(kind).delete`
(edges keep their explicit path). Introduce `applyPatch(entity, patch,
fields)` + default projection helpers; the six `*-entity-state.ts` modules
shrink to their genuine divergence.

### 6. Generic interactive update channel

`canvas-update-entity { kind, id, patch }` dispatching through the registry's
`update`, replacing the per-kind channel + preload method pairs (typed by a
kind→patch map). Coordinate with the IPC track if its channel contract
(`docs/plans/deepen-ipc.md` step 1) has landed — if so, add the channel to
the contract; if not, plain literals are fine, the IPC track migrates them.

### 7. `mutateWorkspace` — the only door (ADR 0025)

`mutateWorkspace(fn, opts)` owns dirty → autosave → layout → undo-boundary;
default one-call-one-undo-step. `document-commands.ts` exports only wrapped
commands; raw mutators go runtime-internal. Registry `create`/`update`/
`delete` dispatch through it (headless converges). Fold the `snapToGrid`
prelude into one `snapGeometryPatch` helper and the page/file device-preset
duplicates into one parametrised set.

### 8. Gesture session

`begin`/`applyDelta`/`finalize` object bracketing many `mutateWorkspace`
calls into one undo step; drag, reorder, and distribute replace their three
hand-rolled `beginBatch`…`endBatch`…`markUndoBoundary` copies. Headless bulk
edits use the same session.

## Verification

Every PR: `pnpm typecheck` + `pnpm test:unit` + `pnpm test:smoke` (this whole
track is runtime/persistence surface). Manual per step: create/edit/delete
each kind interactively and via `specular` CLI, drag (one undo step), undo
round-trips after each change, `.canvas` file diffs stay clean (open a saved
canvas in git diff — node shapes unchanged).
