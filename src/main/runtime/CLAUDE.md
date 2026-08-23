# Runtime State Architecture

## Two-layer state model

| Layer | What | Where |
|---|---|---|
| **Y.Doc** | Workspace data (entities, groups, edges, annotations, viewport, active tab) | `space-doc.ts` |
| **Module variables** | Electron views, interaction mode, hover, drag, layout cache, timers, pages | `runtime-context.ts` |

Pages are hybrid: serializable fields (position, URL, preset) mirror to Y.Doc, but WebContentsView refs stay in `runtime-context.ts`.

## Broadcast path: the runtime store and its patch bus

How ephemeral state reaches renderers. One store, one patch channel, one
snapshot channel — see [ADR 0036](../../../docs/adr/0036-diffed-runtime-store.md).

`src/shared/runtime-store.ts` is the normalized form of `LayoutUpdateData`: a
map of scene entities keyed by id, plus small named **slices** (`camera`,
`selection`, `hover`, `inspect`, `pageScroll`, …). Everything below addresses
one of those two axes, which is what keeps an update's cost proportional to
what moved.

| Path | Producer | Carries |
|---|---|---|
| Mutator → patch | `broadcastRuntimePatch` from the mutator itself (`commitHoverTarget`, the page-scroll handler, the annotation-bbox fold) | one slice, no layout pass at all |
| Layout pass → diff → patch batch | `broadcastSceneUpdate` diffs the rebuilt scene against the baseline | the cells that moved, batched so a pass applies atomically |
| Snapshot baseline | `broadcastSceneSnapshot` on connect, and on the first pass ≥1s after the last snapshot | the whole scene, on top of that pass's patches |

Rules that hold this together:

- **The snapshot baseline is not optional.** It is what makes patches safe to be
  lossy: a dropped or mis-applied patch heals within about a second instead of
  leaving stale chrome. Never remove the cadence.
- **A snapshot never carries news.** Every fan-out sends the cells that moved
  first, so a snapshot only ever repeats what a renderer was already told. That
  is what lets the drift watchdog read any disagreement as a real loss rather
  than as a delivery it happened to make. A path that sends a snapshot without
  the diff behind it puts that back.
- **Filtering is a wire concern.** `runtime-store-filter.ts` names the slices
  each target reads and trims on the way out; the baseline stays the full store.
  `inspect` goes only to agent-layer. The bootstrap handler applies the same
  filter (`seatSceneBootstrap`) and re-seats the baseline through the same
  fan-out, so a renderer never starts holding a slice it will get no updates
  for, and main never diffs against a store nobody holds.
- **Identity is the product.** `shareStructure` runs on the scene main builds
  and on both the snapshot and the projection a renderer reads, because
  `useSlice` and the memoized layers bail out on reference equality.
- **A camera move is a `camera` slice patch.** Scene entities carry canvas-space
  geometry only; each renderer projects it (`src/shared/scene-projection.ts`).
  So a pan or zoom edits one slice and nothing else — `setViewportCamera` lays
  out the native views and calls `broadcastRuntimePatch`, never
  `markDirty('canvas')`. Main still projects for the `WebContentsView` bounds
  Chromium wants in window pixels, through the same helper.
- **`markDirty('canvas') + requestLayout()` still means "the scene changed."**
  It is the right call for a structural edit; it is the wrong call for a slice
  that has a patch producer, which would pay for a whole rebuild to deliver one
  cell.

The renderer half is `src/renderer/shared/runtime-store.ts` (applies snapshots
and patch batches), `runtime-store-feed.ts` (subscribes at module scope, before
the bootstrap request, so nothing sent while React mounts is dropped),
`hooks/useRuntimeStore.ts` (`useSlice`, `useLayoutData`), and
`runtime-store-drift.ts` — the dev-gated watchdog that diffs each arriving
snapshot against what the patch stream accumulated. Zero drift over a session is
the release gate for changes to any of this.

## Global undo stack

One undo stack spans all tabs. Tab switches are tracked transactions in Y.Doc, so pressing undo after switching tabs navigates back to the previous tab and restores its state.

## Diff-sync approach

Y.Doc is NOT the sole source of truth. Runtime arrays are mutated by existing code, then a diff-sync copies changes to Y.Doc:

```
mutation → runtime arrays → scheduleSpaceAutosave() → requestDocSync() → microtask → syncRuntimeToDoc()
```

`syncRuntimeToDoc()` compares runtime state against Y.Doc and writes only the differences. This avoids modifying every mutation site — the sync is automatic via the existing `scheduleSpaceAutosave()` hook.

## Undo/redo flow

Forward: mutations update arrays → diff-sync writes to Y.Doc → UndoManager captures the Y.Doc diff.

Undo (same tab): UndoManager reverts Y.Doc → `afterTransaction` observer fires → `syncDocToRuntime()` patches runtime arrays → deferred `layoutAllViews()`.

Undo (cross-tab): UndoManager reverts Y.Doc including `activeTabId` → observer detects tab change → `destroyActivePages()` → full rebuild from Y.Doc → deferred `layoutAllViews()`.

Side effects after undo run synchronously inside `afterTransaction`; `requestLayout()` defers the layout pass itself to the next event-loop turn, outside `afterTransaction`. See Gotchas → "Undo observer side effects".

## Tab switch as tracked transaction

Tab switches write to Y.Doc via `transitionToTab()`:
1. `applyTabState()` rebuilds runtime arrays (within `withSuppressedDocSync`)
2. `transitionToTab()` writes the new tab's state to Y.Doc as a tracked `'user'` transaction
3. UndoManager captures the diff between old and new tab state
4. `markUndoBoundary()` ensures the tab switch is a discrete undo step

