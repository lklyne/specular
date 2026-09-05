# Changelog

All notable changes to Specular will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/). Do not use prerelease suffixes (`-alpha.N`, `-beta.N`). `update.electronjs.org` filters based on GitHub's "prerelease" release flag (not the SemVer suffix), so tagged-alpha versions *can* reach clients — but mixing SemVer prerelease ordering with GitHub's flag is a footgun. Just increment patch/minor versions and keep releases as non-prereleases.

## [Unreleased]

## [0.8.1] - 2026-09-05 — Canvas agent threads for everyone

0.8.0 was published out of order, ahead of 0.7.2, so auto-updates skipped the
canvas agent threads release. 0.8.1 ships no new changes — it brings every
install up to the same code as 0.7.2.

If you're coming from 0.8.0, here's what you missed:

- **Chat with an agent about your canvas.** The right panel is a conversation:
  comments you drop on pages queue into a draft, and Send hands the whole batch
  to Claude in one run. Follow-ups continue the same session, and past threads
  stick around in a switcher.
- **The composer shows what you're talking about.** A context chip tracks your
  selection, and a second chip switches the model between Opus, Sonnet, and
  Haiku.

## [0.7.2] - 2026-09-02 — Canvas agent threads

### New

- **Chat with an agent about your canvas.** The right panel is now a
  conversation. Comments you drop on pages queue into a draft, and Send hands
  the whole batch to Claude in one run. Follow-ups continue the same session,
  and past threads stick around in a switcher. Transcripts live in your space
  folder as plain JSON.
- **The composer shows what you're talking about.** A context chip tracks your
  selection with a matching icon: the canvas tab, a page favicon, a sticky, a
  shape, a comment, or a multi-select count. A second chip switches the model
  between Opus, Sonnet, and Haiku.
- **Per-note fonts.** Each text entity can carry its own font, and text bodies
  wrap to their width.

### Improvements

- Comment fixes run through the Claude Agent SDK instead of spawning the CLI,
  so runs start faster and stream events more reliably.
- Fix runs default to auto permissions, and a follow-up comment continues the
  run instead of starting over.

### Fixes

- Auto-bind takes over an origin that was bound to a different repo.
- Modifier presses inside an entered page go to the page, not the canvas.

## [0.7.1] - 2026-08-24 — Zoom and border fixes

### Fixes

- **Panning right after a zoom no longer stutters.** The snapshot encode that runs
  when a zoom settles did all its work in one block, so pan input piled up behind it
  and released as a jump. It now waits for you to stop moving the camera, then
  encodes one frame at a time. Zooming out from a zoomed-in page hit this hardest.
- **Page borders are full thickness on all four edges.** On a Retina display the top
  border read about half thickness. The stroke was landing on a half-pixel boundary,
  and the bezel's shadow was darkening the other three edges so the top looked thin
  by comparison.
- Pages without a device shell no longer double-stroke their border, and the ring
  around a bezel follows the corner curve instead of leaving a hairline sliver at
  the corners.

## [0.7.0] - 2026-08-23 — Faster and less battery hungry canvas

### New

- **Focusing a note fills the window.** Focus a markdown note the way you focus
  a page and it grows from its spot on the canvas into a full-window editor,
  then shrinks back when you leave.

### Improvements

Most of this release went into making things way faster. Panning, zooming, and dragging are a smooth 120fps with some magic tricks.

- **Zoom stays sharp.** Pages freeze to rasters while you zoom and go live once
  you land. The frame captured at the end of a zoom is high resolution now, so
  zooming into a page no longer shows a blurry blow-up of the zoomed-out view.
- **Panning got much cheaper.** The dot grid was sloppily drawing points with multiple draw calls, now it's one. 6.4ms down to 0.15ms per frame at low zoom. A pan also keeps the page frames it already captured instead of throwing them out and re-capturing every page a moment later.
- **Dragged pages keep up.** A dragged page moves as a raster, so its border and
  chrome stay locked to it instead of trailing behind.
- Page borders, device shells, and group titles draw on one canvas layer instead
  of a DOM layer each.
- Camera moves apply the moment they arrive instead of waiting on a 16ms timer,
  and each renderer receives only the state it reads.

### Battery

