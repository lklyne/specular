# ADR 0045: An item view is session state, not a mode

**Status:** Proposed. Built this way in `native/crates/specular-interact/src/showing.rs` and `showing/`. The user has not reviewed this ADR.
**Date:** 2026-10-09
**Related:** [ADR 0020](./0020-delete-browser-mode-for-focus-selection.md), which deleted Browser mode. [ADR 0021](./0021-focus-session-as-first-class-concept.md), the focus session. [ADR 0044](./0044-ui-as-pure-models-with-replaceable-renderers.md), the models the tab row is one of.

## Context

The Rust app's chrome has a tab row. Its first tab is the canvas, and each page and Document of the active canvas has a tab that shows it alone. That looks like the Browser mode ADR 0020 removed. Browser mode cost what it did because layout, input gates, toolbar controls, page visibility and persistence each asked which mode was on, and because a page was resized to fill the window.

## Decision

Which view is showing is one value in the session, `Showing::Canvas` or `Showing::Item(EntityId)`, set by `Action::Show`. It is never saved and never an undo step.

**Amended 2026-10-09 (phase 4a).** Two more values sit beside the item, and everything below is derived from the three in `showing.rs`. They are session state like the item: never saved, never an undo step.

- The **lens** is how a tab looks at its item: `Lens::Fill`, `Device` or `Canvas`, set by `Action::SetLens`. Each tab keeps its own for the session, per canvas, beside the tab order (`showing::Tabs`). A tab starts in Fill, except one opened from the canvas by the toolbar's expand button (`Action::OpenTab`), which starts in Device.
- The **eye** is whether anything but the item is drawn, set by `Action::ShowOthers`. It is one choice for every tab and starts open.

| Lens | The item's rect | Camera | Seen with the eye open |
|---|---|---|---|
| Fill | a presentation rect | fitted to it and held | the item and what follows an element of it |
| Device | the stored rect | fitted to it and held | everything |
| Canvas | the stored rect | free, starts fitted, kept by the tab | everything |

With the eye shut every lens shows the item alone: nothing else is drawn or can be hit, not even what is hooked to it.

**Amended 2026-10-09 (phase 4b).** A page in Fill fills the area under the chrome. Its host's viewport follows the presentation rect. That is the one place this ADR lets the view resize a host, and the bullets below say how.

**Amended 2026-10-09 (prepare on press).** A fourth value, the **prepared page**, is the page whose tab is pressed and not yet clicked (`Action::PrepareShow`, `showing/prepare.rs`). Its host is laid out at the size and scale its tab will give it, during the 80 to 100 ms a click takes, so a frame of that size is held when the click shows the tab. Only the host follows it: `pages::laid_out` and the paint LOD read it, and `view`, the camera and `App::page_placement` do not. The compositor keeps the frame of the canvas's size beside the newer ones and draws the one painted at the size the scene asks for. Showing any tab, a canvas switch and a release off the tab let the page go. It is session state like the rest. An item hooked to an element of the page can shift on the canvas while the press is held, since the page reflows.

