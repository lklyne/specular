# Plan — wireframe.json as a structured editor

> **Status — proposed (2026-06-03).** Not started. This closes the gap between
> loose canvas shapes (freeform, directly manipulable, but hand-placed and
> agent-hostile) and `.wireframe.json` (semantic, auto-laid-out, agent- and
> variant-friendly, but currently edited as a single opaque file entity).
>
> Thesis: **converge on the wireframe tree as the model and add shape-like
> direct manipulation on top of it** — the Figma/Framer auto-layout model.
> We do *not* give shapes nested auto-layout, and we do *not* bring freeform
> xy into wireframe. Every interactive edit is a named tree mutation, so the
> same op set drives both humans (canvas gestures) and agents (CLI).

## Target workflow

Produce a low-fi reconstruction of an existing screen, then iterate: select an
element, drag to reorder, add/duplicate a section, relabel text, fan out 5–10
layout variants. Easy for both humans and agents.

## 1. Current state (grounded)

| Capability | Today | File |
|---|---|---|
| Render tree (frame/text/button/input/dropdown/checkbox/toggle/image/divider/spacer) | ✅ | `wireframe/WireframeNodeRenderer.tsx` |
| Inline text edit → persist | ✅ | `WireframeRenderer.tsx` (`updateNodeText`) |
| Drag-to-reorder children within/among frames | ✅ | `WireframeNodeRenderer.tsx` (`reorderNode`, `data-wf-child`) |
| Toggle checkbox/toggle state | ✅ | `toggleNodeState` |
| Raw JSON edit + theme/device chrome | ✅ | `file-popup-contributions/*` |
| Layers panel (node tree, icons, ids) + insert palette **stub** | ✅ (stub) | `right-details-panel/components/WireframeFilePane.tsx` |
| Duplicate whole wireframe entity | ✅ | `rightDetailsPanelApi.duplicateFileEntity` |
| **First-class node selection on canvas** | ❌ | — |
| **Insert / delete / duplicate a node** | ❌ | — |
| **Per-node property editing (direction/gap/sizing/variant/level)** | ❌ | — |

### Persistence & undo (the load-bearing fact)

Wireframe **content** lives in the `.wireframe.json` file on disk. The renderer
fetches it over HTTP (`WireframeInlineRenderer.fetchContent`), edits mutate
React state, and writes go back via `fileApi.writeNoteFile` with a 300ms
debounce. External writes are picked up via the `wireframe-file-changed` event
and on `visibilitychange`.

Consequence *today*: the **Y.Doc owns the file entity** (reference, position,
size), **not the wireframe content** — so content edits don't participate in
undo/redo. **This plan changes that** (§2.1 decision 1, §3.0b): content moves
into the Y.Doc, the file becomes a projection, and the forward/reverse-sync test
clause then *does* apply to every edit op.

### Tree ops are already pure

`reorderNode`, `updateNodeText`, `toggleNodeState`, `findNodeById` are pure
`(file, …) → WireframeFile` functions with internal helpers `findNodeParent`,
`removeNodeFromTree`, `insertNodeInTree`. New ops slot in trivially and are
fully unit-testable with no Electron.

## 2. Key decisions

1. **Undo model — wireframe content becomes Y.Doc-resident; `.wireframe.json`
   becomes a projection.** This is the inversion of the original draft. The
   app already resolves the file-vs-truth tension everywhere else: the Y.Doc is
   truth and disk files are projected from it (`.canvas` at 350ms). Wireframe
   content was the lone persisted thing living *only* in its file. Putting it in
   the Y.Doc gives undo/redo for free from the existing `UndoManager` and
   projects back to `.wireframe.json` (still diffable, still agent-editable —
   see decision 3). See slice §3.0b.
   - **Granularity sub-fork.** *(A1, recommended first)* store the tree as a
     JSON string under a per-file Y type, applying **one transaction per op** so
     each logical edit is exactly one undo step. Fast, low-risk, reuses the JSON
     the file already holds; no CRDT merge of concurrent structural edits (fine
     for single-author wireframes). *(A2, optional deepening)* model the tree as
     structured Y types (`Y.Map` node, `Y.Array` children) for native
     structured undo and real merge — matches how entities/groups/edges already
     live in the Y.Doc, at the cost of a `WireframeNode ↔ Y` mapping layer.
     Recommend shipping A1, leaving A2 as a later deepening if collaborative
     structural editing is ever needed.
