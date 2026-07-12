# ADR 0031 — In-process integration suite replaces the Electron smoke suite

**Status:** Accepted
**Date:** 2026-07-02
**Related:** issue [#278](https://github.com/lklyne/specular/issues/278) (the audit that motivated this), issue [#81](https://github.com/lklyne/specular/issues/81) (the test standard the smoke suite was built under), `tests/README.md` (the bar all suites must clear).

## Context

The Electron-spawning smoke suite (`tests/smoke/`, 24 test files, ~169 tests, ~34s) was the designated coverage for the highest-risk layer — persistence, undo, forward/reverse sync. An audit of ~11 weeks of session history (#278) found it produced roughly zero clean regression catches. Three structural reasons:

1. **It tested a facade.** `src/main/routes/test.ts` (589 lines) was a test-only HTTP entry point wired in parallel to the real IPC handlers. A test could stay green while the shipping path was broken (commit `ef012e7a` was exactly this).
2. **It was gated nowhere.** Spawning Electron made it too slow/fragile for CI, so it ran only when someone remembered — and cloud agent sessions couldn't run it at all (no display, no Electron binary). A suite that only runs ad hoc and passes ~100% of the time is ritual, not protection.
3. **The logic under test never needed Electron.** The truth layer (workspace model, Y.Doc, undo, persistence) is Electron-clean; every Electron call in `src/main/runtime/` sits behind function bodies (`app.getPath`, `screen`, `new WebContentsView`) reached lazily, and renderer-bound sends funnel through `safe-send.ts`.

## Decision

Three tiers, each gated where it is cheap enough to gate:

1. **Unit** (`tests/unit/`, in CI) — pure logic. Unchanged.
2. **Integration** (`tests/integration/`, in CI) — the real main-process runtime booted **in-process** in plain Node. `vitest.integration.config.ts` aliases `electron` to an inert stub (`tests/integration/electron-stub.ts`); `tests/integration/harness.ts` mirrors the boot sequence in `src/main/index.ts` (Y.Doc, undo manager, doc observers, autosave) against a temp dir. Tests call the same exported mutators the IPC handlers and HTTP routes call, and assert on the three production surfaces: runtime arrays, the Y.Doc, and `.canvas` bytes on disk. Renderer-bound broadcasts are captured at the `webContents.send` seam.
3. **Boot** (`tests/boot/`, pre-release only) — the one thing that genuinely needs a window: spawn real Electron, health-check, and round-trip one mutation through the production HTTP door.

Supporting moves:

- The `/canvas/apply` patch semantics moved from the route closure into `src/main/canvas-apply.ts` (`applyCanvasPatch`) so the HTTP route and the integration suite execute the identical function — tested path == shipping path.
- `src/main/routes/test.ts` and the `/test/reset-state` route are deleted. No test-only wiring ships in the app.
- A golden-file test (`tests/integration/canvas-format.test.ts`) snapshots a rich workspace's serialized `.canvas` bytes so format drift surfaces as a reviewable git diff.

## Consequences

- CI now gates the invisible-corruption failure class (silent `.canvas` loss, broken undo, sync echo) on every PR, in seconds, with no display or Electron binary — including cloud agent sessions.
- "After runtime/IPC/persistence changes, run `test:smoke`" becomes "run `test:integration`" — from ~34s + a built app to ~2s from a cold checkout.
- The Electron stub is deliberately shallow (no-op Proxy fallbacks; the fake window reports `isDestroyed()` so the layout engine stays dormant). Anything that depends on real view geometry, focus routing, or renderer behavior is **out of scope** for the integration tier — that remains the boot suite's job (and stays deliberately thin per the non-goals in `tests/README.md`).
- Interaction-machine behavior (gestures, focus, drop ownership) is covered at the unit tier against the controllers' public APIs; the smoke variants duplicated that coverage through HTTP and were dropped rather than ported.
