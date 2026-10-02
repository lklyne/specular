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
| `specular-core` | lib | Camera math, page model, `PageFrame` / `PageSource` contracts, input model, yrs-backed canvas document, JSON Canvas types, synthetic page source |
| `specular-compositor` | lib | wgpu renderer: dot grid + page textures under the camera; IOSurface -> Metal -> wgpu import on macOS |
| `specular-cef` | lib | CEF OSR `PageSource`. Empty unless built with `--features cef` |
| `specular-bench` | lib + bin | Gesture profiles ported from `src/shared/pan-zoom-perf-test.ts`; frame-interval stats in the ADR 0038 lab's field names |
| `specular-app` | bin | winit shell wiring a page source, the compositor, and the camera |

Dependency direction: `core` <- `compositor`, `cef`, `bench` <- `app`. Only
`core` types cross crate boundaries; `core` has no GPU, window, or CEF deps.

## Build, lint, test (any platform)

```sh
cd native
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The default build uses the synthetic page source (animated CPU frames). It
runs anywhere, and it is **not representative**: never put its numbers in the
results table.

Type-checking the CEF code without downloading CEF (cef's docs.rs mode; it
cannot link or run):

```sh
cargo clippy --workspace --all-targets --features specular-app/cef,specular-cef/cef-dox -- -D warnings
# macOS-only code from a Linux box:
rustup target add aarch64-apple-darwin
cargo clippy --target aarch64-apple-darwin --workspace --all-targets \
  --features specular-app/cef,specular-cef/cef-dox -- -D warnings
```

## Run on macOS (Apple Silicon) — the representative configuration

```sh
cd native
cargo run -p specular-app --release --features cef -- path/to/file.canvas
```

The first `--features cef` build downloads the CEF binary distribution
(~300 MB) from `cef-builds.spotifycdn.com`; set `CEF_PATH` to reuse a copy.
CEF on macOS must run from an `.app` bundle holding the CEF framework and the
helper apps; `specular-cef` owns producing that bundle and this section
records the exact command once it lands. With no `.canvas` argument the app
lays out a 3-column demo grid of pages.

Controls: scroll pans, Cmd/Ctrl+scroll zooms (same factor as the Electron
app: `zoom -= deltaY * 0.002`, clamped to 0.02..3).

## Bench

```sh
cargo run -p specular-bench --release        # prints the gesture plan
```

The six profiles (`slow-pan`, `slow-zoom`, `fast-diagonal-pan`,
`slow-pan-zoom`, `fast-pan-zoom`, `zoom-out-then-pan`) match the Electron
test value for value. The comparison method is in the plan doc.

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
