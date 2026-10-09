//! The pointer on the built-in panels: hover, press, and what a release
//! over the pressed control runs.

use glam::Vec2;
use specular_core::{PointerButton, PointerEventKind};

use super::super::{ControlId, MenuTarget};
use super::node::{Run, Surface};
use super::{ContextMenu, PanelHit, layout, sidebar};
use crate::focus::set_pointer_page;
use crate::update::run_action;
use crate::{Action, App, Effect, Gesture, PointerInput, edit};

/// What is under `screen` on a panel, or `None` over the canvas or with the
/// built-in panels off.
pub(crate) fn hit(app: &App, screen: Vec2) -> Option<PanelHit> {
    if !app.session.panel.built_in {
        return None;
    }
    layout(app).hit(screen)
}

/// Whether the pointer is over a panel.
pub(crate) fn over(app: &App) -> bool {
    (app.session.pointer).is_some_and(|pointer| hit(app, pointer).is_some())
}

/// The text field under `screen`, if the pointer is on one.
fn field_at(layout: &super::PanelLayout, screen: Vec2) -> Option<ControlId> {
    let id = layout.hit(screen)?.control?;
    layout
        .node(&id)
        .is_some_and(|node| super::field::is_field(node.chrome))
        .then_some(id)
}

/// Whether the pointer is over a text field, which shows a text cursor.
pub(crate) fn over_field(app: &App) -> bool {
    app.session.panel.built_in
        && (app.session.pointer).is_some_and(|pointer| field_at(&layout(app), pointer).is_some())
}

/// Escape with a dropdown open closes it and does nothing else.
pub(crate) fn cancel(app: &mut App) -> bool {
    let ui = &mut app.session.panel;
    ui.menu.take().is_some() | ui.open.take().is_some()
}

/// Forgets the open dropdown, the hover and the press when the control they
/// name is no longer shown: the selection or the tool changed under them. A
/// field being edited whose control has gone is dropped with what was typed,
/// and one that is still there keeps its caret in view.
pub(crate) fn tidy(app: &mut App) {
    if let Some(edit) = app.session.editing.as_ref().filter(|edit| edit.is_field()) {
        let id = ControlId::from(edit.entity().as_str().to_owned());
        if crate::panel::field_named(app, &id).is_none() {
            app.session.editing = None;
        } else {
            edit::follow_field_caret(app);
        }
    }
    let menu_gone = (app.session.panel.menu.as_ref())
        .is_some_and(|open| crate::panel::context_menu(app, &open.target).is_none());
    if menu_gone {
        app.session.panel.menu = None;
    }
    let ui = &app.session.panel;
    if ui.open.is_none() && ui.hover.is_none() && ui.pressed.is_none() {
        return;
    }
    let toolbar = super::super::toolbar(app);
    let dock = super::super::dock(app);
    let strip = super::super::view_strip(app);
    let mut listed = toolbar.entries();
    listed.extend((strip.tabs.iter()).map(|tab| (tab.id.clone(), Some(&tab.action))));
    if let Some(dock) = &dock {
        listed.extend(dock.entries());
    }
    // A hovered or pressed row of the context menu is shown while it is open.
    let menu = (app.session.panel.menu.as_ref())
        .and_then(|open| crate::panel::context_menu(app, &open.target));
    if let Some(menu) = &menu {
        listed.extend(menu.entries());
    }
    let shown = listed;
    // The sidebar's rows come and go with the canvas; one that has gone
    // leaves a name nothing is laid out under, which does no harm.
    let gone = |id: &Option<ControlId>| {
        (id.as_ref()).is_some_and(|id| {
            !id.as_str().starts_with("sidebar.") && !shown.iter().any(|(entry, _)| entry == id)
        })
    };
    let (open, hover, pressed) = (gone(&ui.open), gone(&ui.hover), gone(&ui.pressed));
    let ui = &mut app.session.panel;
    if open {
        ui.open = None;
    }
    if hover {
        ui.hover = None;
    }
    if pressed {
        ui.pressed = None;
    }
}

/// Offers a pointer event to the panels. Returns whether they took it, in
/// which case the canvas, the tool in hand and the pages see none of it.
pub(crate) fn on_pointer(app: &mut App, input: &PointerInput, effects: &mut Vec<Effect>) -> bool {
    if !app.session.panel.built_in {
        return false;
    }
    match input.kind {
        PointerEventKind::Move => on_move(app, input, effects),
        PointerEventKind::Leave => {
            app.session.panel.hover = None;
            false
        }
        PointerEventKind::Down {
            button,
            click_count,
        } => on_down(app, input, button, click_count, effects),
        PointerEventKind::Up { button, .. } => {
            button == PointerButton::Left && on_up(app, input, effects)
        }
    }
}

