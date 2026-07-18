# ADR 0005 — Unified `Tool` concept

**Status:** Accepted — refined by [ADR 0009](./0009-tool-variants-in-popup-state.md) (variants `shapeKind` and the implicit brush state move out of the union into popup-managed tool defaults). **Note:** `add-text` style was subsequently removed from the union by [ADR 0013](./0013-popup-menus-v2.md); `add-sticky` and `hand` were added. See `src/shared/tool.ts` for the current union.
**Implementation:** Landed. `src/shared/tool.ts` defines the `Tool` union and `toolDuration` table; `UiState.activeTool` is the single source of truth; the toolbar/canvas-bg/right-details preload bridges expose one `setTool(tool)` method (IPC channel `toolbar-set-tool`); `tool-changed` is layered onto the existing `toolbar-selection-changed` and `layout-update` broadcasts via `ToolbarSelectionData.activeTool` and `LayoutUpdateData.activeTool`. The three legacy state machines (`pendingPlacement`, `AnnotationMode`, the `inspect` boolean) and their IPC families are gone. `add-page` carries `presetIndex` / `customSize` / `sourceFrameId` on the variant — a small departure from the ADR's illustrative shape, kept so the existing preset-picker UX survives without re-introducing a parallel state machine. Unit tests cover one-shot auto-revert, persistent stay-active, Escape-to-select, and the gerund mapping for every kind.
**Date:** 2026-05-08
**Supersedes premise of:** the three parallel state machines for `pendingPlacement`, `AnnotationMode` (`'off' | 'comment' | 'draw' | 'region_select'`), and the standalone `inspect` boolean. Also retires the term "annotation mode" as a name for runtime state.
**Related:** [ADR 0004 — Text affordances](./0004-text-affordances-and-spec-extensions.md). The tool list reflects the post-0004 toolbar (`add-text` carries a `style` field for plain vs sticky; `add-document` replaces `add-note`). [ADR 0008](./0008-unified-canvas-item-popup.md) introduces the unified popup that consumes the tool defaults referenced in ADR 0009.

## Context

The codebase had three parallel state machines that all answered the same user-level question — *"what does my next click/gesture do?"* — but used different names, different storage, and different IPC channels:

1. **Placement tools** — runtime variable `pendingPlacement: { entityKind, ... } | null`. One-shot: click toolbar button, place once on canvas, auto-revert.
2. **Annotation modes** — `AnnotationMode = 'off' | 'comment' | 'draw' | 'region_select'`. Persistent: toggle on, do gestures, toggle off.
3. **Inspect** — separate boolean. Persistent.

Plus the implicit `select` default, which had no representation in state at all.

The same user concept ("I'm currently in the comment tool") had three different expressions in code. Effects:

- **Toolbar wiring is duplicated**: each tool family has its own IPC handlers, its own click-to-toggle pattern, its own cursor-label logic.
- **Mutual exclusion isn't enforced**: in principle you could be in placement-mode + annotation-mode + inspect simultaneously. Today only convention prevents it.
- **Keyboard shortcuts are absent**: there's no single state to bind keys against. Adding `T` for text or `P` for page would touch three subsystems.
- **Onboarding burden**: new users (and new contributors) have to learn three vocabularies for one concept.

This is the kind of accidental complexity that compounds — every new tool adds another branch in another state machine.

## Decision

A **Tool** is the single representation of *"what does my next click/gesture do?"* There is exactly one active tool at any moment.

```ts
type Tool =
  | { kind: 'select' }                                                       // default
  | { kind: 'add-page' }                                                     // one-shot
  | { kind: 'add-text'; style: 'plain' | 'sticky' }                          // one-shot
  | { kind: 'add-document' }                                                 // one-shot
  | { kind: 'add-shape'; shapeKind: 'rectangle' | 'ellipse' | 'diamond' }    // one-shot
  | { kind: 'comment' }                                                      // persistent
  | { kind: 'draw' }                                                         // persistent — creates drawing entities
  | { kind: 'region-select' }                                                // persistent
  | { kind: 'inspect' }                                                      // persistent
```

Stored in main runtime as `activeTool: Tool` (default `{ kind: 'select' }`). A small per-kind lookup table `toolDuration: Record<Tool['kind'], 'one-shot' | 'persistent'>` lets the runtime decide when to auto-revert to `select`.

**One-shot tools** revert to `select` after one placement.
**Persistent tools** stay active until replaced by another tool, toggled off (clicking the same toolbar button while active), or Escape.

**Naming choices made within the decision:**

- **"Tool"** as the umbrella concept — universal industry term (Figma, Sketch, Photoshop, Illustrator). Alternatives considered: *Mode* (overloaded with view mode), *Cursor* (too presentational), *Action* (too verb-y).
- **`add-` prefix for placement tools** — matches the toolbar verbiage ("Add page", "Add text"). Alternatives: *place-* (less natural in the UI), bare nouns (collide with entity-kind names).
- **`select` is a tool**, not the absence of one. Gives the default a real name and makes `activeTool` always-defined, simplifying every consumer.

