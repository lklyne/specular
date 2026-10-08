//! The pointer on the built-in panels: hover, press, and what a release
//! over the pressed control runs.

use glam::Vec2;
use specular_core::{PointerButton, PointerEventKind};

use super::super::ControlId;
use super::node::{Run, Surface};
use super::{PanelHit, layout};
use crate::focus::set_pointer_page;
use crate::update::run_action;
use crate::{Action, App, Effect, PointerInput, Tool};

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

/// Whether a wheel or a pinch where the pointer is stays out of the canvas.
/// The popup lets it through (`data-viewport-passthrough` in
/// `CanvasItemPopup.tsx`), so a popup over the canvas is never a dead spot
/// for panning; the toolbar and an open list keep it.
pub(crate) fn swallows_scroll(app: &App) -> bool {
    let surface = (app.session.pointer)
        .and_then(|pointer| hit(app, pointer))
        .map(|hit| hit.surface);
    match surface {
        Some(Surface::Toolbar | Surface::Dropdown) => true,
        Some(Surface::Popup) | None => false,
    }
}

/// Escape with a dropdown open closes it and does nothing else.
pub(crate) fn cancel(app: &mut App) -> bool {
    app.session.panel.open.take().is_some()
}

/// Forgets the open dropdown, the hover and the press when the control they
/// name is no longer shown: the selection or the tool changed under them.
pub(crate) fn tidy(app: &mut App) {
    let ui = &app.session.panel;
    if ui.open.is_none() && ui.hover.is_none() && ui.pressed.is_none() {
        return;
    }
    let toolbar = super::super::toolbar(app);
    let popup = super::super::popup_for(app);
    let mut shown = toolbar.entries();
    if let Some(popup) = &popup {
        shown.extend(popup.entries());
    }
    let gone = |id: &Option<ControlId>| {
        (id.as_ref()).is_some_and(|id| !shown.iter().any(|(entry, _)| entry == id))
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
        PointerEventKind::Down { button, .. } => on_down(app, input, button),
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

fn on_down(app: &mut App, input: &PointerInput, button: PointerButton) -> bool {
    if app.session.gesture.is_some() {
        return false;
    }
    let layout = layout(app);
    let hit = layout.hit(input.screen);
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
    let released_on = layout.hit(input.screen).and_then(|hit| hit.control);
    if released_on.as_ref() != Some(&pressed) {
        return true;
    }
    let Some(node) = layout.node(&pressed) else {
        return true;
    };
    let in_toolbar = (layout.toolbar.as_ref())
        .is_some_and(|bar| bar.nodes.iter().any(|it| it.id.as_ref() == Some(&pressed)));
    match (node.state.enabled, node.run.clone()) {
        (true, Some(Run::Toggle)) => toggle(app, pressed, in_toolbar, effects),
        (true, Some(Run::Act { action, closes })) => {
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
fn toggle(app: &mut App, id: ControlId, in_toolbar: bool, effects: &mut Vec<Effect>) {
    if app.session.panel.open.as_ref() == Some(&id) {
        app.session.panel.open = None;
        return;
    }
    // A tool's popup hangs where a list of the toolbar opens, so opening
    // one puts the tool down.
    if in_toolbar && app.session.tool != Tool::Select {
        run_action(app, Action::SetTool(Tool::Select), effects);
    }
    app.session.panel.open = Some(id);
}
