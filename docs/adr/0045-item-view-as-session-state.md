# ADR 0045: An item view is session state, not a mode

**Status:** Proposed. Built this way in `native/crates/specular-interact/src/showing.rs`. The user has not reviewed this ADR.
**Date:** 2026-10-09
**Related:** [ADR 0020](./0020-delete-browser-mode-for-focus-selection.md), which deleted Browser mode. [ADR 0021](./0021-focus-session-as-first-class-concept.md), the focus session. [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the models the tab row is one of.

## Context

The Rust app's chrome has a tab row. Its first tab is the canvas, and each page and Document of the active canvas has a tab that shows it alone. That looks like the Browser mode ADR 0020 removed. Browser mode cost what it did because layout, input gates, toolbar controls, page visibility and persistence each asked which mode was on, and because a page was resized to fill the window.

## Decision

Which view is showing is one value in the session, `Showing::Canvas` or `Showing::Item(EntityId)`, set by `Action::Show`. It is never saved and never an undo step.

- A page keeps its stored size. The camera is fitted to the item in the free part of the viewport and capped at 100%. No page host changes its viewport and nothing is written to the document.
- The camera is derived after every event, so a window resize, the sidebar and a change of the item's size all refit it. A pan or a zoom does nothing in an item view.
- The canvas's own camera is kept beside the view state. It is the camera a save writes, and the canvas gets it back when the Canvas tab is pressed.
- Hiding is one predicate, `showing::hides`, read where a page's scroll already hides what has left it: `seen` and `shown_rect` in `scroll_follow.rs`. Drawing, hit-testing, outlines, the marquee, guides and edges all read those. Comments have their own gate, `comment::shown`, which asks `showing::hides_comment`.
- What an item view shows is the item and what is hooked to it by a page anchor, with the comments on that page.
- A page shown is entered, so the wheel and the keys are its own at once. Escape leaves it, as on the canvas, and any click on its body goes back in. The Canvas tab leaves it.
- Nothing is made that the view would hide. A press with a creation tool does nothing off the page shown, and the page and Document tools do nothing in any item view. A paste lands on the page shown when all of it can be hooked there, and otherwise does nothing. A file drop does nothing.
- While a page is shown alone it is the only page there is to hook to, and what lands on no page is hooked to it. A stroke that runs out of the page, or a sticky dragged past its edge, stays in view.
- A Document shown is laid out as a reading column: 720 px wide at 100%, or as wide as the window leaves, and as tall as the fit leaves room for. `showing::reading_rect` is that rect. Its stored rect is not written, and a page is never given one.
- A new tab is a page made in a free spot of the canvas and shown. Making the page is the undo step. Undoing it removes the item, so the fallback below puts the canvas back.
- The selection is kept among what is seen, so Select all and Delete cannot reach a hidden item.
- The view falls back to the canvas when its item is deleted or undone away, on a canvas switch and when a space opens. A sidebar row for something hidden goes back to the canvas before it reveals it.

Tabs keep the order they were first listed in, per canvas. The document's only order is the stack, which a bring to front changes. The kept order is session state too, so it starts over from the stack at launch.

## Every reader of the view state

`showing.rs` owns the field. Outside it, these call in. Each is one call to a predicate that `showing.rs` defines.

Settling and switching:

- `update` calls `showing::settle` once after every event. `run_action` hands `Action::Show`, `ShowNext`, `ShowPrevious` and `NewPageTab` to `show`, `step` and `new_tab`.
- `space::ops` calls `leave` on a canvas switch and a space open.
- `reveal::items` and `reveal::comment` call `hides` and `hides_comment` to leave an item view.

What is seen, and where:

- `scroll_follow::seen` and `shown_rect` call `hides`. With `placed_rect` they also call `reading_rect`, which is how a Document's column reaches drawing, hit-testing, the outline and its text layout.
- `handles::handle_target` calls `reading_rect`, so the column has no resize handles.
- `comment::shown` calls `hides_comment`.

Input:

- `select::press` calls `shows`: a click enters the page shown or edits the Document shown, and neither is dragged.
- `notes::on_wheel` calls `shows`, so the Document shown scrolls without being selected.
- `pointer::tool_takes_press` calls `refuses`.

What can be made:

- `Scrolls::of` reads `shown_item`. Anchoring reads pages through `Scrolls`, so `anchor::page_anchor_for` sees only the page shown.
- `asset::insert_selected`, `drop::on_drop` and the image paste call `hides`. `clipboard::paste_items` calls `hides_new`.
- `comment::draft::open` calls `hides_comment`.

Models and saving:

- `view_strip` reads which tab is active.
- `canvas_to_save`, `GET /canvas` and the headless `save` read `canvas_camera`.

`POST /camera/focus` sends `Action::Show` for one page or Document. It reads nothing.

A new reader outside this list is the drift ADR 0020 describes. Add it here or find another way.

## Alternatives

**Resize the page to fill the window.** The browser's own behaviour, and the user turned it down. It is what brought per-mode layout and persistence into the Electron app.

**Filter the document before `view` and hit-testing see it.** One call site instead of two predicates, but every reader of `app.document` outside the scene would need the same filter, and there are dozens.

**Keep the item view per canvas.** `CanvasView` could hold it. A switch closes and recreates page hosts anyway, and resetting is the smaller rule, so a switch goes back to the canvas. Only the tab order is kept per canvas.

**Write the reading column into the Document's rect.** Then the canvas would show a 720 px card after one visit to its tab, and leaving the tab would have to put the old rect back. A rect returned by `seen`, `shown_rect` and `placed_rect` costs three calls in one file, where a scroll shift is already returned.

**Refuse a move that ends off the page shown.** The sticky would snap back. Hooking it to the page keeps what the user did.

**Sort tabs by id or by the stack.** Ids are random hex, so a new tab would land anywhere. The stack reorders on a bring to front.

## Consequences

- The sidebar still lists every item in an item view. It reads the document, not `seen`.
- Delete and the other canvas keys go to the page shown until Escape leaves it (ADR 0022). After Escape they act on the shown item as they do on the canvas: Delete removes it and its tab, and an arrow key moves it on the canvas, which the fitted camera hides.
- Something hooked to the page shown but lying wholly beside it is carried by the page's scroll like anything else, and what the scroll carries is only seen through the page. It disappears while the page is scrolled and comes back at the scroll it was placed at.
- Text pasted on the canvas over a page is still not hooked to it. In an item view it is, because there it could not be seen otherwise.
- In the built-in chrome, Enter in a new tab's address leaves the page not entered, and one click enters it. In the Kit shell the page stays entered while its address is typed.
- `POST /camera/focus` on a group, on several entities or on a rect still sets the camera. While an item view is showing, `settle` puts the fitted camera back, so that call does nothing visible.
- Undoing a new tab goes to the canvas, also when another tab was showing before it.
- Command+Option+Right and Left are not bound while text is edited, where they stay the editor's line start and line end.
- Past about 33 tabs in a 1280 px window the tabs are at their least width and the rest are not drawn. There is no overflow menu.
