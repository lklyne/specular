# native/ — Rust + CEF shell spike

A measurement vehicle, not a rewrite. It answers one question: **would a Rust
shell that embeds Chromium through CEF offscreen rendering (OSR) beat the
Electron app on pan/zoom frame times, memory, and input latency?** The plan,
hypothesis, and results table live in
[`docs/plans/rust-cef-spike.md`](../docs/plans/rust-cef-spike.md).

The shape mirrors what ADR 0038 shipped in Electron — every page is an
offscreen browser painting GPU shared textures, composited on one surface —
so the comparison isolates the shell (Electron main + canvas-bg renderer vs.
one Rust process with wgpu), not the compositing model.

## Crates

| Crate | Kind | Owns |
|---|---|---|
| `specular-core` | lib | Camera math, page model, `PageFrame` / `PageSource` contracts, input model, yrs-backed canvas document (lossless `.canvas` round trip), JSON Canvas types, synthetic page source |
| `specular-compositor` | lib | wgpu renderer: dot grid + page textures under the camera, one-draw SDF shape overlay (borders, outlines, handles, pins), popup layers, shared-texture retirement and the per-page cap of 6; IOSurface -> Metal -> wgpu import on macOS |
| `specular-cef` | lib | CEF OSR `PageSource` (`--features cef`) and the CEF-free helpers it is built from (input translation, coords, config). See [`crates/specular-cef/README.md`](crates/specular-cef/README.md) |
| `specular-bench` | lib + bin | Gesture profiles ported from `src/shared/pan-zoom-perf-test.ts`, frame stats in the ADR 0038 lab's field names, input latency, process-tree footprint and RSS, Electron trace converter, `compare`. See [`crates/specular-bench/README.md`](crates/specular-bench/README.md) |
| `specular-app` | bin | winit shell wiring a page source, the compositor, the camera, input forwarding, and `--bench` |

Dependency direction: `core` <- `compositor`, `cef`, `bench` <- `app`. Only
`core` types cross crate boundaries; `core` has no GPU, window, or CEF deps.

`fixtures/` holds the bench canvases both shells load: `static-9`,
`static-20`, `static-40` (real sites, 1280x800 CSS each), `animated-20`
(a `data:` page with a rAF counter and a CSS animation) and `input` (a
`data:` page that flips its background on `pointerdown`/`keydown`, with a
`<select>` and a text input for the popup and IME checks).

## Build, lint, test (any platform)

```sh
cd native
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The default build uses the synthetic page source (animated CPU frames). It
runs anywhere, and it is **not representative**: never put its numbers in the
results table. The compositor's GPU tests skip with a message when no wgpu
adapter exists.

Type-checking the CEF code without downloading CEF (cef's docs.rs mode; it
cannot link or run):

```sh
cargo clippy --workspace --all-targets --features specular-app/cef,specular-cef/cef-dox -- -D warnings
# macOS-only code from a Linux box:
rustup target add aarch64-apple-darwin
cargo clippy --target aarch64-apple-darwin --workspace --all-targets \
  --features specular-app/cef,specular-cef/cef-dox -- -D warnings
```

## App

```
specular-app [--source synthetic|cef] [--pages N | FILE.canvas]
             [--bench all|id,id,... [--warmup-ms N]] [--window WxH]
             [--paint-policy electron-lod|full-rate]
             [--chrome on|off] [--annotations N]
