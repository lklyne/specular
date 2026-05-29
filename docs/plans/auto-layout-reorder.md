# Auto-layout & drag-reorder

Build plan for the "auto-layout group + drag-to-reorder" feature: a selected
auto-layout group shows a small **center dot** on each child; dragging a dot
inserts that child at a new position and the siblings reflow to make room
(Figma auto-layout reorder). The dot is small at rest and grows when the item
or group is hovered.

**Scope decisions (locked with the user):**

1. **Group-backed.** Auto-layout is a property of a *group*, not an ephemeral
   selection op. Order and (later) spacing/direction persist in the `.canvas`
   file. This extends the existing `WorkspaceGroupLayoutMode` /
   `managedLayout` fields rather than inventing a parallel concept.
2. **Insert / reorder semantics.** Dragging a dot toward a gap inserts the
   child there and shifts the others. Not pure swap. The child sequence is an
   ordered list, reflowed left-to-right (row) on every change.
3. **PR 1 = the core interaction only.** This doc lays out every phase, but the
   first milestone ships drag-reorder + live reflow. The spacing handles,
   direction toggle, align, distribute, and the floating multi-select toolbar
   from the mockup are **Milestone 2 (deferred)**.

> **Promote to ADR before Milestone 1 merges.** The canonical-decisions section
> below is hard-to-reverse (it makes `managedLayout` load-bearing and couples
> layout sequence to `entityOrder`). It should become **ADR 0015 — Auto-layout
> groups** and be linked from `CONTEXT.md`. See "Canonical decisions" §.

---

## Status (2026-05-29, branch `feat/auto-layout`)

**Milestone 1 is implemented and tested** (typecheck clean; 626 unit + 5 new
managed-layout smoke tests pass; the only failing smoke is the pre-existing
environmental `pages > takes a screenshot` capture test). Not yet committed.
ADR 0015 written and linked from `CONTEXT.md`.

| Phase | State | Landed as |
|---|---|---|
| Phase 1 — reflow engine | ✅ done | `computeRowReflow` (`layout-math.ts`); `reflowManagedGroup` / `reflowManagedGroupForChild` (`src/main/managed-layout.ts`); `reflowGroupRow` deleted, `addPageFromSource` re-pointed; resize-commit reflow hook in `register-canvas-drag-ipc.ts`. |
| Phase 2 — reorder model | ✅ done | `managedChildOrder` / `writeManagedChildOrder` (`entity-order-state.ts`); `reorderManagedChild` + `computeReorderDropIndex` (`managed-layout.ts`); `commitAsOneTransaction` (`workspace-observers.ts`) → one undo step. |
| O1 — reachability | ✅ done | `makeAutoLayoutGroup` / `makeAutoLayoutFromSelection`; `POST /groups/auto-layout`; `specular auto-layout` CLI verb; `make-auto-layout` binding (**Shift+Cmd+A**). |
| Phase 3 — gesture | ✅ done | `reorder-handle` hit layer + priority; `begin-reorder-drag` action; `reordering-child` interaction mode; `register-canvas-reorder-ipc.ts`; `reorder-gesture.ts`; `runReorderDrag` in `useCanvasPointerRouter.ts`; preload bridge. Full begin/commit/cancel-{escape,blur,undo} smoke matrix. |
| Phase 4 — dots | ✅ done | `ReorderDotsLayer.tsx` — geometric center dots (small at rest, grow on hover), insertion line during drag, suppressed off-`select`/mid-gesture. Visually verified in-app (dots render on a packed managed row). |

**Open questions — resolved.** O1 → yes (headless command shipped). O2 → new
`reordering-child` mode (D-O2 in ADR 0015). O3 → body drag moves the whole group.

### Dogfooding finding (the reachability wall)

First real use surfaced the friction the plan predicted: a plain selection — or
even a `Cmd+G` freeform group — shows **no affordance**. Reorder is only reachable
after an explicit "make auto-layout" step (`Shift+Cmd+A` / CLI), because dots are
gated on `managedLayout: true`. The user's expectation is blunter than either the
headless command or the deferred wrap-in-frame button:

> **"I don't want a button. I want any multi-selection to have drag-to-reorder."**

This reframes the next step (see "Next" below) and partially pushes back toward
the lightweight selection-op model that scope decision #1 set aside.

---

## What already exists (do not rebuild)

