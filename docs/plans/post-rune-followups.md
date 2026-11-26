# After the Rune spike: the page path, idle, edges, the runner and actors

**For:** [ADR 0053](../adr/0053-page-frames-follow-the-display.md), [ADR 0052](../adr/0052-one-runner-one-state-document.md) and [ADR 0054](../adr/0054-every-editor-is-an-actor.md), plus two findings from [the Rune spike](./rune-spike.md).
**Started:** 2026-11-25, after [ADR 0051](../adr/0051-rune-shell-around-the-compositor.md) was deferred. Everything here is on the Kit shell. Results go in the log at the end, one entry a phase.

## Branches

Every phase builds on `claude/post-rune-followups`, which sits on `claude/rune-spike-record` (the ADRs and `Compositor::with_samples`), which targets `rust-native`. One commit or a few per phase; a phase is meant to become one PR. Phase 5 runs in parallel on its own branch, `claude/analytic-edges`, off `claude/rune-spike-record`.

## Phases

1. **Teardown order and a baseline.** The spike found that stopping CEF while the compositor still holds page IOSurfaces runs into Chromium's teardown watchdog (exit 2 in 6 of 40 runs, up to 9 s), and dropping the textures first fixed it. Check whether the Kit shell has the same order in `specular-shell` (`Runtime::poll_shutdown`, the window's close path) and fix it if so. Then take a release baseline of the Kit with the S6 bench (its scripts are at the tag `archive/rune-spike`, `native/bakeoff/rune-shell/scripts/bench-s6.sh` and `tools/s6_*.py`): idle wakeups and CPU with still pages, pans at 9, 20 and 40 pages, key to present. Later phases compare against it.
2. **A paint wakes the canvas** (ADR 0053 item 2). The page source signals the window when a frame is queued, so a paint does not wait for the next display tick or the 50 ms idle turn. The 30 Hz pump fallback and the idle turn stop being what delivers a frame.
3. **The entered page and a page in Fill paint at the display's rate** (ADR 0053 item 1), and the smaller fixes listed under its Decision: newest paint dropped at the six-frame cap, the retire callback registered only when something is already retired, pages painting while the window is occluded, the 250 ms forced-frame tail after a pointer move in an entered page, the LOD running twice a turn. Then try app-driven begin frames for the entered page (item 4) and adopt them only on the numbers the ADR names.
4. **Idle cost.** Measure again against phase 1's baseline (S6 had the Kit at 176 wakeups/s and 2.9% CPU with still pages, Rune at 32/s and 1.3%). Find what still wakes the Kit with nothing moving and remove it.
5. **Analytic edges, single-sampled pass** (ADR 0053 item 3), in parallel. Lyon meshes get their own anti-aliasing (feathered geometry or SDF segments), the pass runs at `Compositor::with_samples(.., 1)`, and the multiply blend is kept. S4 listed what stair-steps at 1 sample: freehand strokes, highlighters, edges, arrowheads, dashed outlines, and the outlines of diamonds, triangles, hexagons, parallelograms, chevrons and cylinders. Ships with a GPU readback test, as the ADR says.
6. **ADR 0052 without Rune.** Revise the ADR for the Kit shell first (which parts stand, which assumed Rune's headless window), then build what stands.
7. **ADR 0054, every editor is an actor.** Its Cmd+Z question is open and is Lyle's to answer. Build what does not depend on the answer; stop before what does and ask.

## Rules

- Per change: `cargo check` / clippy and tests for the touched crates. Before each phase's commit: the gate in `native/CLAUDE.md` (fmt, workspace clippy, workspace tests). No window smoke test per step; one by-hand pass per branch at the end, by Lyle.
- Release builds and `bundle-macos.sh` are allowed for measuring (phases 1, 3, 4), with `CEF_PATH="$HOME/.local/share/cef"`. Numbers only from release, three runs a case, with what else was running written down.
- Do not start or kill processes you did not start. Commit only your own files, by explicit path. Do not push; the orchestrator pushes.
- A phase that cannot finish commits what compiles, writes `BLOCKED` and the reason in the log, and stops.

## Log

### Phase 5, 2026-11-25, Mac (M3 Max). Analytic edges, single-sampled pass: done

- **Approach:** feathered geometry. Each lyon fill is tessellated from its outline moved half a device pixel in (`scene_pass/inset.rs`), each stroke at its width less a device pixel (thinner ones keep a quarter-pixel core and fade by the difference). `scene_pass/fringe.rs` then welds lyon's duplicate vertices, drops its zero-area triangles, finds the edges only one triangle has and adds a band one device pixel wide outside them, coverage 1 to 0, with outer vertices shared along the corner's miter. Interior vertices never move, so nothing turns over and no inner edge is feathered: no seams, and the multiply pipeline blends each pixel once. The band's offset is a per-vertex `extrude` in device pixels that `vs_mesh` divides by the scale factor, so it stays one pixel at any zoom, scale and mesh-cache zoom. SDF shapes, text and quads are untouched.
- **Default:** `Compositor::new` is single-sampled; `with_samples`, `SCENE_SAMPLES` and `scene_pass/target.rs` are deleted (no caller wanted 4x, and the ADR wants the attachment gone). glyphon renderers are built at 1 sample. `specular-web` checks on wasm32; nothing past WebGL2 is used (four vertex attributes, no multisampling).
- **Tried and dropped:** moving the outline's own vertices in by half a pixel. Long slivers from lyon's fans turned over, covered the band at full coverage (heavier, aliased edges on the freehand X) and double-multiplied highlighter corners (dark specks). A band outside the true edge from 0.5 coverage needed no inset but looked stair-stepped.
- **Evidence:** snapshots of the kitchen sink at zoom 0.25 (scale 1 and 2), 1 (scale 1 and 2) and 3 (the camera's maximum), before (4x, `origin/claude/rune-spike-record`) and after, read as zoomed crops of the drawings, highlighters, outlines, dashes, edges and arrowheads. After matches 4x and has finer gradation on its edges (4x has five levels). Arrowhead tips are a little blunter (miter limit); fills thinner than a device pixel draw slightly heavier. [Crop](../adr/assets/0053/mesh-edges-4x-vs-1x.png).
- **Test:** `mesh_edges_are_part_covered_and_blend_once` (`specular-compositor/tests/gpu.rs`): a diagonal stroke has part-covered pixels and a whole middle; a multiplied wedge tints white once, part-tints its edge and no pixel is darker than its inside. Mutations: no band fails the first (0 part-covered), the band emitted twice fails the last (darkest 64 against 128).
- **CPU (release, headless `--bench slow-pan,slow-zoom` on the kitchen sink, 1200x800, three runs each, other worktrees idle but not checked):** tessellation mean 0.17-0.18 to 0.38-0.42 ms a frame in pan, 0.28-0.29 to 0.51-0.54 ms in zoom; frame CPU mean 1.19-1.33 to 1.67-1.73 ms (pan), 2.57-2.63 to 2.73-2.91 ms (zoom). Most triangles in a frame 6,535 to 17,567 (pan). The pan cost is the screen-space edges, which are tessellated every frame; caching them is the next cut. GPU time not measured.
- **Left:** a window pass by hand (`--snapshot` only so far); GPU time and memory at 1x in the Kit window.
