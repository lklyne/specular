# ADR 0008 — Unified canvas-item popup, selection-driven and tool-driven

**Status:** Accepted (all 11 migration steps landed)
**Date:** 2026-05-10
**Builds on:** [ADR 0002 — Canvas-anchored overlay UI in aboveView](./0002-canvas-anchored-overlay-ui.md), [ADR 0005 — Unified `Tool` concept](./0005-unified-tool-concept.md).
**Refined by:** [ADR 0009 — Tool variants in popup state](./0009-tool-variants-in-popup-state.md) (companion change; tool variants move out of the `Tool` union into tool-mode popup state).

## Context

Today, "menu attached to a canvas item" is implemented inconsistently across kinds:

| Kind | Component | Anchor | Visibility | Contents |
|---|---|---|---|---|
| Page | `CanvasItemChrome` (above-view) | Header slot above body | Always-on in canvas mode | URL bar, back/forward/reload, ⋮ |
| File (markdown/wireframe) | `CanvasItemChrome` | Header slot | Always-on | Filename + rename; wireframe gets theme picker / json toggle behind a hardcoded conditional |
| File (image / video / component) | — | — | None | — |
| Group | `GroupInlineMenu` via `InlineEntityMenu` (canvas-bg, screen-coords) | Above body | Single-select + idle + 150 ms delay | Color, dup, del |
| Sticky note | `StickyNotePopover` via `CanvasItemPopup` (above-view) | Above body | Single-select + idle + 150 ms delay | Color, dup, del |
| Plain text, shape, drawing | — | — | None | — |

Two patterns coexist for the same conceptual UI ("a small menu surfacing options for this canvas item"):

- **Always-on chrome** (`CanvasItemChrome`) — page URL/nav, file filename + rename, wireframe theme.
- **Selection-driven popup** (`CanvasItemPopup` for sticky; the older screen-coords `InlineEntityMenu` for group).

The chrome carries both *identity* (favicon, filename) and *actions* (rename, nav, theme, json toggle, ⋮). The selection popup carries actions only. They overlap, occupy the same screen real estate, and create the inconsistency.

Furthermore, today the user has no way to pick defaults *before* creating a new entity. Stickies are stamped with `DEFAULT_TEXT_COLOR` (yellow constant); drawings are stamped with brush-baked-in pen/highlight color and width. There is no "set the next sticky to red" affordance.

## Decision

One **`CanvasItemPopup`** component for "configure this kind of thing", with two anchor modes:

### 1. Two anchor modes, one component

- **Entity-anchored (selection mode)** — the popup mounts when an entity (or a same-kind multi-selection) is selected. Reads/writes that entity's fields. This is today's behavior, generalized to every kind.
- **Viewport-anchored (tool mode)** — the popup mounts as a fixed strip below the toolbar, centered, when a creation tool with options is active. Reads/writes per-tool defaults stored in app settings. The next entity created by the tool uses those defaults.

The component itself is shared. Only the positioning hook differs (entity-anchored uses `useAnchoredPosition`; tool-anchored uses a viewport-fixed style).

### 2. Mutex rule: tool wins when active

When the active tool is anything other than `select` AND that tool has a popup, only the tool popup shows. The selection popup is suppressed even if entities are selected. When the active tool is `select`, or the active tool has no popup (`region-select`, `inspect`, `comment`, `add-page`, `add-document`), the selection popup behaves normally.

Rationale: the active tool is the user's current verb. If they've left `select`, their intent is no longer about the previously-selected thing. Showing two popups simultaneously, or switching to `draw` and *still* seeing the sticky's color picker, surfaces stale context.

### 3. Visibility rules

**Selection mode:**
- Single-select OR same-kind multi-selection.
- Idle gate (hidden during `dragging-entities`, `marquee`, `resizing-entity`, `dragging-edge`, `panning`).
- 150 ms delay before mounting (prevents flicker during quick click-and-drag).
- Hidden during `editing-entity`.

**Tool mode:**
- Mounts immediately on tool activation; unmounts immediately on deactivation.
- No idle gate; no delay. The tool is itself a stable state, not a transient one.

### 4. Multi-select scope

Same-kind multi-select shows the popup; mixed-kind multi-select shows nothing. Color picker shows no swatch active when colors differ; clicking a swatch applies to every selected entity. Cross-kind intersection logic is deferred — its own substantial design pass under a future ADR if needed.

For text entities, `style: 'plain'` and `style: 'sticky'` count as same kind for color purposes (both back the `text` kind and both have a `color` field). Plain text and sticky are not interchangeable for the style toggle, but for color they are.

### 5. Per-kind contents