| Seed | Where | Note |
|---|---|---|
| `WorkspaceGroupLayoutMode = 'freeform' \| 'row' \| 'grid'` + `managedLayout: boolean` | `src/shared/types.ts`, `group-entity-state.ts` | Already persisted + JSON-Canvas round-tripped (`json-canvas-serializer.ts`, `workspace-restore.ts`). |
| `reflowGroupRow(group)` | `src/main/workspace-pages.ts:115` | Working row-packer — but **pages-only** (reads the `pages` array, orders by `canvasX`), and only fires on `addPageFromSource`. The kernel to generalize. |
| `computeLayoutMetrics` / `computeLayoutPositions(items, kind, colGap, rowGap, origin)` | `src/main/layout-math.ts` | Pure row/column/grid math. Used by design-system cluster tasks. |
| `tidySelectedPages()` | `src/main/workspace-pages.ts:314` | One-shot row pack of a selection — reference for the math, not the live path. |
| Group contiguity in `entityOrder` | ADR 0014, `entity-order-state.ts`, `src/shared/entity-order-math.ts` (`moveBlockBefore`, …) | A group's children are already a contiguous run in `entityOrder`. We reuse this run as the layout sequence (see Phase 2). |
| Drag stack | `useCanvasPointerRouter.ts` → `register-canvas-drag-ipc.ts` → `applyDragDelta` / `interaction-controller.ts` | Per-action renderer handlers + begin/update/commit/cancel IPC + batched undo. The pattern every new gesture follows. |
| Hit-test priority layers | `src/shared/hit-test.ts` (`resize-handle > chrome > anchor > body > background`) | We add a `reorder-handle` layer for the center dots. |
| Guides overlay | `GuidesLayer` in `above-view/App.tsx`, `canvas-guides.ts` | Reuse its line style for the drop-insertion indicator. |

**The floating multi-select toolbar in the mockup does not exist yet.** Per-kind
popups (`ShapePopup`, `GroupPopup`, `FilePopup`) carry no align/distribute/arrange
controls. That toolbar is Milestone 2.

---

## Canonical decisions (→ ADR 0015)

**D1 — An "auto-layout group" is a group with `managedLayout: true`.** Its
children's positions are *derived* (managed), not free. `layoutMode` selects the
packing (`'row'` is the only live mode in Milestone 1; `'column'`/`'grid'`
follow). `'freeform'` groups are unchanged — purely a selection/bbox container.

**D2 — Layout sequence = the group's `entityOrder` run.** A managed group's
children are already a contiguous run in `entityOrder` (ADR 0014 group
contiguity). The order *within that run* is the left-to-right layout sequence.
Reordering a child = `moveBlockBefore` within the run; reflow then reads the run,
not `canvasX`. **No new per-group `childOrder` field.** Tradeoff: in a managed
row, stack-order and layout-order are the same axis — acceptable because z-order
is meaningless for non-overlapping row children; documented in the ADR.

**D3 — Reflow is the single writer of managed children's positions.** Any change
to a managed group — membership, child resize, child reorder, gap/align change —
triggers `reflowManagedGroup(group)`. Managed children never hold authoritative
`canvasX/canvasY`; those are outputs. (They are still persisted so other tools /
freeform fallback render correctly.)

**D4 — Two distinct drags on a managed child.** The **center dot** is the reorder
handle (`reorder-handle` hit layer → `reordering-child` gesture). The child
**body** keeps today's behavior (click to enter/select the child; drag the body
moves the *whole group* as a unit, same as any group drag). You reorder with the
dot, you move/position with the body. This keeps the managed invariant: you can't
free-drag one child out of place.

**D5 — Reflow bypasses grid-snap on the managed axis** (matches today's
`reflowGroupRow`, which `snapToGrid`s the row origin then packs by gutter).
Alignment/distribution guides are **suppressed** during a reorder drag — the
op is managed, so confirming-alignment guides would be noise.

**D6 — Gap & alignment are group fields, defaulted in Milestone 1.** Milestone 1
uses the existing `CLUSTER_HORIZONTAL_GUTTER` constant and start-alignment. The
persisted `layoutGap?: number` and `layoutAlign?` fields land with the toolbar
(Milestone 2) — additive, no migration (absent → default).

**Glossary additions (apply to `CONTEXT.md` when Milestone 1 lands):**
- **Auto-layout group** — a group with `managedLayout: true`; children are
  reflowed, not freely positioned.
