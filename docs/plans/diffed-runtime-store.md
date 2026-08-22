# Plan — diffed runtime store (end state C)

Status: ready for a fresh agent to build.
Author handoff: this is a from-scratch spec. Read it end to end before touching code. Read `src/main/runtime/CLAUDE.md` (two-layer state model) and `docs/interaction-layer.md` §6 (load-bearing invariants) first. They constrain everything below.

## Why this exists

Move the mouse over a selected page and the above-view renderer grows memory fast. I measured ~1.6MB of resident growth per synthesized move with the JS heap flat, and watched the above-view process reach 5.3GB and die with an out-of-memory kill (`render-process-gone:above-view`, exitCode 5, in `~/Library/Logs/Specular/errors.log`).

The cause is not a leak in the usual sense. It is the update granularity. Every runtime change, however small, takes the widest possible path:

- Change detection is three buckets. `markDirty` only knows `canvas`, `sidebar`, `toolbar` (`src/main/runtime/layout-dirty.ts`). Hover, scroll, one element reflow, and a structural edit all raise `canvas`. The system cannot tell "only hover moved" from "an entity was added."
- The scene is rebuilt whole every pass. `buildCanvasLayoutData` (`src/main/runtime/canvas-layout-data.ts`) returns a brand new `LayoutUpdateData` with new identity for every field, including `inspect: buildInspectPanelState()`, the full component tree of the selected page. That payload is ~330KB and 310KB of it is `inspect`.
- Renderers re-render the world. Each renderer drops the whole payload into `useState` (`above-view/App.tsx:507`, `canvas-bg/useCanvasLayoutState.ts:30`, `agent-layer/App.tsx:17`). Fresh identity busts every memo, so every layer re-renders, and after PRs 400/401 that means re-rastering several full-window canvases per broadcast.

So one hover pays for a full scene rebuild, a full serialize, a broadcast to four renderers, and a full re-render on each. The cost is fixed no matter how small the change.

The tell that this is systemic: the team has already carved three bespoke fast-path channels next to the coarse broadcast to escape it. `viewportNudge` for pan/zoom (`src/main/runtime/viewport-nudge.ts`), `pageScrollLive`, and `annotationLiveBbox` (see `src/shared/ipc-contract.ts`). Each is a private cheap path with its own reconcile logic. Hover is the next hot interaction and no one has carved its pipe, so it fell onto the slow road and crashed the renderer.

End state C makes fine-grained updates the default instead of the exception, and folds the three specials into one mechanism.

## What C is

Main keeps a normalized runtime store. A change touches one cell and emits a typed patch on one bus. Renderers hold a store of the same shape and each layer subscribes to the slice it depends on. A hover becomes one patch to one subscriber. Update cost scales with what changed, not with scene size.

Three properties fall out for free, which is the sign C is the real end state and A/B were waypoints:

- Structural sharing (end state A) is automatic. Unchanged slices keep their identity because nothing rebuilt them.
- Payload split by consumer (end state B) is automatic. A subscriber only receives the slice it subscribed to, so `inspect` never reaches renderers that do not read it.
- The three nudge channels become three patch types on one bus instead of three hand-written specials.

### The pattern to imitate

`viewportNudge` already is a patch, hand-built. Study it before writing anything:

- Producer: `broadcastViewportNudge()` in `src/main/runtime/viewport-nudge.ts` pushes `{pan, zoom}` on a dedicated channel, bypassing the layout pass.
- Consumer: `useSceneCameraTransform` in `src/renderer/shared/hooks/useScenePanOffset.ts` applies the delta locally and self-reconciles to identity when the next full `layoutUpdate` lands with a matching pan/zoom.

The whole refactor is: generalize this one hand-rolled case into a typed, keyed patch channel, and route the runtime mutators through it instead of through `markDirty('canvas') + requestLayout()`.

### The one deliberate exception

`viewportNudge` stays its own path even inside C. A pan is a camera transform over a scene that did not change at all, not a scene edit. It is a genuinely different kind of update and it earns a dedicated cheap channel. Do not try to model it as a store patch. One bus with one honest exception beats dogmatic unification.

## Current-state code map

Read these before starting. The refactor touches all of them.

