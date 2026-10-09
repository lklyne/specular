# native/: Specular in Rust

A rebuild of Specular's canvas as one Rust process. It opens the same space
folders and `.canvas` files as the Electron app under `../src`, hosts pages
through CEF offscreen rendering, and answers the same HTTP routes, so the
`specular` CLI drives it.

It started as a spike that measured a Rust and CEF shell against Electron.
That question is answered, and the measurement tools are still here (see
"Bench"). Everything since is the rebuild, done in about a day by agents and
checked almost entirely by script. Read these before trusting any of it:

- [`docs/plans/rust-native-rebuild-handoff.md`](../docs/plans/rust-native-rebuild-handoff.md):
  what works, what no person has checked, known bugs and open decisions.
- [`docs/plans/rust-native-rebuild.md`](../docs/plans/rust-native-rebuild.md):
  the plan, with a status table for every task.
- [`docs/plans/rust-native-rebuild-log.md`](../docs/plans/rust-native-rebuild-log.md):
  the run log, one entry a task.
- [`CLAUDE.md`](CLAUDE.md): how to add a feature and write its test.
- ADRs [0039](../docs/adr/0039-rust-canvas-render-stack.md) to
  [0044](../docs/adr/0044-ui-as-pure-models-with-replaceable-renderers.md):
  the decisions that are hard to reverse.

macOS on Apple Silicon is the only platform anything has run on. The GPUI
Kit shell and the menus are macOS only.

## The shape

```
Event -> update(&mut App, Event) -> Vec<Effect>     no I/O
         view(&App, viewport, &ViewCache) -> Scene  no GPU
         render(&Scene)                             wgpu
```

`update` is the only thing that changes the app. Anything outside it, such as
a file write or a page host, is an `Effect` the shell runs. That is why the
tests and the headless modes below need no window.

## Crates

| Crate | Kind | Owns |
|---|---|---|
| `specular-core` | lib | Camera math, the f32 `Point`, `Size` and `Rect`, the page model, the `PageSource` and `PageFrame` contracts, the input model, the text-measure trait, the synthetic page source, the locator scoring for interaction sync |
| `specular-doc` | lib | The typed `Document`: entities, edges, annotations, `Command`s with inverses, `History`, and the `.canvas` reader and writer |
| `specular-agent` | lib | The agent thread model: the `Threads` store, the thread files' shape, the prompts, the `claude` stream parser, repo bindings. Pure |
| `specular-interact` | lib | `App`, `Session`, `Space`, `Event`, `Effect`, `Action`, `Tool`, `Gesture`, hit-test, `update`, the text editor, the key binding table, and every panel model (toolbar, dock, sidebar, menus, chat, settings, first run) |
| `specular-scene` | lib | The `Scene` display list, `view` with one module a kind, the markdown parser, and the built-in panel painter |
| `specular-api` | lib | The HTTP API with the socket taken off: a request and `&App` in, a read answer or an `Event::Api` out. Also the per-page CDP routing |
| `specular-compositor` | lib | The wgpu renderer. Draws a `Scene` over the dot grid in one 4x multisampled pass: page textures, SDF shapes, glyphon text, lyon paths. Imports page IOSurfaces with no copy on macOS. Implements the text measure |
| `specular-cef` | lib | The CEF offscreen `PageSource` (feature `cef`) and the CEF-free helpers it is built from. See [`crates/specular-cef/README.md`](crates/specular-cef/README.md) |
| `specular-app` | lib + bin | `Runtime`, which owns the `App` and runs every effect: page hosts, space files, images, Documents, clipboard, the HTTP and CDP servers, the agent runner. Also the command line, the headless `--snapshot` and `--script` modes, `--bench`, and the winit window (binary `specular-app`) |
| `specular-shell` | bin | The GPUI Kit window (binary `specular`, ADR 0040). The Kit draws the chrome (tab row, toolbar, dock), sidebar, menus, settings, first run and the chat panel from the models. The compositor draws the canvas in a view under GPUI's. It uses `specular-app`'s `Runtime` and command line |
| `specular-bench` | lib + bin | Gesture profiles, frame stats, per-frame work times, process-tree memory, the Electron trace converter and `compare`. See [`crates/specular-bench/README.md`](crates/specular-bench/README.md) |
| `specular-testkit` | lib, dev only | `TestApp`, snapshot macros and entity builders for tests |

### Dependency direction

As the manifests have it, each crate depending only on crates to its left:

```
core, doc, agent  <-  interact  <-  scene  <-  compositor
                                \-  api
core  <-  cef, bench
everything above  <-  app  <-  shell
```