- This replaces the first rule, "hide all but what is hooked to the item". In Fill the item is the whole view, so what merely lies near it on the canvas has no place: it would be drawn over a rect it was never placed against. Device and the Canvas lens are for the surroundings.
- In Fill a page is laid out at another width than it is stored at. An item or comment hooked to it by a position or a scroll offset was placed against the stored layout, so it is hidden there whatever the eye says. What follows an element (ADR 0032) is shown: an item whose anchor has an element, a comment on an element, and a region whose binding has one. An item or region still waiting for its page's answer is shown until the answer comes. `gates::follows` and `gates::comment_follows` are the rule.
- A Document in Fill is a reading column: 720 px wide at 100%, or as wide as the window leaves, and as tall as the fit leaves room for. `showing::presented_rect` is that rect. Its stored rect is not written. In Device it is the card the canvas has, fitted, with its handles.
- A page in Fill has a presentation rect too: everything `viewport::area` leaves free, in whole pixels, at the stored rect's corner. The camera sits on it corner to corner at 100%, so one CSS pixel is one screen pixel. `showing::fill_rect` decides it.
- The rect reaches the app through two seams. `App::page_placement` returns it with its viewport, which is what hit-testing, pointer forwarding, `gates::refuses`, the clip in `seen` and comment bounds read. `pages::snapshot` carries the size, so a document step taken in Fill, such as a preset picked in the dock, leaves the host as it is.
- `update` compares `pages::presentation` from before the event with the one after `showing::settle`, in `pages::follow_presentation`. A lens, a tab, the window, the sidebar and the right panel each come out as one `SetPageViewport`, and leaving Fill sends the stored viewport back. Nothing here coalesces them. A live window drag sends one per event, and CEF's own resize pipeline paces them.
- A frame painted before a viewport change took effect is never stretched over the new rect. Each frame carries the CSS size it was painted at (`FrameEvent::viewport`) and `PageDraw` the size its rect is laid out at. When they differ the compositor draws the frame at its own pixel size from the top-left corner, cut at the rect, with the canvas showing in what is left. During a window drag CEF paints about one step in three, so most frames are a few pixels behind the rect.
- A page in Fill is drawn with no device frame. `showing::presented` hands `seen` the page without it. The frame belongs to the stored size.
- The stored rect and preset are never written by Fill, so there is no undo step and the `.canvas` file is unaffected. The dock shows the stored size and preset, and a preset picked there is stored without leaving Fill.
- In Device the fit is to the device frame when the page shows one, so a phone's frame clears the chrome.
- No other page host changes its viewport and nothing is written to the document. In Fill and Device the camera is derived after every event, so a window resize, the sidebar and a change of the item's size all refit it, and a pan or a zoom does nothing. The Device fit is capped at 100%.
- In the Canvas lens the camera pans and zooms as on the Canvas tab. The tab keeps where it was left and comes back to it from another tab or another canvas.
- The canvas's own camera is kept beside the view state. It is the camera a save writes in every lens, and the canvas gets it back when the Canvas tab is pressed.
- Hiding is one predicate, `showing::hides`, read where a page's scroll already hides what has left it: `seen` and `shown_rect` in `scroll_follow.rs`. Drawing, hit-testing, outlines, the marquee, guides and edges all read those. Comments have their own gate, `comment::shown`, which asks `showing::hides_comment`.
- In Fill and Device a page shown is entered, so the wheel and the keys are its own at once. Escape leaves it, as on the canvas, and any click on its body goes back in. In the Canvas lens it is selected and entered by the canvas's own second click. The Canvas tab leaves it.
- Nothing is made that the view would hide. With everything seen (Device and the Canvas lens, eye open) anything is made anywhere, by the canvas's rules. What is made on a page in Fill is asked about its element by the capture every anchored item already gets, and stays seen if the page names one. In Fill a press with a creation tool does nothing off the page shown, the page and Document tools do nothing, a paste lands on the page shown when all of it can be hooked there, and a file drop does nothing. With the eye shut nothing is made at all.
- While a page fills the view it is the only page there is to hook to, and what lands on no page is hooked to it. A stroke that runs out of the page, or a sticky dragged past its edge, stays in view.
- The control is three toggles and the eye at the right end of the tool row, in the toolbar model (`ToolbarModel::view`), named `view.lens.fill`, `view.lens.device`, `view.lens.canvas` and `view.others`. The Canvas tab has no item, so it has no control and the eye does nothing there.
- A new tab is a page made in a free spot of the canvas and shown. Making the page is the undo step. Undoing it removes the item, so the fallback below puts the canvas back.
- The selection is kept among what is seen, so Select all and Delete cannot reach a hidden item.
- The view falls back to the canvas when its item is deleted or undone away, on a canvas switch and when a space opens. A sidebar row for something the view cannot bring into sight goes back to the canvas before it reveals it: something hidden, or anything but the item and what is hooked to it while the camera is held.

