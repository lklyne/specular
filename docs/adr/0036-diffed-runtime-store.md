# ADR 0036 — A diffed runtime store replaces the whole-scene broadcast

**Status:** Accepted
**Date:** 2026-08-22
**Related:** [docs/plans/diffed-runtime-store.md](../plans/diffed-runtime-store.md) (the build plan and its three end-state candidates), [ADR 0023 — renderer-owned camera](0023-renderer-owned-camera-gpu-panzoom.md) (**Rejected**; why the camera stays main-owned, which is why `viewportNudge` is a delta and not an ownership transfer)

## Context

Move the pointer over a selected page and the above-view renderer grew about
1.6MB of resident memory per synthesized move with the JS heap flat. Left
running it reached 5.3GB and the process died — `render-process-gone:above-view`,
exitCode 5, in `~/Library/Logs/Specular/errors.log`.

Nothing leaked in the usual sense. The cost of an update was fixed, and the
fixed amount was "the entire scene," so a hover paid what a structural edit
paid:

- **Change detection had three buckets.** `markDirty` knew `canvas`, `sidebar`,
  and `toolbar`. Hover, a scroll frame, one element reflowing, and adding an
  entity all raised `canvas`. Nothing downstream could tell "only hover moved"
  from "the scene changed."
- **The scene was rebuilt whole every pass.** `buildCanvasLayoutData` returned a
  `LayoutUpdateData` with fresh identity for every field, including `inspect` —
  the selected page's entire component tree. The payload measured ~330KB, of
  which ~310KB was `inspect`.
- **Renderers re-rendered the world.** Each dropped the payload into `useState`.
  Fresh identity busts every memo, so every layer repainted, and after the
  drag-freeze and page-ring work that meant re-rastering several full-window
  canvases per broadcast.

The tell that this was structural rather than a bad hover implementation: three
bespoke fast paths had already been carved beside the coarse broadcast to
escape it — `viewportNudge` for pan/zoom, `pageScrollLive` for scroll-following
overlays, `annotationLiveBbox` for element-anchored popovers. Each was a private
cheap channel with its own reconcile logic. Hover was the next hot interaction,
nobody had carved its pipe, so it took the slow road and killed the renderer.

Carving a fourth channel would have worked and would have been wrong. The
recurring need was the design pressure: fine-grained updates were the exception
and had to be earned per interaction, when they should be what the system does
by default.

## Decision

### 1. Main keeps a normalized runtime store

`src/shared/runtime-store.ts` splits `LayoutUpdateData` along the two axes that
change independently: a map of scene entities keyed by id, and about fifteen
small **slices** for everything that is not an entity (`camera`, `chrome`,
`scene`, `selection`, `tool`, `focus`, `hover`, `interaction`, `inspect`,
`annotations`, `edges`, `fixProgress`, `presence`, `pageScroll`,
`annotationBboxes`).

The projections are lossless both ways — `snapshotToStore` and
`storeToLayoutData` round-trip — because the flat snapshot has to stay a real
description of the store for §3 to work.

### 2. One keyed patch bus, addressed two ways

A change emits a **patch** on the `runtimePatch` channel: either
`{ kind: 'slice', slice, value }` replacing one slice wholesale, or
`{ kind: 'entity', id, entity }` replacing (or, with `entity: null`, removing)
one entry of the entity map. A layout pass emits all of its patches as one
**patch batch**, so they cross the wire together and a renderer never paints a
half-applied pass.

Two producers feed it:

- **Mutators that skip the layout pass.** `commitHoverTarget`, the page-scroll
  handler, and the annotation-bbox fold call `broadcastRuntimePatch` directly.
  A hover is one patch to the layers that draw hover chrome.
- **The layout pass.** `broadcastSceneUpdate` diffs the rebuilt scene against
  the baseline (`diffRuntimeStores`) and sends the cells that moved. A pass that
  changed nothing sends nothing.

Renderers hold a store of the same shape and subscribe through `useSlice`, so a
layer re-renders when the value it selected changes rather than when the store
does.

`pageScrollLive` and `annotationLiveBbox` are slices on this bus. Their bespoke
channels are gone.

### 3. The snapshot baseline is load-bearing and must never be removed

`layoutUpdate` stays: it is what a renderer receives on connect, and what it
receives again whenever a pass arrives more than one second after the last
snapshot. Patches ride on top.

This is the entire reason patches are allowed to be lossy. A dropped, dropped-in-
transit, or mis-applied patch converges on the next snapshot instead of leaving
stale chrome on screen forever. Every value a patch can carry is also carried by
the snapshot, and `broadcastSceneSnapshot` re-seats the baseline for all targets
at once so main's model of what each renderer holds cannot skew.

