# ADR 0045: An item view is session state, not a mode

**Status:** Proposed. Built this way in `native/crates/specular-interact/src/showing.rs`. The user has not reviewed this ADR.
**Date:** 2026-10-09
**Related:** [ADR 0020](./0020-delete-browser-mode-for-focus-selection.md), which deleted Browser mode. [ADR 0021](./0021-focus-session-as-first-class-concept.md), the focus session. [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the models the tab row is one of.

## Context

The Rust app's chrome has a tab row. Its first tab is the canvas, and each page and Document of the active canvas has a tab that shows it alone. That looks like the Browser mode ADR 0020 removed. Browser mode cost what it did because layout, input gates, toolbar controls, page visibility and persistence each asked which mode was on, and because a page was resized to fill the window.

## Decision

Which view is showing is one value in the session, `Showing::Canvas` or `Showing::Item(EntityId)`, set by `Action::Show`. It is never saved and never an undo step.

- The item keeps its stored size. The camera is fitted to it in the free part of the viewport and capped at 100%. No page host changes its viewport and nothing is written to the document.
- The camera is derived after every event, so a window resize, the sidebar and a change of the item's size all refit it. A pan or a zoom does nothing in an item view.
- The canvas's own camera is kept beside the view state. It is the camera a save writes, and the canvas gets it back when the Canvas tab is pressed.
- Hiding is one predicate, `showing::hides`, read where a page's scroll already hides what has left it: `seen` and `shown_rect` in `scroll_follow.rs`. Drawing, hit-testing, outlines, the marquee, guides and edges all read those. Comments have their own gate, `comment::shown`, which asks `showing::hides_comment`.
- What an item view shows is the item and what is hooked to it by a page anchor, with the comments on that page.
- The selection is kept among what is seen, so Select all and Delete cannot reach a hidden item.
- The view falls back to the canvas when its item is deleted or undone away, on a canvas switch and when a space opens. A sidebar row for something hidden goes back to the canvas before it reveals it.

Tabs keep the order they were first listed in, per canvas. The document's only order is the stack, which a bring to front changes. The kept order is session state too, so it starts over from the stack at launch.

## Every reader of the view state

`showing.rs` owns the field. Outside it, these call in:

- `update` calls `showing::settle` once after every event.
- `scroll_follow::seen` and `shown_rect` call `hides`. `comment::shown` calls `hides_comment`.
- `reveal::items` and `reveal::comment` call both to leave an item view.
- `space::ops` calls `leave` on a canvas switch and a space open.
- `view_strip` reads which tab is active. `canvas_to_save`, `GET /canvas` and the headless `save` read `canvas_camera`.

A new reader outside this list is the drift ADR 0020 describes. Add it here or find another way.

## Alternatives

**Resize the page to fill the window.** The browser's own behaviour, and the user turned it down. It is what brought per-mode layout and persistence into the Electron app.

**Filter the document before `view` and hit-testing see it.** One call site instead of two predicates, but every reader of `app.document` outside the scene would need the same filter, and there are dozens.

**Keep the item view per canvas.** `CanvasView` could hold it. A switch closes and recreates page hosts anyway, and resetting is the smaller rule, so a switch goes back to the canvas. Only the tab order is kept per canvas.

**Sort tabs by id or by the stack.** Ids are random hex, so a new tab would land anywhere. The stack reorders on a bring to front.

## Consequences

- The sidebar still lists every item in an item view. It reads the document, not `seen`.
- Something created off the page in an item view is hidden as soon as it exists. Creation tools are not gated yet.
- A page in an item view is entered the way it is on the canvas, by a second click. Until then the wheel does nothing over it.
- `Action::FocusSelection` and the dock's Focus button still exist and do nothing in an item view. They are due to be deleted.
- Past about 34 tabs in a 1280 px window the tabs are at their least width and the rest are not drawn. There is no overflow menu.
