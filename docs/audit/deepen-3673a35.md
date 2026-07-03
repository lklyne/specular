# Deepen pass — findings at 3673a35

Run 2 of the audit sequence in `docs/architecture-audit.md`
(`/improve-codebase-architecture`, 2026-07-03). Four parallel explorations over
`src/main/runtime/`, `src/renderer/above-view/`, the IPC/preload bridge
surface, and an entity-kind cross-cut. Vocabulary: module / interface / depth /
seam / adapter / leverage / locality, per the skill's LANGUAGE.md.

## Status & pickup (read this first)

For a fresh agent continuing the audit:

- Runs 0–1 (fallow ground truth, ponytail cut pass) are **done and applied** —
  see `ponytail-36eb937.md` and the `refactor(audit):` commits.
- Run 2 (this document) is **done**: six deepening candidates, ranked below.
- The grill on candidates 1–2 is **done** (2026-07-03): decisions recorded in
  [ADR 0024](../adr/0024-entity-kind-registry-spans-runtime-and-persistence.md)
  and [ADR 0025](../adr/0025-single-workspace-mutation-seam.md), CONTEXT.md
  updated (Entity bullet + kind capability table).
- Remaining work: **kick off the three tracks** — all plans are written and
  shovel-ready: `docs/plans/deepen-runtime.md` (candidates 1→2),
  `docs/plans/deepen-ipc.md` (3→4), `docs/plans/deepen-above-view.md` (5→6).
  Use `/afk-local` or `/afk-feature` per plan doc: one feature branch off
  main per track, one PR per step, integration PR at the end.
- The three tracks touch **disjoint files** — safe to run concurrently.
  Within a track, steps are serial.
- Test gate: candidates 1–2 touch `src/main/runtime/workspace-*.ts`, so their
  PRs require smoke-coverage updates (CLAUDE.md test contract).
- Decision of record: **no GitHub issues for this push** — planning docs under
  `docs/plans/` are the hand-off artifact (`afk-local`/`afk-feature` accept
  them directly). File issues later only if a track stalls or the future
  automated loop (`docs/audit/future.md`) needs dedup targets.

## Preserve (do not simplify away)

Per the audit preamble: `docs/interaction-layer.md` §6 invariants, CONTEXT.md,
accepted ADRs, the two-layer state model (Y.Doc = truth, runtime = ephemeral),
the layered WYSIWYG canvas with live inline pages. Explorations confirmed
these as deep and earning their keep: `entities/contract.ts` registry, the
diff-sync engine (`syncRuntimeToDoc`/`syncMapFromArray`),
`commitAsOneTransaction`, `withSuppressedDocSync`, `workspace-undo.ts`
side-effect queue, `pointer-session.ts`, `useAnchoredPosition`,
`CanvasItemPopup` compound (ADR 0008), the file-renderer plugin registry, and
the main-side `InteractionController` token model (I2/I3).

---

## Ranked candidates

### 1. Finish the entity-kind registry (half-adopted seam)

**Files:** `src/main/entities/contract.ts`, the six `*-entity-state.ts`
modules, `json-canvas-serializer.ts`, `workspace-observers.ts`
(`syncDocToRuntime`), `canvas-layout-data.ts`, `sidebar-builder.ts`,
`workspace-entities.ts`, `delete-selection.ts`, `shared/entity-ops.ts`,
`entity-order-state.ts`.

**Problem:** ADR 0019's `EntityKindDefinition` registry is deep on the
headless apply path (CLI/HTTP) only. Smoking gun: `serialize` and
`defaultSize` are declared on the interface but never dispatched — dead
surface. Everywhere else the kind list is hand-enumerated: serialization is a
6-branch if/else twice (`json-canvas-serializer.ts:88-100`, `:305-329`); the
`[...textEntities, ...fileEntities, ...]` fan-out is copy-pasted ~8×
(`canvas-layout-data.ts:346`, `sidebar-builder.ts` ×5,
`workspace-entities.ts` ×4); interactive delete hand-buckets into 7 arrays
(`delete-selection.ts:31-64`) instead of calling `getEntityKind(kind).delete`;
reverse sync (`workspace-observers.ts:198-308`) hand-lists kinds *and* page
properties (L264-272 — a new persisted page field silently won't restore on
undo), reached through a 20-field `RuntimeStateRefs` bag that makes it
untestable in isolation. Adding one entity kind = **~18 edit sites across
~14 files; exactly one is registry-driven** (full site map in the cross-cut
exploration, reproduced at the end of this doc).