- **Idle pages go quiet.** A page you aren't using freezes instead of running
  CPU-throttled. Fifteen idle pages used to sit at about 86% CPU. Now it's near
  zero, and screenshots still work on a frozen page.
- An HTML file running an animation loop stops ticking the display at 120Hz
  while the app sits idle.

### Fixes

- Shape styling and note labels survive a relaunch. Loading a `.canvas` file
  dropped fill, borders, text alignment, and labels, and the next save wrote
  that loss to disk.
- A mixed selection of a group plus loose items drags as one unit and shows its
  bounding box, no matter which item you grab.
- The first panel you open after launch no longer hangs for most of a second.
- Group membership sticks to stickies and file cards.

### Misc

- Bundled agent-browser updated to v0.34.0.

## [0.6.0] - 2026-08-03 — Markdown Live Preview, Your Own Space Folder

### New

- **Markdown notes read like prose.** `.md` notes render formatted inline — the syntax collapses until your cursor lands on that line, so you're always editing the real source. Cmd+B/I/K work, Enter continues a list, and links open as pages on the canvas with an edge back to the note.
- **Sticky notes grow with their text** and pick up light formatting: bold, strikethrough, and bullets from the selection popup. Side handles rewrap the text, corner handles scale it.
- **You choose where your work lives.** Onboarding asks for a space folder, and Settings → General can change it later — with the option to move your canvases along or start fresh. If the folder goes missing, the app asks instead of quietly saving somewhere else.

### Improvements

- Upgraded to Electron 43 — Chromium 150, faster cold start.
- Folder pickers open next to what you're picking for instead of Downloads.
- Rounded rectangles and pills keep their corner radius at any size.
- Panels read cleaner in light mode.
- The settings window opens instantly, shows your version, and can check for updates.
- `.canvas` files round their geometry, so re-saving doesn't churn the diff.
- "Workspace" is now three clearer words: space (your folder), canvas (a document), tab (a canvas you have open). The sidebar is Canvases, and `specular workspace` is now `specular canvas` — the old verb still works.

### Fixes

- Markdown notes load their content again instead of coming up empty.
- Sticky notes show a caret on the first click, can shrink below the size they were created at, and no longer spend an undo step on a height measurement.
- Region captures that reach past the view edge line up correctly.
- An app that outlives its terminal no longer floods `errors.log`.

### Misc

- Bundled agent-browser updated to v0.33.2.
- `specular annotation` returns the resolved selection — text, urls, files, group members, prior feedback — so agents don't need a second canvas read.

## [0.5.2] - 2026-07-28 — Bug Fixes

### Fixes

- Fixed a bug where tabs with the same name would fight.
- Improved bug with reordering when zoomed out.
- Fixed bug with sticky notes defocusing.

### Misc

- Bundled agent-browser updated to v0.33.1.

## [0.5.1] - 2026-07-27 — Tab Targeting, Selection Annotations

### New

- **Annotate a selection.** Select anything, hit Annotate, and the comment covers the whole selection — the fix loop knows which items you meant.
- **Agents work across tabs.** `--tab <ref>` targets a background canvas; `specular tab` lists, creates, switches, and deletes them.

### Improvements

- Agents don't steal focus when editing a background canvas.
- The fix loop knows where a change belongs: repo-bound pages get edited in place, space-folder files get worked on beside the original, and the comment always wins.
- Fix runs default to edit-and-verify permissions instead of all-or-nothing.
- Focus rings show on Tab, not on click.
- Primary buttons read right in light and dark.
- Bundled agent-browser updated to v0.33.0.

### Fixes

- `specular apply` explains itself instead of hanging on empty stdin.
- Deleting a background tab leaves the one you're on alone.
- The settings window shows an error instead of a blank pane when it fails to boot.

## [0.5.0] - 2026-07-20 — Page Anchoring, Scroll Tracking, Shape Styling

### New

- **Annotations stick to pages.** Comments, stickies, drawings, text, and shapes drawn over a page now belong to that page: they move with it, scroll with its content, fade at the page edges, and hide when the page navigates elsewhere. Anything drawn over empty canvas stays put, as before.
- **Comments can attach to an element.** Point a comment at a button or heading and it tracks that element by a unique selector — through scrolls, reflows, and re-renders. Clicking the comment scrolls the page back to it.
- **Multi-selection in the left sidebar**, with page-anchored items nested under their page.
- **Shape styling**: fill, border color and style, text alignment, and edge stroke controls, all in the selection popup.