2. **Lift pure ops + schema to `src/shared/wireframe/`.** Today they live under
   `src/renderer/`. The CLI/main process must reuse the *same* ops to honor the
   "one op set for humans and agents" thesis, and the layer rules forbid
   `src/renderer/` ← `src/main/`. Shared has no side effects — a clean home.
3. **Agent edit path = CLI verbs through a route that mutate the Y.Doc.** A
   `specular wireframe` verb POSTs to a route that applies the shared op as a
   single Y.Doc transaction (forward sync) → broadcast → disk projection. Edits
   are therefore undoable. Raw on-disk JSON edits keep working as a fallback:
   the watcher (§3.5) *imports* them into the Y.Doc as a transaction, so they're
   undoable and re-projected too.
4. **Schema validation in shared.** One `validateWireframe(file)` used by both
   the renderer JSON mode (already has `jsonError` state) and the CLI, so bad
   trees are rejected with the same helpful message everywhere.

## 3. Slices

Each slice ships independently and green. Test bucket per slice follows
`tests/README.md`: **unit** for pure logic, **smoke** for the running app /
persistence / IPC, **agent** for gesture scenarios.

### 3.0 — Enabling refactor: lift + extend the op module

- Move schema types (`wireframe-types.ts`) and pure tree ops to
  `src/shared/wireframe/`. Renderer imports from shared (behavior unchanged).
- Add ops: `insertNode(file, parentId, index, node)`, `deleteNode(file, id)`,
  `duplicateNode(file, id)` (deep-clone with fresh ids, insert after original),
  `updateNodeProps(file, id, patch)`, `validateWireframe(content) → {ok|errors}`.
- New-id generation must be deterministic-by-input for testability (counter or
  seeded suffix passed in — no `Date.now()`/`Math.random()` in the pure layer).

**Tests (unit).** For each op, a focused case naming the regression:
- `insertNode` places at the right index; out-of-range index is clamped/rejected.
- `deleteNode` removes the subtree and is a no-op for unknown id.
- `duplicateNode` produces a structurally-equal subtree with **all-fresh ids**
  (assert no id collision with the source tree) inserted directly after source.
- `updateNodeProps` patches only the named node; rejects props illegal for the
  node type.
- `validateWireframe` rejects: missing `version`, non-frame root, unknown node
  `type`, child on a non-frame. Round-trips a valid file unchanged.
- Refactor guard: existing `reorderNode`/`updateNodeText` unit tests move with
  the module and still pass (proves the lift preserved behavior).

### 3.0b — Wireframe content under the Y.Doc (undo/redo foundation)

Make wireframe content a projection of Y.Doc state, so every edit op runs as a
Y.Doc transaction and inherits undo/redo from the existing `UndoManager`. Do
this **before** any write-path slice (§3.2+) so those slices target the Y.Doc
apply path from day one and the write plumbing is built once.

**Kept deliberately small — this is the *only* hard dependency for the write
slices, so it is the slice most able to halt an unattended loop (§7). Scope it
to the apply path + projection + severing renderer writes; the renderer's
read-path repoint is split out to the non-blocking cleanup §3.5b.**

- Add a Y.Doc structure for wireframe content keyed by file-entity id
  (granularity per §2.1 — ship **A1**: JSON-string Y type, one transaction per
  op). Seed it from the file on first load; GC it when the entity is deleted.
- **Sever the renderer's _write_ path:** edits route up as Y.Doc ops (forward
  sync), not `writeNoteFile`. The renderer keeps **reading** the projected
  `.wireframe.json` for now (its existing event/visibility re-fetch already
  works), so this slice stays small and additive. The full broadcast-derive
  read path is the separate, non-blocking §3.5b.
- Persistence: project the Y.Doc content back to `.wireframe.json` on the same
  debounce path `.canvas` uses. Tag the write origin + hash so the §3.5 watcher
  ignores self-writes.
