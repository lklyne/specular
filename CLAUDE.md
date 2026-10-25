# Claude

Specular is a spatial canvas for designers and frontend developers. Part browser,
part design iteration surface. Users pull live web content onto a freeform canvas
to think, arrange, and iterate spatially.

The app is one Rust process under `native/`: a GPUI Kit window around a wgpu
canvas, with live pages hosted by CEF offscreen rendering. The Electron app it
replaced was deleted; its source is in git history at `bc5253fa`
(`git show bc5253fa:src/main/<path>`).

Read native/CLAUDE.md before changing code: the architecture, how to add a
feature, the test bar and the gate.
Read native/README.md for the crates, how to run, the headless modes and the bench.
Read CONTEXT.md for the canonical glossary (canvas items, entities, tools, text styles, edges).
Read docs/product.md for product philosophy and audience.
Read docs/file-formats.md for the .canvas spec and persistence details.
Read docs/README.md for a one-line map of every doc and ADR — check it
before starting deep work in an area. Rejected/superseded ADRs record
approaches that were tried and abandoned; read their postmortems before
re-attempting anything in their territory. ADRs 0001–0038 were written for
the Electron app: their decisions about behavior still hold where the Rust
app ported them, their mechanisms do not.

When a domain term resolves during a session (rename, new concept, deprecation), update CONTEXT.md inline. If the decision is hard to reverse and surprising without context, add an ADR under docs/adr/ and link it from the relevant CONTEXT.md entry.

## Build & verify

From `native/`:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                 # no CEF, no window
fixtures/scenarios/run.sh              # headless scenario scripts
cargo run -p specular-shell            # the app, with synthetic pages
```

Real pages need the `cef` feature and an app bundle:

```
export CEF_PATH="$HOME/.local/share/cef"   # the first build downloads about 300 MB
crates/specular-shell/scripts/bundle-app.sh --profile debug
```

`native/README.md` has the dev bundle, the command line and the flags. The
agent helper (`native/helper`, the Agent SDK host under Bun) builds with
`bun install && bun run build`; `bun run typecheck` checks it.

Dev app workflow:
- Run the app in its own terminal or pane, so it can be watched and
  stopped independently of other work.
- Don't kill or restart an app you didn't start.

Run the full gate before every commit. Debug builds unless a task says
otherwise.

## Crash logs

- Panics, page crashes and CEF child-process deaths go to
  `~/Library/Logs/Specular/errors.log` (`specular-app/src/crash_log.rs`).
- Native crash dumps are not written yet: CEF's Crashpad needs a
  `crash_reporter.cfg` in the bundle.

When investigating unexpected quits, `errors.log` is the first place to look.

## Architecture (quick reference)

```
Event -> update(&mut App, Event) -> Vec<Effect>     pure, no I/O
         view(&App) -> Scene                        pure, no GPU
         render(&Scene)                             wgpu
```

`update` is the only thing that changes the app. Anything outside it — a file
write, a page host, an agent run — is an `Effect` the shell runs. That is why
the tests and the headless scenario runs need no window. The document is typed
structs; every change is a `Command` whose `apply` returns its inverse, and
undo is a stack of inverses (ADR 0041).

## Layer rules

The crates and their dependency direction are in `native/README.md`. The rules:

- `specular-core`, `-doc`, `-agent`, `-interact`, `-scene` and `-api` do no
  GPU, window, CEF or file I/O. Only the shell crates and `specular-cef` do I/O.
- A new effect gets its runner once, in the runtime (`specular-app/src/app/`).
- `specular-cef-helper` depends on `cef` alone; each crate linked into it is
  paid for five times in the bundle.
- UI is pure models (`&App` -> plain data) drawn by the Kit shell (ADR 0044).
- No wildcard arms over `Kind`, `Tool` or `Gesture`.
- Workspace lints are the contract: no `unwrap`/`expect` outside tests,
  `#[expect(lint, reason = "...")]` instead of `#[allow]`, docs on public
  items, `// SAFETY:` on every `unsafe` block.

## File format principles