### Improvements

- Marquee selection reads Cmd/Ctrl live during the drag, so you can toggle intersect vs. contain mid-marquee.
- Stack-order menus (bring forward, send back) on canvas item right-click.
- Single-page drags are noticeably faster; group drag and membership handling is more predictable, and Cmd-drag pulls an item out without re-binding it.
- Entering edit mode selects all the text; clicking away saves instead of discarding.
- Resizing only snaps the edges you're actually dragging — the opposite edges stay put.
- Live files: a manual refresh action, an update flash when content changes, and a watcher that survives atomic saves and in-place writes.
- Images pasted from the native macOS clipboard now land on the canvas.

### Fixes

- File context menus close on an outside click.
- Escape mid-drag clears the comment tool's marquee overlay.
- Duplicating a page re-attaches its anchored comments and copies anchored items at their apparent position.
- Device-frame and border settings apply to file entities again.

### Misc

- Removed the wireframe renderer mode.

### Breaking

- Comments record their page binding in a `pageAnchor` field instead of `metadata.pageUrl`. Comments in existing `.canvas` files still load and render, but legacy ones lose the hide-on-navigation URL gate (they behave as canvas-bound) until recreated.

## [0.4.2] - 2026-07-13 — Interaction Sync

### New

- **Interaction sync**: hover and click a page, and its same-origin peers follow along.
- Synced cursors show where each peer is pointing, anchored to the matching element.

### Improvements

- Specular CLI targets like `text=Sign in` survive quoting.
- `wait` forwards `--text` and `--url`.
- Failed ref-targeted actions give a stale-ref recovery hint.
- agent-browser passthrough verbs that can't work now fail with a real error.
- The bundled agent-browser driver resolves correctly — no separate install needed.

## [0.4.1] - 2026-07-09 — Region annotation fixes, arrow-key nudge

### Improvements

- Arrow keys now nudge the selected entities by 5px (grid step with Shift) instead of jumping between pages. Each press is its own undo step, and alignment guides flash as you move.

### Fixes

- Region annotations now capture drawings, stickies, notes, shapes, and file bodies — area captures no longer come back missing your content.
- Pan and zoom pass through region-annotation boxes, so you can scroll and zoom the canvas underneath them.

## [0.4.0] - 2026-07-08 — Per-Page Themes, Gap Handles, Live File Refresh

### New

- **Per-page color scheme**: override light/dark/system on a single page, layered over the app theme. A theme-mode button in the toolbar cycles system → light → dark, and the page popup reuses the same control for its per-page override.
- **Document tool**: Text, Sticky, and Document are now three independent one-shot tools. Document drops a markdown file backed by a fresh `.md` note. (Retires the old text/markdown toggle.)
- **Draggable gap handles**: grab the gap between children in an auto-layout group and drag to resize it, with a live preview and a single undo step. Works for both row and column layouts.
- **Live file refresh**: local-file canvas entities reload on their own when the underlying file changes on disk.

### Improvements

- Expanded shape catalog with independent border styling (color and style); rebuilt selection popups and pickers on shared dropdowns.
- Smoother canvas pan and zoom, with fewer frame hitches and live selection chrome.
- Escape now works inside the annotation composer while you're typing.

### Fixes

- Browser DevTools follow the selected page instead of staying pinned to whatever page was open when you launched them.
- Eyedropper opens the inspect panel on click rather than the moment you pick the tool, so it no longer tears down an open DevTools session.
- Markdown note edits now join the single undo thread, so Cmd+Z steps back through them one at a time instead of deleting the whole note. Placement also previews as a note box.
- Shape inline edit keeps newlines on Enter.
- Editing an edge updates it in place instead of duplicating.
- Images no longer trigger native text selection when dragged.
- Comment composer polish: send button pinned to the corner, scrollbar at the card edge.

## [0.3.3] - 2026-06-30 — Agent Browser Fix, Page Panel Controls

### Fixes
- packaged apps now actually ship the agent-browser binary — Setup no longer
  reports it missing. It's fetched and checksum-verified at build time.
- comment badges fade out gracefully as they leave a page's content band
  instead of drifting onto page chrome