Removing the snapshot cadence would turn every producer bug into permanent,
silent visual corruption. It is not an optimization opportunity. If the cadence
ever needs to change, raise the interval; do not delete the mechanism.

### 4. Each renderer is routed only the slices it reads

`src/shared/runtime-store-filter.ts` names, per target, which slices it draws
from. `inspect` reaches only `agent-layer`; `canvas-bg` takes neither hover nor
gesture nor annotation geometry; `above-view` takes everything except `inspect`
and `presence`.

Filtering happens on the way out, and the baseline stays the full store, so
routing is a wire concern that never leaks into main's model of the scene. A
target simply has no key for a slice it is not routed — absence, not a neutral
value — so a slice that starts being read surfaces as missing rather than
reading as empty forever. The bootstrap handler applies the same filter, so a
renderer never starts out holding a slice it will never be sent an update for.

### 5. Structural sharing on both sides of IPC is the identity floor

`shareStructure` (`src/shared/layout-structural-share.ts`) reuses the previous
value whenever the two are deep-equal, and it runs in three places: on the
scene main builds, on the snapshot a renderer applies, and on the flat
projection a renderer reads back.

Identity is the product here, not a nicety. `useSlice` compares by reference and
the memoized layers bail out on reference equality, so a snapshot that repeats
what a renderer already holds must hand back the same objects or it undoes the
patch bus's whole benefit once a second. Sharing the same helper for the diff's
equality test keeps one deep-comparison rule in the codebase rather than two
that can drift apart.

### 6. `viewportNudge` remains its own channel

A pan or a zoom is a camera transform over a scene that did not change at all.
Nothing in the store's entity map or its slices is edited by the gesture; what
changes is where the same scene is viewed from, at pointer rate, with the
renderer applying the delta locally and self-reconciling to identity when the
next snapshot lands with a matching pan/zoom
(`useSceneCameraTransform`). Modeling that as a store patch would dress a
different kind of update in scene-edit clothing and buy nothing — the camera
slice still exists and still rides snapshots, so the reconcile baseline covers
it either way.

It is a deliberate exception, not an unfinished migration. One bus with one
honest exception beats a dogmatic unification. The three-channel proliferation
that motivated this ADR was a symptom of *scene edits* having no cheap path; the
camera is not a scene edit.

### 7. The drift watchdog is the live guard

The failure mode this design introduces is not "too many updates," which is loud, but a
leaky subscription or a lossy producer leaving stale UI, which is silent. So
every snapshot a renderer applies is diffed against what its patch stream
accumulated, and disagreements are counted and reported
(`src/renderer/shared/runtime-store-drift.ts`).

It is dev-flag gated because the comparison walks the whole store — exactly the
O(scene) work the bus exists to avoid. Zero drift over a dogfooding session is
the release gate for changes in this area, and the watchdog is expected to
outlive the temporary instruments that measured the refactor.

## Consequences

- Update cost scales with what changed rather than with scene size. A hover is
  one slice patch to one renderer; a page scroll frame is one slice patch; a
  structural edit is the entities that moved.
- `inspect` — the single heaviest thing on the wire — is paid for once, by the
  one renderer that draws it.
- A hot new interaction does not have to argue for its own channel. It emits a
  patch like everything else.
- There are two ways for a renderer to be wrong about the scene (a missed patch,
  a mis-routed slice) where before there was one. §3 bounds how long either can
  live to about a second, and §7 makes it observable rather than a bug report
  about "the outline was in the wrong place."
- `LayoutUpdateData` survives as the snapshot shape and as the projection
  consumers not yet split into slices read. It is the migration seam, not a
  legacy artifact: the round-trip is what lets a consumer move to `useSlice` one
  layer at a time.
- Main holds a second copy of the scene (the baseline) so it has something to
  diff against. It is one store, replaced per pass, not a history.

## Alternatives rejected

- **Structural sharing alone (end state A).** Landed first, deliberately, as a
  floor so main was never in a crashing state during the rest of the work — it
  cut resident growth from hundreds of MB per run to single digits. Rejected as
  the destination because it removes no main-side work at all (the scene is
  still rebuilt and re-serialized per pass, for every renderer) and its
  correctness lives in an unenforced referential-equality invariant that any
  future `{...spread}` quietly breaks.
- **Split the payload by consumer (end state B).** A real win, and it falls out
  of the store for free (§4) rather than needing its own mechanism. Alone it
  leaves every renderer still re-rendering its world on every pass.
- **A fourth bespoke channel for hover.** The cheapest change available, and the
  one that guarantees a fifth. Each such channel carries its own reconcile
  logic, which is the part that is easy to get subtly wrong.
- **Deleting `layoutUpdate` once patches work.** See §3.