```

Scroll pans; Cmd/Ctrl+scroll and pinch zoom about the cursor (same factor as
the Electron app: `zoom -= deltaY * 0.002`, clamped to 0.02..3). Click a page
to focus it; pointer, wheel, keys and IME then go to that page. Click empty
canvas to clear focus. Logs go to stderr (`RUST_LOG=debug` for more); on
exit an interactive session logs its input-to-present latency summary.

With the chrome layer on (the default), Alt+drag on a page moves it and
dragging a corner handle of the selected page resizes it (the page's CSS
viewport changes once, on release). `C` toggles the comment tool while no
page has keyboard focus (a focused page gets the key); with the tool armed,
drag on the canvas to draw a region, which is bound to the page the drag
started over or, off any page, to the canvas; a click creates nothing.
`Escape` always cancels the drag, leaves the tool and clears page focus, and
is not forwarded to pages.

### Chrome layer

`--chrome on|off` (default on) switches the per-frame canvas UI the real app
draws: a border on every page (stronger under the cursor), the selection
outline with four screen-sized resize handles, and comment annotations (a
translucent region plus a circular pin at its top-left corner; page-bound
ones move and scale with their page). It exists so the benchmark can answer
whether the shell's frame-time advantage survives UI drawn every frame:
everything goes through the compositor's one-draw shape layer, `--chrome off`
draws no shapes and has no selection or tool, and `--annotations N` (chrome
on only) seeds N page-bound annotations so a run draws a known amount.
`--bench` with chrome on starts with the first page selected. Each bench line
records `chrome`, `annotations` and `maxShapesDrawn`; `compare` prints them
and warns when two runs differ.

It is a cost model, not the product's UI: there is no text (pins are plain
circles), no toolbar or panels, no cursor changes beyond the tool crosshair,
and annotations are not persisted.

`--bench` waits `--warmup-ms` (default 2000) for pages to load, runs each
profile from the same start camera, one step per presented frame at the
monitor's refresh interval, and prints one JSON line per profile to stdout
(the bench crate's `PhaseReport` fields plus `source`, `pages`,
`representative`, `stepIntervalMs`, `maxPaintToSubmitMs`, `chrome`,
`annotations`). Then it exits.

## Morning run on macOS (Apple Silicon)

The representative configuration. Same Mac, built-in 120 Hz display, power
adapter connected, other apps closed, for both shells. Every block is
copy-paste from the repo root unless it says otherwise.

### 1. Build

```sh
cd native
export CEF_PATH="$HOME/.local/share/cef"   # first cef build downloads ~300 MB here
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cargo build --release -p specular-bench
cargo build --release -p specular-app --features cef
crates/specular-cef/scripts/bundle-macos.sh release
APP="$PWD/target/release/specular-app.app/Contents/MacOS/specular-app"
BENCH="$PWD/target/release/specular-bench"
mkdir -p runs
```

CEF must run from the `.app` bundle (framework and helper apps); run the
inner binary directly so stdout, stderr and argv are kept. Re-run
`bundle-macos.sh` after every rebuild: the bundle holds copies.

### 2. Smoke: synthetic, then CEF interactive

```sh
"$APP" --source synthetic --pages 9 --bench slow-pan   # one JSON line, "representative":false
"$APP" fixtures/input.canvas                            # interactive; close the window when done
```

In the interactive window check, and note the answers for the capability
table in the plan: the page loads; clicking it flips the background (that
proves input forwarding); the `<select>` opens a popup drawn over the page
at the right place; typing with a Japanese/Chinese IME into the text input
shows marked (underlined) text before commit; the candidate window sits next
to the caret. Then confirm the zero-copy path is the one in use:

```sh
"$APP" --pages 4 --bench slow-pan --warmup-ms 5000   # want "representative":true, "drawsWithoutTexture":0
```

`representative` turns false when any presented frame came through a CPU
upload (CEF fell back to `OnPaint`). A failing IOSurface import instead
shows up as `drawsWithoutTexture` > 0 with `failed to import` warnings on
stderr.

Closing the interactive window prints an `inputLatency` JSON line on
stdout and logs `shared-surface import cache` hits/misses on stderr. Keep
the latency line: `"$APP" fixtures/input.canvas >> runs/rust-input.jsonl`,
click and type in the page for a minute, close, then append it to a bench
file before `assemble` (`cat runs/rust-static-9.jsonl runs/rust-input.jsonl`).
Misses should stay near Chromium's pool size (a handful per page); misses
close to the number of paints mean the import cache is not hitting.

If CEF fails to start, read "Known risks to check first" in
`crates/specular-cef/README.md` (`CefAppProtocol` on winit's `NSApp`,
retained IOSurface tearing, coded vs visible size, 120 fps).

### 3. Rust/CEF runs: 9, 20, 40 static pages and 20 animated, three runs each

```sh
for fx in static-9 static-20 static-40 animated-20; do
  pages="${fx##*-}"
  : > "runs/rust-$fx.jsonl"
  for i in 1 2 3; do
    "$APP" --window 1600x1000 --bench all --warmup-ms 8000 "fixtures/$fx.canvas" \
      >> "runs/rust-$fx.jsonl" 2>> "runs/rust-$fx.log" &
    pid=$!
    sleep 6 && "$BENCH" rss --pid "$pid" > "runs/rust-$fx-mem-idle-$i.json"
    "$BENCH" rss --pid "$pid" --peak-ms 30000 > "runs/rust-$fx-mem-peak-$i.json" &
    wait "$pid"; wait
  done
  "$BENCH" assemble "runs/rust-$fx.jsonl" --fixture "$fx" --pages "$pages" \
    --memory-idle "runs/rust-$fx-mem-idle-1.json" --memory-peak "runs/rust-$fx-mem-peak-1.json" \
    > "runs/rust-$fx.json"
