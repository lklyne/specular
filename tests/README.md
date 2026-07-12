# Tests

This suite is intentionally small. Coverage gaps are deliberate: see "What's intentionally uncovered" below.

Before adding a test, re-read **The bar** and confirm the test you're about to write earns its keep.

## The bar

A test earns its keep iff **all four** of the following hold:

1. **Catches a real regression.** You can name the production-code change that would break it. If you can't articulate the mutation, the test isn't protecting anything.
2. **Tests an observable outcome** — function output, Y.Doc state, file on disk, captured broadcast — not an internal mechanism. No `vi.mock('../runtime-context')`-style mocks of internal collaborators. Mock at process boundaries (the `electron` module, file system, child processes), never at module boundaries inside the same layer.
3. **Survives refactors.** If production code is reorganized but behavior preserved, the test passes. A test that breaks when you rename a private helper is testing the wrong thing.
4. **Locally legible.** Reading just the test file tells you what regression it protects. No "see the issue tracker for context"; the relevant assertion and setup are in front of you.

If a test fails any one of these, prefer to delete or rewrite it. A smaller suite that pulls its weight beats a larger suite of unknown value.

## Buckets

| Bucket | Lives in | Run with | Gated | Use when |
|---|---|---|---|---|
| Unit | `tests/unit/` | `pnpm test:unit` | CI, every PR | Pure logic — math, parsers, derivations, controllers driven through their public API. No Electron. |
| Integration | `tests/integration/` | `pnpm test:integration` | CI, every PR | The real main-process runtime, in-process: persistence, undo, sync, entity mutations, the `/canvas/apply` door. Plain Node — the `electron` module is aliased to an inert stub. Seconds, no display, no binary. |
| Boot | `tests/boot/` | `pnpm test:boot` | Pre-release | The ~3 checks that genuinely need a window: real Electron boots, serves the HTTP API, round-trips one mutation. Needs a built app (`.vite/build`) and the Electron binary. |
| Agent | `tests/agent/` | `bash tests/agent/run-scenarios.sh` | Out-of-band | Scripted UI scenarios driven by an agent. Not part of CI. |
| Fuzz | `tests/fuzz/` (when present) | `pnpm test:fuzz` | — | Property-based generation against parser-shaped surfaces (`.canvas` files). User-data surfaces only. |

Default to **unit** when the behavior is pure. Reach for **integration** when the regression involves the Y.Doc, persistence, undo, sync, or cross-module runtime behavior. **Boot** exists only to prove the Electron wiring assembles — do not add data-shaped tests there.

## Integration: the in-process harness

`tests/integration/harness.ts` boots the real runtime — workspace model, Y.Doc, undo manager, doc observers, autosave — in plain Node against a temp dir, mirroring the boot sequence in `src/main/index.ts`. `vitest.integration.config.ts` aliases `electron` to `tests/integration/electron-stub.ts`; the fake window reports `isDestroyed()` so the layout engine stays dormant, and every renderer-bound `webContents.send` is captured in `harness.broadcasts`.

Tests import the same exported mutators the IPC handlers and HTTP routes call (`document-commands`, `workspace-undo`, `applyCanvasPatch`, selection-controller, …) and assert on three production surfaces:

1. **Runtime arrays** — what the renderer would be shown
2. **The Y.Doc** (`harness.doc`) — what undo operates on
3. **`.canvas` bytes on disk** (`harness.diskDoc()`) — what survives a relaunch

File pattern (see `tests/integration/undo.test.ts` for the exemplar):

```ts
let harness: WorkspaceHarness
beforeEach(() => {
  harness ??= bootWorkspaceHarness()
  harness.reset()
})
afterAll(() => harness?.dispose())
```

One harness per file — runtime state is module-global in `src/main`, and vitest gives each test file its own process, which is the isolation model. Use `harness.reset()` / `harness.loadFixture()` between tests, and `await settleSync()` after mutations before asserting on the doc, the undo stack, or disk (the forward sync is scheduled on a microtask).

Pages work in-process: the stub fakes `WebContentsView`, so the `page` entity kind, `link`-node fixtures, and page undo/redo all run. Anything that depends on real view geometry, focus routing, or renderer behavior is out of scope for this tier — that's the boot suite's job, and it stays deliberately thin.

`tests/integration/canvas-format.test.ts` pins the serialized `.canvas` format with a golden snapshot (`__snapshots__/rich-workspace.canvas`). Format drift shows up as a reviewable git diff — if you changed the serializer intentionally, regenerate with `--update` and review the diff; if you didn't, it caught a regression.

## Mutation-verification

Integration and unit tests for runtime/persistence/undo/sync must be verified inline before merging: temporarily break the production code the test claims to protect, confirm the test fails, restore. Name the mutation in the test-file header and the commit message so a future reader can replay it.

Example commit message:

```
test: cover autosave debounce coalescing

Mutation-verified by commenting out the 350ms debounce in
scheduleWorkspaceAutosave() and confirming the test now sees two
file writes instead of one.
```

If you can't name a mutation that breaks the test, the test is testing nothing (see **The bar**, criterion 1).

## What's intentionally uncovered

These were considered and deliberately deferred — see issue [#81](https://github.com/lklyne/specular/issues/81) "Non-goals" and [ADR 0031](../docs/adr/0031-in-process-integration-testing.md):

- **Renderer E2E with Playwright+Electron** — high flake/maintenance tax. Reconsider once a UI regression slips past users.
- **View geometry / focus routing / overlay interactivity** — needs real views; covered indirectly by unit tests on the controllers (`focus-reconciler`, `interaction-controller`, `layout-*` math) and by dogfooding. The old Electron smoke tests for these passed without protecting anything (see #278).
- **Visual regression baselines** — UI churns frequently pre-1.0; baselines would be wrong constantly.
- **Performance budgets** — solving a problem we don't yet observe.
- **HTTP transport itself** — routes are thin over tested functions; the boot suite proves the server answers.
- **CI dashboards / flake quarantine policy** — overkill until more layers exist.

If a regression slips past users that one of the above would have caught, that's evidence to add it — file an issue with the specific regression as justification.

## When in doubt

Ask: "If this test passes tomorrow but my refactor today is wrong, would the test have caught it?" If no, you're testing the wrong thing.