- Apply path: `runtime op → one Y.Doc transaction → reverse sync → projection
  to disk → renderer re-fetch`. The pure ops from §3.0 are the transaction
  bodies, unchanged.

**Tests.**
- **smoke (the contract clause)**: apply each op (insert/delete/duplicate/
  reorder/setProps) through the runtime; assert it is **one Y.Doc transaction**,
  that **undo restores the prior tree and redo reapplies** (round-trips
  cleanly), and that the **disk projection** matches after the debounce.
- **smoke**: round-trip persistence — create wireframe entity, edit, reload
  workspace from disk, content matches (new-entity-kind-style coverage for the
  Y.Doc-backed content).
- **unit**: the file↔Y seed/extract codec (parse on seed, serialize on project)
  is a pure round-trip: `extract(seed(json)) === json` for valid trees.

### 3.1 — Canvas node selection

- Add `selectedWireframeNodeId` to the renderer; single-click selects a node
  (draw a selection outline), double-click / Enter edits text (today single
  click conflates select+edit). Esc / background click clears.
- Selecting a canvas node mirrors into the layers panel highlight and vice
  versa (the panel already renders the tree; selection-state pattern exists for
  the inspect tree, `selectedNodeId`/`hoveredNodeId`).

**Tests.**
- **unit**: a small selection reducer/controller — click maps to the correct
  node id via hit data; double-click promotes to edit; Esc clears. (Mirrors the
  existing `canvas-pointer-*` unit tests.)
- **agent** (out-of-band): click a node → outline appears; double-click → input
  focused. Not in CI.

### 3.2 — Insert / delete / duplicate on canvas