- **Reorder handle (center dot)** — the per-child drag target that reorders a
  child within its auto-layout group. A new `hit-test` layer above `body`.

---

## Phase ladder

### Milestone 1 — Drag-reorder (ships the core interaction)

#### Phase 1 — Generalized reflow engine *(main; pure + runtime)*

Lift row-packing out of the pages-only path so any entity kind can be a managed
child.

- Add pure `computeRowReflow(children: {width,height}[], gap, originX, originY)`
  → positions, building on `layout-math.ts`. Unit-tested in isolation.
- Add `reflowManagedGroup(groupId)` in a new `src/main/runtime/managed-layout.ts`
  (or fold into `group-entity-state.ts`): resolve the group's direct children
  *in `entityOrder` run order* across all entity arrays (text/file/shape/
  drawing/page/sub-group), apply `computeRowReflow`, write each child's
  `canvasX/canvasY` via its per-kind mutator, recompute the group bbox.
- Re-point `reflowGroupRow`'s callers (`addPageFromSource`) at the generalized
  function; delete the pages-only version once parity is confirmed.
- Trigger `reflowManagedGroup` on: child resize commit, membership change, and
  (Phase 3) reorder commit.
- **Tests:** unit for `computeRowReflow` (1, 2, N children; zero-width edge);
  smoke for managed-row round-trip (create managed group of mixed kinds →
  reflow → persist → reload → positions stable) and "resize a child → siblings
  reflow, one undo step." (Test contract: new runtime mutator ⇒ forward/reverse
  sync coverage.)

#### Phase 2 — Reorder model *(main; pure)*

- Confirm D2: a helper `managedChildOrder(groupId)` returns child ids in
  `entityOrder`-run order; `reorderManagedChild(groupId, childId, toIndex)` is a
  thin wrapper over `entity-order-math.moveBlockBefore` constrained to the run,
  then `reflowManagedGroup`. Single batched undo step.
- **Tests:** unit on `reorderManagedChild` (move first→last, middle→middle,
  no-op); smoke "reorder persists across reload" + "reorder is one undo step,
  undo restores both order and positions."

#### Phase 3 — Reorder gesture *(shared + renderer + IPC)*

- **Hit-test:** add a `reorder-handle` layer (`src/shared/hit-test.ts`) above
  `body`. A child contributes a center-dot target only when its managed group is
  selected (or the child is selected within it). Dot geometry is a runtime-
  derived slot (centre of the child rect), broadcast on the scene entity like
  chrome slots — not persisted.
- **Action:** new `CanvasPointerAction` `begin-reorder-drag { childId, groupId }`
  in `src/shared/canvas-pointer-actions.ts`.
- **Interaction mode:** new `reordering-child { groupId, childId }` in
  `interaction-types.ts` / `interaction-controller.ts`. (We add a mode rather
  than a `dragging-entities` payload per §5.6: commit semantics genuinely differ
  — reorder+reflow vs free move — and it needs its own drop-index preview and
  cancel path. Flag for ADR review.)
- **IPC:** `canvas-reorder-child-{start,move,commit,cancel}` in a new
  `register-canvas-reorder-ipc.ts`. `start` calls `tryEnter` **before** any
  mutation (gesture-begin ordering gotcha — see `runtime/CLAUDE.md`). `move`
  carries the cursor → main computes the target index and broadcasts a drop
  preview. `commit` calls `reorderManagedChild` inside a batch. `cancel` →
  `cancelActive`.
- **Renderer:** `runReorderDrag` in `useCanvasPointerRouter.ts`, following the
  per-handler conventions (pointer capture, window `blur`→cancel, threshold).
  During the drag: lift the dragged child visually, render a gap insertion
  indicator (reuse `GuidesLayer` line style) at the live target index.
- **Tests:** smoke for the full gesture (begin/commit/cancel-on-escape/
  cancel-on-blur/cancel-on-undo) mirroring the existing drag gesture matrix.

#### Phase 4 — Dot affordance & hover polish *(renderer)*

- Paint the center dots in aboveView (geometric hit target, like anchors —
  *not* `data-overlay-ui` DOM buttons). Small at rest; grow when the item or
  group is hovered. Hover state is renderer-local ephemera (§5.2) — no IPC.