done
grep -h '"representative":false' runs/rust-*.jsonl && echo "NON-REPRESENTATIVE LINES ABOVE: fix before comparing"
```

`--window` sets the window's logical size; match it to the Electron window
you compare against (the platform default is far smaller, so each frame
composites fewer pixels).

The app runs Electron's page-host paint LOD by default (`--paint-policy
electron-lod`: 60/30/15 fps by on-screen scale, texture scale after the
camera settles, no painting off-screen), so both shells do the same work per
page; the policy is recorded in every line and `compare` warns on a
mismatch. `--paint-policy full-rate` is there to measure the LOD's own cost.

The idle sample lands inside the 8 s warmup, after pages have loaded; the
peak sampler covers the rest of the run (it samples for 30 s, longer than
the six profiles take). Each `rust-*.json` holds
all three runs' phases; `compare` takes the median per cell.

### 4. Electron baseline (Specular release build on `main`)

Copy `native/fixtures/*.canvas` into your Specular space folder. For each
fixture: open it as the active tab, zoom so the camera roughly matches the
Rust start camera (pan 40,40, zoom 0.25), wait for pages to settle, then run
this from `native/` (set `fx` each time):

```sh
fx=static-20; pages="${fx##*-}"
SECRET=$(jq -r .secret ~/.specular/specular-mcp.json); H="x-specular-secret: $SECRET"
PID=$(pgrep -xo Specular)    # `pgrep -xo Electron` under pnpm dev
for i in 1 2 3; do
  "$BENCH" rss --pid "$PID" > "runs/e-$fx-mem-idle-$i.json"
  curl -s -H "$H" localhost:29979/perf/page-hosts > "runs/e-$fx-hosts-before-$i.json"
  "$BENCH" rss --pid "$PID" --peak-ms 30000 > "runs/e-$fx-mem-peak-$i.json" &
  curl -s -X POST localhost:29979/perf/pan-zoom/run -H "$H" \
    -H 'Content-Type: application/json' -d '{}' > "runs/e-$fx-run-$i.json"
  wait
  curl -s -H "$H" localhost:29979/perf/page-hosts > "runs/e-$fx-hosts-after-$i.json"
  "$BENCH" electron-trace --response "runs/e-$fx-run-$i.json" --fixture "$fx" --pages "$pages" \
    --memory-idle "runs/e-$fx-mem-idle-$i.json" --memory-peak "runs/e-$fx-mem-peak-$i.json" \
    --page-hosts-before "runs/e-$fx-hosts-before-$i.json" \
    --page-hosts-after "runs/e-$fx-hosts-after-$i.json" > "runs/e-$fx-$i.json"
done
jq -s '.[0] + {phases: (map(.phases) | add)}' runs/e-$fx-[123].json > "runs/electron-$fx.json"
```

For `animated-20` the trace has no quiet gaps between profiles, so run one
profile per request instead (`-d '{"profiles":["slow-pan"]}'` and
`electron-trace ... --profiles slow-pan`), once per profile id; see
`crates/specular-bench/README.md`.

### 5. Compare

```sh
for fx in static-9 static-20 static-40 animated-20; do
  "$BENCH" compare "runs/electron-$fx.json" "runs/rust-$fx.json" > "runs/compare-$fx.md"
done
cat runs/compare-*.md
```

Copy the worst-profile numbers into the results table in
[`docs/plans/rust-cef-spike.md`](../docs/plans/rust-cef-spike.md) and attach
`runs/`. A run that `compare` flags "Not representative" does not count.

## Conventions

- Workspace lints in `Cargo.toml` are the contract: no `unwrap`/`expect`
  outside tests, `#[expect(lint, reason = "...")]` instead of `#[allow]`,
  docs on every public item, `// SAFETY:` on every `unsafe` block, and unsafe
  confined to the smallest module that needs it (the macOS IOSurface import
  and CEF callbacks).
- `thiserror` in libraries, `anyhow` only in `specular-app` / `specular-bench`'s bin.
- Every third-party dependency is declared once in `[workspace.dependencies]`.
- Vocabulary follows [`CONTEXT.md`](../CONTEXT.md): page, canvas item,
  entity, page host, texture scale, frame-rate LOD, painting policy.
