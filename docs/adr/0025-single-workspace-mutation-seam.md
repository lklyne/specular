# ADR 0025 — Single workspace mutation seam

**Status:** Accepted
**Date:** 2026-07-03
**Related:** [ADR 0024 — Entity-kind registry spans runtime and persistence](./0024-entity-kind-registry-spans-runtime-and-persistence.md) (sequenced before this — same files), [ADR 0014 — Canvas stack order](./0014-canvas-stack-order.md).
**Origin:** The deepen pass of the architecture audit (`docs/audit/deepen-3673a35.md`, candidate 2).

## Context

Every workspace mutation must end with the same ritual: mark dirty → schedule autosave → request layout → mark undo boundary, with batching rules for gestures (one undo step per drag, not per tick). Today no module owns that ritual. `document-commands.ts` hand-sequences it at every mutator (30× `scheduleWorkspaceAutosave`, 31× `requestLayout`, 12× `markUndoBoundary`); the gesture bracket (`beginBatch` … `endBatch` … `markUndoBoundary`) is copy-pasted across drag, reorder, and distribute; the ordering rules live as prose gotchas in `src/main/runtime/CLAUDE.md` — tribal knowledge audited per call site. The headless apply path (CLI/HTTP via the entity-kind registry) runs its own parallel bookkeeping.

## Decision

One seam owns the ritual: `mutateWorkspace(fn, opts)` runs the mutation and then performs dirty/autosave/layout/undo-boundary itself, plus a gesture-session object (`begin` / `applyDelta` / `finalize`) for multi-tick interactions.

1. **Structural enforcement — the wrapper is the only door.** `document-commands.ts` exports only wrapped commands; raw per-kind mutators are internal to the runtime layer. Forgetting the trailer stops being possible; the CLAUDE.md ordering gotchas become invariants of one function instead of review vigilance.
2. **Headless and interactive share it.** The entity-kind registry's `create`/`update`/`delete` dispatch through `mutateWorkspace`, so an agent editing via CLI and a user dragging on canvas get identical autosave and undo semantics. Bulk headless edits use the same session batching gestures use.
3. **One call = one undo step, by default.** `mutateWorkspace` marks an undo boundary unless the call happens inside a gesture session; the session brackets many deltas into one step (`begin` … `finalize`). The exceptional case (multi-tick gestures) is explicit; the default matches user intuition. Drag, reorder, and distribute all use the session instead of their three hand-rolled copies of the bracket.

## Consequences

- The one-transaction-per-mutation and one-undo-step-per-gesture invariants become properties of a single testable module.
- `document-commands.ts` shrinks substantially; the `snapToGrid` prelude (5 copies) folds into one shared geometry helper alongside.
- A forgotten session during a new gesture yields too-granular undo — visible and cheap to fix, unlike today's silent merged-undo failure mode.
- PRs touching this land under the integration-coverage requirement for `workspace-*.ts` (CLAUDE.md test contract).
- Implementation plan: `docs/plans/deepen-runtime.md` (steps sequenced after ADR 0024's registry work — same files).