fn on_move(app: &mut App, input: &PointerInput, effects: &mut Vec<Effect>) -> bool {
    // A drag on the canvas, or inside a page, keeps the pointer wherever it
    // goes.
    if app.session.gesture.is_some() || app.session.captured.holder().is_some() {
        app.session.panel.hover = None;
        return false;
    }
    let layout = layout(app);
    let Some(hit) = layout.hit(input.screen) else {
        app.session.panel.hover = None;
        return false;
    };
    app.session.panel.hover = hit
        .control
        .filter(|id| layout.node(id).is_some_and(|node| node.state.enabled));
    app.session.pointer = Some(input.screen);
    app.session.hover = None;
    set_pointer_page(app, None, Vec2::ZERO, input.modifiers, effects);
    true
}

fn on_down(
    app: &mut App,
    input: &PointerInput,
    button: PointerButton,
    click_count: u8,
    effects: &mut Vec<Effect>,
) -> bool {
    if app.session.gesture.is_some() {
        return false;
    }
    let mut layout = layout(app);
    let field = field_at(&layout, input.screen);
    // A press on anything but the field being edited ends the edit first,
    // keeping what was typed, as a blur does.
    let editing = (app.text_edit().filter(|edit| edit.is_field()))
        .map(|edit| ControlId::from(edit.entity().as_str().to_owned()));
    if button == PointerButton::Left && editing.is_some() && editing != field {
        edit::end(app, effects);
        super::forget_layout(app);
        layout = super::layout(app);
    }
    let hit = layout.hit(input.screen);
    if app.session.panel.menu.is_some() {
        // A press anywhere but on the menu closes it, and that is all it does.
        let on_menu = hit
            .as_ref()
            .is_some_and(|hit| hit.surface == Surface::Dropdown);
        if !on_menu {
            app.session.panel.menu = None;
            app.session.panel.pressed = None;
            app.session.pointer = Some(input.screen);
            return true;
        }
    }
    if let Some(open) = &app.session.panel.open {
        // A press anywhere but in the open list or on its trigger closes
        // the list, and that is all it does.
        let in_list = hit
            .as_ref()
            .is_some_and(|hit| hit.surface == Surface::Dropdown);
        let on_trigger = hit
            .as_ref()
            .is_some_and(|hit| hit.control.as_ref() == Some(open));
        if !in_list && !on_trigger {
            app.session.panel.open = None;
            app.session.panel.pressed = None;
            app.session.pointer = Some(input.screen);
            return true;
        }
    }
    let Some(hit) = hit else {
        return false;
    };
    app.session.pointer = Some(input.screen);
    let canvas = hit.control.as_ref().and_then(|id| canvas_of(app, id));
    if let (PointerButton::Right, Some(canvas)) = (button, &canvas) {
        app.session.panel.menu = Some(ContextMenu {
            target: MenuTarget::Canvas(canvas.id.clone()),
            at: input.screen,
        });
        return true;
    }
    // A second press on a canvas renames it where it is.
    if let (PointerButton::Left, 2, Some(canvas)) = (button, click_count, &canvas) {
        edit::begin_field(app, &canvas.rename.id, effects);
        return true;
    }
    if let (PointerButton::Left, Some(id)) = (button, field_at(&layout, input.screen)) {
        edit::begin_field(app, &id, effects);
        if let Some(drag) = edit::press(app, input, click_count) {
            app.session.gesture = Some(Gesture::from(drag));
        }
        return true;
    }
    if button == PointerButton::Left {
        app.session.panel.pressed = hit
            .control
            .filter(|id| layout.node(id).is_some_and(|node| node.state.enabled));
    }
    true
}

fn on_up(app: &mut App, input: &PointerInput, effects: &mut Vec<Effect>) -> bool {
    let Some(pressed) = app.session.panel.pressed.take() else {
        return false;
    };
    app.session.pointer = Some(input.screen);
    let layout = layout(app);
    let menu = app.session.panel.menu.take();
    let released_on = layout.hit(input.screen).and_then(|hit| hit.control);
    if released_on.as_ref() != Some(&pressed) {
        return true;
    }
    let Some(node) = layout.node(&pressed) else {
        return true;
    };
    match (node.state.enabled, node.run.clone()) {
        (true, Some(Run::Toggle)) => toggle(app, pressed),
        (true, Some(Run::Act { action, closes })) => {
            let action = sidebar::picked(app, &pressed, action, input.modifiers);
            // A paste lands where the menu was opened, not where its item is.
            if let (Action::Paste, Some(menu)) = (&action, &menu) {
                app.session.pointer = Some(menu.at);
            }
            run_action(app, action, effects);
            if closes {
                app.session.panel.open = None;
            }
        }
        (false, _) | (true, None) => {}
    }
    true
}

/// Opens the dropdown `id`, or closes it when it is the open one.
fn toggle(app: &mut App, id: ControlId) {
    let open = &mut app.session.panel.open;
    *open = (open.as_ref() != Some(&id)).then_some(id);
}

/// The canvas of the sidebar that the control `id` is the row of, if it is.
fn canvas_of(app: &App, id: &ControlId) -> Option<crate::CanvasRow> {
    if !id.as_str().starts_with("sidebar.canvas.") || !app.session.sidebar.shown() {
        return None;
    }
    crate::sidebar(app)
        .canvases
        .into_iter()
        .find(|row| row.control == *id)
}