- `core`, `doc` and `agent` depend on no other crate here.
- `interact` depends on `doc`, `core` and `agent`. `scene` and `api` each
  depend on `interact`, `doc` and `core`.
- `compositor` depends on `scene` and `core`. It names `doc` and `interact`
  only in its tests.
- `app` depends on all of the above except the testkit. `shell` depends on
  `app`.
- `core`, `doc`, `agent`, `interact`, `scene` and `api` have no GPU, window,
  CEF or file I/O.

Two things differ from the plan's diagram. There is no `specular-ui` or
`specular-render` crate: panel models live in `interact`, panel painting in
`scene`, and the renderer kept the name `specular-compositor`. And the GPUI
shell sits on top of the winit shell's crate, not beside it.

## Build and gate

```sh
cd native
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

This needs no CEF and no window. The compositor's GPU tests skip with a
message when there is no wgpu adapter, and the text tests need a system font.

The default build hosts synthetic pages, which are animated stand-ins. Real
pages need the `cef` feature and an app bundle:

```sh
export CEF_PATH="$HOME/.local/share/cef"   # the first build downloads about 300 MB here
cargo build -p specular-shell --features cef
crates/specular-cef/scripts/bundle-macos.sh debug specular
"target/debug/specular.app/Contents/MacOS/specular" fixtures/input.canvas
```

For the winit shell, build `-p specular-app --features cef` and bundle with
`bundle-macos.sh debug` (the binary name defaults to `specular-app`). CEF only
runs from the `.app` layout. Run the inner binary directly so stdout, stderr
and the arguments are kept, and bundle again after every rebuild, because the
bundle holds copies.

To type-check the CEF code without downloading CEF:

```sh
cargo clippy --workspace --all-targets --features specular-app/cef,specular-cef/cef-dox -- -D warnings
```

## Run

There are two shells over one runtime.

```sh
cargo run -p specular-shell -- [OPTIONS] [FOLDER | FILE.canvas]   # binary `specular`, GPUI Kit
cargo run -p specular-app   -- [OPTIONS] [FOLDER | FILE.canvas]   # winit, built-in panels
```

`specular` is the app: Kit toolbar, sidebar, settings, the chat panel. It is
macOS only. `specular-app` is the older window with the built-in panels. It
stays until the user retires it (plan, "Cleanup tasks", rows 1 and 2). Both
take the same command line, `--bench` included. `--help` prints all of it.

| Flag | What it does |
|---|---|
| `FOLDER` | Open the folder as the space. A folder with no canvas gets the starter space |
| `FILE.canvas` | Open the file's folder as the space and show that file |
| `--space user\|scratch\|PATH` | See "Which space opens" |
| `--pages N` | A demo grid of N pages and no space (default 9, not with a path) |
| `--source synthetic\|cef` | Page backend. Default is `cef` when built with the feature |
| `--window WxH` | Window size in logical pixels |
| `--paint-policy electron-lod\|full-rate` | Page frame-rate and texture-scale tiers. Default is Electron's |
| `--chrome on\|off` | `off` draws no page borders, selection or comments. Keys and gestures still act |
| `--annotations N` | Seed N page-bound comments for a bench run |
| `--snapshot`, `--script` and their options | See "Snapshot and script" |
| `--bench` and its options | See "Bench" |

### Which space opens

Your real space is opt-in, because autosave writes into whatever is open.

- **A path wins.** `FOLDER`, `FILE.canvas` or `--space PATH`.
- **`--space scratch`** opens the scratch space: a copy of the starter space
  in this app's data folder, kept between launches. The title reads
  `Welcome (scratch space)`.
- **`--space user`** opens the space the Electron app has in Settings
  (`spacePath` in its `preferences.json`, read and never written), else the
  folder last chosen in this app. It is refused together with a path.
- **With none of those**, the two shells differ. `specular-app` opens the
  scratch space. `specular` opens the folder you chose in it, and shows the
  first-run view when there is none or the folder is gone.
- A `--bench`, `--snapshot`, `--script`, `--pages` or `--annotations` run
  opens no space. It shows one document and writes nothing.

The data folder is `~/Library/Application Support/Specular Native`.
`SPECULAR_NATIVE_CONFIG_DIR` moves it, which is how to try things without
touching your own preferences. Opening `fixtures/x.canvas` in a window makes
`fixtures/.specular/`, so copy a fixture out before opening it.

Do not run this app and the Electron app on one space at the same time. Each
rewrites the canvas index in `.specular/`.

### Driving it from outside

The running app answers the Electron app's HTTP routes. It takes port 29979
and `~/.specular/specular-mcp.json` unless an Electron app already has them.
Then it binds another port, writes `~/.specular/specular-native-mcp.json`,
and logs the `SPECULAR_DISCOVERY_FILE=` to pass the CLI. Started first, it
holds 29979 and the Electron app then starts with no API. A bench run and a
headless run start no server.

## Snapshot and script

Either binary draws a canvas into a PNG with no window, on the real GPU.
Nothing else is written.

```sh
cargo run -p specular-app -- --snapshot out.png fixtures/kitchen-sink.canvas
cargo run -p specular-app -- --snapshot out.png --snapshot-size 1200x800 \
    --snapshot-camera -400,-900,2 --snapshot-scale 2 FILE.canvas
