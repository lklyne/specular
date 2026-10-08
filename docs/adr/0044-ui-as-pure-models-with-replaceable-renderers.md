# ADR 0044: UI as pure models with replaceable renderers

**Status:** Proposed. Built this way in `native/crates/specular-interact/src/panel/` and beside it during the Rust rebuild. The user has not reviewed this ADR.
**Date:** 2026-10-08
**Related:** [ADR 0039](./0039-rust-canvas-render-stack.md), which chose egui for panels and was never built that way. [ADR 0040](./0040-gpui-kit-hybrid-shell.md), the GPUI Kit shell. [ADR 0042](./0042-update-view-render-loop-effects-only-io.md). [ADR 0008](./0008-unified-canvas-item-popup.md), the popup this models.
**Code:** `specular-interact/src/{panel,sidebar,menu,chat,settings,first_run}`, the built-in renderer in `specular-interact/src/panel/builtin/` and `specular-scene/src/panel/`, the Kit renderer in `specular-shell/src/view/`.

## Context

When the toolbar and item popup were due, the UI library was undecided. ADR 0039 had picked egui on paper. GPUI Kit was about to be tried. The headless runner needed panels it could draw and click with no window, which neither library can do.

## Decision

Every panel is a function from `&App` to a plain data model. A renderer draws the model and sends back the `Action` a control carries. The model is the contract, and a renderer can be swapped without touching it.

- The models: `toolbar`, `popup_for`, `context_menu`, `sidebar`, `menus`, `chat`, `settings`, `onboarding`, `repos_pane`.
- A model holds no pixels, colours or hover state. Icons are named by an enum. Controls are a small set: `Button`, `Toggle`, `Swatches`, `Dropdown`, `Stepper`, `Field`, `Choices`, `Separator`. Each has an id and its `Action`.
- A typed value is a `Field` with a rule that turns the text into an `Action`, so the model holds no editor.
- A property pick is `Action::SetProperty`. It goes to every selected item it means something for, as one undo step.
- `PopupAnchor` is the dividing line between renderers. A popup anchored to a canvas item is drawn in the canvas's own wgpu pass, so it follows the item through a pan in the same frame. A popup anchored to the toolbar can be drawn by a UI library.

Two renderers exist today.

- **Built-in.** Layout and pointer state in `specular-interact`, painting as screen-space scene items. The winit shell and every headless run use it. `view` never draws panels. The shell calls `draw_panels` after it.
- **GPUI Kit.** `specular-shell` maps each control to a Kit component in one adapter file and draws the toolbar, sidebar, toolbar popups, menus, settings, first run and the chat panel. It still uses the built-in renderer for popups beside a canvas item.

## Alternatives

**egui panels in a second pass**, ADR 0039's choice. Immediate mode fits "a function of state that returns events". It was measured at about 1 ms in the bake-off and never built in the app.

**Let the UI library own panel state**, the usual way to write a GPUI or egui app. Turned down because the headless runner and the scenario scripts could then not name a control, and the HTTP API and the menus would each need their own copy of what a button does.

**HTML panels in CEF offscreen browsers**, the plan's fallback. It brings back a JS bridge. Not needed.

## What was measured

- ADR 0040's spike drove Kit buttons and sidebar rows from `toolbar(&App)` and `sidebar(&App)` with no adapter layer.
- When the page chrome, sidebar and context menu models landed, the Kit shell compiled against them after three new match arms.
- The audit counted the built-in renderer at 1,995 lines in interact and 1,612 in scene. The Kit draws the same models in about 1,300 lines.
- With a layout cache, a pointer move builds the built-in layout 0 times warm. It was 3 to 5.
- Scenarios `j` to `n` click controls by id in the built-in renderer and check the saved canvases. About seventy PNGs of the built-in panels were read against the Electron CSS.
- The Kit renderer has been driven only by scripted `NSEvent`s. Nobody has used it by hand.

## Consequences

- Panels are tested as model snapshots with no window, and a script clicks `control shape.color` by name.
- The same `Action` runs from a toolbar button, a menu item, a key binding and an API route.
- There are two renderers to keep in step, about 7,000 lines with tests on the built-in side. The cleanup list's first task is to retire it, and that is blocked on a choice. The Kit cannot run headless, so either scripts name controls through the models with nothing drawn, or the built-in renderer stays as the headless one.
- A new control kind needs an arm in each renderer. The compiler lists them.
- A model cannot express everything a library offers. The Kit's sidebar row takes a string label, so rename in place and the swatch row are plain GPUI elements, and a stepper is two buttons around a value.
- The Kit shell does not yet draw all of every model. At the time of writing it lacks the context menu, sidebar folds and greyed menu items.
- Reversing this means moving panel state into a UI library and rewriting the headless scripts that name controls.
