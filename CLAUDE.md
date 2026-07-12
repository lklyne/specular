# Claude

Specular is a spatial canvas for designers and frontend developers. Part browser,
part design iteration surface. Users pull live web content onto a freeform canvas
to think, arrange, and iterate spatially.

Read CONTEXT.md for the canonical glossary (canvas items, entities, tools, text styles, edges).
Read docs/product.md for product philosophy and audience.
Read docs/architecture.md for the full system map.
Read docs/file-formats.md for .canvas spec and persistence details.
Read docs/interaction-layer.md before adding gestures, overlays, focus
handoffs, or drop targets — the invariants in §6 are load-bearing.

When a domain term resolves during a session (rename, new concept, deprecation), update CONTEXT.md inline. If the decision is hard to reverse and surprising without context, add an ADR under docs/adr/ and link it from the relevant CONTEXT.md entry.

## Build & verify

```
pnpm install                 # install dependencies
pnpm dev                     # start the Electron app
pnpm typecheck               # type-check both node and web tsconfigs
pnpm test:unit               # fast unit tests (no Electron)
pnpm test:integration        # real runtime in-process (no Electron, seconds)
pnpm test:boot               # ~3 real-Electron boot checks (pre-release only)
pnpm build                   # package for distribution
```

For unattended or agent-owned app runs, use `pnpm devctl start|status|logs|restart|stop`.
`devctl` refuses to restart or stop a foreground `pnpm dev` that it did not
start. `pnpm devctl context` prints process state, recent dev/error logs, and
performance-trace metadata as one agent-readable JSON document.

After any structural change, run `typecheck` + `test:unit` at minimum.
After changes to runtime, IPC, or persistence, run `test:integration`.
`test:boot` needs a built app + the Electron binary; run it before releases, not per-change.

## Crash logs

Set up in `src/main/index.ts`:
- JS errors, unhandled rejections, and renderer/child process deaths → `~/Library/Logs/Specular/errors.log`
- Native crash dumps (Crashpad) → `~/Library/Application Support/Specular/Crashpad/completed/*.dmp`

When investigating unexpected quits, `errors.log` is the first place to look — `render-process-gone` entries include a `reason` field (`crashed`, `oom`, `killed`, `launch-failed`).

## Perf tracing

All-process Chromium traces (pan/zoom jank attribution) can be recorded from
the View menu (`Cmd+Alt+Shift+P`), the debug window's Performance section, or
by agents over HTTP (`POST localhost:29979/perf/trace/start` / `/stop` with
`{"summarize": true}`). Trace + summary JSON land in the logs folder above.
Full instructions and how to read results: `docs/perf-tracing.md`.

## Architecture (quick reference)

Electron app with four source layers:

```
src/main/          Main process: state, persistence, HTTP API, CLI, IPC routing
src/preload/       Context bridges: one per renderer overlay (9 total)
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

Forward sync: runtime -> Y.Doc on mutation.
Reverse sync: Y.Doc -> runtime on undo/redo.
Persistence: Y.Doc -> .canvas files on disk (350ms debounce).

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

### Renderer structure

Each renderer is an isolated React app. The main surface is canvas-bg/.
Others (toolbar, left-sidebar, right-details-panel) are
smaller overlay panels with their own preload bridge.

### Entity-renderer plugins

File entities pick a renderer through a small registry, not an extension switch.

- `src/main/plugins/registry.ts` — internal API (`registerEntityRenderer`, `pickRenderer`, `getRendererTagFor`).
- `src/main/plugins/builtin/` — one claim per renderer (image, video, markdown, wireframe, component).
- `src/renderer/canvas-bg/entity-renderers/` — the React mounts plus `RendererSwitch.tsx`.

Main calls `getRendererTagFor` in `buildFileEntitySceneEntity` and broadcasts the
tag on every file scene entity; the renderer reads `entity.rendererTag` and
`RendererSwitch` picks the component. To add a renderer, add a claim file under
`builtin/`, a React file under `entity-renderers/`, and a case in `RendererSwitch.tsx`.

## Layer rules

- `src/renderer/` must NOT import from `src/main/`
- `src/shared/` must have no side effects and no process-specific imports
- `src/preload/` bridges IPC only — no business logic
- `src/main/runtime/` is the single owner of workspace state
- Renderer state is derived from IPC broadcasts, never authoritative

## File format principles

- All data uses open, human-readable, local-first formats
- .canvas files follow JSON Canvas v1.0 with transparent app extensions
- Files live on disk in the user's space folder — the file system is the data model
- Files must be diffable, versionable, and editable by agents and other tools
- No proprietary blobs, no server dependencies for core data

## View modes

Browser and Canvas are different views of the same data:

- **Canvas mode** — spatial freeform surface; nodes arranged freely
- **Browser mode** — traditional tab navigation between pages

Both operate on the same underlying .canvas data and share primitives.
Maximize overlap between modes.

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
- Comment the timeless "why" only when the code is non-obvious. Don't narrate the change — a comment that says what the code "now" does, "no longer" does, "used to" do, or "moved away from" is describing the diff, not the code. Write it as if the prior version never existed; the git history already holds that. (Backward-compat notes explaining why a dead-looking branch must stay are "why", and are fine.)

When refactoring, optimize for clarity, smaller files, and stronger boundaries.

When a UI pattern is shared across entity types (chrome headers, inline menus, selection outlines), prefer compound components over prop-heavy wrappers. Style once in subcomponents, compose differently per use case — like base-ui's `Tabs.Root > Tabs.List > Tabs.Tab` pattern. Use React context for shared state within the compound (theme, visibility), children/slots for per-use content. See `EntityChrome.Root / .DragTrigger / .Title / .Actions / .Button` for the model.

## Common workflows (context, not constraints)

These are the patterns users gravitate toward. The app supports them through
primitives and tools, not dedicated workflow features:

- **Multi-breakpoint iteration** — linked pages at different viewport presets
- **Live site annotation** — annotating running pages to iterate on fixes
- **Research & spatial organization** — collecting and arranging web references
- **Wireframing & broad strokes** — rough spatial layouts before implementation

## Agent integration

Agent interaction is moving to a CLI for better context management.
Agents can also read and write .canvas files directly — they are just JSON.
The HTTP API (src/main/routes/) remains available for runtime interaction.

## Testing patterns

- **Unit tests** — pure logic, no Electron. `tests/unit/`
- **Integration tests** — the real main-process runtime, in-process (electron aliased to a stub). `tests/integration/`, gated in CI. See ADR 0024.
- **Boot tests** — real Electron, ~3 checks, pre-release only. `tests/boot/`
- **Agent tests** — scenario scripts. `tests/agent/`
- **Harness** — `bootWorkspaceHarness()` in `tests/integration/harness.ts` boots the runtime against a temp dir; tests call the same mutators the IPC handlers call and assert on runtime arrays, the Y.Doc, and `.canvas` files.

Before writing a test, re-read `tests/README.md` — it documents the four-criterion bar a test must clear to earn its keep, the bucket selection guide, and the mutation-verification convention.

## Test contract

The suite stays small on purpose. To prevent drift back to a pile of low-value tests, the following apply:

- Any new entity kind ships with integration coverage of its persistence + undo round-trip.
- Any new runtime mutator ships with forward/reverse sync coverage (one Y.Doc transaction per mutation; undo round-trips cleanly).
- PRs touching `src/main/runtime/workspace-*.ts` require integration coverage updates unless the change is pure refactor with no behavior delta.
- No new `.todo()` test merged without a linked issue describing what would unblock it.
- Before writing a test, re-read `tests/README.md` and confirm it clears the four-criterion bar.

## Specular CLI

- Always pass full URLs (including scheme and host) to `specular create page`. The canvas can contain pages from different origins, so bare paths like `/garden` are ambiguous. Use `http://localhost:4321/garden`, not `/garden`.

## Skill files

The specular Claude Code skill lives in three places. Knowing which is which avoids silently-wasted edits:

- **`resources/skills/specular/SKILL.md`** — canonical source. Bundled into the packaged app via `forge.config.ts` (`extraResource`), then copied into every user's `~/.claude/skills/specular/` at app launch by `src/main/skill-install.ts`. Default to editing this when updating guidance that should reach end users.
- **`.claude/skills/specular/SKILL.md`** — repo-local copy Claude Code auto-loads when working inside this codebase. Keep it in sync with `resources/skills/` so in-repo agents and end users see the same guidance. A few dev-only sections (e.g. the tracking-issue GitHub link) are allowed to live here and not ship.
- **`~/.claude/skills/specular/SKILL.md`** — each user's installed copy. Treat as read-only: `src/main/skill-auto-update.ts` compares the bundled hash to what's installed and re-copies the bundled version whenever they differ, so hand-edits here get silently overwritten on the next app launch after a release.

When patching the skill, default to updating both `resources/skills/specular/SKILL.md` and `.claude/skills/specular/SKILL.md` in the same commit.

## Agent skills

### Issue tracker

Issues and PRDs are tracked as GitHub issues in `lklyne/specular`, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage uses the five canonical label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
