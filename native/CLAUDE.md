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
- `specular-scene` owns the `Scene` display list and `view`.
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
4. Tests, three per slice: a command round trip (apply, undo, compare), a
   scripted gesture through `specular-testkit` asserting on the document,
   and a scene snapshot.

Adding an enum variant should make the compiler list every `match` that
needs a new arm. Do not add wildcard arms over `Kind`, `Tool` or `Gesture`.

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