- in-page comment hover labels stay a constant on-screen size when zoomed

### Improvements
- page device controls (frame toggle, rotate, focus) moved into the right
  panel header
- send a single comment to the fix agent from the thread popover, or fix all
  of one page's comments from the page panel
- the fix agent can now answer questions without editing code, and leaves
  resolving the thread up to you
- faster panning on busy canvases

## [0.3.2] - 2026-06-29 — Focus Mode

### New
- **Focus mode** replaces Browser mode — select a page and focus it to present it full-bleed. Navigate between pages from the sidebar while focused, and use the eye toggle to show or hide everything around it (other pages, files, notes, annotations).
- Copy as PNG — right-click an image to copy it straight to the clipboard.

### Improvements
- Clearer modes and more consistent movement between canvas items — selecting an item, stepping into it, and stepping back out now behave the same everywhere.
- Cleaner borders around pages.
- The CLI is simpler and easier for agents to drive.

### Fixes
- Files dropped or pasted onto the canvas intake correctly again.
- `link --label` and edge colors persist again.
- A second gesture starting mid-resize or edge-drag no longer corrupts the undo stack.

## [0.3.1] - 2026-06-02 — Distribute, Reorder, Smart Paste

### New
- **Distribute** — evens out the gaps in a 3+ selection along the dominant axis, keeping the endpoints fixed
- **Drag-to-reorder** — grab the reorder dot on any evenly-spaced multi-selection and shuffle items; siblings reflow to fill the gap
- **Auto-layout groups** — a managed group packs its children into a row, and dragging a child reorders it
- Drag to reorder the canvas stack right from the sidebar, plus keyboard shortcuts for stack order
- Smart paste in markdown notes — paste SVG, JSON, or HTML and it auto-wraps in a fenced code block
- Inspect mode now visualizes flex container gaps with hashed strips between children
- Links that open in a new tab (`target="_blank"`, cmd-click) now spawn a new page on the canvas instead of a native popup — the camera glides to center it on a foreground open

### Improvements
- Replying on an annotation thread resumes the same agent-fix session instead of restarting cold
- Drawing strokes scale with the bounding box when you resize
- Image entities render bare — no card, shadow, or filename chrome (the filename moves to the selection popover)
- Cmd+1 centers any selected canvas item, not just pages
- Annotate tools (draw, comment, inspect) work on an empty canvas with no pages
- Selection outlines unified to a clean 2px across every entity kind

### Fixes
- Scroll never accidentally pans the canvas, and the browser-mode device-shell offset is fixed
- Plain-text placement preview matches the selected text's size and zoom
- Sidebar rename inputs auto-focus and hold focus while you type
- Multi-selection and group resize batch into a single undo step
- Middle-mouse pan no longer sticks when you release the button outside the window
- Neutral sticky note text is readable in dark mode
- A selected drawing wins the hit-test over page chrome
- The live drawing stroke renders above file entities instead of snapping on top after commit

## [0.3.0] - 2026-05-14 — Shapes, Alignment Guides, Live Components

### New
- You can now add basic shapes like squares, circles, and diamonds with inner text to the canvas
- Added a highlighter brush in addition to pen
- Render local interactive HTML files to the canvas
- Connect a local repo and drop live component views onto the canvas (hot-reloads as you edit)
- Better markdown editing with CodeMirror — edit inline, styled view, file-type icons
- Improved settings panel with Skills, Fix, and keyboard Bindings management
- Alignment and spacing guides while you drag and resize
- Axis-locked drag with shift
- Option-drag to copy
- Resize multiple entities at once with a single bounding box
- Click a selected entity again to start editing it (text, stickies, files, drawings)
- Cmd+A selects everything on the canvas
- Paste a URL to drop a page, paste an image to drop a file, paste plain text to drop a sticky
- Click anywhere with the text tool to drop a text node and start typing
- New pages default to framed in a device shell
- Inspector got a richer tooltip and a Chrome-style box-model overlay
- Canvas grid fades smoothly across zoom levels and stays legible when zoomed out
- Press p/t/s/r/o/i/m to switch tools

### Behavior changes
- Cmd+D drops the duplicate next to the source instead of wrapping both in a row group
- When a page has keyboard focus, native shortcuts (Cmd+Z, Cmd+G, arrows, tool keys) go to the page. Press Escape first to use canvas shortcuts.

