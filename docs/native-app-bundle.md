# Specular: the app bundle

How to build `Specular.app`, what is in it, how a release is made, and what
it does not do yet.

## Build

```sh
cd native
export CEF_PATH="$HOME/.local/share/cef"   # the first build downloads about 300 MB here
crates/specular-shell/scripts/bundle-app.sh
open target/release/bundle/Specular.app
```

The script builds the Kit shell with CEF, the CEF helper, the `specular` CLI
and the agent helper (`native/helper`, with Bun), lays the bundle out and
signs it ad hoc. `--no-build` wraps what is already built, `--profile debug`
wraps a debug build, and `--out DIR` puts the app somewhere else. Run it
again after every rebuild, because the bundle holds copies of the binaries.
An ad hoc signature runs on the Mac that built it; any other Mac's
Gatekeeper refuses it.

The bundle reads two folders outside `native/`: `resources/starter-space`
and `resources/skills/specular`. The icon is
`native/crates/specular-shell/assets/AppIcon.icon`, with a compiled copy in
`AppIcon.compiled` for an Xcode too old to read it (`compile-icon.sh`).

## What is in it

```
Specular.app/Contents/
  Info.plist
  MacOS/Specular                                    the `specular` binary
  MacOS/specular-cli                                the `specular` command, from native/crates/specular-cli
  MacOS/specular-helper                             the Agent SDK host, from native/helper
  Helpers/claude                                    the Claude Code runtime the SDK runs
  Frameworks/Chromium Embedded Framework.framework  from CEF_PATH
  Frameworks/Specular Helper.app                    and (GPU), (Renderer), (Plugin), (Alerts)
  Frameworks/Sparkle.framework                      the updater
  Resources/Assets.car, AppIcon.icns                compiled from specular-shell/assets/AppIcon.icon
  Resources/starter-space/                          resources/starter-space
  Resources/bin/agent-browser                       the CLI's browse verbs, from native/scripts/build-agent-browser.sh
  Resources/skills/specular, skills/core            the specular skill, and the one agent-browser reads
```

Each Chromium helper holds `specular-cef-helper`. Chromium starts it with a
`--type=` switch and it runs as a child process.

The name, the bundle id (`com.lyleklyne.specular`) and the data folder
(`~/Library/Application Support/Specular`) are the ones the Electron app
used, so an existing install's preferences, repo bindings and space carry
over.

### The command and the skill

The CLI is the Rust binary `specular-cli`. It is built under that name so it
never lands on the Kit shell's `specular` in `target/`, and the link on the
`PATH` is named `specular`. It finds the bundled agent-browser in
`../Resources/bin` beside itself. A link the Electron app made pointed at its
own `specular-cli.sh`, which this bundle does not have, so the setup dialog
offers to replace it.

Neither is installed until the user says so, in the setup dialog
(`specular-shell/src/kit/setup.rs` over `specular-shell/src/agent_tools.rs`). The
dialog opens by itself once a space is open, on every launch until the user
presses Done, and from Specular > Set up Specular… at any time. "Skip for
now" closes it and leaves it to open again. Done is recorded as
`onboarding.completed` in `preferences.json`.

- The command is linked into `/usr/local/bin`, or `~/.local/bin` when that
  cannot be written. A link already there that runs something else is named
  in the dialog, and is replaced only when the user presses Replace on its
  card. A file that is not a link is never replaced.
- The skill is copied to `~/.claude/skills/specular`. The hash of what was
  written goes in `preferences.json` under `onboarding.skillHashes`.

At each launch an installed skill is brought up to the bundled one when the
installed copy is still the one the app wrote. A skill that was never
installed, or was edited by hand, is left alone.

Nothing is offered or written by a scripted run, a run with
`SPECULAR_NATIVE_CONFIG_DIR` set, or an app opened from its disk image.

## Info.plist

- **Documents.** `.canvas` is declared as an imported type,
  `org.jsoncanvas.canvas`, conforming to `public.json`. The app is an
  Editor for it with rank Alternate, so it does not take the extension from
  an app that owns it. Opening a `.canvas` from Finder opens the folder it
  is in as the space and shows that canvas. The folder is not remembered as
  your space.
- **URL schemes.** None.
- **Updates.** `SUFeedURL` names the release feed and `SUPublicEDKey` the
  key updates are signed with.

## First run and the space folder

The app keeps the folder you chose as `spacePath` in its preferences. With
none chosen it shows the first-run view: "Create a new space…" and "Open an
existing folder…". A folder with no canvas gets the starter space. A folder
with canvases is opened as it is. Nothing is moved or copied from another
folder (ADR 0033).

If the chosen folder is missing at launch, the app opens nothing and shows
the same view with the folder's path, "Locate the folder…" and "Quit".

Settings > General > Change… opens another folder and leaves the current
one alone. It does not offer to move the canvases across.

From the command line, `--space PATH` or a bare path opens that folder or
file, and `--space scratch` opens a throwaway copy of the starter space.
`SPECULAR_NATIVE_CONFIG_DIR` moves the data folder, and
`SPECULAR_SPACE_PICK=FOLDER` answers the folder dialog in a scripted run.

## Logins

A launch with no path argument keeps its pages' profile in `cef-profile/`
in the data folder, so a login survives a quit. CEF allows one process a
profile. A second copy of the app started while the first runs logs that
the profile is in use and keeps its own in memory. A run on a fixture, a
path or the scratch space always uses a profile in memory.

## A release

`bundle-app.sh --sign IDENTITY --notarize --dist` signs every executable with
the hardened runtime, has Apple notarize the app and the disk image, staples
the tickets, and writes beside the app:

- `Specular-<version>-<arch>.dmg`, to download;
- `Specular-<version>-<arch>.zip`, which the updater downloads;
- `appcast.xml`, the Sparkle feed naming that zip, signed with
  `SPARKLE_PRIVATE_KEY` (else the key in the login keychain under the
  account `specular`).

Notarization takes `NOTARY_PROFILE`, or `APPLE_ID`, `APPLE_PASSWORD` and
`APPLE_TEAM_ID`. The version is `[workspace.package] version` in
`native/Cargo.toml`. Pushing a matching `v*` tag runs
`.github/workflows/release.yml`, which does all of this on CI and publishes
the three files as a GitHub release; running the workflow by hand is a
rehearsal that keeps them as a run artifact instead. The feed's address
follows the newest release, so it never changes.

## What remains

- **Universal build.** The bundle is for the Mac's own architecture.
- **Sandbox.** CEF runs with `no_sandbox`. The helpers do not start the
  macOS sandbox.
- **Crash dumps.** CEF's Crashpad needs a `crash_reporter.cfg` in the
  bundle, which it does not have, so only `errors.log` is written.
- **Moving a space.** ADR 0033's "Move my canvases" is not built.
