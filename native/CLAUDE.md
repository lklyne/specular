# native/ — the Rust rebuild

The Rust version of Specular. The plan, the architecture and the task list
are in [`docs/plans/rust-native-rebuild.md`](../docs/plans/rust-native-rebuild.md).
Read its "Greenfield architecture" section before changing anything here.
Vocabulary follows [`CONTEXT.md`](../CONTEXT.md). The run log is
[`docs/plans/rust-native-rebuild-log.md`](../docs/plans/rust-native-rebuild-log.md).

## The shape

```
Event -> update(&mut App, Event) -> Vec<Effect>     pure, no I/O
         view(&App) -> Scene                        pure, no GPU
         render(&Scene)                             wgpu
```

- `specular-doc` owns the typed `Document`. Every mutation is a `Command`;
  `apply` returns the inverse, and undo is a stack of inverses.
- `specular-interact` owns `App`, `Session`, `Event`, `Effect`, `Tool`,
  `Gesture`, hit-test and `update`.
- `specular-scene` owns the `Scene` display list and `view`, one module per
  kind under `src/view/`.
- Only the shell and the CEF crate do I/O. Everything else is testable with
  no window, GPU or CEF.

The Electron app under `../src` is the behavior spec, not the structure
spec. Read the TypeScript to learn what a feature does and to port pure
math and test cases. Do not copy its layering (IPC, broadcasts, stores,
registries). Never edit anything under `../src`.

## Adding a feature

A feature is one vertical slice. It touches the same places every time:

1. `specular-doc`: the `Kind` variant or fields, its `Command`s, its
   `.canvas` read and write.
2. `specular-interact`: the `Tool` or `Gesture` arm and the hit-test arm.
3. `specular-scene`: the `view` arm.
4. Tests, three per slice: a command round trip (apply, undo, compare) in
   `specular-doc`, a scripted gesture through `specular-testkit` asserting
   on the document, and a scene snapshot.

Adding an enum variant should make the compiler list every `match` that
needs a new arm. Do not add wildcard arms over `Kind`, `Tool` or `Gesture`.

### The gesture test

It goes in the `tests/` directory of the crate that owns the behavior
(`specular-interact/tests/gestures.rs` is the model), with
`specular-testkit` as a dev-dependency. A crate's `src/` unit tests cannot
use the testkit on that crate's own types.

```rust
use specular_interact::Key;
use specular_testkit::{ALT, CMD, TestApp, assert_doc_snapshot};

#[test]
fn alt_drag_moves_the_page() {
    let mut app = TestApp::with_pages(2); // p1 at (100, 100), p2 at (700, 100), 400x300
    app.hold(ALT).drag((200.0, 150.0), (260.0, 130.0)).let_go();
    assert_doc_snapshot!(app, @"");       // the failure prints the text to put here
    app.assert_undo_returns_to_start();   // every test that changes the document ends with this
}
```

- Start from `TestApp::with_pages(n)`, `TestApp::with_entities([..])` or
  `TestApp::from_canvas(json)`. Entities come from `page`, `text`, `shape`,
  `file`, `drawing` and `group`, each taking an id and a rect.
  `inside("g", entity)` puts one in a group, and
  `connected(document(entities), "e1", "a", "b")` adds an edge.
- Input chains: `pointer_move`, `press`, `drag_to`, `release`, `drag`,
  `click`, `double_click`, `key`, `chord(CMD, Key::Char('z'))`,
  `type_text("hi")`, `wheel`, `pinch`, `tick`. `hold(mods)` keeps modifiers
  down until `let_go()`. `select`, `tool`, `zoom`, `undo`, `redo` and `act`
  run `Action`s. Anything else goes through `send(Event)`.
- Read back with `document()`, `session()`, `selection()`, `selected()`,
  `selected_ids()`, `rect("p1")` and `entity("p1")`. `take_effects()` drains the effects
  returned since the last drain; call it before the step whose effects the
  test asserts on.
- `assert_doc_snapshot!(app)` keeps its snapshot in
  `tests/snapshots/<test>.snap`; `INSTA_UPDATE=always cargo test` writes
  it. Prefer the inline form for small documents: a failing run prints the
  new text, and `cargo insta accept` (from `cargo install cargo-insta`)
  writes it into the source. Read the snapshot before accepting it.
- A page is select-first (ADR 0022). `click` selects it, a second `click`
  or a `double_click` enters it, and only an entered page gets input.
- The scene snapshot is `assert_scene_snapshot!(app)`, in
  `specular-scene/tests/view.rs`. It prints one line per display-list item
  in paint order. `TestApp::scene_snapshot()` returns the same text, and
  `specular_scene::view(app.app(), viewport)` the `Scene` itself.

## Gate

Run from `native/` before every commit:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The workspace lints are the contract: no `unwrap`/`expect` outside tests,
`#[expect(lint, reason = "...")]` instead of `#[allow]`, docs on public
items, `// SAFETY:` on every `unsafe` block. `thiserror` in libraries,
`anyhow` only in binaries. Declare each dependency once in
`[workspace.dependencies]`. Keep files under about 400 lines.

## Working a task from the plan

You are one of several agents working through the task list, each with a
fresh context. Another agent may be working in this checkout at the same
time on different files.

- Do exactly one task, the one you were given. Read its entry in the plan,
  the run log's "Decisions" section, and the code it builds on.
- Make design choices that serve the end-state architecture, not the
  shortest diff. If the plan is silent or wrong about something, pick the
  simplest option consistent with the architecture, record it under
  "Decisions" in the run log, and keep going.
- Do not use `--release`, do not run `bundle-macos.sh`, and do not start or
  kill any `specular-app` process. A benchmark may be running.
- Commit only your own files, staged by explicit path (never `git add -A`
  or `git add .`). Conventional message, scope `native`. Do not push, open
  PRs, or switch branches.
- If the gate fails in a crate you did not touch, wait a minute and retry
  once; the other agent may be mid-edit. If it still fails, note it in the
  log and gate your own crates with `-p`.
- When done, append a log entry: task id, commit sha, what exists now, what
  the next agent must know, anything that needs a human at a Mac. Keep it
  under 15 lines. Commit the log with your work.
- If you cannot finish, commit what compiles, write `BLOCKED` and the
  reason in the log, and stop.
- End your final message with one line: `TASK <id> DONE <sha>` or
  `TASK <id> BLOCKED <reason>`.