Tabs keep the order they were first listed in, per canvas. The document's only order is the stack, which a bring to front changes. The kept order is session state too, so it starts over from the stack at launch.

## Every reader of the view state

`showing.rs` owns the three values, with `showing/tabs.rs` (what each tab keeps) and `showing/gates.rs` (what is hidden and what is refused). The lens and the eye are matched on only there. Outside, these call in. Each is one call to a predicate that `showing.rs` defines.

Settling and switching:

- `update` calls `showing::settle` once after every event, and then `pages::follow_presentation`, which reads `presented_rect` through `pages::presentation`. `run_action` hands `Action::Show`, `ShowNext`, `ShowPrevious`, `NewPageTab`, `SetLens` and `ShowOthers` to `show`, `step`, `new_tab`, `set_lens` and `set_others`.
- `space::ops` calls `leave` on a canvas switch and a space open, and parks `Session::tabs` with the canvas.
- `reveal::items` and `reveal::comment` call `out_of_reach` and `comment_out_of_reach` to leave an item view.

What is seen, and where:

- `scroll_follow::seen` and `shown_rect` call `hides`. With `placed_rect` they also call `presented_rect` (`seen` through `presented`), which is how a Document's column and a page's fill reach drawing, hit-testing, the outline and text layout.
- `App::page_placement` and `pages::snapshot` call `presented_rect`, which is how a page's fill reaches its host and everything that maps canvas points into the page.
- `pages::laid_out` reads `showing::prepared`, and the runtime's paint LOD reads `App::prepared_page`, so a pressed tab's page gets its viewport and its scale in one resize.
- `handles::handle_target` calls `presented_rect`, so the column has no resize handles.
- `comment::shown` calls `hides_comment`.

Input:

- `select::press` calls `holds`: where the lens holds the camera, a click enters the page shown or edits the Document shown, and neither is dragged.
- `notes::on_wheel` calls `holds`, so the Document shown scrolls without being selected.
- `pointer::tool_takes_press` calls `refuses`.

What can be made:

- `Scrolls::of` calls `only_page`. Anchoring reads pages through `Scrolls`, so in Fill `anchor::page_anchor_for` sees only the page shown.
- `asset::insert_selected`, `drop::on_drop` and the image paste call `hides`. `clipboard::paste_items` calls `hides_new`.
- `comment::draft::open` calls `hides_comment`.

Models and saving:

- `view_strip` reads which tab is active, and `toolbar` reads `App::lens` and `App::shows_others` for the control.
- `canvas_to_save`, `GET /canvas` and the headless `save` read `canvas_camera`.

`POST /camera/focus` sends `Action::Show` for one page or Document. It reads nothing.

A new reader outside this list is the drift ADR 0020 describes. Add it here or find another way.

## Alternatives

**Resize the page to fill the window by writing its rect.** It is what brought per-mode layout and persistence into the Electron app. Phase 4b gets the browser's behaviour without it. The host is resized, the document is not, and the size comes from one function.

**Filter the document before `view` and hit-testing see it.** One call site instead of two predicates, but every reader of `app.document` outside the scene would need the same filter, and there are dozens.

**Keep the item view per canvas.** `CanvasView` could hold it. A switch closes and recreates page hosts anyway, and resetting is the smaller rule, so a switch goes back to the canvas. Only the tab order is kept per canvas.

**Write the reading column into the Document's rect.** Then the canvas would show a 720 px card after one visit to its tab, and leaving the tab would have to put the old rect back. A rect returned by `seen`, `shown_rect` and `placed_rect` costs three calls in one file, where a scroll shift is already returned.

**Refuse a move that ends off the page shown.** The sticky would snap back. Hooking it to the page keeps what the user did.