| Kind | Selection popup | Tool popup |
|---|---|---|
| `page` | URL bar + back/forward/reload + dup + del | — |
| `text` plain | color + dup + del | color |
| `text` sticky | color + dup + del | color |
| `file` | filename label + rename + dup + del + per-renderer contributions (e.g. wireframe theme + json mode) | — |
| `group` | color + dup + del | — |
| `drawing` | brushType (pen/highlight) + color + strokeWidth + dup + del | brushType + color + strokeWidth |
| `shape` | shapeKind (rect/ellipse/diamond) + color + strokeWidth + dup + del | shapeKind + color + strokeWidth |

Page in selection mode stays close to its existing chrome — it gets URL/nav/dup/del in the popup. Page has no tool popup because `add-page` has no per-tool defaults (URL is typed after placement; `presetIndex` is selected through a separate UI not in this work's scope).

### 6. File chrome shrinks to identity-only

Today's file chrome carries both identity (filename) and actions (rename, wireframe theme, json mode, ⋮). Going forward:

- **`CanvasItemChrome` for files keeps identity-only.** Favicon + filename label, rendered passively. No click-to-rename, no action buttons. Persistent (always-on) so the file's name stays visible at a glance.
- **All file actions move into the popup.** Rename, dup, del, and per-renderer contributions (wireframe theme picker, json mode toggle, etc.).

> **Update (2026-05-21):** The persistent file chrome was removed entirely — `FileChromeOverlay` / `FileChrome.tsx` are gone. A file's name now lives only in the selection popup (which already carries rename + actions), so an idle file shows no heading. The hit-test still reserves a chrome strip above each file (`entityHasChrome` in `src/shared/hit-test.ts`), so files remain draggable from that strip and from their body. This is option **B** below ("collapse chrome into popup entirely") applied to files only; pages keep their chrome.

The same split applies in principle to pages — chrome carries favicon + URL display (identity), popup carries dup/del + nav + URL editing — but for this work, the page chrome stays unchanged. We're not removing or rewiring the page URL bar in this pass; the page popup *adds* dup/del/(URL/nav redundancy is acceptable for now) and lives alongside chrome. A follow-up may consolidate.

### 7. Renderer plugin contribution surface

Today, file-renderer-specific options (wireframe theme, json mode) are hardcoded into `FileChrome.tsx` via `WIREFRAME_EXTENSIONS.test(...)` conditionals. Going forward, the entity-renderer plugin registry (`src/main/plugins/registry.ts`) gains a contribution surface so each renderer registers its own popup options:

```ts
registerEntityRenderer({
  match: (entity) => /\.wireframe\.json$/i.test(entity.file),
  rendererTag: 'wireframe',
  editable: true,
  popupContributions: () => [
    { id: 'theme', kind: 'theme-picker', /* ... */ },
    { id: 'json-mode', kind: 'toggle', /* ... */ },
  ],
})
```

The popup composes: shared core (dup/del) + kind options (file = filename + rename) + per-renderer contributions. The `FileChrome.tsx` conditional disappears.

Exact contribution-surface shape (renderer-side React vs main-side declaration, where state lives) is implementation work — not load-bearing for this ADR. The principle is: renderer plugins own their popup options.

### 8. Drawing data model

No new fields on `CanvasSceneDrawingEntity`. Today every drawing has exactly one stroke (each pointerdown→pointerup creates a new drawing entity wrapping one new stroke), so the popup reads/writes that stroke's `color`, `width`, and `brushType` directly. If a drawing somehow has multiple strokes (legacy import), the popup writes uniformly to all strokes; reads show the dominant color, or no active swatch when mixed.

Alternative considered: promote `color`/`strokeWidth`/`brushType` to the drawing entity. Rejected — introduces a new field that has to round-trip through Y.Doc, persistence, and JSON Canvas serialization, for no behavioral gain (per-stroke fields already exist and already work).

### 9. Tool defaults

Per-tool, persistent app settings. Not per-canvas, not in the `.canvas` schema, not in Y.Doc. Stored in user app settings so each tool remembers its last-picked configuration across sessions.

```
tool-defaults:
  add-text.plain.color
  add-text.sticky.color
  add-shape.shapeKind
  add-shape.color
  add-shape.strokeWidth
  draw.brushType
  draw.color
  draw.strokeWidth
```

Read by creation tools when stamping new entities; written by the tool-mode popup. Never participate in undo/redo (they're user preferences, not document data). Exact storage module owned by `src/main/runtime/tool-defaults.ts` (new), backed by the existing app-settings store.

## Alternatives considered

**A. Keep two components (chrome and popup) entirely separate; just standardize the popup so every kind gets one.** Selection-driven popup for every kind, no toolbar mode. Doesn't deliver the "configure tool defaults before creating" behavior; doesn't address the chrome-carries-actions inconsistency that motivated the work. Rejected — under-delivers.

**B. Collapse chrome into popup entirely; selection-driven only.** No always-on chrome at all; filename / URL only visible when an entity is selected. Loses passive identification — you'd have to click each page on the canvas to see its URL. Rejected — too aggressive a UX change for this pass.

**C. Toolbar popup as a popover anchored to each tool button, not a fixed strip.** Couples popup geometry to the toolbar's internal layout (which buttons sit where) and gets weird when a tool is activated by keyboard shortcut rather than click. Rejected — symmetry with the selection popup (both are "a strip with options for the active subject") is cleaner.

**D. Tool defaults live in workspace runtime / Y.Doc (per-canvas).** Each canvas remembers its own tool defaults. Rejected — tool defaults are user preferences, not document data. Different canvases shouldn't disagree about "what color is my pen". App settings is the right home.

**E. Cross-kind multi-select shows the intersection (color always, dup/del always; pages mixed → no color).** Real edge cases (1 page + 3 stickies — what does the popup mean?) and the "this is more shared than it looks" inference is fragile. Deferred to a future ADR if real workflows demand it.

**F. Strokes become first-class selectable canvas items (sub-selection inside a drawing).** Substantial change to selection state, hit-test, undo grain, JSON Canvas serialization. Conflates with this work. Deferred — its own design pass + ADR if pursued.

## Consequences

**Replaces:**
- `GroupInlineMenu` and the canvas-bg `InlineEntityMenu` screen-coords positioning. Group selection menu re-implements on `CanvasItemPopup`.
- `StickyNotePopover` is restructured to be one consumer of a generalized `CanvasItemPopup` — same anchoring, broader content slots.
- The hardcoded wireframe-theme / json-mode conditional in `FileChrome.tsx` is replaced by the renderer plugin contribution surface.
- File chrome's action buttons and rename trigger are removed; chrome shrinks to favicon + filename display.

**Enables:**
- Every canvas-item kind gets a consistent menu surface.
- Configurable tool defaults: pick "the next sticky should be red" before placing it. Drawings get a configurable brush + color + thickness.
- Same-kind multi-select recoloring (3 drawings → red in one click).
- Selection-popup can carry post-creation variant swaps (sticky → plain, rect → ellipse, pen → highlight).
- The `Tool` union shrinks (per ADR 0009).

**Costs:**
- Real refactor across multiple kinds. Sticky and group are partial migrations; plain-text, shape, drawing, file are net-new popup wiring.
- Renderer plugin contribution surface is a new concept on `src/main/plugins/registry.ts` — modest API design + tests.
- New app-settings module for tool defaults; first-time-launch defaults need to ship with the app.
- Existing tests that referenced `GroupInlineMenu` or `StickyNotePopover` need rewriting against the unified component.
- `FileChrome.tsx` shrinks substantially; downstream consumers (rename starting via title click) need their entry points moved into the popup.

**Out of scope:**
- Right-click context menus (selection-driven popup only, per ADR 0002 §4).
- Cross-kind multi-select intersection logic.
- Stroke-as-selectable sub-selection inside a drawing.
- Page chrome consolidation (page chrome stays as today; popup adds alongside).
- Toolbar layout changes beyond what ADR 0009 implies for shape (one button instead of three).
- `add-page` preset picker as a tool popup contribution.
- Brush-type swatch unification with the keyboard-shortcut path for pen/highlight (today there's an implicit shortcut; the popup doesn't replace it, it adds a UI for it).

## Migration

This is a sequence of vertical slices, not a big-bang. Each slice ships green typecheck + unit + smoke.

Progress: All 11 steps landed. Step 5 migrated the group selection popup off `GroupInlineMenu` onto `CanvasItemPopup`, deleted `InlineEntityMenu.tsx`, and factored a shared `toolHasPopup(tool)` helper for the §2 mutex rule. Steps 6–7 landed together with ADR 0009: the `Tool` union shrunk (`add-shape` and `draw` no longer carry sub-kind variants), the toolbar collapsed to one shape button + one draw button, the placement IPC + drawing gesture now read shapeKind / brush / color / strokeWidth from tool defaults, and four new popups (`ShapeToolPopup`, `ShapePopup`, `DrawToolPopup`, `DrawingPopup`) were wired in `above-view`. Steps 8–11 finished the migration: the renderer plugin registry gained `popupContributionTags`, the wireframe theme picker + json-mode toggle moved into the new `FilePopup` via the contribution dispatch in `src/renderer/above-view/file-popup-contributions/`, `FileChrome` shrunk to favicon + filename identity-only, every popup gained same-kind multi-select via `useMultiAnchoredPosition`, a `PagePopup` joined the lineup, and `selectedPageMenu.ts` was renamed to `popupTiming.ts` with the dead `textEntityMenuViewBounds` deleted.

1. ✅ **Generalize `CanvasItemPopup` for content composition.** Today `CanvasItemPopup.Root` is a positioning shell. Add slot primitives or a shared "content frame" so kind-specific popup contents compose consistently (color swatches block, action button block, rename inline-edit block, variant picker block).
2. ✅ **Tool defaults storage.** New module `src/main/runtime/tool-defaults.ts`. Reads/writes app settings; broadcasts changes via an existing IPC channel (or a new one). First-time defaults: sticky yellow, plain transparent/inherit, shape rectangle/black/2px, draw pen/black/2px.
3. ✅ **Tool-mode popup positioning.** New positioning code in aboveView for "fixed below toolbar". Reads the toolbar's screen-space bottom edge from the layout broadcast (may need a new field).
4. ✅ **Migrate sticky note.** `StickyNotePopover` becomes the sticky consumer of the generalized component. Selection popup contents unchanged. Adds tool-mode popup for `add-text` (style: sticky and plain).
5. ✅ **Migrate group.** `GroupInlineMenu` deleted; group becomes another consumer. Same selection contents.
6. ✅ **Add shape popup.** `add-shape` tool popup with `[shapeKind picker] [color] [strokeWidth]`. Selection popup with the same plus dup/del. ADR 0009 lands here in the same PR — `Tool` union shrinks.
7. ✅ **Add drawing popup.** `draw` tool popup with `[brushType picker] [color] [strokeWidth]`. Selection popup writes to inner stroke.
8. ✅ **Renderer plugin contribution surface.** `BaseRendererClaim.popupContributionTags` (string list) declares each renderer's contributions; the wireframe claim opts into `wireframe-theme` and `wireframe-json-mode`. Tags ride on the file scene entity as `popupContributions: PopupContributionTag[]`. Renderer side: `src/renderer/above-view/file-popup-contributions/index.tsx` switches the tags onto React components. `FilePopup` composes rename + plugin contributions + dup/del. `FileChrome` shrunk to favicon + filename only.
9. ✅ **Same-kind multi-select.** All five kind popups (text, group, shape, drawing, page, file) mount on single OR same-kind multi-select. `useMultiAnchoredPosition` returns the union bbox; the popup anchors against it. Shared-value detection drives the active swatch — when colors/variants/widths diverge across the selection, no swatch shows active. Per ADR §4 plain + sticky count as same kind for color. Groups stay single-only (runtime has no multi-group selection concept).
10. ✅ **Page popup.** `PagePopup` consumes `CanvasItemPopup.Root` with back/forward/reload + URL input + dup/del on single-select; collapses to dup/del on multi-page select. Page chrome unchanged per §6.
11. ✅ **Cleanup.** `InlineEntityMenu.tsx` deleted earlier with step 5. `selectedPageMenu.ts` renamed to `popupTiming.ts`; the unused `textEntityMenuViewBounds` function and its private constants are gone. The 150 ms delay constant survives as `POPUP_SHOW_DELAY_MS`, consumed by every selection popup.

Each slice has its own typecheck + unit + smoke gate. The PR may ship as a single coordinated change or as a stack of dependent PRs — TBD by the implementer.

## Tests

- **Unit:** `CanvasItemPopup` mounts/unmounts on selection state transitions; idle gate; 150 ms delay; same-kind multi-select detection; tool-mode mount/unmount on tool activation.
- **Unit:** Tool defaults round-trip through app settings; per-tool isolation (changing draw color does not affect shape color).
- **Unit:** Renderer plugin contribution surface composes correctly per renderer tag.
- **Smoke:** Every kind shows a popup on selection. Tool-mode popup appears below toolbar when a creation tool with options is active. Picking a swatch in the tool popup, then placing an entity, results in an entity with that color. Same-kind multi-select recoloring writes to all selected.
- **Smoke:** Mutex rule — selecting a sticky and then activating `draw` hides the sticky popup and shows the draw popup; switching back to `select` restores the sticky popup.
- **Smoke:** File rename through popup; wireframe theme through popup; json toggle through popup. File chrome no longer carries those affordances.
- **Manual:** Pan/zoom/drag tracking for entity-anchored popup; viewport stability for tool-anchored popup under window resize.
