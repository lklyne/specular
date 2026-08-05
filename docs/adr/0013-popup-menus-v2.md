# ADR 0013 — Popup menus v2: palette, text size, cross-kind morph, toolbar regrouping

> **Note (2026-07).** The wireframe entity-renderer mode referenced in Phase 6
> below (device-frame + rotate-viewport controls on the wireframe `FilePopup`
> variant) was deleted wholesale — see the "delete wireframe mode" PR. The
> renderer-plugin popup contribution surface it depended on was removed too.

**Status:** Accepted
**Date:** 2026-05-15
**Implemented:** 2026-07 (palette resolver, Tool union restructure including `add-sticky`/`add-document` split, text size control, visual token lock, toolbar regrouping, and popup content shipped). §3 cross-kind morph was superseded — see inline note. §7 wireframe `FilePopup` additions were deleted alongside wireframe mode removal.
**Refines:** [ADR 0004 — Text affordances and spec extensions](./0004-text-affordances-and-spec-extensions.md), [ADR 0005 — Unified `Tool` concept](./0005-unified-tool-concept.md), [ADR 0008 — Unified canvas-item popup](./0008-unified-canvas-item-popup.md), [ADR 0009 — Tool variants in popup state](./0009-tool-variants-in-popup-state.md).

## Context

ADR 0008 unified every per-kind menu under one `CanvasItemPopup`, with two anchor modes (selection vs tool). ADR 0009 shrank the `Tool` union by moving `shapeKind` and `brushType` into tool defaults. That work landed cleanly but treated each kind's popup contents as a soft default, not a locked visual design.

Carrying that further has surfaced design decisions the earlier ADRs didn't bind:

