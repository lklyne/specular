# ADR 0010 — Main is the sole shortcut dispatch site

**Status:** Accepted — landed; `src/main/runtime/binding-dispatcher.ts`, `binding-handlers.ts`, and `src/shared/bindings.ts` replace the prior split dispatch. `keyboard-shortcuts.ts`, `useAnnotateToggleShortcut.ts`, and `useAnnotationOverlayShortcuts.ts` were deleted; v/c/d/escape keyboard branches of `useCanvasGlobalShortcuts.ts` were removed.
**Date:** 2026-05-12
**Refines:** [ADR 0005 — Unified `Tool` concept](./0005-unified-tool-concept.md). ADR 0005 §"Decision" identified keyboard shortcuts as the follow-up enabled by unifying `Tool` but left out of that ADR. This is that follow-up.
**Companion to:** [ADR 0011 — Page focus respects native shortcuts](./0011-page-focus-respects-native-shortcuts.md). See [`docs/plans/keyboard-binding-registry.md`](../plans/keyboard-binding-registry.md) for the implementation plan.

## Context

Keyboard shortcut handling today is split across three locations:

1. **Main process** — `src/main/runtime/keyboard-shortcuts.ts` registers a `before-input-event` listener on every WebContentsView. 285 lines of inline `if (input.type === 'keyDown' && input.meta && ...)` blocks.
2. **Renderer process** — three hooks (`useAnnotateToggleShortcut`, `useAnnotationOverlayShortcuts`, the v/c/d branches of `useCanvasGlobalShortcuts`) install `keydown` listeners. They exist to make v/c/d fire when focus is in views where main passes `handleShortcuts: false`.
3. **App menu** — `src/main/runtime/app-menu.ts` declares `accelerator: 'CmdOrCtrl+W'` strings, decoupled from the other two.

The split exists because no single WebContents reliably sees every keystroke. The toolbar, right-details-panel, and pages each have `handleShortcuts: false` in main's listener, and the renderer hooks compensate for some of those gaps. Effects:

- "Where is the `v` shortcut defined?" has three answers depending on which WebContents has focus.
- The shortcut→action mapping is duplicated across files. v/c/d live in `keyboard-shortcuts.ts` *and* in `useAnnotateToggleShortcut.ts`.
- The text-editing suppression predicate is asked in three places: `isTypingTarget(event.target)` inline in each renderer hook, plus `isTextEditingActive()` in main, plus the ref-counted IPC flag that synchronizes them.
- Adding a new shortcut is a 4-file diff and has to consider which views need to fire it.
- No unit tests exist. The interface forces tests to simulate Electron's `Input` events; nobody has paid that cost.
- The late-bound `wireKeyboardShortcuts(...)` setter pattern works around an import cycle, adding a configuration step that's easy to forget.

This is the same accidental-complexity shape ADR 0005 fixed for `Tool`: one user concept, three implementations.

## Decision

Main is the sole dispatch site for keyboard shortcuts. Renderer `keydown` listeners are an anti-pattern for *shortcut handling*.

Concretely:

1. A single binding registry lives in `src/shared/bindings.ts` as pure data: `readonly Binding[]`.
2. A single dispatch function `dispatchKey(table, key, ctx) → BindingId | null` is pure and unit-testable.
3. Main's `before-input-event` is the *only* place that calls `dispatchKey`. Every WebContentsView (overlays, pages, sidebars) has this listener attached via `attachBindingDispatcher(wc, sourceView)`.
4. Each binding declares a `scope: KeyboardSourceView[]` listing which source views can fire it. This replaces the `handleShortcuts: false` per-view flag.
5. Each binding declares a `target: 'main' | KeyboardSourceView`. Most run in main; the few that touch renderer-local React state (`annotation-close-thread`, `annotation-clear-draft`) declare a renderer target. Main dispatches, sends `binding-fire` IPC, and the target renderer runs the handler.
6. The app menu reads from the same table: `accelerator: acceleratorFor('close-tab')`. The menu and dispatcher cannot drift.

DOM `ClipboardEvent` handling (Cmd+C / Cmd+V / Cmd+X for the canvas) stays in a renderer hook, because `event.clipboardData` and `window.getSelection()` exist only renderer-side. Clipboard events are not keystrokes; they're a separate browser event family and stay outside the registry.

