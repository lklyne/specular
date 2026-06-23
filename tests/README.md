# Tests

This suite is intentionally small. Coverage gaps are deliberate: see "What's intentionally uncovered" below.

Before adding a test, re-read **The bar** and confirm the test you're about to write earns its keep.

## The bar

A test earns its keep iff **all four** of the following hold:

1. **Catches a real regression.** You can name the production-code change that would break it. If you can't articulate the mutation, the test isn't protecting anything.
2. **Tests an observable outcome** — function output, IPC broadcast, file on disk, snapshot field — not an internal mechanism. No `vi.mock('../runtime-context')`-style mocks of internal collaborators. Mock at process boundaries (HTTP, file system, child processes), never at module boundaries inside the same layer.
3. **Survives refactors.** If production code is reorganized but behavior preserved, the test passes. A test that breaks when you rename a private helper is testing the wrong thing.
4. **Locally legible.** Reading just the test file tells you what regression it protects. No "see the issue tracker for context"; the relevant assertion and setup are in front of you.

If a test fails any one of these, prefer to delete or rewrite it. A smaller suite that pulls its weight beats a larger suite of unknown value.

## Buckets

| Bucket | Lives in | Run with | Use when |
|---|---|---|---|
| Unit | `tests/unit/` | `pnpm test:unit` | Pure logic — math, parsers, derivations, controllers driven through their public API. No Electron. |
| Smoke | `tests/smoke/` | `pnpm test:smoke` | End-to-end behavior of the running app over its real surfaces (HTTP API, file system, IPC). Spawns one Electron via `tests/smoke/global-setup.ts` and runs serially. |
| Agent | `tests/agent/` | `bash tests/agent/run-scenarios.sh` | Scripted UI scenarios driven by an agent. Out-of-band — not part of CI. |
| Fuzz | `tests/fuzz/` (when present) | `pnpm test:fuzz` | Property-based generation against parser-shaped surfaces (`.canvas` files). User-data surfaces only. |

Default to **unit** when the behavior is pure. Reach for **smoke** when the regression involves Electron, IPC, persistence, undo, or the file system. Use **agent** only for scenarios that can't be expressed through the HTTP API.

## Smoke: the `AppClient` toolkit

`tests/smoke/app-client.ts` exposes a thin HTTP wrapper around the running app. Treat its functions as the public surface of the smoke suite — prefer them over hand-rolled `fetch` calls.

Common entry points:

- **Workspace state** — `getWorkspace`, `getSidebar`, `getSelection`, `getSelectionOverlayState`
- **Page CRUD** — `createPages`, `createFocusedPage`, `deletePages` (create/delete route through `applyCanvas`)
- **Text entities** — `createTextEntities`, `updateTextEntities`, `deleteTextEntities`, `getTextEntities`
- **Selection** — `selectPage`, `selectEntity`, `selectEntities`, `selectGroup`, `enterGroup`, `deselectSelection`
- **Grouping/edges** — `createGroup`, `ungroup`, `deleteGroups`
- **Visual capture** — `takeScreenshot`, `takeSnapshot`, `takeAgentSnapshot`
- **Interaction** — `tryEnter`, `cancelInteraction`, `requestPointerLock`
- **Reset** — `resetSmokeState` (only in tests that need a known-empty workspace)

When the surface you need isn't there, add the helper to `app-client.ts`. Don't shell out to `fetch` from a test file — keeping all HTTP calls in one place is what makes the suite refactor-survivable.

Two utilities for polling/timing live in `tests/smoke/test-utils.ts`:

- `wait(ms)` — sleep. Use sparingly; prefer `waitFor` when you're waiting on a state condition.
- `waitFor(factory, predicate, message, opts?)` — poll a factory function until a predicate matches. Default is 20 attempts at 100ms intervals.

## Mutation-verification

Smoke and unit tests for runtime/persistence/undo/sync must be verified inline before merging: temporarily break the production code the test claims to protect, confirm the test fails, restore. Name the mutation in the commit message so a future reader can replay it.

Example commit message:

```
test: cover autosave debounce coalescing

Mutation-verified by commenting out the 350ms debounce in
scheduleWorkspaceAutosave() and confirming the test now sees two
file writes instead of one.
```

If you can't name a mutation that breaks the test, the test is testing nothing (see **The bar**, criterion 1).

## What's intentionally uncovered

These were considered and deliberately deferred — see issue [#81](https://github.com/lklyne/specular/issues/81) "Non-goals" for the full reasoning:

- **Renderer E2E with Playwright+Electron** — high flake/maintenance tax. Reconsider once a UI regression slips past users.
- **Visual regression baselines** — UI churns frequently pre-1.0; baselines would be wrong constantly.
- **Performance budgets** — solving a problem we don't yet observe.
- **HTTP API and Y.Doc-merge fuzz** — parser fuzz covers the user-data risk; the others are low-leverage.
- **Full-suite backfilled mutation verification** — process burden disproportionate. New tests in covered layers are mutation-verified inline; old tests are not retroactively audited.
- **CI dashboards / flake quarantine policy** — overkill until more layers exist.

If a regression slips past users that one of the above would have caught, that's evidence to add it — file an issue with the specific regression as justification.

## When in doubt

Ask: "If this test passes tomorrow but my refactor today is wrong, would the test have caught it?" If no, you're testing the wrong thing.
