# Connect tool — build plan

Orchestration handoff for [connect-tool.md](./connect-tool.md). That doc is the
design and the source of truth for every decision; this one says only how the
work is cut into branches and what each one has to clear.

## Branch shape

One long-lived feature branch, one PR per step into it, one integration PR at
the end for a human to review.

```
main
 └── feat/connect-tool                 ← feature branch, never merged mid-flight
      ├── feat/connect-tool-geometry   ← step 1 PR
      ├── feat/connect-tool-free-ends  ← step 2 PR
      └── feat/connect-tool-mode       ← step 3 PR
```

`refs/heads/docs` exists as a branch, so git cannot create anything under
`docs/`. Use `docs-*` if a docs-only branch is needed. `feat/*` is clear.

Step PRs target `feat/connect-tool`, not `main`. The integration PR targets
`main` and is the only one a human reviews end to end.

## Gate for every step

- `pnpm typecheck` and `pnpm test:unit` pass.
- `pnpm test:integration` for any step touching runtime, IPC, or persistence —
  steps 2 and 3.
- No manual smoke per step. One smoke of the whole feature branch before the
  integration PR.
- Each step merges into `feat/connect-tool` before the next begins. Steps are
  dependency-ordered and cannot run in parallel.

## Step 1 — geometry and routing

Branch `feat/connect-tool-geometry`. Pure `src/shared/`, no persistence beyond
the type fields, no UI, no new tool.

- `types.ts` + `json-canvas-types.ts` — `routing`, `elbowSplit`,
  `elbowSplitAxis`
- `edge-geometry.ts` — elbow and straight builders; mid-split and the
  4-segment stored-offset case; rounded corners clamped to half the shorter
  adjacent segment
- `edge-geometry.ts` — `autoSides` becomes per-endpoint resolution **against a
  point**, entity cases passing their center. This is the seam the next two
  steps both build on, so it lands here even though nothing yet has a free end.
- `document-commands.ts:656` — the three fields into the patch key list
- `workspace-edges.ts:28` and `:45` — the three fields into **both** hand-copy
  branches
- `json-canvas-serializer.ts` — the three fields round-trip

Tests: unit for the 3-segment, 2-segment L, and 5-segment S paths; corner-radius
clamping; a stored split holding its axis when the far end resolves
perpendicular, producing the 4-segment route; a degenerate 2-segment route
ignoring the split without clearing it; one pinned end coexisting with one auto
end. Integration for the three fields persisting and undo round-tripping.

Done when an edge written directly into a `.canvas` file with
`routing: 'elbow'` renders as an elbow and survives a reload.

## Step 2 — free endpoints

Branch `feat/connect-tool-free-ends`. Depends on step 1's point-based side
resolution.

- `types.ts` + `json-canvas-types.ts` — nullable `fromEntityId` / `toEntityId`,
  plus `fromPoint` / `toPoint`
- One runtime collection: free edges live in `workspaceEdges` and the same
  Y.Doc map. The `edges[]` / `specular.freeEdges` split happens only in the
  serializer, on write, and merges back on read.
- `workspace-edges.ts:85` — the delete cascade stops removing an edge whose
  entity is gone if the other end is free; it detaches instead
- `EdgeLayer.tsx:292` — render an edge with a free end rather than dropping it
- `edge-drag-controller.ts` — a free end is a legal starting state for the
  `edit` branch, so the existing re-route gesture binds it
- **ADR** for `specular.freeEdges` living outside the spec's `edges[]`, with the
  `annotations` / `specular.entityOrder` precedent

Tests: integration for a free-ended edge round-tripping through
`specular.freeEdges`, surviving deletion of the entity at its bound end, and
moving back into `edges[]` when the free end is re-bound.

Done when a free-ended edge hand-written into a `.canvas` file loads, renders,
and a strict JSON Canvas reader still sees a valid file.

## Step 3 — the tool

Branch `feat/connect-tool-mode`. Depends on both prior steps.

- `tool.ts` — union arm, `toolDuration: 'persistent'`, `toolGerund`,
  `toolHasPopup`
- `bindings.ts` + `binding-handlers.ts` — `tool-connect` on `X`
- `tool-defaults.ts` (shared + main) + `register-toolbar-ipc.ts:65` — `connect`
  scope holding `routing`, `color`, `strokeWidth`, `toEnd`; **both** creation
  doors read it
- `canvas-pointer-owner.ts` — connect takes `'tool-gesture'`
- `canvas-pointer-actions.ts` — body hit yields `begin-edge-drag`; background
  hit yields a free-start drag; a click that never drags is a no-op
- `edge-drag-controller.ts` — `fromSide: EdgeSide | null` in the create state,
  cursor-facing rubber-band origin, body release commits `side: undefined`,
  release on the source commits `noop`
- `interaction-types.ts`, `types.ts`, `interaction-controller.ts`,
  `interaction-state.ts` — `routing-edge` mode, mirroring `resizing-gap`
- `EdgeLayer.tsx` — endpoint handles and segment handles on a selected edge
- `EdgePopup.tsx`, new `ConnectToolPopup.tsx`, `canvasItemPopupTable.ts:80`
- `CustomIcons.tsx`, `icons/toolbar/{,dark/}connect.svg`, `toolbarSections.tsx`

Tests: unit for body-release committing `side: undefined` rather than `noop`, a
null-side create drag resolving its origin from the cursor, and a self-release
committing `noop`. Integration for the `routing-edge` mutator — one Y.Doc
transaction per commit, undo round-trips cleanly.

Done when `X` enters the tool, dragging between two entities makes an elbow
edge, dragging from empty space makes a free-ended one, and a selected edge's
crossbar can be dragged.

## Integration PR

Branch `feat/connect-tool` into `main`. Before opening it:

- one manual smoke of the whole feature — draw with the tool, drag a crossbar,
  move entities and watch auto ends rederive, reload the space
- CONTEXT.md **Edge** entry — the connect tool, the auto-attach rule, the
  per-endpoint auto/pinned distinction, `routing` / `elbowSplit`
- `docs/file-formats.md` §Edges — the new `specular.*` fields and
  `specular.freeEdges`
- changelog — the anchor drag now produces elbow edges

## Blocked on a human

The connect glyph (`connect.svg`, light and dark) is design work. Step 3 can
ship the toolbar button against a placeholder; the real icon lands separately.
Nothing else in the build waits on anyone.

## Non-goals

Self-edges, edge-to-edge attachment, drop-in-empty offering to create a shape,
obstacle-avoiding routing, and S-route adjustability are all out. See
[Deferred](./connect-tool.md#deferred). Do not widen scope to reach them.