## Gesture batching

Gestures (drag, resize, reorder, distribute) produce many small updates. Without batching, each would be a separate undo step.

- `beginGestureSession()` (`space-gesture-session.ts`) suppresses doc sync and registers with `mutateWorkspace` so per-tick calls defer their undo boundary
- Tick functions (`applyDragDelta`, `resizeMultiSelection`, registry updates) mutate freely while sync is held
- `session.finalize()` — one sync for the entire gesture, then one `markUndoBoundary()`
- At most one session at a time (one interaction token); a second begin warns and finalizes the stale session
- State derived from a gesture rather than authored by it arrives after the gesture closed — a sticky's measured content height is the current case. `commitUntracked` in `space-observers.ts` writes it in a transaction the UndoManager doesn't track, so it persists without becoming an undo step; inside a session the batch absorbs it instead (`reportContentHeight`)

## UndoManager scope

Tracked (undoable): entities, groups, edges, annotations, entity order, page positions, workspace metadata (active tab).

Not tracked: viewport zoom/pan (in a separate Y.Map excluded from UndoManager scope).

## Key files

- `space-doc.ts` — Y.Doc lifecycle, workspace accessors, snapshot hydration, diff-sync engine
- `space-undo.ts` — UndoManager setup, undo/redo API, selection metadata on undo steps
- `space-observers.ts` — forward sync (runtime→Y.Doc), undo sync (Y.Doc→runtime), cross-tab undo detection, batch control
- `space-model.ts` — owns workspace data arrays (edges, groups, annotations, tabs)
- `runtime-context.ts` — ephemeral state only (views, interaction, layout cache, timers, pages)
- `runtime-patch-broadcast.ts` — the scene bus: baseline, diff, patch batches, snapshot cadence, per-target routing

## Test coverage for this layer

This is a high-risk layer: a bug in persistence, forward/reverse sync, or undo batching can lose user work silently. When you change anything in `space-*.ts` or the diff-sync path:

- Add or update integration coverage under `tests/integration/` for the behavior change. Persistence, undo, and sync each have a dedicated file (`persistence.test.ts`, `undo.test.ts`, `sync.test.ts`) driving the real runtime in-process via `bootWorkspaceHarness()` — see [ADR 0024](../../../docs/adr/0024-in-process-integration-testing.md).
- Mutation-verify the test before committing — name the production-code change you used to confirm the test catches it. See `tests/README.md` for the convention.
- Forward sync changes need a "one mutation → one Y.Doc transaction" assertion. Reverse sync changes need an "undo applies without re-triggering forward sync" assertion.
- Undo batching changes need a "logically-grouped mutations collapse to one undo step; distinct user actions remain distinct" assertion.

See `tests/README.md` for the test bar and the harness API available to integration tests.

## Gotchas

- **Suppress flag**: `withSuppressedDocSync()` prevents sync loops during restore and undo. If you call `scheduleSpaceAutosave()` from inside an undo observer without suppressing, you create a feedback loop where each undo generates a new undo entry.
- **Tab switch suppress**: `applyTabState()` is called within `withSuppressedDocSync` during tab switches to prevent the normal forward-sync from running. The Y.Doc write happens separately in `transitionToTab()`.
- **Focus on page delete**: `removePageAtIndex()` transfers focus to `aboveView` after destroying page webContents, so keyboard shortcuts (including undo) keep working. (Pre-Phase-F this targeted bgView; aboveView now owns canvas-mode keyboard input.)
- **Undo observer side effects**: The undo observer runs `cancelActiveInteraction`, `sendInteractiveState`, `markAllDirty`, and `requestLayout` synchronously inside Y.Doc's `afterTransaction` — safe because the controller is reentrancy-safe (Phase 5d-v2 E1). `requestLayout()` schedules the actual layout pass on the next event-loop turn, so it runs outside `afterTransaction`.
- **Startup undo**: `clearUndoHistory()` is called after `initializeDocObservers()` to wipe any phantom entries from the initial doc sync.
- **Gesture-begin ordering**: Any code path that triggers a layout pass while a renderer-side gesture is armed must enter the gesture's `interactionState` *first*. `requestLayout()` runs `reconcileFocus()`, and if `interactionState.kind` is still `'idle'` the reconciler picks the canvas-mode default (aboveView post-Phase-F) or — if the selection elects a single page — the page itself. The renderer gesture's `window.blur` listener treats the resulting aboveView blur as a cancel and kills the gesture before any movement.

  Two flavors of this gotcha exist; both fix the same way:

  1. *IPC handler that mutates selection* (drag-start). `commitSelection` runs `requestLayout()`, whose debounced pass reconciles focus shortly after. `canvas-drag-{page,entity}-start` calls `tryEnter` before `applyDragStartSelection` so `interactionState.kind` has left `'idle'` by the time that pass runs — see `register-canvas-drag-ipc.ts`.
  2. *Renderer dispatches generic mutation IPC during pointermove* (resize). The router has no preceding "begin" IPC by default, so the first bounds-update IPC's synchronous `requestLayout` reconciles focus while still idle. The fix is a dedicated begin/end pair: `canvas-resize-begin` calls `tryEnter({ kind: 'resizing-entity', target })` so the first move tick reconciles against `'resizing-entity'` (which expects aboveView focus). `runResize` in `useCanvasPointerRouter` dispatches `beginResize` before installing listeners and `endResize` in cleanup.

  When adding a new gesture: if the renderer mutates anything that requestLayouts before the gesture is committed, you need a begin IPC that calls `tryEnter` first. Pages are the canary — non-page entities don't populate `focusedPageId`, so the bug is invisible for them.
