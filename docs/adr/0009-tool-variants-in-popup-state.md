# ADR 0009 — Tool variants live in popup state, not in the `Tool` union

**Status:** Accepted (landed alongside ADR 0008 step 6)
**Date:** 2026-05-10
**Refines:** [ADR 0005 — Unified `Tool` concept](./0005-unified-tool-concept.md). Tool variants for `add-shape` and `draw` move out of the discriminated union into tool-mode popup state (per [ADR 0008](./0008-unified-canvas-item-popup.md)). `add-text` is a deliberate exception.
**Companion to:** [ADR 0008 — Unified canvas-item popup](./0008-unified-canvas-item-popup.md).

## Context

ADR 0005 unified three parallel state machines into one `activeTool: Tool` discriminated union. Two of its variants encoded a *sub-kind* directly in the union:

```ts
{ kind: 'add-shape'; shapeKind: 'rectangle' | 'ellipse' | 'diamond' }
{ kind: 'add-text'; style: 'plain' | 'sticky' }
```

`draw` did not encode brush type in the union, but the toolbar effectively split it via two separate UI affordances (pen and highlight buttons) that mapped to brushType-baked-into-stroke at gesture time.

ADR 0008 introduces a tool-mode popup that surfaces tool defaults (color, thickness, etc.) below the toolbar. Once that popup exists, encoding the variant in the `Tool` union becomes redundant for shape and draw:

- The variant is just another configurable default the popup naturally surfaces, alongside color and thickness.
- The toolbar carries one button per family (`add-shape`, `draw`); the popup picks the variant.
- The same popup pattern in selection mode lets users *swap* an existing entity's variant (rect → ellipse, pen → highlight) — matching the rest of the popup's "configure this kind" purpose.

Keeping the variant in the union forces:

- Three toolbar buttons for shapes (one per `shapeKind`). Or a hidden state where the user can't see what's selected.
- A separate UI mechanism for editing the variant of an existing shape entity — not the popup.
- Asymmetry with `draw`, where brushType already isn't in the union but is just as much a "what variant am I creating" choice.

## Decision

`shapeKind` and `brushType` move out of the `Tool` union and into **tool defaults** (per ADR 0008). The `Tool` union becomes:

```ts
type Tool =
  | { kind: 'select' }       // default
  | { kind: 'hand' }         // persistent
  | { kind: 'add-page' }     // one-shot
  | { kind: 'add-text', style: 'plain' | 'sticky' } // one-shot — note: style removed from union by ADR 0013 §4
  | { kind: 'add-document' } // one-shot
  | { kind: 'add-shape' }    // one-shot
  | { kind: 'comment' }      // persistent
  | { kind: 'draw' }         // persistent
  | { kind: 'inspect' }      // persistent
```

> **Note:** `region-select` shown in earlier drafts was removed by [ADR 0006](./0006-unified-comment-tool.md) before this ADR landed. `add-text.style` was subsequently removed and `add-sticky` added as a first-class tool by [ADR 0013](./0013-popup-menus-v2.md) §4. See `src/shared/tool.ts` for the current union.

- **`add-shape`** carries no `shapeKind`. Defaults are read from `tool-defaults.add-shape.{shapeKind, color, strokeWidth}`. The tool-mode popup shows all three variants as buttons; clicking one updates the default and is persisted.
- **`draw`** unchanged at the union level (no field added). `tool-defaults.draw.{brushType, color, strokeWidth}` replaces the implicit `activeDrawBrush` lookup. The tool-mode popup shows pen + highlight as buttons.
- **`add-text` is the exception.** `style: 'plain' | 'sticky'` stays in the union. The plain/sticky split has been a deliberate two-affordance design since ADR 0004 — they have meaningfully different visual presentation and toolbar exposure (the "Add text ▾" dropdown). A future ADR may revisit, but not as part of this work.

### Toolbar consequences

- Three shape buttons (rect, ellipse, diamond) collapse to one shape button. The popup carries the variant choice. Users who want a specific shape pick it from the popup once; subsequent activations remember it.
- One draw button stays one draw button. The popup carries pen vs highlight. Today's hidden brush switch becomes visible.

### Selection-mode consequences

- Selecting a shape entity → popup includes the `shapeKind` picker. Clicking another variant *morphs* the existing entity's `shapeKind`. (`CanvasSceneShapeEntity.shapeKind` field already exists.)
- Selecting a drawing entity → popup includes the `brushType` picker. Clicking another variant rewrites the inner stroke's `brushType`. (`AnnotationDrawingStroke.brushType` field already exists.)

