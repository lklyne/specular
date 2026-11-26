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

### Phase 1, 2026-11-25. Teardown order fixed; Kit release baseline taken

- **Run:** debug CEF bundle, headless `--source cef --snapshot` runs on the spike's `cef-pages.canvas` (2 local pages) and `still-20.canvas`, before and after the fix; release CEF bundle, the Kit cases of S6's bench. **Read:** `Runtime::exit`/`poll_shutdown`, the Kit's close and Cmd+Q (`kit/shell.rs` `begin_exit`, `kit/canvas.rs`), the headless `shut_down`, the bench's `finish`. All four end through `Runtime::exit`.
- **Teardown:** the Kit had the spike's order. `exit` cleared its hosts but the compositor kept every page's imports and retired IOSurfaces until the window was dropped, after CEF had stopped (no render runs while closing, so nothing reclaimed them). Fix (`c5cdb079`): `Compositor::release_pages` forgets every page, waits for the GPU and drops the retired surfaces; `exit` calls it before the source stops. Before: 1 watchdog exit (code 2, 10.0 s) in 80 runs, the first after bundling; shutdown p50 ~65 ms (2 pages) / ~90 ms (20), tail to 2.0 s. After: 0 in 80, all exit 0, same p50; tails to 4 to 7.6 s remained in the 20-page set while another agent's `rustc` loaded the machine (load 28), so they are not attributed to the fix. The bench's 18 release window runs all exited 0. The Kit's rate (1/80) is far below the spike's 6/40, so the before/after difference is suggestive, not significant. No regression test: nothing headless can see CEF's watchdog, and the synthetic source does not care about the order.
- **Baseline** (`assets/post-rune-baseline.md`, with the exact commands): pan p99 15.7 / 15.4 / 9.2 ms at 9 / 20 / 40 pages, main thread 1.28 / 1.38 / 1.21 ms a frame, pump p95 0.20 / 0.14 / 0.14 ms; idle with still pages 175 wakeups/s and 3.2% CPU, 0 frames; memory 3,007 MB (browser + GPU) at 20 pages; key to present p50 8.3, p95 16.5 ms. No drift from S6's Kit column beyond its own spread. Not quiet: VM, Claude app, a dev server, and another agent compiling at the start; on AC.
- **For phase 2:** the idle wakeups (175/s, 30 pumps/s) are unchanged from S6, so the paint wake has a clean target. Pan 9 has 15 to 16 long frames in every run here and in S6, while 40 pages pans clean; worth a look when the paint path changes. `kit-baseline.sh` is in the baseline page; it needs the `archive/rune-spike` worktree.
