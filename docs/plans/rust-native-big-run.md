# Rust native: the big run

**Status:** In progress on `claude/project-thread-hqnuuf`, one draft PR into `rust-native`.
**Starts from:** `rust-native` at 0.10.2 (`441dbdbc`).
**Approved:** by Lyle on 2026-10-25, every recommended decision below.
**See also:** [`rust-native-rebuild.md`](./rust-native-rebuild.md) (the architecture and cleanup tasks this run finishes), the [handoff](./rust-native-rebuild-handoff.md), and the [checklist](./rust-native-big-run-checklist.md) to walk once at the end.

The Rust app is the product from here on. This run removes what only served the comparison with Electron, makes scenarios the main kind of test, leaves one window, ports the CLI, closes the parity gaps that matter, deletes the Electron app, and tidies what is left.

## How the run works

- One branch off `rust-native`, one draft PR. One or more commits a phase, pushed as they land.
- Phases that touch different files run at the same time, each agent in its own git worktree, merged into the run branch. Phases 3, 4 and 8 share files and run in order.
- The gate holds after every merge, from `native/`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and the headless scenarios.
- Nothing under `src/` changes before phase 7.
- At the end, `native/scripts/validate-all.sh` runs every check in one pass, and the checklist covers what only a person can judge.

## The decisions

1. **Delete the render bake-off.** `native/bakeoff` answered its question (ADR 0039). The images ADRs 0039 and 0040 cite move under `docs/adr/` first, with the handoff's link to them.
2. **Delete `packages/vite`.** Nothing in the Rust app builds with it.
3. **Retire the Electron comparison half of `specular-bench`** (cleanup task 4): `compare`, `electron_trace`, `profile` and their commands. The bench keeps profiles, frame stats and memory for the Rust app alone.
4. **One scenario script format** for the headless runner and the Kit window (`SPECULAR_SHELL_SCRIPT_FILE`). Steps only a window can run are skipped headless, and the run says which.
5. **Scenarios assert on state, not pixels.** Each one checks the state dump and the saved canvas. Screenshots are kept for a few draw rules. Every scenario that changes the document ends with an undo-returns-to-start check.
6. **CI runs the headless scenarios on every push.** It tries a hosted macOS runner for the Kit window. If that runner cannot post events or draw, the reason is written down and window runs stay local or nightly.
7. **Synthetic pages by default.** A separate CEF suite covers page input, scroll, sync and the CLI's browse verbs.
8. **Gesture tests become scenarios.** `specular-interact/tests/it` turns into scenarios grouped by user flow. Unit and per-function tests go. About 150 tests stay: `.canvas` byte and round-trip tests, API contract tests until the CLI replaces them, a few pure-math tests (resize, markdown, text segmentation), a few GPU colour readbacks, and performance pins. `specular-testkit` shrinks to what those need.
9. **One window.** The winit window goes (cleanup task 2), with the built-in toolbar and sidebar (task 1a). Headless runs press controls by name with no panels drawn. `winit` and `muda` are dropped.
10. **Headless is a window like any other.** It becomes a `Runtime<HeadlessWindow>`, so each `Effect` has one runner.
11. **The Kit draws every popup.** The item popup, the context menu and the page popup are redrawn in the Kit, and the rest of `interact/panel/builtin` is deleted (task 1b).
12. **One shell crate.** `specular-app` and `specular-shell` merge (task 3), with one rect type (task 7), one sidebar pick, one polling file watcher, one folder-dialog path and one `specular-cli.sh`.
13. **The CLI is Rust.** The `specular` CLI is ported to Rust on `specular-api`'s types and ships in the bundle in place of the TypeScript one. `print-pdf`, `record`, `component-states` and `design-system` are cut from the CLI, the skill and the API. CLI end-to-end runs replace the API contract tests. `native/helper` keeps only the Agent SDK under Bun and imports nothing from `src/`.
14. **The Electron app is deleted.** `src/`, the TypeScript `tests/`, the Electron build configs and Electron CI go. `resources/` (the starter space, the skill) and what `native/helper` needs stay. The docs, README, `CLAUDE.md` and `CONTEXT.md` describe the Rust app only.