**Toolbar does NOT visually distinguish one-shot from persistent tools.** Users learn duration by use, matching Figma's convention. The toolbar is a single uniform palette.

**View mode (canvas vs browser) is NOT a tool.** It's structural — *"which surface am I looking at?"* — not transient. Stays in its own state.

## Alternatives considered

**A. Keep three state machines, just align names.** Cosmetic fix, leaves accidental complexity, doesn't enable keyboard shortcuts cleanly. Rejected.

**B. Unify only placement + annotation; keep inspect separate.** Half-measure; inspect behaves identically to annotation tools (persistent, gesture-driven, exit-on-Escape). No reason for it to be its own state. Rejected.

**C. Encode `duration` on each Tool variant rather than a lookup table.** Adds a property to every variant, forcing every consumer to think about durations they don't care about. Lookup table keeps the union narrow. Rejected.

**D. Distinguish one-shot vs persistent in the toolbar (e.g. dotted border on persistent ones).** Considered. Figma doesn't bother and users adapt fast; visual indicators are visual noise without a real payoff. Rejected.

**E. Include view mode in the Tool union.** Conflates two different state dimensions: the active tool can change inside either view mode. Rejected.

## Consequences

**Replaces:**
- `pendingPlacement` runtime variable, `AnnotationMode` enum, the `inspect` boolean → all three collapse into `activeTool: Tool`.
- IPC channels: `toolbar-add-page`, `toolbar-add-text-entity`, `toolbar-add-note`, `toolbar-add-shape`, `toolbar-toggle-annotate-mode`, `toolbar-toggle-draw-mode`, `toolbar-toggle-region-select-mode`, `toolbar-toggle-inspect-mode` → one channel `toolbar-set-tool` carrying a `Tool` payload.
- The term "annotation mode" as a name for runtime state. Annotations themselves remain (comments and drawings are still annotations); the *mode of being in the comment tool* is just a tool.

**Enables:**
- Single source of truth for "what's the active tool" — derive cursor labels, toolbar highlight, hit-test branching, keyboard shortcuts from one state.
- Keyboard shortcuts trivial to add (`V` → select, `T` → add-text, `P` → add-page, `S` → add-shape, `C` → comment, `D` → draw, `I` → inspect — exact bindings TBD).
- New tools added by extending the union and the duration table, not by inventing a new state machine.
- Glossary gets one entry; ADRs reference one concept.

**Costs:**
- Real refactor: three state machines collapse into one. Touches main runtime, IPC, toolbar, cursor-label rendering, and any code that branched on `pendingPlacement` / `AnnotationMode` / `inspect`.
- Existing tests that reference annotation-mode terminology need renaming (mechanical).
- `frame-focus` and `editing-text` carve-outs in `gate-predicate.test.ts` need to be re-expressed in terms of `activeTool` where they intersected with `AnnotationMode`.

**Out of scope:**
- Keyboard shortcut bindings — enabled by this ADR but the specific keys are a follow-up.
- Right-click context-menu integration — separate concern.
- Tool palette UI redesign — toolbar layout is unchanged; only the underlying state collapses.
- Multi-tool / "modifier" tools (e.g. hold Space to temporarily activate pan) — orthogonal; modifier state is layered over `activeTool`, not part of the union.

## Migration

1. Define the `Tool` discriminated union and `toolDuration` table in `src/shared/tool.ts`.
2. Add `activeTool: Tool` to main runtime; default `{ kind: 'select' }`. Remove `pendingPlacement`, `AnnotationMode`, `inspect` boolean.
3. Replace toolbar IPC with one channel: `toolbar-set-tool` (renderer → main) and `tool-changed` (main → renderers, layered on the existing layout broadcast).
4. Toolbar buttons fire `setTool({ kind: ... })` and read `activeTool` to highlight the active button. Add the `Add text ▾` dropdown items per ADR 0004.
5. Cursor-label and status-bar narration derive from `activeTool` via the gerund mapping (`select` → "selecting", `add-page` → "adding page", etc.).
6. Hit-test / pointer router branches that depended on annotation-mode flags re-route through `activeTool`.
7. Tests: smoke tests that toggled `AnnotationMode` now call `setTool`. Unit tests cover the duration auto-revert and Escape behavior.

## Tests

- Unit: `setTool` auto-reverts a one-shot tool to `select` after a placement event; persistent tools do not auto-revert; Escape returns to `select` from any tool; `select` ↔ `select` is a no-op.
- Unit: cursor-label gerund mapping for every tool kind.
- Smoke: clicking each toolbar button sets `activeTool` correctly; the active button is highlighted; placing a one-shot tool entity returns to `select`; toggling a persistent tool a second time returns to `select`.
- Smoke: keyboard `Escape` exits any active tool to `select`.
