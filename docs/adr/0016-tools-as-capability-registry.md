# ADR 0016 — Tools as a capability registry

**Status:** Proposed
**Date:** 2026-05-31
**Extends:** [ADR 0005 — Unified `Tool` concept](./0005-unified-tool-concept.md). ADR 0005 unified tool *identity*; this ADR proposes unifying the remaining per-tool axes (enablement, target, palette, cursor, popup, bindings, behavior) behind one registry.
**Related:** [ADR 0006 — Unified comment tool](./0006-unified-comment-tool.md), [ADR 0008 — Unified canvas-item popup](./0008-unified-canvas-item-popup.md), [ADR 0009 — Tool variants in popup state](./0009-tool-variants-in-popup-state.md), [ADR 0010 — Main as sole shortcut dispatch site](./0010-main-as-sole-shortcut-dispatch-site.md). Architectural precedent: the entity-renderer registry (`src/main/plugins/registry.ts`, `src/main/plugins/CLAUDE.md`).
**Origin:** Surfaced while reviewing the three-surface tool-availability bug fixed across #181 / #184 / #185. Full design spec and discussion in issue #186.

## Context

ADR 0005 unified *"what does my next click do?"* into one `Tool` union with a single owner (`UiState.activeTool`). That settled tool **identity**. But identity is only one of the axes a tool has, and the others were never unified. Today a single tool's behavior is smeared across roughly six files and three IPC payloads:

| Axis | Where it lives today |
|---|---|
| Identity | `src/shared/tool.ts` — the `Tool` union (unified ✓) |
| Duration (one-shot/persistent) | `tool.ts` — `toolDuration` table |
| Cursor / status gerund | `tool.ts` — `toolGerund()` switch |
| Popup ownership | `tool.ts` — `toolHasPopup()` + `src/renderer/above-view/*ToolPopup.tsx` |
| Parameters / defaults | `src/shared/tool-defaults.ts` (separate registry ✓, ADR 0009) |
| Keybindings | `src/shared/bindings.ts` — `BindingId` arms (`tool-comment`, `tool-draw-pen`, …) (separate registry ✓, ADR 0010) |
| **Enablement** | `tool-mode.ts` `sanitizeForFeatureFlags` + **9 `DRAWING_FEATURE_ENABLED` reads across 7 files** |
| **Target state** | `inspect.available = pages.length > 0`; the (now-removed) `annotateAvailable` |
| Palette placement | hardcoded JSX in `src/renderer/toolbar/toolbarSections.tsx` |

Add the `isAnnotationTool` / `isPlacementTool` / `toolAnnotateOverlay` predicates and a tool's behavior touches more files still. None of this is enforced by the type system: adding a tool means editing the union, the duration table, the gerund switch, the popup predicate, the binding union, and the toolbar JSX, then remembering to gate it consistently on every surface.

**The cost is observable.** *"Can you use this tool right now?"* is computed independently on the toolbar, in main's `setActiveTool`, and in the right panel. The last three PRs each fixed **one surface's copy** of the same decision, one at a time:

- **#181** removed the `hasPages` `disabled` on the toolbar buttons.
- **#184** removed the `sanitizeForPages` collapse in main's `setActiveTool`.
- **#185** removed the `annotateAvailable: pages.length > 0` gate on the right panel's "Add comments" button.

Three PRs, one decision, three places — the canonical signature of duplicated derived state, where the bug is fixed in N places and the (N+1)th surface is still wrong.

By contrast, this codebase already solved the equivalent problem for **entity renderers**: a renderer is one claim file under `plugins/builtin/` + one React file + one `RendererSwitch` case (`src/main/plugins/CLAUDE.md`). Each renderer declares `claims`, `priority`, `editable`, `rendererTag`, `popupContributionTags`; the dispatch site asks the registry and every surface is a projection. Tools are the one major interactive concept that never received that treatment.

A second, subtler problem hides inside "availability": it conflates two different questions.

- **Activation** — *can you switch to this tool at all?* After #181/#184/#185 the only surviving gate is `DRAWING_FEATURE_ENABLED` (hard-coded `true`). This axis is collapsing toward "always".
- **Target state** — *with the tool active, is there something to act on?* `inspect.available = pages.length > 0` is real and stays: you can activate inspect with no pages, but there is nothing to inspect.