cargo run -p specular-app -- --script steps.txt FILE.canvas
```

- The camera is `fit` (the default) or `x,y,zoom`, with the pan in screen
  pixels.
- A script is one step a line: `click x y`, `drag x1 y1 x2 y2`, `key cmd+z`,
  `type some text`, `tool shape`, `control shape.color`, `snapshot out.png`,
  `save out.canvas` and more. [`CLAUDE.md`](CLAUDE.md), "Looking at what it
  draws", has the whole list.
- The chrome (tab row, toolbar and dock) is the built-in one, drawn and
  clickable.
- Pages are synthetic unless the bundled binary is run with `--source cef`.
  Then the real pages load first and a script's input reaches an entered
  page.
- The clipboard and new Documents are kept in memory, and the clock moves
  only on `wait`, so a script draws the same frames every run.

The GPUI window has its own script driver for an agent with no hands:
`SPECULAR_SHELL_SCRIPT="wait 2000; click 587 58; shot /tmp/a.png; quit"`
posts real `NSEvent`s and captures the window. Set `SPECULAR_FLOAT_WINDOW=1`
with it. A covered window, a locked screen or a sleeping display draws
nothing, and the capture then looks like blank pages.

## Fixtures and scenarios

- `fixtures/kitchen-sink.canvas`: every kind in every style, with edges,
  comments, a Document and an image. Snapshot it after any change to
  `specular-scene` or `specular-compositor` and look at the PNG.
- `fixtures/input.canvas`: one page that flips its background on a press,
  with a `<select>` and a text input. For input, popup and IME checks.
- `fixtures/pages.canvas`: real pages beside stickies, for the CEF scenario.
- `fixtures/static-9`, `static-20`, `static-40`, `animated-20`: the spike's
  bench canvases.
- `fixtures/bench/`: canvases that each load one part of the renderer (500
  and 2,000 stickies, 300 drawings, 200 edges, 50 Documents, `mixed`), with
  `generate.py` and the run scripts.
- `fixtures/scenarios/`: eighteen whole sessions as scripts, `a` to `p`.
  `run.sh` runs them headless into `runs/qa/` and `check.py` compares the
  canvases they save. A PNG has no check, so open it. The
  [README there](fixtures/scenarios/README.md) lists each session.
- `fixtures/scenarios/cef/`: sessions that need the bundled CEF app.
  `pages.txt` is real pages, comments and scroll. `cli-pages.sh` runs every
  browse verb of the CLI against two pages. `sync.sh` runs a sync set of two
  widths against its own web server.
- `fixtures/scenarios/app/first-run.sh`: the first run of the release bundle
  on a throwaway home folder.

`runs/` is where all of these write. It is not in git.

## Bench

`--bench` runs in either window, and each line says which (`"shell"`: `winit`
or `kit`). Each profile prints one JSON line with frame stats and a `work` object: milliseconds a frame in update, view, cull,
shaping, batching, tessellation, build, glyphs, upload and submit, and the
most items, batches, draw calls, glyphs and triangles one frame drew.

```
specular     --bench all|slow-pan,slow-zoom,idle [--bench-target window|headless]
             [--bench-duration-ms N] [--warmup-ms N] [--window WxH] FILE.canvas
