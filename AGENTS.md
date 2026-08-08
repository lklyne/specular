# Agents

Specular is a spatial canvas for designers and frontend developers. Part browser,
part design iteration surface. Users pull live web content onto a freeform canvas
to think, arrange, and iterate spatially.

Read docs/product.md for product philosophy and audience.
Read docs/architecture.md for the full system map.
Read docs/file-formats.md for .canvas spec and persistence details.

## Build & verify

```
pnpm install                 # install dependencies
pnpm dev                     # start the Electron app (foreground)
pnpm typecheck               # type-check both node and web tsconfigs
pnpm test:unit               # fast unit tests (no Electron)
pnpm test:integration        # real runtime in-process (no Electron, seconds)
pnpm test:boot               # ~3 real-Electron boot checks (pre-release only)
pnpm build                   # package for distribution
```

Run `pnpm dev` in its own terminal or pane so the app can be watched and stopped
independently. Don't kill or restart a dev app you didn't start.

After any structural change, run `typecheck` + `test:unit` at minimum.
After changes to runtime, IPC, or persistence, run `test:integration`.

## Architecture (quick reference)

Electron app with four source layers:

```
src/main/          Main process: state, persistence, HTTP API, CLI, IPC routing
src/preload/       Context bridges: one per renderer overlay (10 total)
src/renderer/      React apps: canvas-bg, toolbar, sidebar, inspector, overlays
src/shared/        Types and pure utilities shared across processes
```

State flows down: Main owns truth -> IPC broadcasts -> Renderers display.
Actions flow up: Renderer -> preload API -> IPC -> Main mutates state.

### Two-layer state model (details in src/main/runtime/CLAUDE.md)

```
Y.Doc (Yjs)         Persisted, undoable workspace state (entities, groups, edges)
Runtime variables    Ephemeral state (views, interaction mode, hover, timers)
```

### Key domains in src/main/runtime/

```
workspace-*          Persistence, tabs, model, Y.Doc, undo, autosave
runtime-core.ts      High-level state mutations
runtime-context.ts   Ephemeral state (zoom, pan, interaction, views)
selection-*          Selection state and mutations
page-*.ts            Page (webview) creation and lifecycle
layout-*.ts          View positioning, z-order, dirty tracking
*-entity-state.ts    Per-entity-kind mutations (text, file, group, drawing)
```

## Layer rules

- `src/renderer/` must NOT import from `src/main/`
- `src/shared/` must have no side effects and no process-specific imports
- `src/preload/` bridges IPC only — no business logic
- `src/main/runtime/` is the single owner of workspace state
- Renderer state is derived from IPC broadcasts, never authoritative

## Terminology

We follow the JSON Canvas spec (jsoncanvas.org) for our data model nouns:

- **Node** — any entity on the canvas (text, link, file, group)
- **Edge** — a connection between two nodes
- **Canvas** — a single .canvas file; the spatial document
- **Space** — a folder of canvases (like an Obsidian vault)
- **Page** — Specular's term for live web items (a URL rendered at a position on the canvas). See [ADR 0003](./docs/adr/0003-page-as-canonical-name-for-live-web-items.md).

## File format principles

- All data uses open, human-readable, local-first formats
- .canvas files follow JSON Canvas v1.0 with transparent app extensions
- Files live on disk in the user's space folder — the file system is the data model
- Files must be diffable, versionable, and editable by agents and other tools

## Code principles

Build toward small, obvious pieces.

- Keep `App.tsx` files thin — orchestrate state, hooks, and views only.
- Extract by responsibility, not by size. One behavior, one view, or one utility per file.
- Prefer pure helpers for math, shaping, and derived state.
- Prefer concrete hooks over generic wrappers. Name hooks after the behavior they own.
- Delete dead code and stale APIs quickly. Do not preserve unused abstractions.
- Reuse shared primitives only when behavior is truly shared.
- If the cleanest solution wants a different shape, propose that change.
- Preserve existing UX unless a cleaner design is intentionally chosen.

## Testing patterns

- **Unit tests** — pure logic, no Electron. `tests/unit/`
- **Integration tests** — real runtime in-process, electron stubbed. `tests/integration/`
- **Boot tests** — real Electron, pre-release only. `tests/boot/`
- **Agent tests** — scenario scripts. `tests/agent/`