## Alternatives considered

**A. Keep variants in the union; the popup reads the union.** The popup surfaces the variant from the active Tool's payload. Doesn't reduce state; doesn't enable post-creation variant swap; toolbar still needs three shape buttons. Rejected — the popup is a strictly better home for variant choice.

**B. Move all variants out of the union including `add-text` style.** Cleanest pattern but loses `add-text`'s established two-affordance toolbar UX (the "Add text ▾" dropdown is real and the plain/sticky distinction is more semantically loaded than rect-vs-ellipse). Deferred — could be revisited later if the asymmetry hurts.

**C. Move only `shapeKind` out, leave `draw` brushType where it is (in stroke creation only, not in the union or in defaults).** Half-measure. `draw` brushType already isn't in the union, but it's also not user-facing — there's no way to set "I want my next drawings to be highlighter". Rejected — same UX gap as before.

**D. Encode the popup as part of the Tool union (e.g. `Tool` carries the popup contributions).** Bakes view concerns into the runtime tool state. The popup is a renderer concern, the tool is a runtime/IPC concept. Rejected — wrong layer.

## Consequences

**Replaces:**
- `Tool` payload field `shapeKind` on `add-shape` variants.
- `activeDrawBrush(tool)` helper, which read brush from a separate place — now reads from tool defaults.
- Three-shape-button toolbar layout.

**Enables:**
- Selection-popup variant swap for shapes and drawings (post-creation).
- Tool-defaults persistence for shape variant and brush type, parallel to color/thickness.
- Toolbar simplification (one shape button, one draw button).

**Costs:**
- IPC payloads referencing `shapeKind` on the Tool need migration. The `setTool` channel either drops the `shapeKind` field on `add-shape` payloads (clean) or accepts it for one release as a transition (not recommended — the migration is small enough to do cleanly).
- Toolbar UI changes: three shape buttons → one shape button. Existing toolbar tests/screenshots need updating.
- Code that branched on `tool.shapeKind` now reads from tool defaults via a getter. Mostly mechanical.
- Existing canvases are unaffected — `CanvasSceneShapeEntity.shapeKind` is per-entity and orthogonal to the tool's variant.

**Out of scope:**
- `add-text` style migration.
- `add-page` `presetIndex` (which today rides on the tool variant in a way ADR 0005 noted as a "small departure"). The preset picker has its own UX; folding it into the tool popup is a separate question.
- Keyboard shortcuts to jump to a specific shape (R/E/D, etc.). If desired, they activate `add-shape` and write the chosen variant to defaults; bindings are a follow-up.

## Migration

1. Drop `shapeKind` from `Tool` union variants in `src/shared/tool.ts`. Update consumers.
2. Add tool-defaults storage (per ADR 0008 §"Tool defaults") with keys for `add-shape.shapeKind`, `add-shape.color`, `add-shape.strokeWidth`, `draw.brushType`, `draw.color`, `draw.strokeWidth`.
3. Toolbar collapses three shape buttons to one. Existing keyboard shortcuts (if any) re-target.
4. `add-shape` tool popup wires the three shape buttons to tool defaults. `add-shape` placement reads `tool-defaults.add-shape.shapeKind` instead of `tool.shapeKind`.
5. `draw` gesture reads `tool-defaults.draw.brushType` instead of `activeDrawBrush(tool)`. `activeDrawBrush` is deleted.
6. Selection popup for shape entities exposes the variant picker; clicking another shape morphs the existing entity.
7. Selection popup for drawing entities exposes the brush picker; clicking another brush rewrites the stroke.

## Tests

- **Unit:** `Tool` union no longer carries `shapeKind`; `setTool({ kind: 'add-shape' })` is a complete tool spec. Tool-defaults read/write isolated per tool.
- **Unit:** Selecting a shape and writing a new `shapeKind` via the popup updates the entity's `shapeKind` field.
- **Unit:** Selecting a drawing and writing a new `brushType` via the popup rewrites the inner stroke's `brushType`.
- **Smoke:** One shape button on the toolbar; popup picks the variant; placing creates an entity with that variant. Repeating the gesture without changing the popup variant uses the persisted default.
- **Smoke:** Drawing with pen, switching to highlight in the popup, drawing again — second drawing has highlighter brush.
- **Smoke:** Existing canvases with shape entities continue to render and behave correctly (no migration needed for `.canvas` data).