specular-app --bench ...        # the same run in the winit window
```

- `--bench-target headless` draws into a texture with no vsync and times each
  frame until the GPU is done. Its frame intervals mean nothing. Read `work`.
- `idle` is a seventh profile, run only when named. In a window it should
  report `framesDrawn: 0`.
- `fixtures/bench/run.sh LABEL headless|window synthetic|cef [winit|kit]`
  runs pan, zoom and idle over the bench canvases into `runs/perf/LABEL/`
  and prints a table. To compare the windows pass `--window 1600x960`: on a
  laptop screen macOS shrinks a 1600x1000 winit window to 968 high and
  leaves the Kit's alone. `idle.py` is a 30 second idle run with the process tree's CPU.
  `memory.py` prints the tree's footprint by kind of process.
- Synthetic pages upload on the CPU and always animate. Their numbers are
  not the real cost, and a canvas with them is never idle.

For numbers worth keeping, use a release build with CEF on a quiet machine:
power connected, display awake, window uncovered, nothing else building. A
build running elsewhere doubles the numbers.

```sh
cargo build --release -p specular-bench
cargo build --release -p specular-app --features cef
crates/specular-cef/scripts/bundle-macos.sh release
APP="$PWD/target/release/specular-app.app/Contents/MacOS/specular-app"
BENCH="$PWD/target/release/specular-bench"
"$APP" --pages 4 --bench slow-pan --warmup-ms 5000   # want "representative":true, "drawsWithoutTexture":0
```

`representative` is false when a presented frame came through a CPU upload.
A failing IOSurface import shows as `drawsWithoutTexture` above 0.

### The spike's comparison against Electron

The spike's fixtures were last run three times each during the performance
pass, on the Rust side only. This is that loop:

```sh
mkdir -p runs
for fx in static-9 static-20 static-40 animated-20; do
  pages="${fx##*-}"
  : > "runs/rust-$fx.jsonl"
  for i in 1 2 3; do
    "$APP" --window 1600x1000 --bench all --warmup-ms 14000 "fixtures/$fx.canvas" \
      >> "runs/rust-$fx.jsonl" 2>> "runs/rust-$fx.log" &
    pid=$!
    sleep 12 && "$BENCH" rss --pid "$pid" > "runs/rust-$fx-mem-idle-$i.json"
    "$BENCH" rss --pid "$pid" --peak-ms 30000 > "runs/rust-$fx-mem-peak-$i.json" &
    wait "$pid"; wait
  done
  "$BENCH" assemble "runs/rust-$fx.jsonl" --fixture "$fx" --pages "$pages" \
    --memory-idle "runs/rust-$fx-mem-idle-1.json" --memory-peak "runs/rust-$fx-mem-peak-1.json" \
    > "runs/rust-$fx.json"
done
```

Take the idle memory sample at 12 seconds, not 6. At 6 the process tree still
holds about 500 MB it lets go of by 10, and that early sample is what made
the spike read as heavier than Electron.

The Electron half (capture a trace from a release Electron build over HTTP,
convert it with `specular-bench electron-trace`, then `specular-bench
compare`) is written up in
[`crates/specular-bench/README.md`](crates/specular-bench/README.md). It has
not been rerun since the spike, so check it against the current Electron app
before relying on it. The spike's results table in
[`docs/plans/rust-cef-spike.md`](../docs/plans/rust-cef-spike.md) was never
filled in. The numbers are in the run log under "Performance".

## Bundle

```sh
export CEF_PATH="$HOME/.local/share/cef"
crates/specular-shell/scripts/bundle-app.sh
open "target/release/bundle/Specular Native.app"
```

This builds `Specular Native.app`: the GPUI Kit shell with CEF, the starter
space and an icon, signed ad hoc, so it runs on the Mac that built it. It
opens on a first-run view until you choose a space folder.
[`docs/native-app-bundle.md`](../docs/native-app-bundle.md) has the layout,
what it shares with the Electron app, and what distribution still needs.

`crates/specular-cef/scripts/bundle-macos.sh` is the plain development
bundle used above. It wraps a built binary in the layout CEF needs and adds
no icon, starter space or signature.

## Conventions

- Workspace lints in `Cargo.toml` are the contract: no `unwrap` or `expect`
  outside tests, `#[expect(lint, reason = "...")]` in place of `#[allow]`,
  docs on every public item, `// SAFETY:` on every `unsafe` block, and unsafe
  kept to the smallest module that needs it.
- `thiserror` in libraries, `anyhow` only in binaries.
- Every third-party dependency is declared once in
  `[workspace.dependencies]`. `gpui-kit` and `gpui-pre` are pinned with `=`,
  and a test fails if the lockfile has other versions.
- No wildcard arms over `Kind`, `Tool` or `Gesture`.
- Vocabulary follows [`CONTEXT.md`](../CONTEXT.md), which has a "Rust
  rebuild" section mapping its terms to the Rust names.
- The Electron app under `../src` is the behavior spec, not the structure
  spec. Do not edit it from here.