- The color set was six muted pastels (`canvas-colors.ts`) — the visual design now calls for **eight** swatches with a theme-aware "neutral" leading slot.
- The text/sticky/markdown affordances overlapped in confusing ways. Plain text and `.md` documents are user-facing siblings; sticky is a distinct concept; the existing `add-text` + `add-document` + `style: 'plain' | 'sticky'` mixture conflated all three.
- Text rendering has no per-entity **size** control today — sticky/plain-text/shape-label all render at one hardcoded size.
- The element-anchored comment composer ([CONTEXT.md §Annotations](../../CONTEXT.md#annotations)) has no first-class name field, so the right-panel comment list reads as "comment on `<div>`".
- Page-popup scope was carved out of ADR 0008 §6 with the note "follow-up may consolidate." The follow-up is now: device-frame toggle and viewport rotation move from the right panel into the popup.
- Toolbar visual grouping has drifted — text, sticky, and document sit under one "Add text ▾" dropdown, while their semantic homes are different.

## Decision

### 1. Eight-slot color palette, soft/vivid hues, theme-aware neutral

The popup color row exposes **eight slots**, left → right: `neutral · purple · blue · cyan · green · yellow · orange · red`. The same lineup for every kind that picks a color (sticky, text, shape, drawing/pen, group).

Slot 1 ("neutral") is **theme-aware and role-aware**. The same on-disk encoding resolves to a different RGB depending on (a) the active color mode and (b) the entity role:

| Role | Light mode | Dark mode |
|---|---|---|
| Surface-fill (sticky, shape fill, plain-text background if any) | Light | Dark |
| Ink (pen / highlighter stroke, plain text glyphs) | Dark | Light |

Stickies marked neutral recede into the canvas; pen strokes marked neutral stand out.

Slots 2–8 (the hues) carry **two palettes**. The muted-pastel set works for surfaces that should sit back; a punchier saturated set works wherever contrast matters. The palette a surface uses is fixed by the surface, not picked by the user — choosing "purple" picks the slot; the surface decides which hue:

| Palette | Surfaces |
|---|---|
| **soft** (muted pastels) | sticky body, shape fill + stroke, highlighter brush |
| **vivid** (saturated) | plain-text glyphs, edges/connectors, pen brush, group accent |

The draw tool derives its palette from the active brush — pen → vivid, highlighter → soft. No re-keying is needed when the brush toggles: the stored value is the *slot*, and the renderer picks the hue from the brush.

**Disk format (JSON Canvas v1.0 compliant — preset numbers + Specular extension):**

`color` stores the **slot**, not a hue — the palette is the surface's, resolved at render. The same `"2"` reads muted orange on a sticky and punchy orange on a pen.

- Neutral → `specular.colorRole: "neutral"`. The `color` field is omitted, or carries `"1"` as a fallback for other JSON Canvas readers.
- Hues → the JSON Canvas preset number: `"1"`–`"6"` for the six spec hues (red/orange/yellow/green/cyan/purple) and `"7"` for blue. The spec only numbers six colors; `"7"` is a Specular extension — other JSON Canvas apps render the first six per their own preset and fall back for blue.
- A literal `#RRGGBB` hex is still accepted in `color` (custom colors, and canvases saved before the two-palette split) and renders verbatim — no palette resolution.

Resolution lives in `src/shared/canvas-colors.ts`; the module exposes a `role` parameter (theme-aware neutral) and a `soft`/`vivid` `palette` parameter that the render sites pass from the surface they paint. An agent (or hand-edited `.canvas`) only needs to write a preset number — the palette follows the item it lands on.

This supersedes the earlier "hues are fixed pastels regardless of theme/role" wording: legacy `"1"`–`"6"` values are no longer frozen to the muted hexes — they resolve per surface like any other preset.

### 2. Text size as a per-entity property

Every kind that renders text — `text` (plain and sticky) and `shape` (the inner label) — gains a `textSize` property. The popup surfaces it as a labeled dropdown ("Small ▾") with presets:

| Preset | Pixels |
|---|---|
| Small | 14 |
| Medium | 32 |
| Large | 56 |
| Extra large | 96 |
| Huge | 144 |

Plus a raw-pixel input at the bottom of the open dropdown for arbitrary values (8–256 range, integers). Tool defaults gain `textSize` for `add-text`, `add-sticky`, and `add-shape`. Pen popup deliberately does **not** use the labeled-dropdown pattern — pen stroke width stays as two inline preview buttons because the visual is the value.

Shape stroke width remains in the data model but is **not** exposed in the popup in this pass; future work.

### 3. Cross-kind morph (text ↔ file)

> **Superseded (2026-07).** This section is retired. Text and Document are now two
> independent one-shot toolbar tools (`add-text` and `add-document`); the tool you
> pick determines the entity. The `short`/`long` popup toggle, the `add-text.textKind`
> preference, and the cross-kind morph (`morph-text-file` IPC + `TextKindToggle` +
> the markdown-morph popup contribution) were all deleted — this ADR's rejected
> alternative **C** is what shipped. Convert-in-place is no longer offered; retype or
> copy/paste to change kind. The rest of the section is kept for history.

Plain text and Document (`.md`) are presented to users as two flavors of one "text" concept. The popup's leading variant pair (`short` / `long`) toggles between them:

- **Tool mode** — `short` stamps a `text` entity with `textStyle: 'plain'`; `long` stamps a `file` entity backed by a new `.md` file on disk.
- **Selection mode** — clicking the inactive variant **morphs** the existing entity across kinds:
  - *short → long*: write the text body to a new `.md` file in the workspace, replace the `text` entity with a `file` entity at the same rect, strip color/size fields (markdown content owns its own formatting).
  - *long → short*: read the `.md` content, flatten to plain text, replace the `file` entity with a `text` entity at the same rect.

Both directions trigger file CRUD and are lossy. One undo step reverses the morph including the file write/delete. No confirmation dialog — the popup tile is the affordance.

Sticky is **not** in this toggle. Sticky has its own toolbar entry and its own popup.

### 4. `Tool` union restructure

```ts
type Tool =
  | { kind: 'select' }       // default
  | { kind: 'add-page' }     // one-shot — "frame" in the toolbar (icon, not name)
  | { kind: 'add-text' }     // one-shot — no style variant; popup picks short/long
  | { kind: 'add-sticky' }   // one-shot — separate first-class tool, not a text style
  | { kind: 'add-shape' }    // one-shot
  | { kind: 'comment' }      // persistent
  | { kind: 'draw' }         // persistent
  | { kind: 'inspect' }      // persistent
```

Changes from ADR 0009's union:
- **`add-sticky` added.** Sticky is its own tool. The old `{ kind: 'add-text', style: 'sticky' }` is gone.
- **`add-document` removed.** Markdown files are reached via the text popup's `short → long` toggle, not a top-level tool.
- **`add-text` loses `style`.** Plain text is now the only direct creation path of the text tool; the popup decides short vs long.

Tool-defaults table updates accordingly:

| Tool | Defaults keys |
|---|---|
| `add-text` | `color`, `textSize` |
| `add-sticky` | `color`, `textSize` |
| `add-shape` | `shapeKind`, `color`, `strokeWidth`, `textSize` |
| `draw` | `brushType`, `color`, `strokeWidth` |

### 5. Toolbar regrouping

Left → right: *nav* (`select`, `hand`) → *create* (`draw`, `add-sticky`, `add-shape`, `add-page`) → *annotate* (`add-text`, `comment`, `inspect`) → *view* (theme, zoom). Plain text moves to *annotate* because writing words on the canvas is an annotation act; sticky sits in *create* because the sticky is the thing itself.

The "Add text ▾" dropdown is removed.

### 6. Element name on element-anchored annotations

Element-anchored annotations gain a `elementName` field — a single-line label like "Submit button" or "Hero CTA". The pending composer presents it as the topmost input, above the message thread. The new composer also subsumes the comment-thread surface (replies, etc.) that the current composer doesn't carry. Canvas-point and region anchors don't get an element name (anchor itself is the identity).

### 7. Page popup picks up device frame + rotate

The selection-mode `PagePopup` (and `FilePopup` for wireframe-rendered files) gain:

- **Device frame toggle** — same `showDeviceFrame` boolean today exposed in the right-panel `DeviceFrameSection`. Persistent active state.
- **Rotate viewport** — momentary action that swaps between `portrait` and `landscape` (same code as `PagePane`'s existing rotation control). No persistent active state on the button — each click flips orientation.

These are *also* still available in the right panel; the popup adds a faster path, not a replacement.

### 8. Visual treatment

One fill color drives every interactive state. No borders on idle, hover, or active.

**Figma source of truth.** All popup pixel/color/radius values pulled from `file://hgwwoe0EzUrErdviULmRtb` (the **agent-canvas** Figma file). Per-popup row node ids:

| Row | Light-mode node | Dark-mode node |
|---|---|---|
| Pen popup | `360:10` | `360:66` |
| Sticky popup | `362:123` | — |
| Shape popup | `362:165` | — |
| Page popup | `362:210` | — |
| Text popup (short text) | `362:257` | — |
| Text popup (long / `.md`) | `362:297` | — |
| Toolbar (header) | `362:600` | — |
| Element-name composer | `362:672` | — |

Use `mcp__plugin_figma_figma__get_design_context` or `…__get_screenshot` against these node ids to refresh any visual reference during implementation.

| Token | Light | Dark |
|---|---|---|
| Container bg | `#ece9e7` | `#3a3836` |
| Container border (1px) | `#dcdcda` | `#414141` |
| Container shadow | `0 10 8 -6 rgba(0,0,0,.18), 0 4 16 0 rgba(199,193,188,.5)` | `0 10 8 -6 rgba(0,0,0,.58), 0 4 16 0 rgba(0,0,0,.5)` |
| Active / hover button fill | `#fdf8f5` | `rgba(253,248,245,0.1)` |

- Container radius 10; inner button radius 6.
- Container padding 4; internal gap 4.
- Inner button 24×24; color swatch 20×20 with 12px dot inside.
- **Active swatch** = outer ring in the swatch's own color (not ink).
- **Action buttons** (copy/trash/back/forward/reload) get the fill only on hover; no destructive variant for trash.
- **Toggle buttons** (brush, stroke, shape variant, swatches, dropdown triggers) get the same fill on hover *and* while they're the current pick.

## Alternatives considered

**A. Keep six JSON Canvas presets, drop one hue for neutral.** Cleaner mapping (slot 1=neutral, 2–6=hues) but forces dropping orange or yellow to make room. Rejected — the eight-slot palette is the visual design lock, and the spec already permits hex strings for additional colors.

**B. Encode neutral as its own hex (e.g. `#ffffff` or `#000000`) chosen at write time for the current theme.** Disk format simpler — no Specular extension. Rejected — loses theme-awareness; reopening a canvas in the other theme would surface a stale-looking neutral that no longer matches the canvas surface.

**C. Keep `add-document` as a top-level tool; only add the cross-kind morph in selection mode.** Preserves backwards compatibility for users who learned the document tool. Rejected — the toolbar shrinks meaningfully and the short/long popup toggle is the same affordance whether you're creating or editing. Two paths to the same thing is the confusion ADR 0008 set out to remove.

**D. Confirmation dialog before text → file morph.** File creation is a side effect a user might not expect. Rejected — undo reverses the morph including the file. The popup tile is the affordance; a dialog would be friction against a reversible operation. (We may revisit if telemetry shows users morphing accidentally.)

**E. Active-swatch ring in ink color (high-contrast black/white).** Reads more like a system focus ring; clearer "this is the current pick" signal. Rejected — the ring-in-own-color reads as "this color, emphasized," which is the right meaning. The ink-color ring would make every active swatch look the same in its mode.

**F. Persistent destructive tint on trash.** Some apps tint the trash button red at rest as a warning. Rejected — the popup is contextually scoped (you've already selected a specific thing); the trash icon glyph carries enough warning. Resting tint would make the popup feel alarming.

**G. Multi-select short/long morph applies across mixed selections.** Selecting one text + one file and clicking either tile morphs the dissenter. Rejected — multi-kind selection already shows no popup per ADR 0008 §4. This is consistent with that.

## Consequences

**Replaces:**
- `canvas-colors.ts` six-pastel array. Replaced by eight-slot palette with neutral resolver.
- `add-document` tool entry, IPC channel routing, toolbar button.
- `{ kind: 'add-text', style: 'plain' | 'sticky' }` discriminated variant. Two separate tools (`add-text` + `add-sticky`).
- Right-panel `DeviceFrameSection` as the only path to the device-frame toggle (it stays; the popup adds a second path).
- Existing pending comment composer. The new element-anchor composer also subsumes the comment-thread surface for the element-anchored flow.
- Six-pastel default colors in tool defaults storage.

**Enables:**
- Per-entity text sizing (plain text, sticky, shape inner label).
- Single user-facing surface for "I want some text here" — pick short or long, change your mind later.
- Theme-aware neutral that adapts to canvas surface color without a per-canvas color setting.
- Wireframe file device-frame toggle reachable from the canvas without round-tripping to the right panel.
- A first-class label for element-anchored comments, surfaced in the right-panel comment list.

**Costs:**
- `.canvas` schema changes for `specular.colorRole` and `specular.textSize`. Other JSON Canvas readers ignore them.
- Existing canvases with `color: "1"`–`"6"` digit values now resolve per surface (a `"2"` sticky stays muted; a `"2"` pen stroke becomes punchy). This is intended, not a migration — the preset *is* the slot. Canvases saved during the brief window when popups wrote literal hex keep that hex and render verbatim until re-picked.
- `add-document` removal: existing canvases with `file` entities pointing at `.md` files are unaffected (data is the same). The CLI / agent integration may reference `add-document` and needs updating.
- Cross-kind morph implementation: needs the file-create + entity-replace pipeline to be transactional so undo reverses both halves.
- Element-name field is a schema addition on the `Annotation` type. Backfill is unnecessary (empty string is fine for older annotations).
- The toolbar Header redesign carries its own tests (toolbar layout snapshots, keyboard-shortcut tests) and may stack as a follow-up PR rather than landing in the same PR as the popups.

**Out of scope:**
- Shape stroke width in the popup (future).
- Right-click context menus.
- Cross-kind multi-select intersection.
- Stroke-as-selectable sub-selection inside a drawing.
- Toolbar visual tokens beyond what the popup already pins (the toolbar will reuse the popup's container/button styling; specific buttons and shortcuts are a stacked PR).
- Bulk re-keying of canvases saved with literal hex during the pre-split window — they render verbatim and self-heal on the next pick.

## Icons

The Figma `lucide/<name>` layer names are *origin labels*, not a guarantee the glyph still matches stock lucide-react. Two surfaces are treated differently:

- **Toolbar** glyphs are custom illustrations (gradient fills, drop-shadows, multi-path bodies, accent colors) — every one of them, even nodes named `lucide/*`. They live in `src/renderer/shared/CustomIcons.tsx` as React components rendering from `src/renderer/shared/icons/toolbar/*.svg`. The `<img>` wrapper sidesteps SVG `<defs>` id collisions and keeps Figma-exported colors verbatim.
- **Popup-row** glyphs (`lucide/copy`, `lucide/trash`, `lucide/square`, etc.) are stock lucide shapes converted to filled paths at 14×14. Visual delta vs stock `lucide-react` at popup-button size is negligible, so the implementation uses `lucide-react` directly — no extraction needed.

The pen-popup-specific custom glyphs (`PenSlimIcon`, `PenMarkerIcon`, `StrokeThinIcon`, `StrokeThickIcon`) also live in `CustomIcons.tsx` as inline JSX so the `ink` prop can preview the active pen color.

### Source-of-truth node ids

Figma file: `hgwwoe0EzUrErdviULmRtb` (the **agent-canvas** Figma file).

| Toolbar slot | Figma node | Component (in `CustomIcons.tsx`) | Raw SVG |
|---|---|---|---|
| Select | `362:602` | `SelectToolIcon` | `icons/toolbar/select.svg` |
| Hand (pan) | `362:606` | `HandToolIcon` | `icons/toolbar/hand.svg` |
| Draw (highlight) | `410:16` | `DrawHighlightToolIcon` | inline JSX (`ink` + `isDark`) |
| Draw (pen) | `178:150` | `DrawPenToolIcon` | inline JSX (`ink` + `isDark`) |
| Add sticky | `362:625` | `AddStickyToolIcon` | `icons/toolbar/add-sticky.svg` |
| Add shape | `362:631` | `AddShapeToolIcon` | `icons/toolbar/add-shape.svg` |
| Add page | `362:636` | `AddPageToolIcon` | `icons/toolbar/add-page.svg` |
| Add text | `362:649` | `AddTextToolIcon` | `icons/toolbar/add-text.svg` |
| Comment | `362:653` | `CommentToolIcon` | `icons/toolbar/comment.svg` |
| Inspect | `362:658` | `InspectToolIcon` | `icons/toolbar/inspect.svg` |
| Theme | `362:663` | `ThemeToolIcon` | `icons/toolbar/theme.svg` |
| Zoom chevron | `362:668` | `ZoomChevronIcon` | `icons/toolbar/zoom-chevron.svg` |

The Figma toolbar visual order matches the §5 ordering (`select, hand, draw, add-sticky, add-shape, add-page, add-text, comment, inspect, theme, zoom`).

| Pen-popup custom | Figma node (light / dark) | Component |
|---|---|---|
| Pen-slim brush | `360:12` / `360:67` | `PenSlimIcon` |
| Pen-marker brush | `360:22` / `360:77` | `PenMarkerIcon` |
| Stroke-thin preview | `360:37` / `360:91` | `StrokeThinIcon` |
| Stroke-thick preview | `360:39` / `360:93` | `StrokeThickIcon` |

### Re-extracting from Figma

Toolbar SVGs match each Figma frame's nominal size (20×20, 18×18 for comment/inspect, 12×12 for the zoom chevron). Drop shadows are stripped from the SVGs and reapplied in CSS via `filter: drop-shadow(...)` on the rendered `<img>`. This keeps each glyph on a consistent grid and gives every icon an alpha-correct shadow — fixing the multi-element bounding-box shadow problem (an icon with two paths like `add-page` would otherwise get one rectangular shadow under both).

When pulling fresh exports, clone the icon, set `clipsContent = true` so the SVG bounds match the frame, strip `DROP_SHADOW`/`INNER_SHADOW` from every node in the subtree, then export:

```ts
mcp__plugin_figma_figma__use_figma({
  fileKey: 'hgwwoe0EzUrErdviULmRtb',
  code: `
    function strip(n) {
      if ('effects' in n) n.effects = n.effects.filter(e => e.type !== 'DROP_SHADOW' && e.type !== 'INNER_SHADOW');
      if ('children' in n) for (const c of n.children) strip(c);
    }
    const original = await figma.getNodeByIdAsync('362:614');
    const clone = original.clone();
    clone.x = -10000; clone.y = -10000;
    if ('clipsContent' in clone) clone.clipsContent = true;
    strip(clone);
    figma.currentPage.appendChild(clone);
    const bytes = await clone.exportAsync({ format: 'SVG' });
    clone.remove();
    let s = ''; for (let i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
    return s;
  `,
});
```

The exported SVG can be saved as-is; no JSX conversion or id-rescoping needed because the toolbar icons render via `<img>`. The shared CSS shadow lives in `toolbarSections.tsx` (`TOOLBAR_GLYPH_SHADOW`).

**Dark-mode toolbar.** Currently only light-mode SVGs are committed — the Figma file does not define dark-mode toolbar variants (only the pen popup has `360:66` etc.). Theme handling for the toolbar is deferred to Phase 8: either pull dark-mode equivalents into a parallel `icons/toolbar-dark/` set and theme-switch at render time, or use a CSS filter (e.g. `filter: invert()` with a per-icon tweak) on the existing assets.

## Migration

Vertical slices, each green on `typecheck` + `test:unit` + `test:smoke`. May ship as one PR or as a stack.

1. **Palette resolver.** `canvas-colors.ts` gains the eight-slot definition + a `(slotIndex, role, theme) → rgb` function. Old six-digit mapping stays as a fallback path. New `specular.colorRole` and hex `color` values are read/written by serializer.
2. **Tool union restructure.** `Tool` updates per §4. `add-document` removed, `add-sticky` added, `add-text.style` removed. Toolbar buttons re-bind; IPC channels (`setTool`) accept the new shape; keyboard shortcuts re-target.
3. **Text size.** New `textSize` field on `text` and `shape` entities (and tool defaults). Renderer reads it. Old entities default to `Small` (14) if absent.
4. **Visual lock.** Container tokens, button styles, swatch primitives updated in `CanvasItemPopup` to match §8. Tailwind variables refreshed.
5. **Cross-kind morph.** New IPC channel `morph-text-file` invokes file-create or file-delete + entity-replace transactionally. Selection-mode popup wires the toggle.
6. **Text-size dropdown.** New popover component for the labeled dropdown (✓-prefixed presets + raw input). Wires to `textSize` on entity / tool defaults.
7. **Element-name composer.** `Annotation.elementName` field added. Pending composer rewritten to surface the name input + comment thread.
8. **Page popup additions.** Device-frame toggle and rotate-viewport actions wired in `PagePopup` and (for wireframe renderer) `FilePopup`.
9. **Toolbar regrouping.** Stacked PR if needed — re-orders buttons, removes `add-document` button, adds the new `add-sticky` button.

## Tests

- **Unit:** Palette resolver returns expected RGB for every (slot, role, theme) tuple. Cross-kind morph helpers preserve text content. `Tool` union exhaustiveness check still compiles.
- **Unit:** Text-size dropdown commits raw input only on Enter/blur; out-of-range values are clamped silently.
- **Unit:** Element-name field round-trips through Y.Doc and `.canvas` serialization.
- **Smoke:** Create a sticky → pick neutral → toggle theme → sticky color follows the theme.
- **Smoke:** Create plain text → click `long` in popup → assertion that a new `.md` file exists in the workspace and the canvas now shows a `file` entity at the same rect. Undo reverses both halves.
- **Smoke:** Pick text size from the popup → re-open canvas → size persists. Pick raw 64px → renders at 64px.
- **Smoke:** Comment on a page element → composer surfaces "Element name" field → name appears in right-panel list.
- **Smoke:** Page popup device-frame toggle and rotate-viewport produce the same visual result as the right-panel equivalents.
- **Manual:** Eight-slot palette renders on light and dark canvas; neutral reads as theme-appropriate in both modes for sticky (recedes) and pen (contrasts).

## Deferred

- **Shape border width control.** The shape popup originally exposed two stroke-width swatches (thin / thick) alongside fill color and size. The current Figma lock for shapes omits the border-width row entirely, so the swatches are commented out in `ShapePopup.tsx` and `ShapeToolPopup.tsx`. The underlying `strokeWidth` property and `STROKE_WIDTH_PRESETS` helpers remain — when border width returns to the visual design (either as inline swatches, a numeric input, or a "Style" sub-popup), uncomment the section and revisit placement relative to `Size` and color.