**Three modes, each with its own code.** Fill, Device and Canvas could each be a variant of `Showing` with its own hiding, camera and input. They differ in two things only, the rect and whether the camera is held, so they are one value that three functions match on.

**The eye open shows everything in every lens.** Simpler to say. In Fill the canvas's neighbours would then be drawn over a reading column they were never placed against, in the state every Document tab starts in.

**With the eye shut, make things on the item anyway.** They would be hidden as they were made. Opening the eye when something is made is the other way out, and it changes a choice the user made for every tab.

**Sort tabs by id or by the stack.** Ids are random hex, so a new tab would land anywhere. The stack reorders on a bring to front.

## Consequences

- A comment on an element is drawn at the box it recorded, carried by the scroll. It does not ask its page again, so after the reflow of Fill it can sit beside its element. A region does follow.
- An item made on a page in Fill whose page names no element under it is hidden when the answer comes.
- A screenshot taken through the API while a page fills its tab is drawn from a copy of the app at another viewport, so the page is presented at that size with the pixels of this one.
- The sidebar still lists every item in an item view. It reads the document, not `seen`.
- In Device with the eye open the camera is still held, so what is seen is what lies in the margins around the item. The Canvas lens is for looking further.
- The lens and the eye have no keys yet.
- Delete and the other canvas keys go to the page shown until Escape leaves it (ADR 0022). After Escape they act on the shown item as they do on the canvas: Delete removes it and its tab, and an arrow key moves it on the canvas, which the fitted camera hides.
- Something hooked to the page shown but lying wholly beside it is carried by the page's scroll like anything else, and what the scroll carries is only seen through the page. It disappears while the page is scrolled and comes back at the scroll it was placed at.
- Text pasted on the canvas over a page is still not hooked to it. In an item view it is, because there it could not be seen otherwise.
- In the built-in chrome, Enter in a new tab's address leaves the page not entered, and one click enters it. In the Kit shell the page stays entered while its address is typed.
- `POST /camera/focus` on a group, on several entities or on a rect still sets the camera. While a lens holds the camera, `settle` puts the fitted camera back, so that call does nothing visible.
- Undoing a new tab goes to the canvas, also when another tab was showing before it.
- Command+Option+Right and Left are not bound while text is edited, where they stay the editor's line start and line end.
- Past about 33 tabs in a 1280 px window the tabs are at their least width and the rest are not drawn. There is no overflow menu.

## Amendment, 2026-10-10

The item view is camera and size management over the one canvas, and nothing more. Two parts of it were more than that and are gone.

- **The Canvas lens is removed.** It was a second canvas inside the item view, with a camera the tab kept. Lenses are Fill and Device, and in an item view the camera is always held on the item. Nothing is kept per tab but the lens.
- **The eye is removed.** Fill keeps its rule (only the item and what follows one of its elements is seen and can be made). Device sees and makes everything by the canvas's rules. Superseded the same day by "the eye and the mark" below.
- **The item shown has no selection chrome.** In Fill and Device it draws no outline and no resize handles, and its handles cannot be hit. Superseded the same day by "the eye and the mark" below.
- **Leaving centres the item at the canvas zoom.** Minimize, the Canvas tab and closing the tab showing keep the zoom the canvas had and centre the item's stored rect (with its device frame) in the free part of the viewport, so the item shrinks in place. If the item is gone the canvas gets its camera back unchanged. Closing the tab showing goes to the canvas.

## Amendment, 2026-10-10: the eye and the mark

Two rules of the amendment above changed after a day of use.

