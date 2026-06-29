# Changelog

All notable changes to Specular will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/). Do not use prerelease suffixes (`-alpha.N`, `-beta.N`). `update.electronjs.org` filters based on GitHub's "prerelease" release flag (not the SemVer suffix), so tagged-alpha versions *can* reach clients — but mixing SemVer prerelease ordering with GitHub's flag is a footgun. Just increment patch/minor versions and keep releases as non-prereleases.

## [Unreleased]

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