**Solution:** Make the other layers call the registry. Reclaim the dead
members (serializer + `footprintForItem` dispatch through it); add per-kind
runtime-state declarations (array, persist/scene projections,
rebuild-from-doc) so sync, stack order, scene building, and sidebar iterate
registered kinds; route interactive delete through the apply path's `delete`;
drive the six near-identical `*-entity-state.ts` CRUD bodies from field
descriptors, keeping only real divergence per kind (file's renderer/repo
inference, shape's sizing, text's `defaultWidthMode`). Also folds in: the
page/file device-preset command duplication
(`document-commands.ts:1074-1208`) via a shared device-metadata capability.

**Not unified (false-abstraction line):** per-kind React components (popups,
body layers), `resizeConfigForEntity`-style genuinely-kind-specific UX, and
the file-renderer plugin registry (orthogonal axis, well-factored). A thin
declarative interaction-capability record (`hasChrome`, `hasAnchors`,
`minSize`, `aspectMode` — today's `hit-test.ts:430-440` predicates) is the
most that layer should absorb.

**Benefits:** Locality — "add a kind" collapses to one handler file + its
React surfaces; the reverse-sync bug class gets one owner. Tests — register a
fake kind, assert it round-trips through persistence/undo/stack-order/scene
instead of wiring a 20-function ref bag.

**Open design questions (for the grill):** what lives on
`EntityKindDefinition` vs a sibling runtime declaration; whether `edge` (not
in the registry at all today) joins it; migration order.

### 2. A single "mutate workspace" seam

**Files:** `document-commands.ts` (1,218 lines), `workspace-observers.ts`,
`workspace-undo.ts`, `workspace-autosave.ts`.

**Problem:** No module owns "apply this mutation; I handle the rest." Every
mutator hand-sequences mutate → `markDirty` → `scheduleWorkspaceAutosave` →
`requestLayout` → `markUndoBoundary` (30× autosave, 31× requestLayout, 12×
markUndoBoundary in `document-commands.ts` alone); the gesture ritual
(`beginBatch`…`endBatch`…`markUndoBoundary`) is copy-pasted across drag
(`initializeDrag`/`applyDragDelta`/`finalizeDrag`), `reorderSelection`
(L1016), and `distributeSelection` (L1060); the `snapToGrid` prelude has 5
copies (L669, 718, 747, 815, 849). The ordering rules are the CLAUDE.md
gotchas — tribal knowledge audited per call site.

**Solution:** `mutateWorkspace(fn, opts)` wrapper owning the trailer, a
gesture-session object (`begin/applyDelta/finalize`) shared by
drag/reorder/distribute, and one `snapGeometryPatch` helper.

**Benefits:** One-transaction-per-mutation and one-undo-step-per-gesture
become enforced invariants of a single testable module; `document-commands.ts`
shrinks substantially. Sequence *after* candidate 1 (same file).

### 3. A typed IPC channel contract

**Files:** all 15 `src/main/ipc/register-*.ts`, all 18 `src/preload/*.ts`,
`src/shared/types.ts` (the 8 `*ElectronAPI` interfaces).

**Problem:** A channel's interface (name + payload + direction) has no seam —
restated as a raw string at every adapter. `'theme-changed'` appears in 11
files; main handles 125 channels while preloads send 198; drift is
runtime-silent. `CanvasBgElectronAPI` (`types.ts:1619-1849`) fronts a
117-send pass-through bridge — a seam contextIsolation forces to exist,
implemented with zero leverage.

**Solution:** One typed channel map in `src/shared/` (name → payload type +
direction); `send<T>`/`on<T>` key off it; the preload send-half becomes
derived-by-loop; the 8 `*ElectronAPI` interfaces move out of `types.ts`. The
broadcast-payload vocabulary in `types.ts` **stays together** — it's
cohesive; splitting it loses locality. Also folds in: duplicate
`right-details-panel-*` vs `canvas-*` device channels collapse (both already
converge on the same `document-commands` functions).

**Benefits:** Channel drift becomes a compile error; a new action drops from
6–7 file edits to contract entry + main handler + renderer call.

### 4. One broadcast seam for the nine renderer views

**Files:** `preferences.ts` (`broadcastTheme` — 9 hand-named targets +
per-call `isDestroyed()` guards), `window-init.ts` (repeats all 9),
`layout-engine.ts`, `view-refs.ts` (9 module-level let+setter pairs),
`broadcastToDebugTargets` (a second incompatible mini-fan-out).

**Solution:** View registry (`{ webContents, roles }`) behind
`broadcast(channel, payload, filter?)`; `view-refs.ts` globals collapse into
it. Rides on candidate 3's contract — sequence after it.

### 5. Unify renderer pointer authority in above-view

**Files:** `above-view/App.tsx` (1,579 lines), `useCanvasPointerRouter.ts`.

**Problem:** "Input has one authority" holds in main but not in the renderer:
three parallel input adapters (pointer router, `useAnnotationDrawingGestures`,
and a 137-line inline placement/comment capture handler at App.tsx:907-1044)
arbitrated by five overlapping booleans (`routerOwnsCanvasPointers`,
`toolGestureOwnsCanvasPointers`, `overlayInteractive`, `skipPointerCapture`,
`commentToolBlocked`). The inline handler duplicates the router's
threshold/session/dispatch idiom. Separately, "is the pointer over this
page's content rect" is computed three ways (App.tsx:807-826, 1076-1083 and
1173-1178 are verbatim-identical) — hit-test knowledge invariant I9 says
lives in `src/shared/`.