- All data uses open, human-readable, local-first formats
- .canvas files follow JSON Canvas v1.0 with transparent app extensions
- Files live on disk in the user's space folder — the file system is the data model
- Files must be diffable, versionable, and editable by agents and other tools
- No proprietary blobs, no server dependencies for core data

## Code principles

Build toward small, obvious pieces.

- Extract by responsibility, not by size. One behavior, one view, or one utility per file.
- Prefer pure functions for math, shaping, and derived state.
- Delete dead code and stale APIs quickly. Do not preserve unused abstractions.
- Reuse shared primitives only when behavior is truly shared.
- If the cleanest solution wants a different shape, propose that change.
- Preserve existing UX unless a cleaner design is intentionally chosen.
- Keep files under about 400 lines.
- Comment the timeless "why" only when the code is non-obvious. Don't narrate the change — a comment that says what the code "now" does, "no longer" does, "used to" do, or "moved away from" is describing the diff, not the code. Write it as if the prior version never existed; the git history already holds that. (Backward-compat notes explaining why a dead-looking branch must stay are "why", and are fine.)

## Common workflows (context, not constraints)

These are the patterns users gravitate toward. The app supports them through
primitives and tools, not dedicated workflow features:

- **Multi-breakpoint iteration** — synced pages at different viewport presets
- **Live site annotation** — annotating running pages to iterate on fixes
- **Research & spatial organization** — collecting and arranging web references
- **Wireframing & broad strokes** — rough spatial layouts before implementation

## Agent integration

Agents drive the running app with the `specular` CLI (`native/crates/specular-cli`),
which talks to the app's HTTP API. They can also read and write .canvas files
directly — they are just JSON. The agent chat in the app runs Claude Code through
`native/helper`.

## Test contract

The suite stays small on purpose. The bar and what earns a test are in
native/CLAUDE.md ("Tests"). In short:

- Any new entity kind ships with a `.canvas` round trip and an undo round trip.
- Any new `Command` ships with a test that its inverse returns the document
  to the start.
- A behavior test ends with the undo-returns-to-start check.
- A new CLI verb or route gets a case in `native/fixtures/scenarios/cli/cases.py`.
- A bug that only showed up in use goes into `native/fixtures/scenarios/`.

## Specular CLI

- Always pass full URLs (including scheme and host) to `specular create page`. The canvas can contain pages from different origins, so bare paths like `/garden` are ambiguous. Use `http://localhost:4321/garden`, not `/garden`.

## Skill files

The specular Claude Code skill lives in three places. Knowing which is which avoids silently-wasted edits:

- **`resources/skills/specular/SKILL.md`** — canonical source. `bundle-app.sh` copies it into the app bundle (`Contents/Resources/skills/specular`), and the app's setup dialog installs it into `~/.claude/skills/specular/` (`specular-app/src/agent_tools.rs`). Default to editing this when updating guidance that should reach end users.
- **`.claude/skills/specular/SKILL.md`** — repo-local copy Claude Code auto-loads when working inside this codebase. Keep it in sync with `resources/skills/` so in-repo agents and end users see the same guidance. A few dev-only sections (e.g. the tracking-issue GitHub link) are allowed to live here and not ship.
- **`~/.claude/skills/specular/SKILL.md`** — each user's installed copy. Treat as read-only: at each launch the app brings it up to the bundled skill when the installed copy is still the one the app wrote. A hand-edited copy is left alone and stops getting updates.

When patching the skill, default to updating both `resources/skills/specular/SKILL.md` and `.claude/skills/specular/SKILL.md` in the same commit.

## Releases

The version is `[workspace.package] version` in `native/Cargo.toml`. A `v*`
tag that matches it makes `.github/workflows/release.yml` build, sign,
notarize and publish the DMG, the updater zip and the Sparkle feed. The
`release` skill walks it. `docs/native-app-bundle.md` has the bundle's layout.

## Agent skills

### Issue tracker

Issues and PRDs are tracked as GitHub issues in `lklyne/specular`, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Triage uses the five canonical label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