- **The eye is back, for Device only, and starts shut.** `Action::ShowOthers` sets it, `App::shows_others` reads it, and the control is `view.others`, left of the lens toggles. It is one choice for every tab and canvas, and it is never saved. Shut, a Device tab shows its item and what is hooked to it: an item anchored to the page by a position, a scroll or an element, and every comment on the page. The page is laid out as it is stored, so all of that sits where it was put. Open, Device shows the whole canvas, as before. Fill ignores the eye and keeps its rule. A Fill tab has no `view.others`.
- **With the eye shut, Device makes things the way Fill does.** A creation tool pressed on the page makes an item hooked to it. A press off the page makes nothing, and neither do the page and Document tools. `showing/gates.rs` holds the rule in `narrowed`, which now answers `Others::Following` for Fill and `Others::Hooked` for Device with the eye shut.
- **Selection chrome follows one answer, `App::mark`.** It returns `Mark::Outline`, `Mark::Entered` or nothing, and the selection outline and the resize handles both read it (`specular-interact/src/mark.rs`). An entered page on the canvas has no outline and no handles, and is drawn as it is unselected, shadow included. Its edge takes presses that the side handles used to take. The item shown in a tab has no mark while it is entered or fills the view. A page in a Device tab that Escape has left gets the outline back until a click enters it. No item shown in a tab has resize handles.

## Amendment, 2026-10-10: camera transition

The camera moves between the canvas, Device and Fill by a spring instead of landing at once. It runs on three changes only: the canvas to an item view, an item view to the canvas, and a lens change. A switch from one item tab to another, and a restore (a canvas switch, undo, reveal), are instant.

- **Only `session.camera` is animated.** The item's layout rect and its page's host change at once. The camera starts where the new layout rect has the screen centre and width the old one had, so nothing jumps, and ends at the camera the view holds. The item's centre moves in a line and the zoom in log space.
- **The spring is closed-form**, evaluated from the time since the first tick after the change (response 0.28 s, damping ratio 0.86), so it is pure and lands exactly on the target.
- **The target moves.** While it runs, `settle` keeps the target fitted after every event and samples the transition for the camera. A leave aims at a fixed camera, and `canvas_camera` reports that one, so a save during it writes what it would have without it.
- **Camera input ends a leave** where the camera is: a wheel, a pinch, a zoom action, a gesture, a canvas switch.
- **A session switch** (`Event::Motion`) turns it on. The windowed shells do; the testkit, `--snapshot`, `--script` and benchmarks do not, so the camera lands at once there.

## Amendment, 2026-10-12: the open tabs are saved

The tabs were lost at every quit, which made opening them feel wasted. Two statements above no longer hold: that the lens is never saved, and that the tab order starts over at launch.

- **What is saved.** Each canvas's open tabs, in order, each with its lens. They go in the `.canvas` file's `appState.itemTabs`, beside the camera (`docs/file-formats.md`). `appState` already holds what belongs to looking at a canvas and not to the canvas, so the document proper is still untouched: opening a tab makes no undo step and changes no node.
- **What is not saved.** Which tab is showing, the eye, and the prepared page. A space still opens on its Canvas tab, and a canvas switch still goes back to it.
- **When.** `showing::enter`, `set_lens` and `close_tab` push `Effect::Save` when a tab opens, closes or changes lens. That is the debounced autosave the document uses, so nothing is written on a frame. Switching between tabs that are open asks for no save.
- **Loading.** `OpenedCanvas::tabs` carries what the file had, and `Tabs::from_saved` drops any tab whose item is gone or cannot be shown alone. `Tabs::saved` leaves the same ones out of a write.
- **One more reader.** `App::canvas_to_save` returns the tabs with the document and the camera (`SavedCanvas`). Add it to "Models and saving" above.
- **A background canvas keeps its tabs when its file is edited from outside.** It used to lose them with the rest of its view.
- **Electron.** It never had item tabs and saved nothing like them, so there was no key to match. It drops `itemTabs` when it writes the file.

A per-space file under `.specular/` was the other place. The camera is already per canvas in the `.canvas` file and a tab names an entity of that one canvas, so the tabs go with it: a canvas that is renamed, duplicated or deleted needs no second file kept in step.
