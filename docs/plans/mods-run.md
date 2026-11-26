# Mods run log (ADR 0055)

The build of [ADR 0055](../adr/0055-chrome-as-files-then-scripted-panels.md)
(the ADR itself is in PR 457), one section a phase: what was built, what each
phase's end test measured, and the calls made without asking.

## Phase 0: the spike

**Result: go.** `gpui-shell` 0.11 runs inside our GPUI pin with no fork and
no patch, and a scripted toolbar meets phase 3's numbers by a wide margin.

What was built: `crates/specular-mods`, the mod host. It loads a mod folder
(`mod.json` + `main.js`) on a `gpui-shell` runtime of its own, gives it the
`specular` host module (`state`, `props`, `shared`, `setShared`, `run`,
`commands`, `notify`), and records which keys each mod read so it renders
again only when one of them moves (`Reads`, `Revisions`, a port of Herdr's
`publish` and `LiveReads`). `tests/toolbar.rs` loads a toolbar mod over a
`panels/toolbar` fixture in a headless GPUI window and measures it.

Numbers, all from `cargo test -p specular-mods --release --test toolbar --
--nocapture` (Linux x86_64, 4 cores, GPUI's headless test platform; CI runs
the same test on macOS):

| | Release | Debug |
|---|---|---|
| Load and first render | 11 ms | 36 ms |
| Script render, median of 300 | 0.08 ms | 1.1 ms |
| Script render, p95 | 0.13 ms | 1.2 ms |
| Script render, slowest | 0.40 ms | 1.7 ms |
| Materialize, median | 0.03 ms | 0.19 ms |
| Script renders in 100 idle frames | 0 | 0 |
| Render when an unread key moves | none | none |

Phase 3's gate is a median under 2 ms and none over 8 ms; the release build
is 25x and 20x inside it.

Build cost, measured on Linux with a bare GPUI Kit window against the same
window with `gpui-shell` (release, 4 cores):

- Build time: about 5.5 minutes added to a clean release build. Nothing is
  added to an incremental build of our crates.
- Binary size: 28 MB to 64 MB stripped (+35 MB). Most of it is the runtime
  itself (110,000 lines, with GPUI's inspector reflection that it turns on
  for the script's style surface), the QuickJS JIT, and the `fetch` and
  `websocket` modules' HTTP and TLS stack. The macOS figure will differ; it
  is worth reading off the first release build.

Patches we would need upstream: none to run. Two would be nice to have,
neither blocking:

- The budgets (render 2 s, event 5 s) are private. Ours are measured instead
  (`ModStats`, `specular mod stats`), as the ADR planned.
- The engine tier is private. Debug builds stay on the interpreter; release
  builds tier hot functions up to the JIT. Upstream notes the JIT crashing on
  Linux and Windows under many short-lived runtimes in debug, which is why
  debug stays on the interpreter. Specular is macOS only and holds a handful
  of long-lived runtimes, so this is a watch item, not a stop.

Findings that change the ADR's text:

- `default-features = false` drops the JavaScript engine too: `quickjs` is a
  default feature. The pin is `default-features = false, features =
  ["quickjs"]`.
- `gpui-shell`'s manifest file is `gpui-shell.json`, not `mod.json`, so ours
  sits beside it with no clash. A mod has no `gpui-shell.json`; the runtime
  loads `main.js`.
- Host functions do not know which script called them. Each mod gets its own
  instance of the `specular` module (and its own runtime), which is what
  lets a read be recorded against the mod that made it and gives per-mod
  numbers.
- The host owns hot reload: `ShellRuntime::watch` reloads a mod on save and
  keeps the old build running when the new one fails, which is the ADR's
  "a mod that fails at load keeps its old build".

Call made: the mod host is its own crate, `specular-mods`, rather than a
module inside `specular-shell`. The Kit window builds only on macOS; this
crate builds and tests on Linux too, so the mod host is checked headless
everywhere. The ADR's rule still holds: the command registry and the keyed
state live in `specular-interact` with no runtime dependency, and a change of
runtime touches `specular-mods` and the shell alone.