Main, broadcast:
- `src/main/runtime/canvas-layout-data.ts` — `buildCanvasLayoutData()` builds the full `LayoutUpdateData`; `sendAnnotationLayoutUpdate()` (line ~180) fans it to `aboveView` and `cursorOverlayWindow`.
- `src/main/runtime/layout-engine.ts` — `layoutAllViews()` sends `layoutUpdate` to `bgView` (line ~595) then calls `sendAnnotationLayoutUpdate`; `requestLayout()` (line ~702) coalesces at 16ms.
- `src/main/runtime/layout-dirty.ts` — the three-bucket dirty flag set.
- `src/main/runtime/window-init.ts` — initial layout sends (lines ~211, ~258).

Main, runtime state owners (the mutators that will emit patches):
- `src/main/runtime/runtime-core.ts` — `setHoveredPage` (line ~235), `setHoverEntity`; both currently `markDirty('canvas') + requestLayout()`.
- `src/main/runtime/runtime-context.ts` — hover, zoom, pan, interaction state.
- `src/main/runtime/selection-controller.ts`, `selection-state.ts` — selection.
- `src/main/ipc/register-page-chrome-ipc.ts` — `pageScrollOffset` and `elementAttachmentPositions` handlers that `markDirty('canvas') + requestLayout()` per forwarded move.

Existing fast-path channels (the model, and the things to fold in):
- `src/main/runtime/viewport-nudge.ts` + `src/renderer/shared/hooks/useScenePanOffset.ts` — keep as the exception.
- `pageScrollLive`, `annotationLiveBbox` in `src/shared/ipc-contract.ts` — candidates to become patch types.

Renderers, consumers:
- `src/renderer/above-view/App.tsx` (line ~507) — the heavy one, the crash site.
- `src/renderer/canvas-bg/useCanvasLayoutState.ts` (line ~30).
- `src/renderer/agent-layer/App.tsx` (line ~17) — the only `layoutUpdate` consumer that reads `inspect` (`InspectPopoverLayer.tsx`).
- cursor-overlay window — receives via `sendAnnotationLayoutUpdate`, reads no `inspect`.

Types and contract:
- `src/shared/types.ts` — `LayoutUpdateData` (line ~557), `CanvasSceneEntity`, `CanvasHoverTarget`, `ViewportNudge`.
- `src/shared/ipc-contract.ts` — channel registry.

## Build sequence

Ship as tracer-bullet vertical slices, one PR per phase into a feature branch, then one integration PR. Each phase must stand on its own and leave the app working. Per-phase gate is `pnpm typecheck` + `pnpm test:unit` + `pnpm test:integration`; one manual smoke per branch before merge, not per commit. Do not over-split a phase because it is large; ~1000 LOC in one PR is fine. Block only on a real dependency gap.

Land phases 0 and 1 first regardless. They stop the crash and prove the mechanism on the worst offender before committing to the full store.

### Phase 0 — safety net (ship first)

Adopt end state A as a floor so main is never in a crashing state during the rest of the work.

- Add structural sharing to `buildCanvasLayoutData`: reuse the previous pass's object for any entity and any top-level field that did not change, so identity is stable across passes.
- Wrap the above-view layer components in `React.memo` at their boundaries so referentially-equal slices skip re-render.
- Verify against the repro: `POST localhost:29979/perf/alloc-profile {"durationMs":6000,"synthesizeMoves":true}` with a page selected. Resident growth should drop from hundreds of MB per run to single-digit MB. There is a temporary instrument on this branch, `src/renderer/above-view/ipc-tally.ts` wired in `above-view/App.tsx`, that logs per-channel counts and heap to `errors.log` every 2s. Use it, then delete it in the integration PR.

This phase is a floor, not the goal. It does not remove any main-side work. Keep going.

### Phase 1 — hover as the first patch (tracer bullet)

Prove the whole store idea end to end on one slice before generalizing.