- Suppress dots during any active interaction (drag/resize/marquee/edit) and
  while a tool other than `select` is active, matching popup suppression rules.
- **Tests:** none required beyond Phase 3 (pure visual); verify manually.

### Next — Reorder on *any* multi-selection *(revised priority, from dogfooding)*

This is now the priority ahead of the Milestone 2 toolbar. Direction from the
user after first use:

> "I don't want a button. I want any multi-selection to have drag-to-reorder."

**Goal.** Select 2+ entities → a center dot appears on each → drag a dot to
reorder them; siblings shift to make room. No `Cmd+G`, no "make auto-layout"
step, no toolbar button. Reorder becomes a property of *having a selection*, not
of *being a managed group*.

**Tension with the Milestone 1 canonical decisions — must be resolved first.**
M1 made auto-layout deliberately group-backed and persisted (D1), with the
layout sequence living in the group's `entityOrder` run (D2) and reflow as the
sole writer of managed positions (D3). A loose multi-selection has none of that:
no group, no contiguous run, no "managed" positions. So this step is *not* just
loosening the dot-gating — it needs its own commit semantics. The two paths
would share the gesture + dot rendering and diverge at commit.

**Design options (pick before building):**

- **A — Pure positional permute (no group, nothing persisted but positions).**
  Dots show on any 2+ selection. The selection's current spatial order (sort by
  position along the dominant axis) defines the sequence; dragging permutes the
  items among the slots they already occupy, preserving the selection's outer
  bounds and gaps. One undo step, position writes only; `entityOrder` untouched.
  Closest to "just let me drag to reorder." *Open*: how to define slots for a
  not-quite-a-row selection (even-distribute within bbox? keep existing gaps?),
  and whether to show dots at all for clearly-scattered (non-collinear)
  selections.
- **B — Ephemeral managed row for the gesture's duration.** Same as A but
  expressed through the existing reflow engine: treat the selection as a
  transient row while dragging, reflow on each tick, leave items ungrouped on
  release. Reuses `computeRowReflow`; still persists nothing but positions.
- **C — Silent auto-group into a managed row.** Rejected for now: the user
  explicitly doesn't want a button *or* a surprise group; silently mutating the
  data model on a drag is the wrong feel.

**Recommended:** A (or A-via-B internally). It honors "no button, it just
works," keeps loose selections loose, and leaves the persisted managed-group
feature (M1) intact for explicit auto-layout rows.

**Work implied (sketch, not committed):**
- Hit-test: gate the `reorder-handle` layer on *being in the current
  multi-selection* (2+), in addition to the existing managed-group rule. Likely
  an eligibility predicate (collinear-enough selection) so dots don't appear on
  arbitrary scatter.
- A pure slot/permute helper (selected boxes + target order → new positions
  within current bounds), unit-tested like `computeRowReflow`.
- A selection-reorder commit path distinct from `reorderManagedChild` (no group,
  no `entityOrder` write) — one undo step.
- Dots + insertion indicator already exist (`ReorderDotsLayer`); extend their
  gating to the selection case.

**Open questions for the user:**
- **N1 — Slot model.** For a selection that isn't a clean row, do we (a) only
  show dots when items are roughly collinear, (b) always allow reorder and
  even-distribute within the bounding box, or (c) snap into a row on first
  reorder? 
- **N2 — Persistence.** Should selection-reorder ever persist anything beyond
  the new positions (e.g. remember the row so re-selecting keeps the dots), or
  stay purely ephemeral until the user explicitly makes an auto-layout group?
- **N3 — Axis.** Row-only first, or detect vertical/grid selections and reorder
  along the dominant axis?

### Milestone 2 — Auto-layout toolbar *(deferred; documented for continuity)*

Not in PR 1. Each is a later phase/PR:

- **Wrap selection → auto-layout** (the frame icon): convert a freeform
  multi-selection into a managed row group (or mark an existing group managed).
  This is the entry point that makes auto-layout reachable on arbitrary content.
- **Draggable gap handles** (the vertical bars between items) → persisted
  `layoutGap`.
- **Direction toggle** row/column/grid (live `layoutMode` reflow; generalize
  `computeRowReflow` to the existing `layout-math` column/grid paths).
