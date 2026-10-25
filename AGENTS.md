# Agents

Specular is a spatial canvas for designers and frontend developers. Part browser,
part design iteration surface. Users pull live web content onto a freeform canvas
to think, arrange, and iterate spatially.

The app is one Rust process under `native/`. [`CLAUDE.md`](CLAUDE.md) is the
full guide for agents working in this repo; this file is the short version.

Read native/CLAUDE.md for the architecture, how to add a feature and the test bar.
Read native/README.md for the crates and how to run the app.
Read CONTEXT.md for the glossary.
Read docs/product.md for product philosophy and audience.
Read docs/file-formats.md for the .canvas spec and persistence details.

## Build & verify

From `native/`, before every commit:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
fixtures/scenarios/run.sh
```

`cargo run -p specular-shell` runs the app with synthetic pages. Real pages need
`CEF_PATH` and a bundle (`crates/specular-shell/scripts/bundle-app.sh --profile debug`).
Run the app in its own terminal or pane. Don't kill or restart an app you didn't start.

## Architecture (quick reference)

```
Event -> update(&mut App, Event) -> Vec<Effect>     pure, no I/O
         view(&App) -> Scene                        pure, no GPU
         render(&Scene)                             wgpu
```

- `specular-doc` owns the typed `Document`; every change is a `Command` whose
  inverse is the undo step.
- `specular-interact` owns `App`, `Event`, `Effect`, `Tool`, `Gesture` and `update`.
- Only the shell crates and `specular-cef` do I/O. Everything else runs in tests
  with no window, GPU or CEF.

## Terminology

We follow the JSON Canvas spec (jsoncanvas.org) on disk:

- **Node** — a disk-layer word for an entry in a `.canvas` file. In code and UI it is an entity / canvas item.
- **Edge** — a connection between two canvas items
- **Canvas** — a single .canvas file; the spatial document
- **Space** — a folder of canvases (like an Obsidian vault)
- **Page** — a live web page on the canvas, a `link` node on disk

## File format principles

- All data uses open, human-readable, local-first formats
- .canvas files follow JSON Canvas v1.0 with transparent app extensions
- Files live on disk in the user's space folder — the file system is the data model
- Files must be diffable, versionable, and editable by agents and other tools

## Code principles

Build toward small, obvious pieces.

- Extract by responsibility, not by size. One behavior, one view, or one utility per file.
- Prefer pure functions for math, shaping, and derived state.
- Delete dead code and stale APIs quickly. Do not preserve unused abstractions.
- Reuse shared primitives only when behavior is truly shared.
- If the cleanest solution wants a different shape, propose that change.
- Preserve existing UX unless a cleaner design is intentionally chosen.
