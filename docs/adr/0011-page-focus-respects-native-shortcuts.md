# ADR 0011 — Page focus respects native shortcuts

**Status:** Accepted — fully landed. Policy P2 is in effect via `firesFromPageFocus` flags in `src/shared/bindings.ts` and `pageFocusActive` gating in `src/main/runtime/binding-dispatcher.ts`. Two opt-in bindings: `escape-page-focus` (scope: page) and `reset-viewport` (scope: all views).
**Date:** 2026-05-12
**Refines:** [ADR 0001 — Click to enter frame focus](./0001-click-to-enter-frame-focus.md). ADR 0001 established the page-focus mode where the focused page receives native pointer input; this ADR extends the same trust to keyboard input.
**Companion to:** [ADR 0010 — Main is the sole shortcut dispatch site](./0010-main-as-sole-shortcut-dispatch-site.md). See [`docs/plans/keyboard-binding-registry.md`](../plans/keyboard-binding-registry.md) for the implementation plan.

## Context

When a page is keyboard-focused (per ADR 0001's `pageFocus` runtime state), main's `before-input-event` listener intercepts the keystroke first via `watchModifierKeys` attached at `page-factory.ts:262`. Today the handler runs *all* registered shortcuts before the page sees the keystroke:

- `Cmd+Z` fires our Yjs undo even if the user is editing a GitHub comment box, a Notion page, or any other web app that has its own undo stack.
- `Cmd+G`, `Cmd+W`, arrow keys, and other shortcuts likewise pre-empt the page.
- Only single-letter tool keys (v, c, d) are excluded via the `handleShortcuts: false` flag.

This is surprising. The whole premise of ADR 0001's page focus is that the user has *chosen* to interact with the page natively — pointer events go to the page, native scrolling works, form inputs accept text. The same trust should extend to keyboard shortcuts. A user editing a Notion page expects Cmd+Z to undo the last character they typed, not to undo a canvas action they made several minutes earlier.

The current behavior is also dangerous because it's invisible: nothing on screen tells the user that their keystroke is going to Specular instead of the page. They press Cmd+Z to fix a typo and silently lose canvas state instead.

Three policies considered:

- **P1 — status quo.** Specular's shortcuts win while page-focused. Page apps lose Cmd+Z, Cmd+F, Cmd+S, etc.
- **P2 — page wins broadly; Specular reserves hand-of-god keys.** Anything the page might bind, the page gets. Specular keeps only escape-page-focus (back to canvas) and reset-viewport (rescue from a weird zoom).
- **P3 — page wins everything.** Even Escape is hijackable by the page. Users lose the keyboard-only path to exit page focus; a hostile or buggy page can trap them.

## Decision

Policy **P2**. When `pageFocus` is non-null:

1. The only bindings that fire are those with `firesFromPageFocus: true`.
2. Exactly two bindings opt in:
   - `escape-page-focus` — Escape returns the user to canvas mode (deselects the page, runs the focus reconciler).
   - `reset-viewport` — Cmd+1 resets zoom/pan. Doesn't mutate the document; safe to interrupt anything.
3. Every other binding (Cmd+Z, Cmd+G, Cmd+W, arrows, single-letter tool keys, etc.) falls through to the page natively — the dispatcher returns `null` and does not call `event.preventDefault()`.

A user who wants Specular's undo while a page is focused presses Escape first, then Cmd+Z.

## Alternatives considered

**A. P1 (status quo).** Reject. Documented above. Steals expected behavior from page apps; surprising; invisible.

**B. P3 (page wins everything).** Reject. Removes the reliable rescue path. A page that captures Escape (or that crashes its renderer in a way that swallows input) can trap the user; they have no keyboard-only exit. P2 keeps Escape sacred for safety. Cmd+1 is the same kind of guarantee for the viewport.

**C. Heuristic detection (page wins only if a known editable element inside the page has focus).** Tempting but unreliable. Detecting "is an editable element focused inside the page" requires polling DOM state across process boundaries; it's racy, slow, and wrong for non-standard editors (Notion, Google Docs, CodeMirror) which don't always use native focusable elements. P2 picks a coarser but predictable rule.

**D. User-configurable policy.** Per-user setting "give Cmd+Z to pages vs Specular." Reject for v1 — adds settings surface for a question that should have a single defensible answer. Revisit if users push back.

## Consequences

**Replaces:**
- The implicit "main always intercepts modifier keys" behavior in `keyboard-shortcuts.ts`.

**Enables:**
- Page web apps work as users expect — Cmd+Z in a comment box undoes the character, Cmd+F opens find-in-page, Cmd+S triggers the app's save handler.
- The page-focus trust model from ADR 0001 extends cleanly to keyboard input. Same intuition: "I clicked in, I'm using this thing natively."

**Costs:**
- Behavior change. Users who learned to press Cmd+Z while page-focused expecting canvas undo will need to learn the new flow (Escape, then Cmd+Z). Changelog entry required.
- The Escape-out path becomes load-bearing in a way it wasn't before. Bugs that break Escape from page focus are now blocking (no other way out).

**Out of scope:**
- A visible "you are in page focus" affordance. ADR 0001's pointer-focus already implies the user knows; if we add a chrome cue for keyboard focus too, that's a separate UX question.
- Allowing pages to opt out (a page that wants Specular to keep handling shortcuts). Hypothetical and unmotivated.

## Tests

- **Smoke:** with a page focused, Cmd+Z does not invoke Specular's undo (the keystroke reaches the page).
- **Smoke:** with a page focused, Escape exits page focus and returns to canvas mode.
- **Smoke:** with a page focused, Cmd+1 resets the viewport.
- **Smoke:** with a page focused on a known editable page (a local fixture page with a textarea), typing Cmd+Z in the textarea undoes the most recent character there, and the canvas undo stack is unchanged.
- **Smoke:** Escape-then-Cmd+Z performs the canvas undo as expected.