The #185 fix was subtle precisely because comment's gate was an *activation* gate (wrong — comment is always on) while inspect's is a *target* gate (right — no page means nothing to inspect). Same-looking code, opposite correctness, decided by scattered booleans with no single declaration.

## Decision

Model **tools as a first-class capability registry**, mirroring the entity-renderer registry. A tool becomes one self-describing `ToolDefinition`; every UI surface becomes a pure projection of the registry; the `Tool` union remains the persisted/wire form.

### 1. `ToolDefinition` — one module per tool

```ts
// src/shared/tools/contract.ts
export interface ToolDefinition<K extends ToolKind = ToolKind> {
  kind: K

  // Identity & palette  (was: toolbar JSX, toolDuration, toolGerund)
  label: string
  icon: IconRef
  group: 'navigation' | 'create' | 'annotate' | 'inspect'
  order: number
  duration: ToolDuration            // one-shot | persistent
  gerund: string                    // "commenting"

  // Enablement — the SINGLE activation gate
  // Replaces sanitizeForFeatureFlags + the 9 scattered flag reads.
  enabledWhen?: (ctx: ToolContext) => boolean   // default: always enabled

  // Target contract — activation vs "has something to act on", as policy
  target?: {
    requires: 'page' | 'selection' | 'none'
    whenAbsent: 'inert' | 'disabled'
  }

  // Parameters — delegates to the existing tool-defaults registry (ADR 0009)
  defaults?: ToolDefaultsRef

  // Popup — was: toolHasPopup + above-view dispatch (ADR 0008)
  popup?: ToolPopupRef

  // Keybindings — delegates to the existing bindings registry (ADR 0010);
  // variants (draw-pen / shape-rectangle) declared here, written to defaults
  bindings?: ToolBinding[]

  // Behavior — the gesture contract; runtime handlers keep their homes,
  // referenced by kind, not absorbed into the registry
  activate?(ctx: ToolContext): void
  onDeactivate?(ctx: ToolContext): void
}
```

`ToolContext` is evaluated in main and is the *one* place tool gating reads its inputs: `{ pages, selection, viewMode, flags, … }`.

### 2. A registry mirroring `plugins/registry.ts`

```ts
registerTool(def: ToolDefinition): void   // dup-id guard, like registerEntityRenderer
getTool(kind: ToolKind): ToolDefinition
listTools(): readonly ToolDefinition[]
```

Built-in tools live in `src/main/tools/builtin/` (`select.ts`, `comment.ts`, `draw.ts`, `inspect.ts`, `add-page.ts`, …) and register at boot via `registerBuiltInTools()`, exactly like `registerBuiltInPlugins()`.

### 3. One projection consumed by every surface

Main computes a single `ToolView[]` and broadcasts it on one channel (`tools-view`), replacing the scattered tool fields on `ToolbarSelectionData` / `DevtoolsPanelData`:

```ts
interface ToolView {
  kind: ToolKind
  label: string; icon: IconRef; group: string; order: number
  active: boolean                          // kind === activeTool.kind
  enabled: boolean                         // enabledWhen(ctx) ?? true
  targetState: 'ready' | 'inert' | 'none'  // from target contract + ctx
}
```

Every surface becomes a pure map over `ToolView[]` with **zero** local availability logic: the toolbar renders the palette by `group`/`order` with `disabled={!view.enabled}`; the right panel's comment/inspect toggles read `enabled` + `targetState`; cursor/status read `gerund`; keybindings are generated from each tool's `bindings`; popups dispatch from `popup` refs.

### 4. Activation vs target as declared policy

```ts
// inspect.ts
target: { requires: 'page', whenAbsent: 'disabled' }  // nothing to inspect
// comment.ts
target: { requires: 'page', whenAbsent: 'inert' }     // active, nowhere to anchor yet
```

`setActiveTool` enforces `enabledWhen` (single gate) and the `whenAbsent: 'disabled'` policy; the projection sets `targetState`. The three-surface bug becomes **structurally unconstructible** — one declaration, no second place to forget.

## Alternatives considered

**A. Do nothing.** The duplication is benign *today only because* `DRAWING_FEATURE_ENABLED` is hard-coded `true`, so every copy agrees by accident. The moment any gate goes dynamic, the fix-in-N-places pattern returns; meanwhile dead gates and a constant `annotateAvailable` teach the wrong pattern to the next contributor. Rejected as the end-state (but see "incremental path").

