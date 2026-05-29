# Selection reorder — drag-to-reorder any aligned multi-selection

> **Refactor of the auto-layout WIP.** Supersedes the "Next — Reorder on *any*
> multi-selection" section of [`auto-layout-reorder.md`](./auto-layout-reorder.md)
> and reframes [ADR 0015](../adr/0015-auto-layout-groups.md). Milestone 1 (the
> persisted, group-backed auto-layout row) stays — it is *demoted* from "the
> entry point for reorder" to "the opt-in way to make a reorder permanent."

## Goal

Select 2+ entities that form an evenly-spaced row → a **center dot** appears on
each → drag a dot to reorder; siblings reflow to keep the row even. No `Cmd+G`,
no "make auto-layout" step, no toolbar button. Reorder is a property of *having
an aligned selection*, not of *being a managed group*. Nothing persists but the
new positions.

**The eligibility rule is FigJam's, and it is the spec:**

- Multi-selection with **equal gaps** between consecutive items along the
  dominant axis → dots show. (FigJam screenshot: four squares, equal spacing,
  dot on each + gap bars between.)
- The moment spacing becomes **unequal** → dots disappear. (Same four squares,
  uneven spacing, no dots.)

Equal-gap detection does double duty: it gates the affordance *and* it defines
the slots/gap the reflow packs into. A selection that isn't already a clean row
has no well-defined slots, so it gets no dots — the ambiguity never arises.

## The inversion — why this is a refactor, not an add-on

Milestone 1 and this goal point opposite directions on the source of truth:

