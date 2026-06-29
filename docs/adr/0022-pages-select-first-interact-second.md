# ADR 0022 — Pages adopt select-first / interact-second interactivity

**Status:** Accepted
**Date:** 2026-06-28
**Supersedes premise of:** [ADR 0001](./0001-click-to-enter-frame-focus.md) (the "single-selected = interactive" model the codebase drifted into after 0001's total-focus PoC)
**Implements:** issue #124

## Context

After ADR 0001's total-focus model was replaced by the always-on `aboveView`
interactive layer, page interactivity collapsed onto a single condition:
**single-selected = interactive**. The moment a page became the sole selection
it became the keyboard target *and* had pointer input forwarded into its web
content (`shouldFocusSelectedPage`, `pageSelectionOverlayStates`,
`routePointerDown` → `forward-pointer-down` all keyed off single-selection).

Because a click — or even finishing a drag — single-selects a page, the user
landed in interactive mode with no second deliberate action. The concrete bug:
selecting a content page that focuses a text field (a search box, an editor)
made the page the keyboard target, so `delete-selection` (no `firesWhileTyping`)
was skipped by the dispatcher and **Delete fell through into the page instead of
removing the frame**. There was no "selected but not interactive" state.

## Decision

Pages mirror the sticky / shape / text model: **selection and interactivity are
two states, separated by a deliberate enter gesture.**

A new ephemeral runtime variable `interactivePageId: string | null` (main,
`runtime-context.ts`) names the *entered* page. It is the single gate for all
three facets of interactivity:

- **Keyboard** — `shouldFocusSelectedPage` returns the page only when
  `selection.entityId === interactivePageId`. A merely-selected page leaves
  keyboard on `aboveView`, so canvas shortcuts (Delete, etc.) act on the frame.
- **Pointer forwarding** — `routePointerDown` on a page body is three-way:
  entered page → `forward-pointer-down`; single-selected-but-not-entered →
  `enter-page-interactive`; otherwise → `page-body-press` (select / drag).
- **Content blocker** — `pageSelectionOverlayStates` reports `interactive` only
  for the entered page; a selected page keeps its blocking overlay on.

**Enter:** the second deliberate click on an already-selected page, a
double-click on a page body (`routePointerDoubleClick` → `enter-page-interactive`
as a race-proof path), or **entering a focus session** on the page — focus *is*
the second click, so a focused page is interactive immediately with no extra
click. The click/double-click paths dispatch via `canvas-enter-page-interactive`
→ `enterPageInteractive(pageId)`; focus drives it through `syncInteractiveToFocus`
(`viewport-control.ts`), called right after every `beginFocusSession` /
`endFocusSession` so `interactivePageId` tracks the focused page.

**Exit (back to selected):** Escape (`escape-page-focus` clears
`interactivePageId` before falling through to deselect), clicking away or onto
another entity (selection moves off the entered page → `commitSelection` clears
it), leaving the focus session (dismiss / camera-change / re-focus →
`syncInteractiveToFocus` clears it), and page deletion.

**Delete guard:** `delete-selection` carries `firesFromPageFocus`, so Delete
reaches main even while the focused page owns keyboard. `deleteSelection` drops
the `interactivePageId` from its page targets — the frame is only removed once
the user is back to selected-only.

The entered page is broadcast as `LayoutUpdateData.interactivePageId` so the
renderer router can build its `CanvasPointerContext`.

## Consequences

**Fixes:** Delete (and other canvas shortcuts) reliably act on a selected page,
including content pages with autofocused inputs — the reported regression.

**Behavior change:** interacting with a page's web content now takes a
deliberate enter (second click / double-click); the first click only selects.
Dragging a page by its body leaves it selected-but-not-interactive.

**Ephemeral:** `interactivePageId` is never persisted or undoable (like
selection). Automation-interactive pages (`automationInteractivePageCounts`)
remain an orthogonal path.

**Out of scope:** forwarding the *entering* click into the page (the blocker is
still on at that instant), and a per-page "interactive" affordance in chrome.
