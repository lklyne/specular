# CONTEXT — Specular domain glossary

Canonical terms used across the codebase, ADRs, and plans. Resolve language conflicts here. See `CLAUDE.md` for build/architecture; see `docs/adr/` for decisions of record.

---

## Canvas data model

Follows [JSON Canvas v1.0](https://jsoncanvas.org) on disk; uses Specular-native terms in runtime and UI.

- **Canvas item** — the user-facing and component-naming term for a thing on the canvas. Kinds: page, file, group, text, drawing, shape. Use this in docs, UI copy, and component names (`CanvasItemChrome`, `SidebarCanvasItem`).
- **Entity** — the runtime / persistence term for the same concept. Use this in `src/main/runtime/` types (`PersistedPageEntity`, `CanvasEntityKind`, `CanvasSceneEntity`). One entity ⇔ one canvas item.
- **Node** — JSON Canvas spec term. Use **only** in the disk-schema layer (`src/shared/json-canvas-types.ts`, `json-canvas-serializer.ts`). Never as a synonym for entity in runtime code.
- **Edge** — a connection between two canvas items. Same name across all three layers.
- **Canvas** — a single `.canvas` file; the spatial document.
- **Space** — a folder of canvases (Obsidian-vault analogue).
- **Page** — Specular's term for live web items: a URL rendered at a particular viewport size at a particular position on the canvas. The runtime `Page` (the `WebContentsView` wrapper in `src/main/runtime/page-factory.ts`) is the same concept — there is no separate "frame entity vs page wrapper" duality. Multiple pages can share the same URL (the multi-breakpoint workflow). Serialized as JSON Canvas `link` nodes; the serializer is the only place the two names meet. See [ADR 0003](./docs/adr/0003-page-as-canonical-name-for-live-web-items.md).

## Text affordances

The `text` and `file` kinds back three user-facing affordances. The toolbar exposes them under a single **`Add text ▾`** dropdown.

| UX affordance | What it is | Spec mapping |
|---|---|---|
| **Text** | Short text, no background. Plain floating text on the canvas. | `text` node + `specular.textStyle: 'plain'` |
| **Sticky note** | Short text inside a colored card. Padding, bg color. The default. | `text` node + `specular.textStyle: 'sticky'` |
| **Document** | Longer markdown content as a `.md` file on disk; reusable across canvases. | `file` node, markdown renderer (selected by extension via `src/main/plugins/registry.ts`) |

Text and Sticky note are the same `text` kind with a render-style toggle. Document is a separate `file` kind because its content lives on disk, not in the `.canvas` JSON.

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

- **Entity rect** — the full bounding rect of an entity, body + chrome as one layout unit ([ADR 0002](./docs/adr/0002-canvas-anchored-overlay-ui.md)). Pan / zoom / drag / resize operate on this rect.
- **Body sub-rect** — the part of the entity rect that holds the entity's content (the live document for a page, the image for a file, etc.). Resize handles and edge anchors attach here.
- **Chrome slot** — the part of the entity rect reserved for canvas-anchored overlay UI. Per-kind, runtime-derived, **not persisted** in the `.canvas` schema.

## Stack order

The front-to-back ordering of canvas items. Topmost = frontmost. Backed by the `entityOrder` array on the workspace (a Y.Array, round-tripped through the `.canvas` JSON Canvas node order). See [ADR 0014](./docs/adr/0014-canvas-stack-order.md).

- **Stack order** — user-facing term for the concept. The sidebar tree mirrors stack order top-to-bottom (topmost row = frontmost item). Pages (`WebContentsView`s) are stacked on the canvas in the same order: frontmost stack slot = topmost child view.
- **`entityOrder`** — the persisted data-model array. Code-only term; do not surface in UI copy. One flat array regardless of how the sidebar groups things — the section partition below is purely a render-time view, not a data split.
- **Group contiguity** — every group's descendants (recursive) plus the group's own id form a single contiguous run inside `entityOrder`. The group's id sits at the front of its run, so `entityOrder.indexOf(groupId)` *is* the group's stack position. A group occupies one stack slot — reordering the group moves the whole run as a unit. Within that run, each child group's subtree is also contiguous (the invariant nests).
- **Stack-order mutations** — *Bring forward*, *Send backward*, *Bring to front*, *Send to back*. Shortcuts: `Cmd+]`, `Cmd+[`, `Cmd+Shift+]`, `Cmd+Shift+[` (Figma/Sketch convention). Sidebar rows can also be dragged or moved with `↑`/`↓` — moving a row up = bringing it forward.

### Sidebar sections — *Notes* and *Pages*

The canvas has two interactive paint surfaces, and the sidebar mirrors them as two labelled sections so the user can see why stack order behaves the way it does.

- **Notes** (top section) — text, sticky, document (file), drawing, shape. These render in `aboveView` and always paint above pages by architecture.
- **Pages** (bottom section) — live web pages. These render as `WebContentsView` children between `bgView` and `aboveView`.

Stack-order mutations are scoped per section: bring-forward/backward move within Notes or within Pages, not across the divider. Cross-section drag has no drop target. *Note* is a sidebar-grouping label — it is not a kind in the data model and never appears in `.canvas` files.

**Groups with mixed contents** — a group is one entity with one stack slot (its contiguous run in `entityOrder`), but its members can paint on both surfaces. The sidebar honours both facts via **split representation**: a group whose members straddle the two paint surfaces appears as **two rows**, one in each section, sharing id, name, and selection state. Expanding the Notes row reveals only note-children; expanding the Pages row reveals only page-children. Operations are unitary — sending-backward from either row moves the *whole* group run as one unit, so the notes and pages rows shift simultaneously within their respective sections. Pure-note and pure-page groups appear in only one section. Dissolve removes both rows. Render rule: walk `entityOrder`; for each group, partition members by section and emit a row in each section that has any members. Visually link the two rows (matching colour bar / chain glyph) so users read them as one group.

**Edges** — edges participate in stack order (FigJam parity: send-to-back, bring-forward, etc.) by being interleaved into `entityOrder` alongside entities. They render in `EdgeLayer` (within `aboveView`) and silently default to the top of the stack when newly created. They do **not** appear as rows in the sidebar — the sidebar shows entities, not connective tissue. Stack-order mutations on edges happen on the canvas (select edge → keyboard / context menu). Sidebar drag-reorder operates on the entity subsequence only; edge indices in `entityOrder` are untouched by sidebar drags (rule (i)).

Cross-surface stacking — putting a sticky behind a page, or a page in front of a drawing — is not supported today: `aboveView` always paints above all pages. The two-section sidebar surfaces that constraint directly instead of pretending one flat order can override it.

JSON Canvas note: the spec emits nodes and edges into separate arrays. The interleaved `entityOrder` (including edge ids) is persisted as a Specular extension (`specular.order` or equivalent); other JSON Canvas tools round-trip the data but lose precise stack interleaving for edges. Acceptable trade-off — same pattern as `specular.textStyle`.

## Auto-layout

A group can *manage* its children's layout instead of letting them float freely. See [ADR 0015](./docs/adr/0015-auto-layout-groups.md).

- **Auto-layout group** — a group with `managedLayout: true`; its children are reflowed into a row (`layoutMode: 'row'`, the only live mode in Milestone 1), not freely positioned. `'freeform'` groups are plain selection/bbox containers. Reflow (`reflowManagedGroup`, built on the pure `computeRowReflow`) is the single writer of a managed child's `canvasX/canvasY` — those become outputs, re-derived on any membership / resize / reorder change.
- **Layout sequence** — the left-to-right order of a managed row *is* the order of the group's run within `entityOrder` (no separate `childOrder` field). Reordering a child rewrites that run; reflow reads it. In a managed row, stack-order and layout-order are therefore the same axis (z-order is meaningless for non-overlapping row children).
- **Reorder handle (center dot)** — the per-child drag target that reorders a child within its auto-layout group, dragging it to a new slot while siblings reflow. A geometric `hit-test` layer above `body` (below `anchors`), painted in `aboveView` like edge anchors — small at rest, larger when the child or its group is hovered. Dragging the child **body** still moves the whole group as a unit; only the dot reorders. The drag runs in the `reordering-child` interaction mode and commits as one undo step.
- **Make auto-layout** — the headless entry point (no toolbar yet): `Shift+Cmd+A`, `specular auto-layout <ids|groupId>`, or `POST /groups/auto-layout`. Marks a selection (or existing group) as a managed row, seeding the sequence to current left-to-right order. The wrap-in-frame button, gap handles, direction/align/distribute controls are Milestone 2.

## Drag affordances

Modifiers and visual feedback that ride on top of entity drag (and resize) gestures. The single magnetic pull on the canvas is grid-snap (`snapToGrid`, 20 px); everything below either projects the delta before the snap (axis lock) or renders informationally after it (alignment / distribution guides). See [ADR 0012](./docs/adr/0012-alignment-guides-are-visual-only.md).

- **Axis lock** — holding `Shift` during an entity drag (with or without `Alt`/`Option` for copy) constrains movement to a single axis. Live: the dominant axis is recomputed every frame from cursor offset to drag origin, so the lock can flip H↔V mid-drag without releasing the mouse. Projection happens in main: `aboveView` adds a `shiftKey` flag to each `canvas-drag-entity` IPC update, and `applyDragDelta` zeros the smaller-magnitude axis. The locked axis bypasses grid-snap (entity holds its exact origin coordinate); the free axis still grid-snaps. Multi-select drag applies the same constrained delta to all entities.
- **Snap candidate** — an entity that contributes alignment edges during a drag or resize. Snapshot is taken once at gesture begin from the current viewport-visible entity set (rect intersects the available canvas viewport rect), excluding entities in the active selection. Pages and groups (snap to bbox) are included; group children inside an included group don't double-contribute. Pan is locked during drag, so the snapshot is stable.
- **Alignment guide** — a 1 px solid line in the canvas accent color, rendered in `aboveView`'s drag overlay layer, that confirms the dragged (or resized) entity's edge or center coincides with a snap candidate's edge or center within 0.5 px. Each candidate contributes 6 edges: top, bottom, left, right, horizontal-center, vertical-center; the dragged entity contributes its own 6 reference points (only the moving edge for resize). Guides are detected after grid-snap runs — they confirm an alignment, never create one — so they appear honestly and never lie. The line spans from min to max of the candidate and dragged rects on the snapping axis.
- **Distribution guide** — `==` measure marks rendered alongside alignment guides when the dragged entity sits at equal distance from two or more snap candidates along an axis (post-grid-snap). Same visual-only contract: detection only, no pull.

## Input authority

- **Page focus** — runtime state `{ id, since } | null` in main. When set, the focused page receives native pointer input; aboveView's gate is closed. When null, aboveView is the sole input authority. See [ADR 0001](./docs/adr/0001-click-to-enter-frame-focus.md). (ADR 0001 was authored under the old "frame" name; the runtime variable is currently `frameFocus` and renames to `pageFocus` in the migration.)
- **Gate** (a.k.a. **input gate**) — `aboveView.setVisible(...)` predicate. Open in canvas mode iff `pageFocus === null`. The single arbiter of who receives canvas-region pointer events.
- **Pointer router** — `src/renderer/above-view/useCanvasPointerRouter.ts`. Single window-level capture-phase pointerdown listener that runs the shared `hitTest` and dispatches a typed `CanvasPointerAction`. Yields to any element inside `[data-overlay-ui]`.
- **Hit-test priority table** — 5 layers, top wins: `resize-handles > chrome (geometric) > anchors > body > background`. Lives in `src/shared/hit-test.ts`. Geometric only — DOM overlay UI in aboveView resolves above all of them structurally. Refinement for `entity-body` on text/sticky/shape and editable file renderers (markdown, wireframe, video): when the hit entity is the sole current selection, no modifier is held, and nothing else is editing, the router emits `begin-entity-press` instead of `begin-entity-drag` — a stationary release routes to `canvas-request-entity-edit` while threshold-crossing movement falls through to drag. File renderers opt in via an `editable: boolean` flag on their plugin claim (broadcast as `rendererEditable` on the file scene entity); non-editable renderers (image, component placeholder) gracefully fall through to drag, and dblclick on those kinds is a noop rather than entering `editing-entity` mode with no editor on screen (issue #49 / `docs/interaction-layer.md` §4.2.1).
- **Edit mode** — runtime variable `editingEntityId: string | null` in main, derived from `interactionState` (controller mode `editing-entity`). Mutually exclusive with every other interaction mode (`dragging-entities`, `marquee`, `resizing-entity`, `dragging-edge`, `panning`). One IPC vocabulary covers every editable canvas item — `canvas-request-entity-edit`, `canvas-commit-entity-edit`, `canvas-cancel-entity-edit` — and main is the sole token holder. Renderers mount their editable surface (textarea / contentEditable / rename input) **iff** `editingEntityId === myId`; the read-only render is shown otherwise. Lifecycle: blur → commit, Escape → cancel (revert), pointer outside the editing entity's body → commit and swallow the click (the user clicks again to act on the new target), drag attempt on the editing entity → refused silently, drag attempt elsewhere → commits and swallows, selection change / entity deletion / undo / tab switch / window blur → cancel (the renderer's blur handler saves any in-flight text). Owned by `src/main/runtime/editing-entity-runtime.ts`.

## Overlay UI in aboveView

Per [ADR 0002](./docs/adr/0002-canvas-anchored-overlay-ui.md) and [ADR 0008](./docs/adr/0008-unified-canvas-item-popup.md). All canvas-anchored UI renders in aboveView's React tree, marked `data-overlay-ui` so the pointer router yields.

- **`CanvasItemChrome`** — persistent overlay UI for **identity** rendered while an entity exists. Per ADR 0008 §"File chrome", chrome carries identity affordances only (favicon + URL/filename + drag handle); all *actions* move into the popup. Examples: page favicon + URL display, file filename label.
- **`CanvasItemPopup`** — the unified popup for "configure this kind". One component, two anchor modes:
  - **Entity-anchored (selection mode)** — mounts when one entity (or a same-kind multi-selection) is selected, idle, after a 150 ms delay. Hidden during drag/marquee/resize/edit. Reads/writes that entity's fields.
  - **Viewport-anchored (tool mode)** — mounts under the toolbar, centered, when a creation tool with options is active (`add-text`, `add-shape`, `draw`). Reads/writes per-tool defaults in app settings; the next item created uses those defaults.
  - When a non-`select` tool with options is active, the tool popup wins; the selection popup is suppressed. Tools without options (`inspect`, `comment`, `add-page`, `add-document`) fall through to the selection popup.
  - Right-click context menus are out of scope.
- **`useAnchoredPosition(entityId, slot)`** — pure positioning hook. Reads the layout broadcast aboveView already receives, returns screen-space coords for the entity's chrome slot.
- **`EntityChrome` compound** — the `Root / DragTrigger / Title / Actions / Button` primitives composed inside `CanvasItemChrome` consumers. Style once, compose differently per consumer.

## Tool defaults

Per-tool, persistent app settings (not per-canvas, not in `.canvas`). Read by creation tools when stamping new entities; written by the tool-mode popup. Stored in user app settings (`~/Library/Application Support/Specular/...`) so each tool remembers its last-picked configuration across sessions and canvases.

| Tool | Defaults keys |
|---|---|
| `add-text` (plain) | `color` |
| `add-text` (sticky) | `color` |
| `add-shape` | `shapeKind`, `color`, `strokeWidth` |
| `draw` | `brushType`, `color`, `strokeWidth` |

Tool defaults never participate in undo/redo and never round-trip through Y.Doc — they're user preferences, not document data. See [ADR 0008](./docs/adr/0008-unified-canvas-item-popup.md) §"Tool defaults".

## Tools

A **Tool** is the single representation of "what does my next click/gesture do?" There is exactly one active tool at any moment. Tools are mutually exclusive; you switch by toolbar click, keyboard shortcut, or Escape (which returns to `select`). See [ADR 0005](./docs/adr/0005-unified-tool-concept.md), amended by [ADR 0006](./docs/adr/0006-unified-comment-tool.md) (comment tool subsumes region-select), and refined by [ADR 0009](./docs/adr/0009-tool-variants-in-popup-state.md) (variants move to tool defaults).

```ts
type Tool =
  | { kind: 'select' }                              // default
  | { kind: 'add-page' }                            // one-shot
  | { kind: 'add-text', style: 'plain' | 'sticky' } // one-shot
  | { kind: 'add-document' }                        // one-shot
  | { kind: 'add-shape' }                           // one-shot — shapeKind in tool defaults
  | { kind: 'comment' }                             // persistent — click for point/element comment, drag for region comment
  | { kind: 'draw' }                                // persistent — brushType in tool defaults
  | { kind: 'inspect' }                             // persistent
```

- **One-shot tools** auto-revert to `select` after one placement.
- **Persistent tools** stay active until toggled off, replaced, or Escape.
- The toolbar does **not** visually distinguish one-shot from persistent — users learn the duration by use.
- Tool name → cursor-label gerund: `select` → "selecting", `add-page` → "adding page", `comment` → "commenting", `draw` → "drawing", `inspect` → "inspecting".
- **Variants live in tool defaults, not in the union.** `add-shape` no longer carries `shapeKind`; `draw` no longer encodes `brushType` via implicit Tool state. Both are picked from the tool-mode popup and persisted to app settings (per ADR 0009). `add-text` is the deliberate exception — `style` stays in the union because plain vs sticky has been a long-established two-button affordance ([ADR 0004](./docs/adr/0004-text-affordances-and-spec-extensions.md)).

Replaces three previously-parallel state machines: `pendingPlacement`, `AnnotationMode`, and the `inspect` boolean. The legacy term "annotation mode" no longer names a state — annotations themselves remain, but the *mode of being in the comment tool* is just a tool.

**Not a tool:** **View mode** (canvas vs browser). View mode answers "which surface am I looking at?", not "what does my next click do?" — it's structural, not transient. Stays in its own state.

## Annotations

Comments live on the canvas as a single user-facing concept ("comment") and a single runtime entity (`Annotation`). One tool — `comment` — produces all of them; the gesture decides the **anchor** (per [ADR 0006](./docs/adr/0006-unified-comment-tool.md)):

- **Click on a page element** → element anchor (DOM selector + bbox).
- **Click anywhere else on the canvas** → canvas-point anchor (the click position).
- **Drag a marquee** → region anchor (canvas-rect, may span pages).

Discriminated by `anchor.type: 'element' | 'canvas' | 'region'`. The legacy `Annotation.kind` field is redundant once the anchor is the source of truth.

**Resting visual on the canvas** is asymmetric and matches today's behavior:
- Region anchor → dashed rose-400 rectangle, always visible (filtered only by `status`). Click opens the thread. Region rects are in canvas coords — they do **not** track page scroll (intentional: regions mark canvas space, not page content).
- Element anchor → no resting visual; lives in the right panel and surfaces via composer (pending) or popover (opened from panel). Element popovers re-query the live bbox via `selector` on every layout tick / page scroll, so they track scroll. If the selector no longer matches, the popover stays at its last-known position with a "stale anchor" indicator.
- Canvas-point anchor → same as element — no resting visual; selection from the right panel reveals a temporary marker at the canvas point and opens the thread popover.

**Pending composer** — single component that mounts after the gesture and before the comment is committed. Placement is a thin function over the anchor: above-right of the element bbox, adjacent to the click point, or above-right of the region rect. Esc cancels; click outside commits (if non-empty) or discards (if empty); only one pending composer exists at a time.

## Keyboard bindings

A **Binding** is one entry in the keyboard registry: `{ id, defaultKey, scope, target, when?, firesWhileTyping?, firesFromPageFocus?, label }`. The registry is the single source of truth — the dispatcher reads it, the app menu reads it, future tooltips and Bindings settings read it. See [ADR 0010](./docs/adr/0010-main-as-sole-shortcut-dispatch-site.md) and [`docs/plans/keyboard-binding-registry.md`](./docs/plans/keyboard-binding-registry.md).

- **BindingId** — string-literal union of every shortcut's canonical name (e.g. `'tool-comment'`, `'undo'`, `'select-all'`). Lives in `src/shared/bindings.ts`.
- **NormalizedKey** — `{ key, cmd, alt, shift }` where `key` is lowercased and `cmd` collapses `Cmd`/`Ctrl` per platform. The cross-process key representation; both Electron `Input` events and DOM `KeyboardEvent`s normalise to this.
- **Binding dispatcher** — pure function `dispatchKey(table, key, ctx) → BindingId | null` in `src/shared/bindings.ts`. The only dispatch happens in main's `before-input-event` listener, attached to every WebContentsView (see ADR 0010).
- **Keyboard source view** — which WebContentsView produced a keystroke (`'aboveView' | 'canvasBg' | 'toolbar' | 'leftSidebar' | 'rightDetailsPanel' | 'devtoolsHeader' | 'devtoolsResizeHandle' | 'page'`). Each binding declares the source views it can fire from via `scope: KeyboardSourceView[]`. Distinct from **page focus** (ADR 0001): page focus says "which page receives native input"; keyboard source view says "which view produced this keystroke."
- **Binding target** — `'main' | KeyboardSourceView`. Where the handler runs. Most bindings run in main; a few (annotation overlay) run in a renderer because their state is renderer-local React `useState`.
- **`firesWhileTyping`** — opt-in flag for a binding to fire when `isTextEditing` is true. Default `false`. Only `undo`, `redo`, `reset-viewport`, and `escape-tool` opt in.
- **`firesFromPageFocus`** — opt-in flag for a binding to fire when a page has keyboard focus (per [ADR 0011](./docs/adr/0011-page-focus-respects-native-shortcuts.md)). Default `false`. Only `escape-page-focus` and `reset-viewport` opt in.

**Keyboard shortcuts and tools.** Every `Tool['kind']` has a default key, enforced by TypeScript exhaustiveness. Variant keys (e.g. Shift+R for diamond, Shift+M for highlight) activate the tool *and* write the variant to **tool defaults** per ADR 0009. Pressing a tool's key while that tool is already active is a no-op (FigJam reference); Escape is the only keyboard path back to `select`.

## UI copy voice

- **Sentence case** — capitalize the first word only. Default for menus, buttons, dialog text, chrome labels: "Reveal codebase in finder", "Delete project…", "Rename".
- **Lowercase gerund narration** — used **only** for cursor labels, status bar text, and live captions. "clicking submit", "dropping in frame", "looking at toolbar". No quotes, arrows, or colons.

---

*This file is the canonical glossary. When a term resolves during planning or grilling, update it here. Don't couple to implementation details — only include terms that are meaningful to the domain, not internal helpers.*