- Add a typed patch channel to the contract, e.g. `runtimePatch`, carrying `{ kind, ...payload }`.
- On the main side, change `setHoveredPage` / `setHoverEntity` (`runtime-core.ts`) to emit a `hover` patch instead of `markDirty('canvas') + requestLayout()`. The hover target stops riding the full layout pass.
- On the renderer side, add a small hover store that only `SelectionOutlineLayer` (and any hover-driven chrome) subscribes to. It applies the patch and self-reconciles when a full `layoutUpdate` with a matching hover lands, exactly like `useSceneCameraTransform` does for pan.
- Do the same for the forwarded-move callbacks in `register-page-chrome-ipc.ts` that currently relayout on `pageScrollOffset` and `elementAttachmentPositions`, if they turn out to fire on hover moves. Confirm with the requestLayout cause counter (see Instrumentation and acceptance).

Exit criteria: with a page selected, moving the mouse produces zero full `layoutUpdate` broadcasts from hover alone (watch `ipc-tally`), and the hover outline still tracks correctly. Resident growth under `synthesizeMoves` is flat.

### Phase 2 — the normalized store and patch bus

Generalize phase 1's one-off into the real mechanism.

- Define the store shape in `src/shared/`: a normalized map keyed by entity id plus the small top-level slices (camera, selection, hover, interaction, tool, inspect, presence). This is the diffed form of `LayoutUpdateData`.
- Main side: a patch producer that diffs the new store against the last and emits per-slice patches. Route it from the layout pass so a structural edit still works, but now emits patches, not a monolithic rebuild-and-send.
- Renderer side: a store that applies patches, plus a `useSlice(selector)` subscription so each layer re-renders only when its slice changes. Model the subscription on a standard selector store; do not invent bespoke plumbing per layer.
- Keep `layoutUpdate` alive as the full-snapshot channel used on connect and as the periodic reconcile baseline. Patches ride on top. This is what makes the migration safe: a dropped or mis-applied patch self-heals on the next snapshot.

### Phase 3 — fold in the specials, route each renderer to its slices

- Convert `pageScrollLive` and `annotationLiveBbox` into patch types on the bus. Delete their bespoke channels once nothing reads them.
- Route `inspect` only to agent-layer (end state B falls out here). Canvas-bg, above-view, and cursor-overlay stop receiving it entirely.
- Leave `viewportNudge` as its own channel. Document why in the ADR.

### Phase 4 — cleanup and docs

- Delete `ipc-tally.ts` and its wiring, the `alloc-profile.ts` / `perf.ts` debug additions on this branch (confirm with the user first), and any dead `markDirty('canvas')` calls that patches replaced.
- Write ADR 0036 (see below).
- Update `CONTEXT.md` and `src/main/runtime/CLAUDE.md` to describe the store and the patch bus as the new default, with `viewportNudge` named as the one exception.

## Test contract

Per `CLAUDE.md` and `tests/README.md`, before writing any test re-read `tests/README.md` and clear the four-criterion bar.

- Every new runtime mutator that emits a patch ships with integration coverage: one patch per mutation, and a full `layoutUpdate` snapshot still reconciles a renderer store that has drifted. Use `bootWorkspaceHarness()` in `tests/integration/harness.ts`.
- Patch-then-snapshot convergence is the load-bearing property. Add an integration test that applies a sequence of patches, then a snapshot, and asserts the store matches a fresh `buildCanvasLayoutData`. This is the safety property that lets patches be lossy.
- Unit-test the diff producer (given old store + new store, emits the minimal correct patch set) and the reducer (given store + patch, produces the expected store).
- PRs touching `src/main/runtime/space-*.ts` require integration coverage updates.

## Invariants and risks

- `docs/interaction-layer.md` §6 invariants are load-bearing. The expected-focus model, the hit-test priority, and the page-cursor bridge must keep working. Patches change how state reaches renderers, not who owns it. Main still owns truth.
- The danger lives in the transition, not the destination. A half-built store with a leaky subscription gives stale UI and missed updates, which are nastier than "too many updates." The full-snapshot reconcile baseline (kept in phase 2) is the mitigation. Never remove it.
- Do not rip out `viewportNudge`. It is the exception, not a victim.
- Reverse sync (Y.Doc → runtime on undo/redo) still drives the layout pass. Confirm undo/redo still repaints correctly after each phase.
- Presence cursors and agent-driven input ride the same broadcast today. Check `presenceCursors` and the agent-layer after phase 3.

## What done looks like