- **Align** within the layout (start/center/end on the cross axis) → `layoutAlign`.
- **Distribute** (one-shot) — already have `tidySelectedPages` math to lift.
- **The floating multi-select toolbar shell** itself (a multi-select variant of
  `CanvasItemPopup` anchored to the selection bbox via `useMultiAnchoredPosition`).

> **Reachability note — resolved, then superseded.** Milestone 1 shipped the
> headless "make auto-layout from selection" command (O1), so reorder is reachable
> beyond page rows. Dogfooding showed even that is too hidden — the live priority
> is now **reorder on any multi-selection** (see "Next" above), which makes the
> wrap-in-frame button mostly redundant for reachability. This floating toolbar
> remains the home for *spacing / direction / align / distribute*, not the entry
> point for basic reorder.

---

## Build order

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4      (Milestone 1 ✅ done)
   (reflow)   (order)    (gesture)   (polish)
                                        │
                                        ▼
                        Next: reorder on any multi-selection
                        (resolve N1–N3, then build the
                         selection-reorder commit path)
                                        │
                                        ▼
                              Milestone 2 — toolbar
```

Milestone 1 was strictly sequential — each phase the substrate for the next —
and is complete. The next slice (selection-reorder) reuses Phases 3–4's gesture
and dots and adds a non-group commit path; it now leads Milestone 2.

Per the team's AFK-fast-route convention, each phase is one PR even if sizeable;
don't pre-split on LOC. Block only on the real dependency edges above.

---

## Risks & invariants to respect

- **I1 / layout pass.** All view-stack/visibility/bounds mutation stays in
  `layoutAllViews`. Reflow writes entity data → `markDirty` → layout pass; never
  `setBounds` directly.
- **I2/I3 token discipline.** `canvas-reorder-child-start` must `tryEnter`
  before mutating; commit/cancel must pair. Reorder is mutually exclusive with
  every other gesture.
- **Gesture-begin ordering** (`runtime/CLAUDE.md`). The `start` IPC enters
  `reordering-child` before any `requestLayout`-triggering mutation, or focus
  reconciliation cancels the gesture mid-drag (pages are the canary).
- **Undo batching.** Reorder = `entityOrder` move + reflow position writes →
  one `beginBatch`/`endBatch` + `markUndoBoundary`. One user action, one undo.
- **Group contiguity (ADR 0014).** `reorderManagedChild` must keep the child
  inside the group's run; reuse `enforceGroupContiguity`. Never let a reorder
  leak a child out of its run.
- **JSON Canvas round-trip.** `managedLayout`/`layoutMode` already serialize;
  new `layoutGap`/`layoutAlign` (Milestone 2) are additive Specular extensions
  (absent → default), same pattern as `specular.textStyle`.
- **Mixed-surface groups (ADR 0014 split rows).** A managed group can contain
  both notes and pages. Reflow operates in canvas space on all children
  regardless of paint surface; verify the sidebar split-row representation is
  unaffected (it reads `entityOrder`, which we mutate — confirm rows stay paired).

---

## Open questions for the user

**All Milestone 1 open questions are resolved** (see Status §). Kept here for the
record; live questions now live under "Next" (N1–N3).

- **O1 — Reachability in Milestone 1.** ✅ Resolved: shipped the headless
  command (`Shift+Cmd+A` / `specular auto-layout` / `POST /groups/auto-layout`).
  Dogfooding then showed even this is too hidden → see "Next".
- **O2 — New mode vs payload.** ✅ Resolved: new `reordering-child`
  `InteractionController` mode (ADR 0015 D-O2).
- **O3 — Body drag of a managed child** (D4). ✅ Resolved: body drag moves the
  whole group; only the dot reorders.

---

## Test contract for this feature

- New entity behavior on the **group** kind ⇒ smoke coverage of managed-row
  persistence + undo round-trip (Phase 1).
- New runtime mutators (`reflowManagedGroup`, `reorderManagedChild`) ⇒
  forward/reverse sync coverage: one Y.Doc transaction per op, undo round-trips
  cleanly (Phases 1–2).
- New gesture ⇒ full begin/commit/cancel-on-{escape,blur,undo,tab-switch} smoke
  matrix, mirroring the entity-drag gesture tests (Phase 3).
- Touches `src/main/workspace-*.ts` (`workspace-pages.ts` reflow re-point) ⇒
  smoke update required (not a pure refactor — behavior generalizes).
- Re-read `tests/README.md` and clear the four-criterion bar before each test.