**B. Centralize only enablement (a `Record<ToolKind, {enabled, reason}>` broadcast).** This repeats the alternative ADR 0005 explicitly rejected — encoding a property on every variant that most variants don't care about. It also re-conflates activation with target-state, and pays for *one* axis (enablement) that is actively collapsing. Right-sized for an incremental fix, wrong as the architecture. Rejected as the end-state.

**C. Tools-as-registry (this decision).** Amortizes across *all* per-tool axes at once — identity, duration, cursor, popup, params, bindings, palette, enablement, target — several of which are real and growing. It is the same move already made for entity renderers, so the pattern, the layer discipline, and the "how to add one" docs already exist to copy. Accepted.

**D. Replace the `Tool` union with class instances.** Rejected — persistence (`.canvas`), undo/Y.Doc, and the agent/CLI story all require tool *state* to be flat, diffable JSON. The registry wraps behavior *beside* the persisted value, exactly as the entity-renderer registry sits beside the persisted file entity.

## Consequences

**Collapses / removes:**
- `tool.ts`: `toolDuration`, `toolGerund`, `toolHasPopup`, `isAnnotationTool`, `isPlacementTool`, `toolAnnotateOverlay` → fields/refs on `ToolDefinition`.
- `tool-mode.ts`: `sanitizeForFeatureFlags` → per-tool `enabledWhen`; `setActiveTool` becomes the sole enforcement point.
- The **9 `DRAWING_FEATURE_ENABLED` reads across 7 files** → one read inside `draw`'s `enabledWhen`. (Flags that gate *non-tool* concerns — entity rendering, drawing-entity-state, keybinding registration — stay where they are; see "Non-goals".)
- `inspect-session.ts`: `annotateAvailable` and the dead `setInspectMode` → deleted; `inspect.available` → the `target` contract.
- `bindings.ts`: per-tool `BindingId` arms → generated from registry `bindings`.
- `toolbarSections.tsx`: hardcoded tool list + `hasPages`/`drawingEnabled` props → a `ToolView[]` map.
- Tool fields on `ToolbarSelectionData` / `DevtoolsPanelData` → one `ToolView[]` payload.

**Invariants preserved:**
- `Tool` stays a flat, JSON-diffable tagged union — the persisted/wire form. Registry refs and plugin IDs are never persisted; the tool is recovered from its `kind`.
- Layer rule: renderer surfaces dispatch by broadcast `ToolView[]`, never importing the registry from `src/renderer/` (same discipline as `RendererSwitch`).
- Main remains the sole owner of `activeTool` and the sole shortcut-dispatch site (ADR 0005, ADR 0010).
- Variants stay in tool defaults (ADR 0009).

**Non-goals:**
- Cross-cutting feature flags stay above tools. `DRAWING_FEATURE_ENABLED` also gates entity rendering and keybinding registration; the registry centralizes only the *tool's* read of it.
- Gesture/runtime handlers keep their homes (`register-canvas-drag-ipc`, inspect-session runtime, drawing-entity-state); the registry references behavior by `kind`.
- Target-state is still runtime-evaluated: the registry declares the *contract* (`requires: 'page'`); whether a page exists is live `ToolContext`.
- View mode (canvas/browser) is not a tool (ADR 0005).

**Enables:** plugin- and agent-authored tools (consistent with the agent-CLI / read-write-`.canvas` direction in CLAUDE.md), build-/flag-varied tool sets without surface edits, and per-tool isolation tests (mirror `__resetRegistryForTests`).

## Adoption trigger

This is the end-state, not a mandate to refactor a working 8-tool union today. The pragmatic incremental path — consolidate enablement into one `isToolEnabled(kind)` predicate, delete the dead gates — is the **first field of this registry** and can land independently (it is deliberately registry-shaped). Promote to the full registry when **"add a tool" stops being something only core devs do in TypeScript** — i.e. when a second dynamic activation gate appears, or when tools become an extension point for plugins/agents the way entity renderers and `.canvas` files already are. Until then, the union + one enablement predicate is the right altitude.

When adopted, add `src/main/tools/CLAUDE.md` documenting the contract and the "how to add a tool" steps, mirroring `src/main/plugins/CLAUDE.md`, and update the **Tools** entry in `CONTEXT.md` to link this ADR.