## Alternatives considered

**A. Status quo (split dispatch).** Reject. Causes the duplication and drift documented in Context. The Bindings UI mentioned in the recent settings-window work has nothing to bind against; future contributors keep adding renderer hooks.

**B. Renderer-side dispatch as primary, main-side as fallback.** Renderers run dispatch on their own keydown listeners; uncaught keys forward to main. Reject. Distributes the table across processes (it'd live in `src/shared/` but be evaluated independently in each), keeps the typing-target check duplicated, and re-introduces the v/c/d-in-multiple-views problem for any future contributor adding shortcuts in a renderer.

**C. Per-renderer registries.** Each view declares its own bindings. Reject. The Bindings pane (future) can't show a single table; the menu accelerators can't be derived; users debugging a shortcut have to know which view owns it.

**D. Single registry, dispatch in any process.** The table is shared but the dispatcher runs in whichever process has focus. Reject. Re-introduces "where is dispatch happening for this keystroke" as a question, and the keystrokes a renderer sees are a subset of what main sees (some keys arrive at main's `before-input-event` before any renderer keydown listener fires). Main is the only process that sees every keystroke reliably.

**E. Promote all renderer-local state to main so every handler can live in main.** Specifically, promote `pendingAnnotation` and `openThreadId` (the two renderer-state cases) to main runtime variables broadcast via layout. Reject as part of this work — it's a separate refactor and not load-bearing. The `target: 'main' | KeyboardSourceView` field is a clean expression of where each handler lives without forcing the state migration.

## Consequences

**Replaces:**
- `src/main/runtime/keyboard-shortcuts.ts` — replaced by `binding-dispatcher.ts` + `binding-handlers.ts`.
- `useAnnotateToggleShortcut.ts`, `useAnnotationOverlayShortcuts.ts` — deleted.
- v/c/d/escape/delete keyboard branches of `useCanvasGlobalShortcuts.ts` — deleted (clipboard extracted into `useCanvasClipboard.ts`).
- `wireKeyboardShortcuts(...)` late-bound setter — deleted.
- `watchModifierKeys(wc, { handleShortcuts: boolean })` — replaced by `attachBindingDispatcher(wc, sourceView: KeyboardSourceView)`.
- `handleShortcuts: false` per-view flag — replaced by per-binding `scope`.

**Enables:**
- Pure-function test surface: every binding is a one-line unit test.
- Bindings settings pane (future) reads one table.
- Toolbar tooltips (future) read one table.
- New shortcuts are a one-row diff, not a four-file diff.
- Menu accelerators cannot drift from dispatch.
- New tools are forced to have a default key (TypeScript exhaustiveness over `Tool['kind']`).

**Costs:**
- Two renderer-targeted bindings (`annotation-close-thread`, `annotation-clear-draft`) cost one IPC roundtrip per keystroke. Imperceptible in practice; flagged for future profiling.
- Behavior changes — see Migration.

## Migration

The plan in `docs/plans/keyboard-binding-registry.md` steps through this in four PRs: registry-only addition, port main, port renderers, port menu. Each step ends in a working app.

The behavior changes that land alongside:

- Tool keys no longer toggle on second keypress (FigJam reference).
- Duplicate no longer auto-groups (separate but adjacent cleanup).
- `Cmd+A` is now a real canvas binding (`select-all` for entities) outside text editing.
- Page focus no longer captures `Cmd+Z` and friends (see [ADR 0011](./0011-page-focus-respects-native-shortcuts.md)).

Changelog entries required for each.

## Tests

- **Unit:** the binding table is well-formed (every `Tool['kind']` has a default key; no two bindings share `(key, scope, modifiers)`).
- **Unit:** `dispatchKey` returns the right `BindingId` for every binding × source-view × context combination.
- **Unit:** `acceleratorString` round-trips every modifier combination.
- **Smoke:** every shortcut fires the right action end-to-end (canvas-region and page focus).
- **Smoke:** renderer-targeted bindings (annotation overlay) fire correctly via the IPC path.
- **Smoke:** menu accelerators derived from the table show the same key string as the binding's `defaultKey`.