- Wire the existing insert-palette stub in `WireframeFilePane` to `insertNode`.
- Per-selected-node actions: delete, **duplicate** (the headline — "another card
  like this", "another section"), via panel buttons and/or canvas affordance.
- All route through the Y.Doc apply path from §3.0b (one transaction per op),
  not a direct file write.

**Tests.**
- **unit**: covered by 3.0 (the ops). Add a thin controller test that the
  panel action dispatches the right op with the selected node id.
- **smoke**: create a `.wireframe.json` file entity via `AppClient`; apply an
  insert/duplicate/delete through the edit surface; assert one Y.Doc
  transaction, that **undo reverts** and **redo reapplies**, and that the disk
  projection reflects the new tree after the debounce. (Reuses the §3.0b
  harness; catches a broken op→transaction wiring.)

### 3.3 — Per-node property editing

- Panel editors for the selected node: frame `direction`/`gap`/`padding`,
  `width`/`height` as `fill`/`hug`/px, button `variant`, text `level`,
  input/dropdown `label`/`options`. Each calls `updateNodeProps`.

**Tests.**
- **unit**: `updateNodeProps` per-type matrix (3.0) + controller mapping editor
  change → op.
- **smoke**: change `direction` of a frame; assert disk reflects it and the
  re-rendered layout snapshot field changes (observable, not internal).

### 3.4 — Agent CLI parity

- `specular wireframe <fileId|path> insert|delete|duplicate|reorder|set <node> …`
  → POST to a new route that validates, then applies the **shared** op (§3.0)
  as a single Y.Doc transaction (the §3.0b apply path). Broadcast + disk
  projection follow; the edit is undoable from the canvas like any other.
- Document the verb set + node schema in the specular SKILL.md and
  `references/wireframes.md` (the agent-facing recipe).

**Tests.**
- **smoke**: drive each verb via the HTTP route the CLI uses; assert disk
  content + workspace broadcast. Invalid node id / illegal prop → 4xx with a
  legible error (validates the shared validator is wired).
- Round-trip: create → duplicate via CLI → re-fetch tree equals expected.

### 3.5 — Import external edits into the Y.Doc

- Add a file watcher on `.wireframe.json` assets. A genuine external edit (agent
  `Write`, git checkout) is parsed, validated, and **imported as a Y.Doc
  transaction** (origin-tagged `external`) — so it lands in the projection,
  shows live, and is undoable. Self-writes from §3.0b's projection step are
  ignored via the content hash to break the write→watch→write loop.

**Tests.**
- **smoke**: write the file out-of-band; assert the import produces one Y.Doc
  transaction, the broadcast reflects it (no visibility event needed), and a
  subsequent undo reverts to the pre-import tree.
- **smoke**: project-then-watch does **not** re-import (self-write is suppressed
  by hash) — guards against an infinite write loop.

### 3.5b — Renderer derives content from the broadcast (cleanup, non-blocking)

Finishes the §3.0b repoint: replace `WireframeInlineRenderer`'s HTTP fetch +
`writeNoteFile` debounce with deriving content straight from the IPC broadcast
(aligns with "renderer state is derived from broadcasts, never authoritative").
Purely a simplification — by here the Y.Doc is already truth and the file is a
projection, so this changes no behavior, only removes the file round-trip from
the read path. Deliberately last so it never sits on the critical path.

**Tests.**
- **smoke**: after the repoint, a Y.Doc edit re-renders the canvas without any
  file re-fetch (assert the broadcast drives the update); existing §3.2 round
  trips still pass.

### 3.6 — (Optional deepening) Structured Y types (granularity A2)

If collaborative structural editing or finer merge is ever needed, migrate the
A1 JSON-string representation to structured Y types (§2.1 A2). Not required for
undo/redo — A1 already delivers it. Flagged so the A1 choice is a conscious,
reversible step, not a ceiling.

## 4. Sequencing

**This is the dex-subtask order for an AFK build — create subtasks in exactly
this sequence** (the loop builds in order and halts on first failure, so the
ordering is blast-radius control, §7):

1. `3.0` — pure ops. Unit-only, lowest risk. Banks a safe first win.
2. `3.1` — canvas selection. Ephemeral runtime; depends only on `3.0`, *not* on
   `3.0b`. Front-loaded so two PRs land before the risky slice.
3. `3.0b` — Y.Doc apply path + undo. The one hard dependency for all writes;
   kept small (§3.0b). The slice most likely to halt the loop.
4. `3.2` — insert / delete / duplicate (the core iteration loop).
5. `3.3` — per-node property editing.
6. `3.4` — agent CLI parity.
7. `3.5` — external-edit import.
8. `3.5b` — renderer broadcast-derive cleanup. Non-blocking; deliberately last.

`3.6` (A2) is **excluded** from the AFK build — only if collaborative
structural editing is ever needed. Variant fan-out needs no new code beyond
`3.2`: `duplicateFileEntity` → mutate via CLI/canvas → lay out in a row.

## 5. Out of scope

- Freeform xy positioning of nodes (incompatible with auto-layout; not wanted).
- Nested auto-layout for loose shapes (reinvents the tree across N entities).
- New node types beyond the existing ten (revisit only if reconstruction
  fidelity demands it).
- Structured Y types / CRDT merge of concurrent structural edits (§3.6 — A1
  ships undo without it).

## 6. Definition of done (epic)

A human can select, reorder, duplicate, delete, relabel, and re-style elements
of a wireframe directly on the canvas; an agent can perform the identical
operations via `specular wireframe …`; both edit one `.wireframe.json` file;
and the workflow of fanning a reconstruction into 5–10 independently-arrangeable
variants is a documented, tested path. Every pure op has unit coverage; every
persistence/broadcast path has smoke coverage.

## 7. Running this unattended (AFK)

`scripts/afk-loop.sh` builds one PR per slice in dex order and **self-halts** —
it never spins or burns tokens indefinitely. It `exit 1`s and waits for a human
when a step PR fails CI, when a worker fire opens no PR (task blocked/failed), or
at `MAX_ROUNDS` (default 40). It only merges on green CI.

Implications for stepping away:
- **`3.0b` is the slice most likely to halt the loop.** It's front-loaded behind
  two safe wins (`3.0`, `3.1`) and scoped small on purpose, so a halt there
  strands the least and leaves a small, reviewable PR.
- **Don't soften the gate.** Keep `AFK_SOFT_CHECKS` at its default (`fallow`) —
  do **not** add `typecheck`/`test` to it, or a broken `3.0b` could merge and
  poison every downstream slice.
- **When you're back:** the open step PR is where it stopped. Check `3.0b`'s PR
  first; fix it and re-run `afk-loop.sh` — the loop resumes from dex state.
