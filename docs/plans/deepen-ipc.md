# Deepen the IPC/bridge surface — typed channel contract + broadcast seam

Track 2 of 3 from the deepen pass (`docs/audit/deepen-3673a35.md`, candidates
3–4). Self-contained: a fresh agent can build from this doc alone. Feature
branch off `main`; one PR per step; integration PR at the end.

## Goal

Give every IPC channel one typed contract (name + payload + direction) in
`src/shared/`, derive the preload bridges from it, and put all
broadcast-to-views fan-out behind one seam. Channel drift becomes a compile
error instead of a runtime silence.

## Ground truth (measured at 3673a35)

- 125 distinct `ipcMain.on/handle` channels in main vs 198 distinct
  `ipcRenderer.send` channels in preload — no shared constant anywhere.
- `'theme-changed'` restated as a raw literal in 11 files;
  `'canvas-update-text-entity'` in 3; `'layout-update'` in 4.
- `src/preload/canvas-bg.ts`: 117 `ipcRenderer.send` forwarders + 15 `on<>` +
  4 `invoke`, fronted by the ~230-line `CanvasBgElectronAPI` interface at
  `src/shared/types.ts:1619-1849`. 8 `*ElectronAPI` interfaces total live in
  `types.ts`, each consumed by exactly one preload bridge + its renderer.
- Broadcast fan-out is hand-rolled: `preferences.ts` `broadcastTheme()` names
  9 targets with per-call `isDestroyed()` guards; `window-init.ts:204-378`
  repeats the same 9 for init; `layout-engine.ts:240-572` sends 7 channels to
  3 named views inline; `view-refs.ts` holds 9 module-level let+setter pairs;
  `broadcastToDebugTargets` (`preferences.ts:268`) is a second, incompatible
  mini-fan-out.
- Duplicate channel namespaces: `preload/right-details-panel.ts` forwards some
  mutations to the same main channels as `canvas-bg.ts`
  (`canvas-update-text-entity`, …) but device/preset commands go through
  panel-specific channels (`right-details-panel-set-page-preset`) that main's
  `SINGLE_FIELD_COMMANDS` table (`register-right-details-panel-ipc.ts:71-137`)
  fans back to the *same* `document-commands` functions.

## Constraints

- `src/preload/` bridges IPC only — no business logic (CLAUDE.md layer rules).
- The recent `on<T>` helper (`src/preload/ipc-helpers.ts`) is the pattern to
  extend, not replace.
- The broadcast-payload type vocabulary in `src/shared/types.ts`
  (`LayoutUpdateData`, `CanvasScene*`, `Persisted*`, sidebar/presence/…) is
  cohesive and **stays together** — only the 8 `*ElectronAPI` interfaces move.
- Do not change channel semantics, payload shapes, or renderer behavior — this
  track is seam-consolidation only.

## Steps (one PR each)

### 1. Channel contract + first migration slice

Create `src/shared/ipc-contract.ts`: a typed map
`{ [channel]: { dir: 'renderer→main' | 'main→renderer', payload: T } }` (or
two maps, one per direction — pick whichever keeps inference simple). Add a
`send<C>` helper beside `on<T>` in `ipc-helpers.ts`, both keyed by the
contract. Migrate one vertical slice to prove the shape: `theme-changed`
(11 files) and `layout-update`, including main-side senders. Grep-verify the
migrated literals appear only in the contract.

### 2. Migrate the remaining bridges and registrars

All 18 preload files and 15 `register-*-ipc.ts` files read channel names from
the contract. The send-half of `canvas-bg.ts` becomes a loop/derivation over
the contract entries instead of 117 hand-written forwarders (keep the
`invoke` and subscribe methods explicit if derivation gets clever — boring
wins). Add a unit test or lint-grep check asserting no raw channel literals
remain outside the contract.

### 3. Relocate the 8 `*ElectronAPI` interfaces

Move each out of `shared/types.ts` next to its bridge (or derive from the
contract). Renderers keep importing a named type; the payload vocabulary in
`types.ts` is untouched.

### 4. Collapse the duplicate panel channels

Point `preload/right-details-panel.ts` device/preset commands at the existing
`canvas-*` channels; shrink `SINGLE_FIELD_COMMANDS` accordingly. Pure
pass-through deletion — both paths already converge on the same
`document-commands` functions.

### 5. Broadcast seam

`src/main/runtime/view-broadcast.ts` (or similar): a view registry of
`{ webContents, roles }` entries registered at view creation, behind
`broadcast(channel, payload, filter?)` that owns the `isDestroyed()` guard.
Collapse `broadcastTheme`, the `window-init.ts` init repeats,
`broadcastToDebugTargets`, and `layout-engine.ts`'s inline sends onto it;
`view-refs.ts`'s 9 globals fold into the registry (keep exported getters if
callers need direct refs).

## Verification

Every PR: `pnpm typecheck` + `pnpm test:unit`. This track is all
runtime/IPC surface — run `pnpm test:smoke` on every step. Manual check per
step: theme toggle reaches all views (incl. settings + debug windows), canvas
entity edits from the right details panel still apply, layout updates render.
