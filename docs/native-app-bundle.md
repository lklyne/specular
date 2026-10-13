# Specular Native: the app bundle

How to build `Specular Native.app` from the Rust rebuild, what is in it, how
it stays apart from the Electron app, and what it does not do yet.

## Build

```sh
cd native
export CEF_PATH="$HOME/.local/share/cef"   # the first build downloads about 300 MB here
crates/specular-shell/scripts/bundle-app.sh
open "target/release/bundle/Specular Native.app"
```

The script runs `cargo build --release -p specular-shell --features cef`,
lays the bundle out and signs it ad hoc. `--no-build` wraps the binary that
is already there, `--profile debug` wraps a debug build, and `--out DIR`
puts the app somewhere else. Run it again after every rebuild, because the
bundle holds copies of the binary.

## What is in it

```
Specular.app/Contents/
  Info.plist
  MacOS/Specular                                    the `specular` binary
  MacOS/specular-helper                             the Agent SDK host and the CLI, from native/helper
  Helpers/claude                                    the Claude Code runtime the SDK runs
  Frameworks/Chromium Embedded Framework.framework  from CEF_PATH
  Frameworks/Specular Helper.app                    and (GPU), (Renderer), (Plugin), (Alerts)
  Frameworks/Sparkle.framework                      the updater
  Resources/Assets.car, AppIcon.icns                compiled from specular-shell/assets/AppIcon.icon
  Resources/starter-space/                          resources/starter-space
  Resources/specular-cli.sh                         the `specular` command
  Resources/bin/agent-browser                       the CLI's browse verbs, from native/scripts/build-agent-browser.sh
  Resources/skills/specular, skills/core            the specular skill, and the one agent-browser reads
```

Each Chromium helper holds `specular-cef-helper`. Chromium starts it with a
`--type=` switch and it runs as a child process.

### The command and the skill

The CLI is the Electron app's source (`src/main/cli-commands.ts`), compiled
into `specular-helper` as its `cli` service. `specular-cli.sh` runs it, and
sits where the Electron app kept its own wrapper, so a link that app made
on the `PATH` runs this one.

Neither is installed until the user says so, in the setup dialog
(`specular-shell/src/setup.rs` over `specular-app/src/agent_tools.rs`). The
dialog opens by itself once a space is open, on every launch until the user
presses Done, and from Specular > Set up Specular… at any time. "Skip for
now" closes it and leaves it to open again. Done is recorded as
`onboarding.completed` in `preferences.json`, the key the Electron app used,
so someone who went through setup there is not asked again.

- The command is linked into `/usr/local/bin`, or `~/.local/bin` when that
  cannot be written. A link already there that runs something else is named
  in the dialog and its row starts unticked. A file that is not a link is
  never replaced.
- The skill is copied to `~/.claude/skills/specular`. The hash of what was
  written goes in `preferences.json` under `onboarding.skillHashes`.

At each launch an installed skill is brought up to the bundled one when the
installed copy is still the one the app wrote. A skill that was never
installed, or was edited by hand, is left alone.

Nothing is offered or written by a scripted run, a run with
`SPECULAR_NATIVE_CONFIG_DIR` set, or an app opened from its disk image.

## Side by side with the Electron app

| | Electron | Rust |
|---|---|---|
| Name | Specular | Specular Native |
| Bundle id | `com.lyleklyne.specular` | `com.lyleklyne.specular.native` |
| Data folder | `~/Library/Application Support/Specular` | `~/Library/Application Support/Specular Native` |
| Preferences | `preferences.json` there | its own `preferences.json` |
| Repo bindings | `repos.json` there | its own `repos.json` |
| Page profile | Chromium's, in the data folder | `cef-profile/` in its data folder |

The Rust app reads two of the Electron app's files and writes neither:

- `preferences.json`, for `spacePath`. The first-run view offers that
  folder as "Use the space from Specular". It is opened only if chosen.
- `repos.json`, while the Rust app has no `repos.json` of its own. The
  bindings made in Electron show in Settings > Repos, and the first change
  made in the Rust app writes its own file.

Two things are still shared:

- The space folder, if you point both apps at one. Each keeps the canvas
  list in `.specular/` and rewrites it, so run one at a time.
- The agent port. The first app to start takes 29979 and
  `~/.specular/specular-mcp.json`. The Rust app, started second, logs the
  port and the discovery file it fell back to.

## Info.plist

- **Documents.** `.canvas` is declared as an imported type,
  `org.jsoncanvas.canvas`, conforming to `public.json`. The app is an
  Editor for it with rank Alternate. Opening a `.canvas` from Finder opens
  the folder it is in as the space and shows that canvas. The folder is not
  remembered as your space. The Electron app claims no document type, so
  nothing competes.
- **URL schemes.** None. The Electron app registers none with the system
  (`local-file:` is a scheme inside its own pages). A scheme both apps
  claimed would open whichever one the system saw last.

## First run and the space folder

The app keeps the folder you chose as `spacePath` in its own preferences.
With none chosen it shows the first-run view: "Create a new space…", "Open
an existing folder…", and "Use the space from Specular" when the Electron
app has one. A folder with no canvas gets the starter space. A folder with
canvases is opened as it is. Nothing is moved or copied from another
folder (ADR 0033).

If the chosen folder is missing at launch, the app opens nothing and shows
the same view with the folder's path, "Locate the folder…" and "Quit".

Settings > General > Change… opens another folder and leaves the current
one alone. It does not offer to move the canvases across.

From the command line, `--space PATH` or a bare path opens that folder or
file, `--space scratch` opens a throwaway copy of the starter space, and
`--space user` opens the Electron app's space. `SPECULAR_NATIVE_CONFIG_DIR`
moves the data folder, and `SPECULAR_SPACE_PICK=FOLDER` answers the folder
dialog in a scripted run.

## Logins

A launch with no path argument keeps its pages' profile in `cef-profile/`
in the data folder, so a login survives a quit. CEF allows one process a
profile. A second copy of the app started while the first runs logs that
the profile is in use and keeps its own in memory. A run on a fixture, a
path or the scratch space always uses a profile in memory.

## What remains

These need your credentials or a decision, and none is started:

- **Developer ID signing.** The bundle is signed ad hoc, so it runs on the
  Mac that built it. Another Mac's Gatekeeper refuses it. Signing for
  distribution needs the certificate, the hardened runtime and CEF's
  entitlements on the helpers (JIT and unsigned executable memory for the
  renderer).
- **Notarization.** Needs the Apple ID, team id and an app-specific
  password that `forge.config.ts` reads from the environment.
- **A disk image or installer.** The script stops at the `.app`.
- **Updates.** No feed and no updater. The Electron app's GitHub release
  feed serves the Electron app only.
- **Universal build.** The bundle is for the Mac's own architecture.
- **Sandbox.** CEF runs with `no_sandbox`. The helpers do not start the
  macOS sandbox.
- **The CLI and the skill.** Not bundled and not installed (see above).
- **Moving a space.** ADR 0033's "Move my canvases" is not built.