### Fixes
- Edges are easier to grab when zoomed out — hit targets scale with zoom
- Cursor video recording captures at native resolution and includes the cursor
- Drawing strokes keep uniform width regardless of how fast you draw
- Sticky note resize and edit focus
- Marquee box renders above page content instead of getting clipped
- Cmd+D works on multi-selections and groups
- Hotkeys keep working even when a page has an autofocused input
- Wireframes survive malformed nodes instead of crashing
- App-menu DevTools opens for the focused view


## [0.2.7] - 2026-04-23 — Cursor Trails, Multi-Select

### New
- WebGPU particle trails behind agent cursors — speed-gated emission with noise-driven drift, so trails concentrate during fast movement and dissipate when idle
- Shift/Cmd-click toggles entities in and out of the current selection across frames, text, files, drawings, and groups

### Improvements
- Canvas zooms out to 2% (down from 10%), so huge spatial workspaces fit on screen
- Frames stay draggable when you click into one from a multi-selection

### Fixes
- Selected groups drag and resize cleanly under rapid input
- Shift/Cmd-click on a singly-selected frame now toggles selection instead of falling through to the webpage

### Misc
- Release skill documents the new two-commit flow (changelog, then version bump)

## [0.2.6] - 2026-04-21 — Presence Polish, Debug Window

Lots of refinement to how agent cursors move and retire, plus a new debug window for inspecting presence in flight.

### Improvements
- Agent cursors render in canvas space, so they no longer rubber-band during pan/zoom
- Cursor motion follows a Catmull-Rom spline with distance-scaled animation — short hops feel instant, long travel reads as intentional
- Scroll animates with an ease curve and dwells before moving, so the cursor lands at the origin before the page shifts
- Cursors fade out gracefully on idle-retire and session-done instead of popping
- Single-item creates, updates, and deletes across text, files, frames, links, groups, annotations, and camera focus all move the cursor now
- Each frame gets its own agent-browser session, so driving multiple frames in one app session routes to the right place
- `specular link <fromId> <toId>` accepts positional args alongside the stdin batch form

### Fixes
- Click timing: the cursor actually arrives before mousePressed lands, with a full travel+dwell window
- Agent cursor projections account for chrome height, so clicks no longer land 44px above their target
- Same-frame attach_frame no longer bounces the cursor to frame center
- Unresolvable click/fill refs no longer snap to frame center
- Sidebar inline edit layout
- Presence sessions refresh `lastSeenAt` on lookup, so active flows can't be reaped mid-sweep

### New
- Standalone debug window with a presence timeline and motion playground

### Misc
- LICENSE file (PolyForm Shield 1.0.0)
- Expanded README with feature list, install instructions, and MCP docs
- CONTRIBUTING, CODE_OF_CONDUCT, and SECURITY docs
- `.env.example` covering available environment variables
- Specular skill moved into the repo so branch edits stop leaking globally
- Internal planning docs moved to `docs/internal/`

## [0.2.1-alpha.9] - 2026-04-07

### Fixed
- Frame borders, chrome UX, and grid visibility tweaks

## [0.2.1-alpha.1] - 2026-04-06

### Added
- macOS code-signing and notarization
- Auto-updates via `electron-updater`
- GitHub Releases publishing via `@electron-forge/publisher-github`
- Release CI/CD workflow (`.github/workflows/release.yml`)
- Convenience release scripts (`pnpm release:alpha`, `release:patch`, `release:minor`)
- App icon source and generated `.icns`

## [0.2.0] - 2026-03

### Added
- Spatial canvas with real Chromium `WebContentsView` browser frames
- MCP server for agent control of canvas and browsers
- Agent presence with live cursor and task status
- Commenting and annotation overlay
- Device frame shells with preset sizes (iPhone, iPad, Laptop, etc.)
- Entity grouping with freeform, row, and grid layout modes
- Edge connections between canvas entities
- Yjs-based undo/redo with global undo stack
- Video recording with frame targeting
- CDP proxy for stable agent browser automation
- Contextual right panel with per-entity panes
- Floating UI menus for frame and entity actions
- Left sidebar with entity tree and tabs
- Obsidian `.canvas` file format for workspace persistence
- Smoke tests and agent test harness