| | Source of truth | Derived |
|---|---|---|
| **M1 (managed group)** | stored order (the group's `entityOrder` run) | child positions (reflow output) |
| **This goal** | positions (geometry) | order (read from geometry per gesture) |

M1 is Figma's *persistent container* model: a stored child order, positions
recomputed from it. This goal has **no container** — so there is nowhere for a
stored order to live, and geometry *must* be the truth. Reorder becomes
position→position; the "sequence" is an ephemeral read of the boxes at
gesture-start.

Geometry-is-truth is also the **simpler** implementation, because positions are
already persisted, already undoable, already drag-writable. We add **zero new
persisted state**. Concretely, it deletes a complication M1 had to invent:
`commitAsOneTransaction` (`workspace-observers.ts:132`) exists *only* to merge
`writeEntityOrder`'s own transaction with reflow's diff-synced position writes
into one undo step. With no stored order to write, a reorder is just a batched
multi-entity position write — one transaction through the normal drag batch. The
ADR's "Undo batching" section and the "reorder is one Y.Doc transaction" smoke
assertion both defend a problem D2 *created*; this model removes it instead.

## Eligibility: the "reorderable row" (pure, the heart of the feature)

A new pure module `src/shared/reorder-row.ts`. No Electron, no DOM — unit-tested
in isolation like `computeRowReflow`. Shared so the hit-tester, the renderer
painter, and the main-side commit all use *one* definition (the M1 duplication
between `hit-test.ts` and `ReorderDotsLayer.tsx` is exactly what we must not
repeat).

```ts
interface Box { id: string; x: number; y: number; width: number; height: number }

interface ReorderableRow {
  axis: 'x' | 'y'                  // dominant axis (larger center spread)
  order: string[]                  // ids sorted along axis
  gap: number                      // the common gap (avg of equal gaps)
  origin: { x: number; y: number } // min corner — packing start
  boxesById: Map<string, Box>      // frozen sizes/cross-axis at detect time
}

/** Equal-gap row detector + eligibility gate. Returns null when the selection
 *  is not a clean, evenly-spaced, non-overlapping line. */
function detectReorderableRow(
  boxes: readonly Box[],
  opts?: { gapTolerance?: number }, // default ~1px (snapped content is exact)
): ReorderableRow | null

/** Drop index for a cursor position along the row's axis: how many *other*
 *  items have their center before the cursor. (Generalizes M1's
 *  computeReorderDropIndex off a groupId onto a box list.) */
function dropIndexForCursor(row: ReorderableRow, cursorAlongAxis: number, movingId: string): number

/** Repack the row with `movingId` moved to `dropIndex`. Packs along `axis`
 *  from `origin` by `gap`; each item KEEPS its own cross-axis coordinate
 *  (a box that sat lower stays lower — see open question Q1). Returns only
 *  the positions that changed. */
function reorderRowPositions(row: ReorderableRow, movingId: string, dropIndex: number): Map<string, { x: number; y: number }>
```

`detectReorderableRow`, sketch:

```
if boxes.length < 2 → null
axis = spread(centers.x) >= spread(centers.y) ? 'x' : 'y'
sort by leading edge along axis
gaps[i] = next.leadingEdge - cur.trailingEdge      // along axis
if any gap < 0 → null                              // overlap = not a row
if max(gaps) - min(gaps) > gapTolerance → null     // unequal = no dots
return { axis, order, gap: avg(gaps), origin: minCorner, boxesById }
```

That's the whole eligibility brain. Dumb, total, testable.

## Architecture — keep / generalize / demote

### Keep wholesale (already group-agnostic in spirit)

- **`computeRowReflow`** (`layout-math.ts:59`) — stays the *managed-group* packer
  (shared top/start-alignment). The selection path uses `reorderRowPositions`
  instead because it preserves per-item cross-axis; both are tiny pure packers.
- **The gesture** — `runReorderDrag` (`useCanvasPointerRouter.ts:1083`), the IPC
  family (`register-canvas-reorder-ipc.ts`), the `reorder-gesture.ts`
  coordinator, pointer capture + blur/escape/undo cancel matrix. Group-free
  already; only the *begin payload* and *commit target* change.
- **Dot + insertion-line visuals** (`ReorderDotsLayer.tsx`) — keep the SVG; swap
  the gating source (see below).
- **The `reorder-handle` hit layer + priority slot** (`hit-test.ts`,
  `interaction-priority.ts`) — keep; swap the eligibility source.

### Generalize (the new spine)

1. **`src/shared/reorder-row.ts`** — the pure module above.
2. **One shared dot selector** — `reorderableDots(layout) → { id, center }[]`,
   the single source for *both* the hit-tester (`collectReorderHandleTargets`)
   and the painter (`ReorderDotsLayer`). Eligibility = the union of two doors
   (next section). Kills the copy-pasted predicate.
3. **`reorderSelection(orderedIds, movingId, dropIndex)`** (main runtime) — the
   position-only commit. Computes positions via `reorderRowPositions`, writes
   each through its per-kind mutator inside one `beginBatch`/`endBatch` +
   `markUndoBoundary` (the same batched-multi-write shape `resizeMultiSelection`
   already uses). No `entityOrder`, no `managedLayout`, no
   `commitAsOneTransaction`.
4. **Rename the mode** `reordering-child { groupId, childId, dropIndex }` →
   `reordering-row { ids, movingId, dropIndex, axis }` in
   `interaction-types.ts` (+ controller/state/snapshot). The shared interaction
   type is the most expensive-to-reverse surface (§5.6); "child" bakes in the
   group assumption, and the renderer needs `ids` to draw the insertion line
   door-agnostically.

### Two doors, one gesture (managed group demoted, not deleted)

The gesture + dots + mode are shared; only **eligibility** and **commit**
branch. Exactly the "share the gesture, diverge at commit" split the old plan's
"Next" predicted — now made concrete:

| Door | Eligibility | Commit |
|---|---|---|
| **Selection** (new, primary) | `detectReorderableRow(selectedBoxes)` ≠ null | `reorderSelection` — positions only, ephemeral |
| **Managed group** (M1, persisted) | a managed-row group is selected → its children | `reorderManagedChild` — rewrites `entityOrder` run, persists |

`reorderableDots(layout)` returns the union; `reorder-gesture.ts` records which
door armed the gesture and dispatches the matching commit. M1's persisted row
keeps working unchanged; it's just no longer the *only* way to see a dot.

This preserves the reframe: **the default is ephemeral and free; `make-auto-layout`
(`Shift+Cmd+A` / CLI / HTTP) becomes purely "crystallize this arrangement into a
managed row so future inserts auto-reflow" — a deliberate later act, not a
precondition.** It also keeps `reflowManagedGroup` earning its keep as the
engine behind multi-breakpoint page rows (`addPageFromSource`).

### Unchanged (managed door internals)

`reflowManagedGroup`, `reorderManagedChild`, `makeAutoLayoutGroup`,
`managedChildOrder` / `writeManagedChildOrder`, the `entityOrder` coupling, the
`make-auto-layout` binding/CLI/HTTP, `commitAsOneTransaction`. All scoped to the
managed door now.

### No longer the story (but not deleted)

The reachability narrative "make-auto-layout is how you get reorder." Dots now
come from eligible selections; the headless command only adds persistence.

## Gesture freeze (stability)

Freeze the `ReorderableRow` at gesture start (slots, gap, order, box sizes,
cross-axis). `dropIndexForCursor` runs against the frozen non-moving slots; the
live preview and commit repack the frozen order. Freezing avoids a feedback loop
between the live-preview reflow and re-detection mid-drag.

## Prerequisites — do these before `/afk-local`

*Not a worker task — human setup.* This refactor builds **on Milestone 1** (the
persisted managed-group auto-layout), which currently sits **uncommitted** on
`feat/auto-layout`. Phase C references M1 code directly (`reorderManagedChild`,
the renamed `reordering-child` mode, the `reorder-handle` hit layer,
`ReorderDotsLayer`), so M1 must exist in the branch the loop builds from.

`/afk-local` creates `claude/feat-selection-reorder` from `origin/main`, and every
worker fire runs `git reset --hard origin/<branch>` — so uncommitted M1 would be
**wiped**, and a main-based branch wouldn't contain M1 at all. Before kicking off,
pick one:

- **(a) Land M1 first (recommended).** Commit the M1 WIP on `feat/auto-layout` and
  merge it to `main` (it typechecks and passes its managed-layout smoke per the M1
  plan). Then `/afk-local docs/plans/selection-reorder.md` branches cleanly from
  main with M1 present.
- **(b) Branch off M1 manually.** Commit + push `feat/auto-layout`, create
  `claude/feat-selection-reorder` from it, commit the plan + the `dex` epic, and
  run `scripts/afk-loop.sh <epic-id>` directly (bypassing the main-based branch
  creation).

Either way the working tree must be clean and pushed before the loop starts.

## Working agreement — applies to every phase

- **Read this plan top-to-bottom first.** The design is locked in §Goal, §The
  inversion, §Eligibility, and §Architecture — don't re-open it. Block only on a
  real dependency gap or a decision genuinely outside this plan; otherwise proceed
  on the documented default (see §Open questions).
- One PR per phase; don't pre-split on LOC (AFK fast-route).
- Honor the interaction-layer invariants (`docs/interaction-layer.md` §6) and the
  gesture-begin ordering rule (`src/main/runtime/CLAUDE.md`): a begin-IPC must
  `tryEnter` before any layout-triggering mutation. Pointer events only; canvas
  coord math from `src/shared/coords.ts`; no `setTimeout(0)` — `markDirty` + return.
- `src/renderer` must not import from `src/main`; `src/shared` stays side-effect-free.
- Verify before opening the PR: `pnpm typecheck && pnpm test:unit` (always) +
  `pnpm test:smoke` (Phases B, C). The pre-existing `pages > takes a screenshot`
  smoke is environmental — ignore it; everything else must be green.
- Semantic commits, no emojis; `Co-Authored-By` trailer on commits, the Claude
  Code line on PR bodies. UI-facing copy in sentence case. The `reordering-child →
  reordering-row` rename is internal — keep it out of any changelog headline. If a
  step must merge a PR, `gh auth switch --user lklyne` first.

## Build order

```
Phase 0 ──► A ──────► B ──────────► C ───────────────► D
 (docs)    (kernel)  (commit path)  (gating+gesture)   (live preview)
```

Phase 0 first — lock the inversion in ADR 0015 before code builds on it. A→B→C is
the dependency spine. D is independent polish after C, and its visual check needs
a human (the headless loop can't eyeball the canvas).

## Phase 0 — Lock the model in docs (ADR 0015 + CONTEXT.md)

Before starting: see §Working agreement. No source changes — encode the inversion
in the canonical docs so the code phases build on a settled model.

- **ADR 0015** (`docs/adr/0015-auto-layout-groups.md`) — add **D7 — Selection
  reorder:** geometry-is-truth, position-only, ephemeral; eligibility = equal-gap
  row (FigJam parity); the gesture + dots are shared with the managed door,
  diverging only at commit (`reorderSelection` vs `reorderManagedChild`). Reframe
  **D1**: a managed group is the opt-in *persistent crystallization* of a
  reorderable arrangement, not the entry point for reorder.
- **CONTEXT.md** (Auto-layout section) — add **Reorderable row:** an evenly-spaced
  multi-selection (equal gaps along the dominant axis) that exposes reorder dots;
  ephemeral, no group, nothing persisted but positions. Clarify dots now appear on
  eligible selections, and `make-auto-layout` is the persist-it upgrade.
- **Done when:** both docs updated; `pnpm typecheck` still clean.

## Phase A — Pure kernel: `src/shared/reorder-row.ts`

Before starting: see §Working agreement; the signatures live in §Eligibility —
implement them, don't redesign them. Depends on nothing.

Build the pure module — `detectReorderableRow`, `dropIndexForCursor`,
`reorderRowPositions` — exactly as specced in §Eligibility. No Electron, no DOM;
`src/shared` stays side-effect-free.

- **Done when (unit tests):** equal-gap row → eligible; one unequal gap → null;
  overlapping items → null; 2 items; vertical column; gap-tolerance boundary;
  mixed-width repack moves the correct edges; cross-axis preserved (Q1);
  drop-index at both ends. `pnpm typecheck && pnpm test:unit` green.

## Phase B — Selection commit path: `reorderSelection` (main runtime)

Before starting: see §Working agreement. Depends on Phase A.

Add `reorderSelection(orderedIds, movingId, dropIndex)` in the main runtime:
compute target positions via `reorderRowPositions`, write each entity through its
per-kind mutator inside one `beginBatch`/`endBatch` + `markUndoBoundary` (mirror
`resizeMultiSelection`'s batched multi-write). **No** `entityOrder` write, **no**
`managedLayout`, **no** `commitAsOneTransaction`. Add a test-only route + an
`AppClient` helper so smoke can drive it.

- **Done when (smoke):** reordering a loose equal-gap selection permutes positions
  to the new sequence; **one undo step** restores them; `entityOrder` is unchanged
  and **no `managedLayout` group appears on disk** (mutation-verify: assert the
  disk snapshot has no group). Forward/reverse sync per `runtime/CLAUDE.md`.
  `pnpm test:smoke` green.

## Phase C — Gating + gesture rewire (two doors)

Before starting: see §Working agreement and §Architecture ("Two doors" +
"Gesture freeze"). Depends on A + B. This phase edits the shared interaction type
— the costliest surface to reverse — so get the rename right.

- One shared `reorderableDots(layout) → { id, center }[]` selector returning the
  **union** of the two doors (eligible selection via `detectReorderableRow`, plus
  a selected managed-row group's children). Consume it in **both** `hit-test.ts`
  `collectReorderHandleTargets` and `ReorderDotsLayer` — delete the duplicated
  predicate.
- Rename the mode `reordering-child { groupId, childId, dropIndex }` →
  `reordering-row { ids, movingId, dropIndex, axis }` across `interaction-types.ts`,
  the controller, `interaction-state.ts`, and the snapshot.
- `begin-reorder-drag` drops the required `groupId` (carry `movingId`; main
  resolves which door armed it). `reorder-gesture.ts` freezes the row at start and
  branches commit: selection → `reorderSelection`; managed group →
  `reorderManagedChild`.
- **Done when (smoke):** full begin/move/commit/cancel-{escape,blur,undo,tab-switch}
  matrix for the **selection** door (mirror the M1 matrix in
  `managed-layout.test.ts`); regression that the **managed** door still commits via
  `reorderManagedChild`; an **unequal-gap** selection exposes **no** reorder hit
  target. `pnpm typecheck && pnpm test:unit && pnpm test:smoke` green.

## Phase D — Live reflow preview (renderer)

Before starting: see §Working agreement. Depends on C; independent polish.

During the drag, reflow siblings visibly via renderer-side `reorderRowPositions`
against the frozen row, instead of only painting an insertion line — the
difference between "elegant" and "mechanical." Pure renderer ephemera: no IPC, no
data change.

- **Done when:** siblings shift to make room as the dragged dot crosses slot
  boundaries; release commits to the previewed arrangement. **Verify in-app** — the
  headless loop can't eyeball this, so a human (or an attended `/verify` pass)
  confirms the acceptance feel: dots on an equal-gap selection, dots vanish when
  spacing is uneven, drag reorders with live reflow, one undo restores, and reload
  shows only moved positions (no group, no `managedLayout`). `pnpm typecheck` green.

## Open questions (with dumb defaults)

- **Q1 — Cross-axis on reorder.** *Default: each item keeps its own cross-axis
  coordinate* (a box that sat lower stays lower; we only permute along the
  dominant axis). Matches the FigJam screenshot (the offset 4th box keeps its
  offset). Alternative: snap all to a shared baseline — defer to Milestone 2
  align.
- **Q2 — Column support.** The predicate is axis-generic; ship **row** first,
  flip on column in the same module once row is solid (near-free).
- **Q3 — Staircase edge case.** Equal x-gaps + monotonically rising y would
  currently qualify as a row. Acceptable for v1 (rare; reorder still does
  something sane). Add a cross-axis-overlap guard only if it bites.
- **Q4 — Gap tolerance.** Start ~1px (snapped/distributed content has exact
  gaps). Widen only if dogfooding shows near-even rows frustratingly excluded.
- **Q5 — Mode vs payload (§5.6).** Kept as a mode (`reordering-row`) because the
  preview + cancel path differ; but note it's now a close cousin of
  `dragging-entities` (commit is also a position write). Revisit collapsing to a
  payload if the two converge further.

## Test contract for this feature

- New pure module ⇒ unit coverage of every predicate branch (Phase A).
- New runtime mutator (`reorderSelection`) ⇒ forward/reverse sync: one Y.Doc
  transaction, undo round-trips, nothing persisted but positions (Phase B).
- New/renamed gesture ⇒ full begin/commit/cancel-{escape,blur,undo,tab-switch}
  smoke matrix (Phase C).
- Re-read `tests/README.md`; clear the four-criterion bar before each test.

*(The ADR 0015 / CONTEXT.md amendments are Phase 0 above — they land first.)*