ADRs 0039 to 0044 and 0047 are Accepted with this run.

## Phases

### Phase 0. Docs

Refresh `rust-native-rebuild.md` and the handoff to match the code: themes, signing, Sparkle, element attachment, svg and gif, and HTML file nodes are done. Write the decisions above into the plan. Mark ADRs 0039 to 0044 and 0047 Accepted. Save this plan.

### Phase 1. Deletions

Delete `native/bakeoff` (after moving the images ADRs 0039 and 0040 cite, and the handoff link, under `docs/adr/`), `packages/vite`, and `specular-bench`'s Electron comparison half (`compare`, `electron_trace`, `profile` and their commands).

### Phase 2. One scenario format

- One script format, run by the headless runner and by the Kit window through `SPECULAR_SHELL_SCRIPT_FILE`. Window-only steps are skipped headless.
- Scenarios assert on the state dump and the saved canvas. Screenshots only for a few draw rules.
- Every scenario that changes the document ends with an undo-returns-to-start check.
- CI runs the headless scenarios on every push, and tries a hosted macOS runner for the Kit window. If it cannot post events or draw, record why and keep window runs local or nightly.
- Synthetic pages by default. A separate CEF suite for page input, scroll, sync and the CLI's browse verbs.

### Phase 3. Tests become scenarios

Convert `specular-interact/tests/it` gesture tests into scenarios grouped by user flow. Delete unit and per-function tests. Keep about 150: `.canvas` byte and round-trip tests, API contract tests (until phase 5), a few pure-math tests (resize, markdown, text segmentation), a few GPU colour readbacks, and performance pins. Shrink `specular-testkit` to what remains.

### Phase 4. One window

- Retire the winit window (cleanup task 2) with the built-in toolbar and sidebar (task 1a). Headless presses controls by name with no panels drawn. Drop `winit` and `muda`.
- Make headless a `Runtime<HeadlessWindow>` so each `Effect` is run once.
- Redraw the item popup, the context menu and the page popup in the Kit, and delete the rest of `interact/panel/builtin` (task 1b).
- Merge `specular-app` and `specular-shell` (task 3). One rect type (task 7), one sidebar pick, one polling file watcher, one folder-dialog path, one `specular-cli.sh`.

### Phase 5. The CLI in Rust

Port the `specular` CLI to Rust, sharing `specular-api`'s types, and ship it in the bundle in place of the TypeScript one. Cut `print-pdf`, `record`, `component-states` and `design-system` from the CLI, the skill and the API. Replace the API contract tests with CLI end-to-end runs. `native/helper` keeps only the Agent SDK under Bun and imports nothing from `src/`.

### Phase 6. Parity

- New-tab links (`target=_blank`, `window.open`, Cmd-click, middle click) open a page beside the source, as Electron does.
- Errors and crashes are logged to `~/Library/Logs/Specular/errors.log`.
- Reduced motion is honoured, including the Prism avatar and the run bar.
- Settings to hide agent cursors and synced cursors.
- `/annotations/fix` and the page snapshot routes.
- A scenario for each.

### Phase 7. Delete the Electron app

Delete `src/`, the TypeScript `tests/`, the Electron build configs and Electron CI. Keep `resources/` (starter space, skill) and what `native/helper` needs. Update the docs, README, `CLAUDE.md` and `CONTEXT.md` to describe the Rust app only.

### Phase 8. Tidy

Narrow `pub` to `pub(crate)` (task 8) and delete what turns up dead. Split the files over 400 lines and the longest functions (tasks 11 and 12).

## Validation

`native/scripts/validate-all.sh` runs, in order:

1. fmt, clippy and tests
2. every headless scenario
3. every Kit window scenario
4. the CEF real-page suite, on a bundled build
5. the CLI end-to-end runs against the running app
6. a release bundle build and first-run check

It prints one pass or fail table and writes screenshots and logs to `native/runs/validate/`.

## At the end

The by-hand checks only a person can do (trackpad, IME, live resize, a second display, VoiceOver, the redrawn popups, new-tab links) are in [`rust-native-big-run-checklist.md`](./rust-native-big-run-checklist.md).