With a page selected, moving the mouse emits per-slice patches, not full `layoutUpdate` broadcasts. Above-view re-renders only the outline layer on hover. Resident memory under a 60-second synthesized-move run stays flat. `inspect` reaches only agent-layer. `pageScrollLive` and `annotationLiveBbox` are gone, folded into the bus. `viewportNudge` remains. ADR 0036 records the decision and the exception.

## Instrumentation and acceptance

The two instruments already on this branch prove the memory axis, which is the primary success signal, but they are not enough on their own. They see the output (broadcasts arriving at above-view) and the symptom (resident growth), not the cause or the correctness. Keep both, add three, and read each phase's acceptance off them.

Keep:

- `POST localhost:29979/perf/alloc-profile {"durationMs":60000,"synthesizeMoves":true}` (`src/main/alloc-profile.ts`) with a page selected. It drives synthetic moves over CDP and reports `rssKb` before/after with the JS-heap split. This is the standard repro and the acceptance gate for every phase. Flat resident growth is the pass condition.
- `src/renderer/above-view/ipc-tally.ts` (wired in `above-view/App.tsx`). Per-channel arrival count, bytes, and heap every 2s to `errors.log`. It auto-wraps any new `on*` subscription, so a new `onRuntimePatch` shows up without extra wiring.

Add:

1. requestLayout cause counter (required for phase 1, was going to be deferred). In `layout-engine.ts`, sample `new Error().stack`'s top frame inside `requestLayout()` and log a per-cause histogram every 2s. This is how you prove a slice is migrated: watch `hover` fall off the histogram in phase 1, `scroll` and `element-positions` in phase 3. You need this and not just `ipc-tally`, because phase 0's structural sharing makes a broadcast cheap while it still fires. Broadcast count alone would still show `layoutUpdate` on every hover and wrongly read as failure. The cause counter shows whether the pass happened at all.

2. Reconcile-drift watchdog (the important one for C, required for phases 2 and 3). On the renderer side, when a full `layoutUpdate` snapshot lands, diff the patch-accumulated store against it and log a counter of mismatched slices. This turns the patch-then-snapshot convergence property from a test-time claim into a live watchdog. C's real danger is a leaky subscription or a lossy patch producing stale UI silently, and this is the only thing that catches it during dogfooding. Ship it behind a dev flag; it can stay in the tree longer than the others. Zero drift over a session is a release gate.

3. Per-renderer wire accounting (required for phase 3). Today's `ipc-tally` is above-view only. Phase 3's claim, "inspect reaches only agent-layer, each renderer gets only its slice," needs visibility into all four renderers. Either lift `ipc-tally` into a shared hook mounted in each renderer, or add main-side accounting of bytes per channel per target every 2s. Then total wire bytes at rest is one number you watch fall across phases, and inspect vanishing from three of the four renderers is directly observable.

One correctness property the runtime instruments do not cover: that the hover outline still visually tracks the pointer after phase 1. Confirm it with a scripted test, drive `synthesizeMoves` and assert `SelectionOutlineLayer`'s rendered position follows, as an integration or agent test rather than a bespoke runtime probe.

Per-phase acceptance, read off the above:

- Phase 0: alloc-profile resident growth drops from hundreds of MB per run to single-digit MB.
- Phase 1: cause counter shows zero `hover` entries; alloc-profile flat under synthesized moves; the outline-tracks test is green.
- Phase 2: drift watchdog reads zero over a dogfooding session; the convergence integration test is green.
- Phase 3: wire accounting shows `inspect` only on agent-layer and total bytes-at-rest down sharply; `pageScrollLive` and `annotationLiveBbox` are gone.
- Phase 4: remove all added instruments and the branch's temporary ones, except any single readout you decide to keep as a permanent perf HUD line.

## Context this plan came from

Diagnosis and the three end-state explainers (A structural sharing, B split payload, C this) live in the vault at `~/Documents/specular/refactor-{a,b,c}-*.html`, on the `focus markdown` canvas. C won on end simplicity and speed because update cost stops scaling with scene size and three special channels collapse into one. A is the smallest change and the worst architecture (its correctness hides in unenforced referential-equality invariants). B is a real battery win but half an answer alone. This plan builds C while landing A first as a safety floor.