**Solution:** Placement and comment become `CanvasPointerAction` kinds with
`run*` handlers behind the router (the shape `interaction-layer.md` §4.6
already documents); the five booleans collapse into one pure
`canvasPointerOwner(state)` selector; extract `pointerOverPageContent` into
shared next to `hitTest` plus one `usePageInputForwarding` hook (cursor
mirror + hover-forward + wheel-route). Also folds in: the 8-popup mount mux
(App.tsx:1486-1566) becomes a kind-keyed table.

**Preserve:** §4.6 intentional divergences (`runForwardPointer` no-blur,
begin-before-patch resize ordering); the main/shared/renderer mode split.

### 6. A pure gesture-decision seam under the router

**Files:** `useCanvasPointerRouter.ts` (1,068 lines — `runEntityPress`,
`runPageBodyPress`, `runResize`, `runEdgeDrag`, …).

**Problem:** The pure classification seam (`routePointerDown`) is deep and
unit-tested, but the bug-bearing logic — press-vs-drag threshold promotion,
phantom-blur guards, begin-before-patch focus ordering, commit-vs-cancel
outcomes — lives inside window listeners firing live IPC, exercisable only by
booting Electron. The most bug-prone decisions sit below the tested seam.

**Solution:** Extract a pure gesture-decision reducer (state + event →
promote/commit/ignore) so `run*` shells keep only IPC + listener wiring —
`pointer-session.ts` is the model. Unit tests cover mode transitions and
commit/cancel rules. Sequence after candidate 5 (same files).

---

## Appendix — add-a-kind edit-site map (cross-cut exploration)

18 edit sites, 1 registry-driven: `shared/types.ts` (kind literal + 3
unions), `json-canvas-serializer.ts` (serialize + deserialize arms),
`shared/json-canvas-types.ts` (node type), new `runtime/<kind>-entity-state.ts`
(~150-line copied template), `document-commands.ts` re-exports,
`canvas-layout-data.ts:346` spread arm, `sidebar-builder.ts` ×5,
`workspace-entities.ts` ×4, `shared/entity-ops.ts` `footprintForItem` arm,
✅ `entities/builtin/<kind>.ts` + `entities/index.ts` (the one registry
site), `useCanvasPointerRouter.ts` `resizeConfigForEntity` +
`patchDispatcherForKind` arms, `shared/hit-test.ts` predicate arms, new
`<Kind>BodyLayer.tsx` + App.tsx dispatch arm, new `<Kind>Popup.tsx` + App.tsx
JSX arm, `delete-selection.ts` bucket + switch + call, preload
`api.update<Kind>Entity` + IPC handler pair.
