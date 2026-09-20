# Task: one three.js canvas for canvas-bg (orchestrate, build, validate)

You are orchestrating a multi-phase build in the Specular repo. Goal: replace
canvas-bg's three drawing surfaces (the dot grid, the group backgrounds and the
2D page canvas) with one `three/webgpu` scene. above-view keeps every DOM layer
it has today and is not touched. You plan, delegate, measure and decide.
Subagents write the code. Read CLAUDE.md first.

The canvas stays two layers on purpose. Notes paint above pages. That is the
product model in ADR 0014, which the sidebar's Notes and Pages sections teach,
and this task keeps it. What the port buys is modest and measured: about 5%
less CPU, one canvas where canvas-bg has three surfaces, and a scene that later
work (effects, or DOM rasterized into the scene) can build on. Build nothing
for that later work now.

This work ships, unlike the spike it follows. Each phase lands behind a flag,
is measured against today's renderer, and only then replaces it.

## What is already known (do not re-derive)

Read ADR 0038, section "Follow-up: one surface for pages and everything else",
including both spike write-ups at its end. The short version:

- Plain three with a `VideoFrameTexture` per page costs the same as
  hand-written WebGPU and 10 to 15 points less CPU than today's 2D canvas at
  thumbnail zoom. Gestures hold 120 fps. There is no performance case against
  three and no need to patch it.
- three 0.184 uploads each new frame with `copyExternalImageToTexture`. It is
  not zero-copy, and the copy costs nothing measurable.
- A native layer was measured and rejected. Electron draws every web view
  through one compositor layer, so native pixels cannot sit between web views.
- The CPU that remains is Chromium's capture. No renderer moves it. Do not
  chase CPU in this task beyond "not worse than today".

Reference code lives on `spike/webgpu-page-surface` and never merges. Read it,
do not cherry-pick it:

- `src/preload/canvas-bg.ts` and `src/renderer/canvas-bg/spike/useVideoFrameStore.ts`
  for the `VideoFrame` transport, the `ready` handshake and frame lifetimes.
- `src/renderer/canvas-bg/spike/threeRenderer.ts` for the working three setup:
  WebGPU backend check, y-down orthographic camera in device px, negative y
  scale with `DoubleSide`, `renderOrder` for stacking.
- `src/renderer/canvas-bg/spike/SpikePageSurface.tsx` for paint scheduling that
  mirrors `CanvasItemSurface`.

Findings from that spike that still bind you:

- A held `VideoFrame` keeps one of its page's 6 texture slots. Hold one per
  page, close the old one on replace, on page removal and on unmount. Frames
  posted before the renderer listens leak a slot until GC, so keep the `ready`
  handshake. Still call `imported.release()` in the preload.
- three uploads at render time and skips invisible meshes, so a hidden page
  never touches a closed frame. Frames that arrive before the renderer is ready
  never get an arrival callback, so treat an unseen frame at draw time as an
  arrival.
- React StrictMode mounts twice in dev. The first WebGPU device gets destroyed.
  That is expected, not an error.
- A second full-window canvas costs real compositor time. The end state is one
  canvas, not a three canvas stacked on the 2D ones.
- Renderer hot reload leaves the paint loop dead. Restart the app before any
  measured run, and after any preload or main change.

## The layer model to build

Today two renderers draw the canvas. `canvas-bg` draws the dot grid
(`CanvasGridSurface`), group backgrounds (`GroupBackgroundLayer`, DOM) and
pages with their shells and borders (`CanvasItemSurface`, 2D canvas).
`above-view` owns all input and draws everything else as DOM and SVG:
`StackedCanvasItems` (edges, drawings, shapes, stickies, file bodies), then
selection, guides, handles, popups and annotations. Pages live in the lower
renderer, so everything in above-view paints over them. That is intended.

The target is one ordered scene in `canvas-bg`, drawn in this order:

1. Grid, at the bottom.
2. Group backgrounds.
3. Pages and device-framed files in scene order, each painted whole before the
   next: shell, border, shadow, live texture, popup. Z-order stays data.
   `canvasItemDrawOrder.ts` already produces the list, focused page last. Keep
   using it.

above-view is out of scope. Stickies, notes, shapes, drawings, edges, file
bodies, selection, guides, handles, popups and annotations stay DOM there, and
above-view stays the input authority. The pointer router hit-tests with the
pure `hitTest()` in `src/shared/hit-test.ts` against the layout snapshot, not
against pixels, so nothing in this task should move a hit target.

The presence particle trail is also out of scope. It already runs
`three/webgpu`, but it mounts in the separate `agent-layer` window above
everything (`agent-layer/App.tsx` renders `AgentCursorLayer`), so it cannot
join this scene. ADR 0038's option 3 says it would. Correct that in ADR 0039.

