# CONTEXT — Specular domain glossary

Canonical terms used across the codebase, ADRs, and plans. Resolve language conflicts here. See `CLAUDE.md` for build and layout, `native/CLAUDE.md` for the architecture, and `docs/adr/` for decisions of record.

Specular is the Rust app under `native/`. The Electron app it replaced was deleted; its source is in git history at `bc5253fa`. Older ADRs describe that app's mechanisms (IPC, `aboveView`, the Y.Doc, the runtime store). The terms below keep the domain meaning and give the Rust name where one exists. The last section maps each core term to its Rust type.

---

## Canvas data model

Follows [JSON Canvas v1.0](https://jsoncanvas.org) on disk; uses Specular-native terms in the app and the UI.

- **Canvas item** — the user-facing term for a thing on the canvas. Kinds: page, file, group, text, drawing, shape. Use this in docs and UI copy.
- **Entity** — the code term for the same concept (`specular_doc::Entity`). One entity ⇔ one canvas item. The headless surface (CLI / HTTP API) is verb-primary: read, patch and act on the canvas as a document ([ADR 0019](./docs/adr/0019-canvas-as-document-cli.md)). Every change, interactive or headless, is one `Command` and one undo step ([ADR 0041](./docs/adr/0041-typed-document-inverse-command-undo.md)).
- **Node** — JSON Canvas spec term. Use **only** in the disk-schema layer (`specular-doc/src/canvas/`). Never as a synonym for entity.
- **Edge** — a connection between two canvas items. Same name on disk and in code.
- **Flow** — entities laid out along the edges between them, as a state diagram of screens: each in a rank (a column, or a row for `TB`) so the arrows run one way, and each edge given a **route** (`specular_interact::flow_plan`, the layout adapted from Pele's layered layout). Made by an `apply` patch's `{"kind":"flow"}` layout or `specular arrange flow`. One flow is one undo step.
- **Route** — the right-angle path a flow gave an edge, stored on it (`Edge::route`, `route` on disk). It is drawn only while its ends still meet their entities' sides; once either moves, the edge is drawn as a curve until the flow is laid out again ([ADR 0048](./docs/adr/0048-flow-routes-drawn-while-they-meet-their-ends.md)).
- **Canvas** — a single `.canvas` file; the spatial document. This is the user-facing noun for the document everywhere: the sidebar section is "Canvases" and its actions say "Add/Rename/Delete canvas". ("Workspace" is retired from all vocabulary per [ADR 0033](./docs/adr/0033-user-chosen-space-folder.md) §7.)
- **Space** — a folder of canvases (Obsidian-vault analogue). The chosen folder *is* the space: `.canvas` files sit directly inside it, alongside `assets/` and a `.specular/` metadata directory. One space is open per window, user-chosen at first run and in Settings → General, with prompts — never guesses — for a space that is missing at launch ([ADR 0033](./docs/adr/0033-user-chosen-space-folder.md)).
- **Tab** — one canvas open in the app, named by a tab id in the space's index. Internal and CLI term only — the sidebar presents tabs to the user as canvases. `specular canvas` reports the active tab and every tab (id, name, entityCount), so an agent can tell which canvas answered. The tab row's tabs are a different thing (see **Showing and item view**).
- **Tab ref** — how a caller outside the app names a tab: an id or an exact name. Id first, then exact name; an ambiguous name errors with the matching ids and an unknown ref errors with the available tabs. It never guesses. `specular tab new <name>` creates a tab without activating it (and refuses a duplicate name, which would make every name-based ref ambiguous); `specular tab switch <ref>` is the only CLI verb that moves the user's focus.
- **Tab context** — a write that targets a background tab without moving the user's focus: `--tab <ref>` on any verb. The write runs in that canvas's own history and is one undo step there; it never reaches the active canvas's undo stack.
- **Page** — Specular's term for live web items: a URL rendered at a particular viewport size at a particular position on the canvas. Multiple pages can share the same URL (the multi-breakpoint workflow). Serialized as JSON Canvas `link` nodes; the `.canvas` reader and writer are the only place the two names meet. An HTML `file` node is a page too: hosted live at its file's `file://` URL, it stays a `file` node on disk ([file formats](./docs/file-formats.md)). See [ADR 0003](./docs/adr/0003-page-as-canonical-name-for-live-web-items.md).
- **Viewport preset** — a device a **page** can be sized as: a label, a `deviceId`, and a viewport in CSS pixels. The list is `VIEWPORT_PRESETS` in `native/crates/specular-doc/src/presets.rs`. A file names a preset twice, as `presetIndex` and `metadata.deviceId`, so a row never changes its index, id or size, and a new device goes at the end; a label may change to the device that has that screen now. The size menu's order and grouping (Phones, Tablets, Computers) is `PRESET_MENU`, separate from index order. A **retired** preset is a row in no menu section that is kept so old files open (the 834×1194 iPad Pro 11). The id says which preset and the art name says which drawing, so several presets may share one drawing; that mapping and each drawing's geometry are generated from the device frames' `manifest.json` by `native/scripts/device-art-table.py`.
- **Device frame style** — how a framed **page**'s device is drawn: **clay**, the flat bezel every device of a kind shares, or **realistic**, the device's own drawing (SVGs converted from the Flutter `device_frame` package, under `native/crates/specular-app/assets/device-frames/`). Stored per page as `metadata.deviceFrameStyle: "realistic"`; absent means clay. `metadata.showDeviceFrame` stays the on/off flag and is true for both. Every phone and tablet preset but the Duo has a drawing, and Laptop is a MacBook Pro (`DeviceArt`, generated into `native/crates/specular-doc/src/device_art/table.rs`); the desktops and the Duo are clay only. A device with a drawing has one shell in both styles, the drawing's body (its main outline, without the buttons or a laptop's base, which overhang the shell), so changing style never moves or resizes the page. A realistic page with no drawing to show (another preset, or a custom size) is drawn in clay and keeps its stored style. The size menu names the three choices None, Simple (clay) and Realistic (`page.frame.none|simple|realistic`); the frame key steps none → clay → realistic and leaves realistic out when no selected page can show it.
- **Device frame color** — whether a framed **page**'s device is drawn light or dark, in either **device frame style**. Stored per page as `metadata.deviceFrameColor: "light" | "dark"`; absent follows the app's theme. Clay takes its bezel, ring, indicator and shadow from that theme's palette whatever the app's theme is. Realistic picks one of two drawings of the same shape (`<name>.svg` dark, `<name>-light.svg` light). The control (`page.frame-color.light|dark`) is two segments, shown only while a selected page shows a frame; once pressed, the page no longer follows the theme.
- **Framed image** — an image **file** shown on the screen of the device it was taken on, in the same **device frame style**s and colours as a page. It keeps the page's keys in its `metadata` (`showDeviceFrame`, `deviceId`, `deviceFrameStyle`, `deviceFrameColor`) and no orientation: it lies on its side when it is wider than tall. Putting a frame on finds the device from the image's size (`screenshot_device`: a menu preset's screen at one, two or three pixels a point, either way up, else the nearest shape within 5%); an image of no device's shape gets a plain frame with no `deviceId`. The image's rect stays the screen at whatever size it was placed, so the shell is scaled to it, and the node is placed by its screen in the `.canvas` as every file node is. A realistic frame needs the image within 1% of the drawing's screen shape, or it is drawn in clay. Clay draws no notch or home button, as a screenshot shows its own. The file dock's `image.device` list holds `image.frame.none|simple|realistic` and `image.frame-color.light|dark`; the frame key steps images as it steps pages.
- **Sync set** — a group of 2+ pages that share navigation, scroll, and interactions: navigating, scrolling, or interacting with one drives the others (see **Interaction sync**). Membership is a shared `syncId` string on each page (`null` when unsynced), persisted on the `link` node and independent of groups — a sync set can span groups, and grouping does not imply syncing. Distinct from a visual **Edge** between pages. See [ADR 0027](./docs/adr/0027-sync-sets-decouple-navigation-sync-from-groups.md).
- **Interaction sync** — the third member of the sync family (navigation sync, scroll sync, interaction sync), ambient and always on for every sync set: the source page's hover and clicks replay on every peer. Semantic, not positional — the source captures each event as an element **locator bundle** (id / testid / role+name / text / structural path, plus a within-element offset fraction); each peer scores the bundle against its own live DOM and replays input at *its own* element's position, so the same menu opens at every breakpoint. **Actions are confident-or-skip:** a click fires on a peer only on an unambiguous element match — an ambiguous or failed match drops the click with a synced-cursor wiggle, never a coordinate guess. Source authority doubles as loop prevention: only the page receiving real user input broadcasts, so replayed events on peers cannot echo. Every shared navigation is a fresh sync point that heals any divergence. Scope: hover + clicks; peers on a different origin are skipped. See [ADR 0030](./docs/adr/0030-interaction-sync-is-semantic-not-positional.md).

## Text affordances

The `text` and `file` kinds back three user-facing affordances. The toolbar exposes **Text**, **Sticky note**, and **Document** as three independent one-shot tools — the tool you pick determines the entity, with no post-pick mode toggle or cross-kind morph.

| UX affordance | What it is | Spec mapping |
|---|---|---|
| **Text** | Short text, no background. Plain floating text on the canvas. | `text` node + `specular.textStyle: 'plain'` |
| **Sticky note** | Short text inside a colored card. Padding, bg color. The default. | `text` node + `specular.textStyle: 'sticky'` |
| **Document** | Longer markdown content as a `.md` file on disk; reusable across canvases. | `file` node naming a `.md` file |

Text and Sticky note are the same `text` kind with a render-style toggle. Document is a separate `file` kind because its content lives on disk, not in the `.canvas` JSON; the `.md` file is the source of truth, and its edits are undoable.

Default `textStyle` when the field is absent is `'sticky'` — preserves rendering of legacy canvases without migration. New "Add text" placements stamp `'plain'`.

## Specular extensions to JSON Canvas

JSON Canvas v1.0 tolerates additional fields on nodes; Specular adds its own under a single namespaced object so they don't collide with other tools' extensions and so this glossary can list them in one place:

```jsonc
{
  "id": "...",
  "type": "text",
  "x": 0, "y": 0, "width": 200, "height": 60,
  "text": "...",
  "specular": {
    "textStyle": "plain"   // 'plain' | 'sticky'
    // future Specular-only fields go here
  }
}
```

Conventions:
- Specular-only fields live under the top-level `specular: {}` object.
- Reading other tools' canvases: missing fields fall back to documented defaults.
- Reading our canvases in other tools: the `specular` object is silently ignored; nodes still render as standard JSON Canvas (text/file/link/group).
- Genuinely new node kinds (`drawing`, `shape`) remain top-level `type` values, since they aren't JSON Canvas spec kinds and no fallback rendering is meaningful.

## Entity geometry

- **Entity rect** — the full bounding rect of an entity. Equals the body rect: there is no separate chrome band ([ADR 0028](./docs/adr/0028-retire-chrome-header-slot-model.md)). Pan / zoom / drag / resize operate on this rect.
- **Camera** — the `{ x, y, zoom }` viewport transform, held in the session. Every canvas item, pages included, is drawn at its camera-projected rect; a page is a texture, so a pan or zoom moves no native view.
- **Content-sized bounds** — an axis whose size is owned by the rendered content, not by the stored entity. The size of a text body is only knowable after layout, so it is measured and written back; the stored value is an *output* of rendering on that axis, never an input. Today: the height of any text body in `widthMode: 'fixed'` — a sticky note or resized plain text (it grows with the wrapped text, so the box always contains it and never scrolls) — and both axes of plain text in `widthMode: 'auto'`.

  Rules, each learned from a bug where the card and the selection outline disagreed about the bottom edge:
  1. **One writer per axis.** A content-sized axis is written by its measurement and nothing else. A resize drops `height` from every text change — a second writer means the two values disagree until the next measurement lands.
  2. **Never grid-snap a measurement.** Grid snapping is a policy for edges the *user drags*, so they land on the grid. A measured size snapped up carries up to a full grid step of dead space under the last line. Round to whole pixels, and round up — a fractional height rounded down clips descenders.
  3. **A measurement is never an undo step.** It is derived from a user action, not a user action itself. Undo restores the axis the user drove — width, text size — and the height re-derives.
  4. **A floor on a measured axis scales with whatever the handle scales.** An empty sticky stays note-shaped instead of hugging its text down to a strip of padding. A sticky is floored at its own width (a note is square), up to its text-size height and never less than the default note's 200: a scale handle moves the width and the size together, so a scaled note stays square, a side handle makes a wide, short one, and the text size control leaves the height alone. Plain text floors at one line instead: it has no card behind it, so an empty one should read as a caret, not a box.
- **Reflow vs scale** — the two things a resize handle can mean for an entity with a content-sized axis. Side handles (`e`/`w`) *reflow*: width changes, the text keeps its size and rewraps, so a wider body usually gets shorter. Corner and `n`/`s` handles *scale*: the font tracks the width ratio from gesture start (FigJam-style) and the height follows the resized text, so a vertical drag fills the box it asked for. Every text body — sticky or plain — follows both rules.
- **Body sub-rect** — the entity's content area (the live document for a page, the image for a file, etc.). Equal to the entity rect. Resize handles and edge anchors attach here.

## Stack order

The front-to-back ordering of canvas items. Topmost = frontmost. Backed by the `entityOrder` array, persisted as `specular.entityOrder` in the `.canvas` file. See [ADR 0014](./docs/adr/0014-canvas-stack-order.md).

- **Stack order** — user-facing term for the concept. The sidebar tree mirrors stack order top-to-bottom (topmost row = frontmost item).
- **`entityOrder`** — the persisted data-model array. Code-only term; do not surface in UI copy. One flat array regardless of how the sidebar groups things.
- **Group contiguity** — every group's descendants (recursive) plus the group's own id form a single contiguous run inside `entityOrder`. The group's id sits at the front of its run, so `entityOrder.indexOf(groupId)` *is* the group's stack position. A group occupies one stack slot — reordering the group moves the whole run as a unit. Within that run, each child group's subtree is also contiguous (the invariant nests).
- **Stack-order mutations** — *Bring forward*, *Send backward*, *Bring to front*, *Send to back*. Shortcuts: `Cmd+]`, `Cmd+[`, `Cmd+Shift+]`, `Cmd+Shift+[` (Figma/Sketch convention). Sidebar rows can also be dragged or moved with `↑`/`↓` — moving a row up = bringing it forward.

**Group-aware marquee selection** — the marquee's hit set (leaves passing the intersect/contain test) resolves through two promotions. Enclosure: a fully enclosed group is always one unit — its hit descendants collapse into it, nested enclosed groups collapse to the outermost, and enclosure is the only way to select a group with no hit children (empty, or crossed only through whitespace). Scope: the deepest group containing every remaining unit is the selection's scope — a marquee confined to one group's contents selects those hits individually, but any hit outside a group promotes that group to a unit (each unit resolves to its outermost ancestor strictly inside the scope). So a marquee spanning group children *and* anything else grabs the whole group, never a partial slice of it.

**Selection scope** — the resolved meaning of a selection for gestures and verbs: member ids (top-level items, groups as unexpanded members — what reparent/group/ungroup act on), operand ids (the flat set gestures move/clone/delete — groups expanded to descendants, page-anchored items attached), and bounds (the union rect). Pressing any operand — including a descendant of a selected group — drags the whole selection. Drag, resize, copy/paste, duplicate, and delete all derive from this one resolution. See [ADR 0034](./docs/adr/0034-one-selection-scope-resolver.md).

**Edges** — edges participate in stack order (FigJam parity) by being interleaved into `entityOrder` alongside entities, and default to the top of the stack when created. They do **not** appear as rows in the sidebar. Stack-order mutations on edges happen on the canvas. Sidebar drag-reorder operates on the entity subsequence only; edge indices in `entityOrder` are untouched by sidebar drags.

JSON Canvas note: the spec emits nodes and edges into separate arrays. The interleaved `entityOrder` (including edge ids) is persisted as a Specular extension; other JSON Canvas tools round-trip the data but lose precise stack interleaving for edges.

## Auto-layout

A group can *manage* its children's layout instead of letting them float freely. See [ADR 0015](./docs/adr/0015-auto-layout-groups.md).

- **Auto-layout group** — a group with `managedLayout: true`; its children are reflowed into a row or column (`layoutMode: 'row' | 'column'`; `'grid'` is not a live managed mode), not freely positioned. `'freeform'` groups are plain selection/bbox containers. Reflow is the single writer of a managed child's position — it is an output, re-derived on any membership / resize / reorder change. Gap between children is the group's `layoutGap` (absent → default gutter).
- **Group drop target** — the innermost group whose pre-drag bounds contain the release pointer. Dropping a canvas item on it reparents the item; releasing outside every group removes membership. Command-drag suppresses drop binding so the item stays spatially above the group without joining it; Option-drag also opts out while copying. The same membership command backs canvas drops and sidebar drops.
- **Layout sequence** — the left-to-right order of a managed row *is* the order of the group's run within `entityOrder` (no separate `childOrder` field). Reordering a child rewrites that run; reflow reads it.
- **Reorderable row** — an evenly-spaced multi-selection: ≥2 boxes that read as a row (equal gaps along the dominant axis, no overlap, within ~1px tolerance). It exposes reorder dots even with no group — nothing is persisted but the new positions. Making an auto-layout group is the opt-in upgrade that persists the arrangement. See [ADR 0015 D7](./docs/adr/0015-auto-layout-groups.md).
- **Reorder handle (center dot)** — the per-box drag target that reorders a box to a new slot while its row reflows. Appears on each child of an auto-layout group *and* on each box of a reorderable selection. Its hit-test layer sits above `body` and below `anchors`. Dragging the box **body** still moves the selection/group as a unit; only the dot reorders. The drag commits as one undo step.
- **Gap handle** — the drag target between adjacent items of a line that edits the spacing (Figma-style): a strip spanning the gap and the full cross-axis extent, just below the reorder handle in hit-test order. Two doors, mirroring the reorder dots: a managed row/column group (commit writes the persisted `layoutGap`), and a loose reorderable selection (commit just moves the entities, each keeping its cross-axis coordinate). Cursor movement along the packing axis maps 1:1 onto the gap (clamped ≥ 0), and the commit is one undo step.
- **Make auto-layout** — the persist-it upgrade for a reorderable arrangement, and the headless entry point: `Shift+Cmd+A`, `specular auto-layout <ids|groupId> [--gap N]`, or `POST /groups/auto-layout`. Marks a selection (or existing group) as a managed row *or column* — the axis is seeded from the arrangement's dominant spread — and seeds the sequence to current order along that axis.
- **Arrange** — the multi-select "tidy up" (row / column / grid), reachable from the dock, `specular arrange <row|column|grid> [ids]`, or `POST /selection/arrange`. By default it *keeps the cluster's current footprint* and only regularizes the spacing inside it: row/column pin the outer items and even the gaps along one axis while aligning the other; grid keeps the existing 2-D structure (rows and columns read off current positions, holes preserved) and evens gaps on both axes. Packing to a fixed gap is the exception, reachable only via the CLI's `--gap` flag. Nothing persists but the new positions; one undo step restores the prior arrangement. See [ADR 0026](./docs/adr/0026-arrange-preserves-footprint.md).

## Drag affordances

Modifiers and visual feedback that ride on top of entity drag (and resize) gestures. The single magnetic pull on the canvas is grid-snap (20 px); everything below either projects the delta before the snap (axis lock) or renders informationally after it (alignment / distribution guides). See [ADR 0012](./docs/adr/0012-alignment-guides-are-visual-only.md).

- **Axis lock** — holding `Shift` during an entity drag (with or without `Alt`/`Option` for copy) constrains movement to a single axis. The dominant axis is recomputed every frame from cursor offset to drag origin, so the lock can flip H↔V mid-drag. The locked axis bypasses grid-snap (the entity holds its exact origin coordinate); the free axis still grid-snaps. Multi-select drag applies the same constrained delta to all entities.
- **Snap candidate** — an entity that contributes alignment edges during a drag or resize. Snapshot is taken once at gesture begin from the viewport-visible entity set, excluding entities in the active selection. Pages and groups (snap to bbox) are included; group children inside an included group don't double-contribute.
- **Alignment guide** — a 1 px solid line in the canvas accent color that confirms the dragged (or resized) entity's edge or center coincides with a snap candidate's edge or center within 0.5 px. Each candidate contributes 6 edges: top, bottom, left, right, horizontal-center, vertical-center; the dragged entity contributes its own 6 reference points (only the moving edge for resize). Guides are detected after grid-snap runs — they confirm an alignment, never create one. The line spans from min to max of the candidate and dragged rects on the snapping axis.
- **Distribution guide** — `==` measure marks rendered alongside alignment guides when the dragged entity sits at equal distance from two or more snap candidates along an axis (post-grid-snap). Same visual-only contract: detection only, no pull.

## Input

- **Page interactive (entered)** — the page the user has *entered*; only it receives pointer and keyboard input. A merely-**selected** page is not interactive — keys stay with the canvas so canvas shortcuts act on the page. Enter via a second click on the selected page or a double-click on its body; exit (back to selected) via Escape / click-away / selecting elsewhere. Delete never removes the entered page; it is removed once it is back to selected-only. See [ADR 0022](./docs/adr/0022-pages-select-first-interact-second.md).
- **Keyboard target** — the page keys belong to, derived (never stored) from selection, the entered page, the gesture and the tool. Pages render offscreen, so keys reach the target page as forwarded key events, after the app's binding table has had first refusal.
- **Hit-test priority** — top wins: `resize-handles > anchors > reorder/gap handles > body > background`. Chrome that hugs an entity resolves above all of them. On text, sticky, shape and editable files, a press on the sole selection's body with no modifier is a press, not a drag: a stationary release starts editing, while movement past the threshold falls through to drag.
- **Edit mode** — one entity at a time is being edited, and editing is exclusive with every other gesture. Lifecycle: blur → commit, Escape → cancel (revert), pointer outside the editing entity's body → commit and swallow the click (the user clicks again to act on the new target), drag attempt on the editing entity → refused, selection change / entity deletion / undo / tab switch → end the edit.

## Shapes

- **Shape catalog** — every shape kind is one row in a table: a label, a path in a normalized 0–100 box, an optional stroke-only line overlay (cylinder rim), and an optional text inset for the label box. Adding a shape is one row. Paths stretch to any size with a uniform border; straight-edged shapes stretch cleanly, and rounded ones (rounded/pill/cylinder/cloud) distort their curves under non-square scaling — acceptable for flowchart use. The picker icon draws the same path, so it always matches the drawn shape.
- **Shape border** — a shape's outline is independent of its fill. `color` drives the fill; `borderStyle` (`solid` / `dashed` / `none`, absent = solid) and `borderColor` (absent = derive from the fill hue) drive the outline, with `strokeWidth` as thickness.
- **Shape fill and label alignment** — `fillStyle` (`solid` / `none`, absent = solid) controls whether the shape body is painted without discarding its selected `color`. Shape labels independently persist horizontal `textAlign` (`left` / `center` / `right`) and vertical `textVerticalAlign` (`top` / `middle` / `bottom`), absent = center/middle. These serialize under the shape node's `specular` object.
- **Shape fill is theme-derived, not a second palette.** The hue slots carry no dark variant: light mode lightens the hue toward paper, dark mode darkens it toward the canvas so the shape is a tinted panel with the pastel hue as a light outline (readable label on both).

## Tool defaults

Per-tool, persistent app settings (not per-canvas, not in `.canvas`). Read by creation tools when stamping new entities; written from the dock while the tool is in hand. Stored in the app's preferences so each tool remembers its last-picked configuration across sessions and canvases.

| Tool | Defaults keys |
|---|---|
| `add-text` (plain) | `color` |
| `add-text` (sticky) | `color` |
| `add-shape` | `shapeKind`, `color`, `strokeWidth` |
| `draw` | `brushType`, `color`, `strokeWidth` |

Tool defaults never participate in undo/redo — they're user preferences, not document data. See [ADR 0008](./docs/adr/0008-unified-canvas-item-popup.md) §"Tool defaults".

## Tools

A **Tool** is the single representation of "what does my next click/gesture do?" There is exactly one active tool at any moment. Tools are mutually exclusive; you switch by toolbar click, keyboard shortcut, or Escape (which returns to Select). See [ADR 0005](./docs/adr/0005-unified-tool-concept.md), amended by [ADR 0006](./docs/adr/0006-unified-comment-tool.md) (comment tool subsumes region-select), and refined by [ADR 0009](./docs/adr/0009-tool-variants-in-popup-state.md) (variants move to tool defaults). The Rust enum is in **Rust names** below.

- **One-shot tools** (add page, text, sticky, document, shape) auto-revert to Select after one placement.
- **Persistent tools** (comment, draw, inspect) stay active until toggled off, replaced, or Escape.
- The toolbar does **not** visually distinguish one-shot from persistent — users learn the duration by use.
- Tool name → cursor-label gerund: select → "selecting", add page → "adding page", comment → "commenting", draw → "drawing", inspect → "inspecting".
- **Variants live in tool defaults, not in the tool.** The shape kind and the brush are picked in the dock and persisted to app settings (ADR 0009).
- **Keyboard shortcuts and tools.** Every tool has a default key. Pressing a tool's key while that tool is already active is a no-op (FigJam reference); Escape is the only keyboard path back to Select.

**Not a tool:** focus. A tab brings one item into attention ([ADR 0045](./docs/adr/0045-item-view-as-session-state.md)); see **Showing and item view**.

## Annotations

Comments live on the canvas as a single user-facing concept ("comment") and a single entity (`Annotation`). One tool — comment — produces all of them; the gesture decides the **anchor** (per [ADR 0006](./docs/adr/0006-unified-comment-tool.md)):

- **Click on a page element** → element anchor (DOM selector + bbox).
- **Click anywhere else on the canvas** → canvas-point anchor (the click position).
- **Drag a marquee** → region anchor (canvas-rect, may span pages).

Discriminated by `anchor.type: 'element' | 'canvas' | 'region'`.

**Page binding is `pageAnchor`, written once at creation.** An annotation is either page-bound (carries `pageAnchor { pageId, pageUrl }`, the same field anchored entities use — see §Page anchoring) or canvas-bound (no `pageAnchor`). Element anchors bind to their page. A **region binds iff its marquee grabbed page content** — decided at creation, no mode, no toggle. A grab-less region is canvas-anchored: it marks canvas space, never moves with pages, never hides. Canvas points never bind. Annotations in older files without a `pageAnchor` load as canvas-bound (`metadata.pageName` remains as display context only).

**Resting visual on the canvas:**
- Region anchor → dashed rose-400 rectangle, always visible while on-document. Click focuses the comment and switches the canvas agent thread. A **canvas-anchored** region stores `anchor.canvasRect` (canvas coords) and stays put; a **page-anchored** region stores `anchor.docRect` (the page's document CSS px), so it travels with page drags **and scroll-follows**, fading at the page's content edges when scrolled out.
- Element anchor → no resting visual beyond its badge; the conversation lives in the right-panel agent thread. Focusing the comment flashes its message in the right panel.
- Canvas-point anchor → same as element — no resting visual beyond its badge; focusing from the badge or panel switches the agent thread and rings the pin.

**Canvas agent thread** — the right panel is a general chat, not a per-comment drill-in. Transcripts persist in the space under `.specular/threads/<tab-id>/` (`<id>.json` plus `index.json` with `activeThreadId`), not in the `.canvas`. Visible messages plus `claudeSessionId` only; closed threads stay on disk and drop out of context.
- **Artifact** — something a turn made or edited, drawn as a card under the agent's answer. It is a file one of the agent's own tools wrote (read off the `claude` stream) or a canvas entity one of its `specular` CLI calls added or changed (the run is started with `SPECULAR_SESSION_ID` set to its thread id, and the API attributes each call by the `x-specular-session-id` header). Saved on the agent message as `artifacts`; a card whose item is on the canvas reveals it.
- **Agent model** — the model the next run uses, in every thread, chosen from the chip beside Send. The list is the Agent SDK's `supportedModels()` asked through `specular-helper` at launch, with the `opus` / `sonnet` / `haiku` aliases standing in until it answers; the choice is `agentModel` in the preferences file and defaults to `sonnet`.
- **Thread avatar** — the Prism drawn beside each thread in the right panel's list, and larger at the head of the open thread's run bar, where it tumbles in spring-driven bursts while the agent works. It animates only while that thread's agent works and otherwise holds a still picture, different per thread. See [ADR 0047](docs/adr/0047-thread-avatar-prism-on-wgpu.md).

Comments and chat are one conversation. Agent replies live only in the thread, never on the pin. A new comment queues into a **draft** unless the active thread is still unsent; more comments join that draft. **Send** runs the agent. Follow-up stays in that thread's composer. The next new comment after Send starts another draft. Submit on the pin queues, it does not send.

**Auto-fix** is the per-origin opt-out of Send (`boundOrigins[].autoFix` on the origin→repo binding; the Auto chip beside the composer's folder chip). A comment on an auto origin joins the active thread and is sent the moment it is placed, aimed at that pin. A run already in flight keeps it queued and drains it when the run ends. Canvas-bound comments have no origin to opt in with and always wait for Send.

**New** and **Close** are separate. The switcher keeps threads. Close archives (`status: closed`). New is a clean transcript.

One pin maps to at most one thread via `annotation.metadata.threadId`. Clicking a badge focuses the pin and selects that thread. The panel flashes the pin's message on every click, including a re-click of the focused pin. Back, Esc on the canvas, resolve, or delete clears the focus. There is no on-canvas thread popover.

The live **pill** sits above the composer and is this turn's intent, not the conversation's identity: DOM node > focused comment pin > canvas selection > empty (the canvas name, or "specular"). Composer focus does not clear canvas selection. Placing a comment selects that pin. The agent prompt names the selected entity ids and asks it to focus there; that is guidance, not a write fence.

Home **cwd is the space folder**. One UI transcript. A DOM node on a linked origin writes that repo; otherwise canvas/CLI and space files. Bind a repo from the page's controls (folder icon; inferred when the page URL matches a connected repo) or Settings → Repos.

**Comment draft** (ADR 0006 amendment) — distinct from an agent thread's `draft` status above: this is the not-yet-saved comment itself, before it becomes an `Annotation` at all. The gesture that starts a comment opens a draft, not a composer on the canvas. The canvas draws only a passive marker from it (an outline, a pin, or a dashed rect); the composer is the right panel's chat field, which shows the draft as a removable chip above the message. Pressing Enter/Send there saves the comment; removing the chip or pressing Escape drops the draft with nothing saved. One draft at a time; a new one replaces the old. Leaving the comment tool clears a draft anchored to that tool's gesture; switching canvas tabs clears any draft.

**Navigation hides page-bound visuals.** When a page navigates away from `pageAnchor.pageUrl` (hash-insensitive), its badges and region rects disappear until the page returns to that URL. Canvas-bound annotations always show. The sidebar dims instead of hiding.

**Sidebar shows the anchoring relationship.** A page acts as a folder for content anchored to it: unresolved page-bound annotations render as collapsible child rows under the page in the left sidebar, after the page's anchored entities. Click focuses the comment and switches the agent thread; rows dim when the page has navigated away from the annotation's URL.

**Selection annotation** — a region annotation created from a multi-selection (`POST /selection/annotate`, or `specular annotate-selection`). It is a plain region anchor over the selection's union bounds. Selected-entity membership rides in `metadata.selectionEntityIds`, and `metadata.selectionTarget` names the one artifact the request is about — the selected page or file entity — derived only when the selection names exactly one.

## Page anchoring

The generic "hook to a page" utility ([ADR 0031](./docs/adr/0031-page-anchored-entities.md)). A canvas item is either **free-form / canvas-bound** (default) or carries a `PageAnchor { pageId, pageUrl? }`. Consumers: `text` (stickies), `drawing`, `shape`, and annotations (§Annotations).

- **Placement decides for entities.** An entity whose center lands inside a page's body anchors to it — on creation and on drag end. Dragging it off the page frees it. Command-drag suppresses that drag-end binding so the entity stays canvas-bound above the page. Grouped entities never auto-anchor. Annotations bind once at creation and are never re-resolved.
- **Anchored items travel with their page.** Drag/nudge sets expand to include anchored entities; those positions stay in canvas coordinates (no stored offset). A page-anchored region annotation stores its rect in the page's *document* space (`anchor.docRect`), so it travels with page drags for free.
- **Anchored items scroll-follow.** The anchor stamps the page's scroll offset at placement (`scrollX/scrollY`); the item is drawn shifted by the delta between the page's live scroll and that reference, and the shift folds into stored coordinates on reanchor (the **scroll rebase**). Scroll-following items are clipped and edge-faded inside the page.
- **Element attachment** ([ADR 0032](./docs/adr/0032-element-attachment.md)). A page anchor may carry a derived reference to the DOM element the item was placed over (`element { selector, docX, docY }` — the element's document position at capture). Attached items render corrected by how far that element has moved since capture, so they survive page reflow. The attachment is machine-derived from placement, re-derived whenever position changes, persisted but **outside undo scope**, and never a visibility gate: an unresolvable selector means zero correction. One element per item, translation-only tracking.
- **Anchored means document-bound.** While the page shows a different URL than the anchor's, the item is not drawn and not hit-testable, and its sidebar row dims — one rule for entities and annotations.
- **Sidebar nesting.** Anchored entities and page-bound annotations render as child rows of their page instead of in the root lists.
- **Sidebar reveal.** Selecting a page-bound comment or an anchored canvas item restores the page's recorded document when necessary and smooth-scrolls the live page to the bound content.
- Anchors persist transparently: as a `pageAnchor` field on the JSON Canvas node (`specular` extension block for text nodes) and on the annotation record.
- Deleting a page clears the anchors that reference it — entities and annotations go canvas-bound in place.

## Theme & color scheme

Scheme resolution is a three-layer chain: **OS appearance → app theme mode → page color scheme override**. Each layer defaults to inheriting the one above.

- **App theme mode** — the tri-state app preference (`system` | `light` | `dark`), persisted in the preferences file. Drives the app chrome theme *and* every page's `prefers-color-scheme` — toggling it previews all non-overridden pages in the other scheme.
- **Page color scheme override** — per-page `colorScheme` (`'light'` | `'dark'`, absent = inherit). Persisted top-level on the page's link node (alongside `presetIndex` — link-node fields predate the `specular: {}` convention), undoable like any page field, carried through duplicate and copy-paste.

## Presence

Live cursors on the canvas: agent sessions, so a watching user can follow what an agent is doing, and the user's own cursor echoed onto sync peers. Presence follows truth: a cursor shows an act when it happens and never invents one ([ADR 0029](./docs/adr/0029-presence-acts-anchored-to-truth.md)).

- **Presence cursor** — the per-source animated cursor (color, position, activity, label). Sources: an agent session or **interaction sync** (see **Synced cursor**). Ephemeral: never persisted, expires on idle. Visibility is a per-source preference (agent cursors, synced cursors); hiding is drawing only — a hidden agent keeps acting and interaction sync keeps replaying.
- **Announced act** — how an agent's cursor arrives before its act: a patched agent-browser sends `Specular.willAct` with the point it is about to act on, the app holds the answer until the agent cursor is drawn there, and agent-browser then resolves again and dispatches. See [ADR 0046](./docs/adr/0046-agent-browser-announces-its-acts.md).
- **Synced cursor** — the presence cursor sourced from interaction sync: the user's own cursor reflected onto each sync peer, always on while the source page is receiving input. Viewport-proportional gliding as the base layer, snapping to the peer's *own* resolved element whenever the locator matches confidently. The target halo appears only when anchored, i.e. only where a click would actually land. Same treatment as agent cursors with one reserved color so it reads as the user's reflection. A brief wiggle has exactly one meaning: a refused click ("can't do that here").

## Cloud sync & sharing

Proposed, not built — see [ADR 0018](./docs/adr/0018-cloud-sync-and-canvas-sharing.md). The ADR was written for the Electron app's Y.Doc; the Rust app has no CRDT ([ADR 0041](./docs/adr/0041-typed-document-inverse-command-undo.md)), so its sync design would start over. The local-first disk path (`.canvas` files) is the only shipping mode. Terms recorded ahead of implementation:

- **Doc id** — the stable identifier for a canvas as a synced unit. Distinct from a tab id or a filename.
- **Split data plane** — the doc holds *references*, blob storage holds *bytes*. Image/video binaries never enter the doc. Mirrors the local model (bytes in `assets/`, a reference on the entity).
- **Asset id** — a stable id stored on a `file` entity instead of a location, resolved per environment to a local `assets/` path or a cloud key, so a `.canvas` stays portable. Asset versions are content-addressed and immutable.
- **Sandbox origin** — the dedicated origin that serves HTML assets, so a generated page has no path to tokens.
- **Capability link** — a share URL carrying a doc id plus a scoped, expiring **capability token** (`view` | `comment` | `edit`). The token, not the path, is the security boundary.
- **Agent as peer** — an agent joins a canvas as an ordinary authenticated peer via a capability link and edits doc data headlessly. Agent tokens are narrower, short-lived, audit-logged and separately revocable.
- **Snapshot tile** — how a live **page** renders in a web client: its last desktop-captured screenshot, with a captured-at timestamp surfaced honestly ("captured 2h ago"). Not scrollable or clickable, but a valid comment target.

## UI copy voice

- **Sentence case** — capitalize the first word only. Default for menus, buttons, dialog text, chrome labels: "Reveal codebase in finder", "Delete project…", "Rename".
- **Lowercase gerund narration** — used **only** for cursor labels, status bar text, and live captions. "clicking submit", "dropping in frame", "looking at toolbar". No quotes, arrows, or colons.

## Rust names

Each core term above and its Rust type. See `native/README.md` for the crates and [ADRs 0041 to 0044](./docs/README.md) for the decisions.

- **Entity and Kind.** A canvas item is `specular_doc::Entity`: an id, a rect, a label, an optional page anchor, an optional parent and a `Kind`. `Kind` is an enum of the six kinds (`Page`, `Text`, `File`, `Group`, `Drawing`, `Shape`): a new kind is a new variant, and the compiler lists every `match` that needs an arm. A sticky note is `Kind::Text` with `TextStyle::Sticky`. A Document is `Kind::File` naming a `.md` file. Node stays a disk-layer word, used only in `specular-doc/src/canvas/`.
- **Command.** The one way to change a document. `Document::apply(Command)` returns the command that undoes it, and `History` is a stack of those inverses. One user action is one command or one `Command::Batch`, so one undo step ([ADR 0041](./docs/adr/0041-typed-document-inverse-command-undo.md)).
- **Space and Canvas.** `specular_interact::Space` is the open folder and every canvas in it. Each `Canvas` has its own document, undo history, camera and selection, and the active one's are `App`'s own fields ([ADR 0043](./docs/adr/0043-per-canvas-state-in-a-space.md)). A Tab is a `Canvas`, named by a `CanvasId`, which is the tab id in the space's index.
- **Tool.** `specular_interact::Tool`, one enum with nine variants: `Select`, `AddPage`, `AddText`, `AddSticky`, `AddDocument`, `AddShape`, `Draw`, `Comment`, `Inspect`. Plain text and sticky are two variants, not one with a style. There is no hand tool. Variants such as the shape kind and the brush live in `ToolDefaults`.
- **Gesture.** `specular_interact::Gesture`, the drag in flight: `Move`, `Resize`, `Marquee`, `Comment`, `Place`, `Draw`, `TextSelect`, `EdgeDrag`, `Line` (a reorder or gap drag in an auto-layout row). It owns the pointer until the button comes up. A gesture writes its unfinished rects into the document as it goes, and the release records one undo step.
- **Action.** `specular_interact::Action`, what a key binding, a menu item, a panel control or an API act route asks for. Keyboard bindings are one table of chord, context and action.
- **Event and Effect.** `Event` is everything that can happen to the app: input, a page reporting something, a clock tick, a file answer, an API request. `update(&mut App, Event)` returns `Effect`s, and an `Effect` is the only way anything outside the app happens: host a page, write a file, set the cursor, run an agent ([ADR 0042](./docs/adr/0042-update-view-render-loop-effects-only-io.md)). One function receives all input.
- **Session.** The state that is not saved or undone: camera, selection, active tool, gesture, hover and focus.
- **Scene.** `specular_scene::Scene`, a flat list of draw items in paint order that `view(&App)` builds each frame. Each item is in canvas space or screen space. The compositor draws it and knows nothing about entities. Chrome that hugs an entity is a screen-space item in the same list.
- **Annotation.** `specular_doc::Annotation`, with the three anchor types. The comment tool's drag is `Gesture::Comment`.
- **Page host.** The shell's CEF offscreen browser for a page entity. `Event` and `Effect` name a page by its `EntityId`, and the shell keeps the table from that to the backend's `PageId`.
- **Model.** A panel's content as plain data, built by a function of `&App` (`toolbar`, `dock`, `sidebar`, `chat`) and drawn by the GPUI Kit shell ([ADR 0044](./docs/adr/0044-ui-as-pure-models-with-replaceable-renderers.md)).
- **Chrome and dock.** The chrome is three rows across the top of the window: the tab row (the title bar strip), the toolbar, and the dock. The dock is a fixed, full-width bar holding the options of the creation tool in hand, else the controls of the selection, else nothing. It never moves with the canvas. Everything under the chrome starts at `CHROME_HEIGHT`.
- **Showing and item view.** `specular_interact::Showing` is what the window shows under the chrome: `Canvas`, or `Item(EntityId)` for one page or Document alone. The second is an item view. It is session state, set by `Action::Show`, never saved or undone ([ADR 0045](./docs/adr/0045-item-view-as-session-state.md)). The tabs that are open and their lenses are saved with the canvas, in `appState.itemTabs`, and come back at the next launch on the Canvas tab. An item view is camera and size management over the one canvas, with no second canvas inside it. One more value sits beside the item: the **lens** (`Lens`, `Action::SetLens`), how the tab looks at it, kept per tab. **Fill**: the item fills the view (a Document is a reading column; a page is laid out at the area under the chrome, at 100%, and beside it only what follows one of its elements shows). **Device**: the stored size fitted. Beside the item it shows only what is hooked to it until the **eye** is opened (`Action::ShowOthers`, `view.others`, one choice for every tab, shut to begin with), and then the rest of the canvas is around it. The camera is always held on the item. Selection chrome follows the **mark** (`App::mark`): an entered page on the canvas has no outline and no resize handles and is drawn as it is unselected, the item shown has no mark while it is entered or fills the view, and no item shown has resize handles. The controls at the right end of the tool row are `view.others` in Device, `view.lens.<fill|device>` and then `view.minimize` (back to the canvas), which sits last, where `view.open` does on the canvas. Closing the tab showing shows its neighbour (the tab that took its place, or the one before it when it was the last), and the canvas only when no other tab is left. Leaving the item view, by Minimize, the Canvas tab or closing the last tab, keeps the zoom the canvas had and centres the item's stored rect in the free part of the viewport, so the item shrinks in place. The camera moves between the canvas, Device and Fill by a short spring (`showing::motion`), centred on the item; a switch from one item tab to another is instant, and the layout changes at once. The word tab has two meanings and the Rust names keep them apart. A **Tab** above is an open canvas, a `CanvasId` (`tab_N`), and stays the CLI's word (`specular tab`, `--tab`). A tab of the tab row is a `ViewTab` of the `ViewStrip` model (`view_strip(&App)`), with the control ids `view.canvas` and `view.item.<entity id>`. UI copy may call either one a tab. In Fill and Device a page shown is entered at once. A Document in Fill is laid out as a reading column 720 px wide whose stored rect is untouched (`showing::presented_rect`). A page in Fill has a **presentation rect** from the same function: its host's viewport follows it and goes back to the stored one when Fill is left, the stored rect and preset are not written, and the dock still shows and edits the stored size. What a creation tool or a paste makes in Fill, or in Device with the eye shut, is hooked to the page shown, or is not made; in Device with the eye open it is made by the canvas's rules. The row ends in `view.add`, which with Command+T makes a page in a free spot of the canvas (`free_spot::place`, the placement the API uses) and shows its tab with the caret in its address. Command+L puts the caret there for the page in the dock, and Command+Option+Right and Left show the next and the previous tab.

---

*This file is the canonical glossary. When a term resolves during planning or grilling, update it here. Don't couple to implementation details — only include terms that are meaningful to the domain, not internal helpers.*