## R3F or plain three

The user wants React Three Fiber if it costs nothing. It is not installed and
was not measured. Settle this in phase 1, before anything depends on it:

- Build the page layer once behind an interface that either host can drive.
- Measure plain three, then R3F with `frameloop="demand"` and `invalidate()` on
  frame arrival and camera change. Texture updates stay in refs, outside React.
  Check how R3F v9 accepts a `WebGPURenderer` through its async `gl` factory
  against the current R3F docs, not from memory.
- R3F wins if it is within 5 points of plain three at every zoom and holds
  120 fps in gestures. Adding `@react-three/fiber` is approved on that result.
  Anything else it needs is a stop-and-ask.
- If R3F loses, ship plain three and record the numbers.

## Phases

Branch from wherever the perf work has landed. Check whether
`claude/osr-texture-lod-perf` has merged to main. If it has, branch from main.
If not, branch from it. Other Claude panes share this worktree, so check
`git status` before staging, stage only files you touched, and leave
`.agents/skills/` untracked.

Each phase is one PR into a feature branch, even when it is large. Do not split
a phase for size. Split only on a real dependency gap.

0. Plan. Write `docs/plans/unified-three-canvas.md` with the phase list, the
   flag, the measurement protocol and per-phase acceptance checks. Draft ADR
   0039 for the decision, including why the canvas stays two layers, and link
   it from CONTEXT.md and docs/README.md. Read ADR 0014, ADR 0023 (rejected,
   read its postmortem), ADR 0036 and docs/interaction-layer.md section 6
   first. Main owns the camera. Do not move camera ownership into the
   renderer. ADR 0023 tried that and was abandoned.
1. Pages in three, behind a runtime flag, default off. Full parity with
   `CanvasItemSurface`: shells, borders, shadows, rounded corners, fill-mode
   pages, device-framed files, popup widgets with anchoring and close
   inference, first-frame requests, pruning. Settle R3F here. While the flag is
   on, `CanvasItemSurface` does not mount. Never run both.
2. Grid and group backgrounds move into the scene. The grid is a shader, not
   geometry per dot. Match today's dot size, spacing, fade with zoom and theme
   colors. Group backgrounds match today's fill, radius and border.
3. Flip the default, then delete: `CanvasItemSurface`, `usePageFrames`'s bitmap
   path and the `createImageBitmap` branch of the preload, `CanvasGridSurface`,
   `GroupBackgroundLayer`, and the flag. Update CLAUDE.md, CONTEXT.md,
   docs/architecture.md, docs/interaction-layer.md section 4.7 and ADR 0038's
   status. Fallow is a required check and fails on unused files and exports, so
   run it before the PR.

Camera changes arrive as a `camera` slice patch. Any renderer that projects
from the camera must coalesce to one draw per animation frame. Pages must stay
in lockstep with their chrome and with above-view's DOM during a pan.

## Delegation

Use subagents for the building, and the smallest model that fits each job.
Default to Sonnet for coding. Haiku is fine for mechanical edits and for
reading or summarizing files. Do the planning, the synthesis and the measuring
yourself.

- Give each subagent its own files so parallel agents cannot collide. Fix the
  interface between them yourself before you launch them, and put it in both
  briefs word for word.
- Tell each subagent what it may not touch, that it must not commit, stash,
  switch branches or run the app, and what to report back.
- Run independent subagents in parallel, in one message. In phase 1 the
  transport, the page meshes and the popup path are independent once the scene
  interface exists. In phase 2 the grid and the group backgrounds are.
- Per-step gate is `pnpm typecheck` and `pnpm test:unit`. Add
  `pnpm test:integration` when a step touches `src/main/runtime/`, IPC or
  persistence. No smoke runs inside the loop and no end-of-task review pass.
- Read a subagent's diff before you build on it. The spike's one wiring bug was
  a dedupe that recorded a payload before the call that could fail.
- Keep a journal at `docs/plans/unified-three-canvas-journal.md`, one entry per
  step, with what landed, what was measured and what is next, so a fresh
  context can resume.

Tests follow tests/README.md. Pure helpers such as draw ordering, projection
and popup placement get unit tests. This work adds no entity kind and no
runtime mutator, so it owes no new integration coverage unless a phase changes
main.

## Validating each phase

Only one dev app can run at a time (ports 29979 and 9333), so measurement is
serial and you do it, not the subagents.

- The app runs in the herdr split pane. Find it with
  `herdr pane list --workspace "$HERDR_WORKSPACE_ID"` and confirm with
  `herdr pane process-info` that it is this worktree's Electron before you
  touch it. Restart with `herdr pane send-keys <pane> C-c`, wait about 6 s,
  `herdr pane run <pane> "pnpm dev"`, poll `http://127.0.0.1:29979/canvas`
  until it stops returning 000, then wait about 16 s for 60 pages to load.
  Do not kill a dev app you cannot identify as this one.
- The secret rotates per launch. Read `~/.specular/specular-mcp.json` and send
  it as `x-specular-secret`.
- Measurement scripts from the spike session are in
  /private/tmp/claude-501/-Users-lyleklyne--herdr-worktrees-telescope-worktree-clear-river-ac42/aceb1f7b-d2d5-4eab-b026-023a9875212d/scratchpad/
  Copy them to your own scratchpad first. If they are gone, rebuild them:
  - measure.mjs: CPU per process group from deltas of `cumulativeCpuSeconds`
    on GET /perf/metrics (the cpuPercent field is unreliable), plus
    WindowServer from `ps -o time= -p $(pgrep -x WindowServer)`, pulsing
    GET /selection every 2 s so idle throttling does not zero the numbers.
  - zoom.mjs: `window.electronAPI.zoomSet(z)` on the toolbar target over CDP
    port 9333. /camera/focus only pans.
  - gesture.mjs and gesturecpu.mjs: canvas-bg rAF rate during a zoom ramp and
    pan sweep, and CPU during a 12 s zoom oscillation between 0.1 and 0.25.
  - hosts.mjs: summary of GET /perf/page-hosts. gpumem.mjs: `footprint -p` on
    the GPU process.
  - shotnow.mjs: POST /window/screenshot to a PNG.
- Test canvas: the "Perf test (animated)" tab, 30 animating and 28 static
  pages, pan (518, 158).

For every phase, flag off against flag on, in fresh launches:

1. Steady-state CPU by process at zoom 0.1, 0.352 and 0.6, app total and app
   plus WindowServer. Two launches, two samples each. Report the spread.
   Pass is flag on no worse than flag off by more than 5 points.
2. Gestures. rAF rate, p95 and frames over 25 ms during the ramp and sweep.
   Pass is 120 fps with no frame over 25 ms, matching flag off.
3. GET /perf/page-hosts. `sendFailures` and `framesDroppedForPoolPressure`
   stay at 0. `outstandingTextures` at rest is at most 1 per page.
4. GPU-process memory, both launches.
5. Pixels. Screenshot flag off and flag on at each zoom and compare them. Pages
   must land inside their borders to the pixel, upright, with correct corners
   and colors. Check a popup (`<select>` and a date input on the fixture page),
   a fill-mode focus session, a device-framed file, dark theme, a window
   resize, and a tab switch away and back.
6. Lockstep. Run POST /perf/pan-zoom/visual-run and scan the captured frames
   for pages separating from their borders, from the grid, or from
   above-view's DOM during a pan. It must be no worse than flag off.
7. Phase 2 only. Compare the grid at zoom 0.1, 0.352, 1 and 2 in both themes,
   and a group with pages inside it, against flag off.

Record the numbers in the journal and in the PR body. When a phase passes, give
the user a short manual smoke checklist in chat, one per branch before merge,
not one per step.

## Deliverables

1. The plan doc, the journal and ADR 0039, kept current.
2. One PR per phase into the feature branch, with measured numbers and manual
   test steps in the body, then one integration PR for the user to review.
   With stacked PRs, retarget the child before merging its base.
3. Run the unslop skill on every PR body, ADR, plan and doc change before it
   goes out.
4. A final table in ADR 0039 of today's renderer against the finished one,
   and an honest list of what was cut or deferred.
5. Leave the app running on the default path at zoom 0.352, pan (518, 158).

## When to stop and ask

- R3F needs any dependency beyond `@react-three/fiber`, or it fails its gate
  and you think it should ship anyway.
- A phase fails its CPU or gesture check and one round of fixes does not
  recover it. Report the numbers. Do not tune for hours.
- Anything in `src/renderer/above-view/` would have to change, or a hit target,
  persistence or undo behavior would move. Pixels move in this task. Behavior
  does not.
- You need to change `page-host.ts` beyond the frame transport, or anything in
  `src/main/runtime/space-*.ts`.
- Text, notes, shapes, drawings or edges would have to move into the scene to
  make something work. That is a separate decision with its own spike.
- Lockstep between the scene and above-view's DOM cannot be held during a pan.
  See issue #410 and ADR 0023 before trying anything clever.
- Results between runs disagree by more than about 10 points and you cannot
  explain why.
- The work has gone churny, with the same files reworked across several steps
  and no measured progress. Stop, write up where it stands, and hand the user
  the manual test list.
