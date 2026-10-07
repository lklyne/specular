//! Pointer routing.
//!
//! A left press goes to the active tool first. A gesture it starts owns the
//! pointer until the button comes up, and no page sees any of it. Only the
//! entered page (the one with keyboard focus) hears the pointer: moves over
//! its body, presses on it, and everything up to the release of a press it
//! got.

use glam::Vec2;
use specular_core::{PointerButton, PointerEventKind};
use specular_doc::EntityId;

use crate::focus::{pointer_to, set_pointer_page};
use crate::{
    App, Effect, Gesture, Hit, PointerInput, Tool, draw, edit, gesture, hit, place, select,
};

pub(crate) fn on_pointer(app: &mut App, input: &PointerInput, effects: &mut Vec<Effect>) {
    match input.kind {
        PointerEventKind::Move => on_move(app, input, effects),
        PointerEventKind::Leave => {
            // A drag that leaves the window still ends at its release.
            if app.session.gesture.is_none() {
                app.session.pointer = None;
                app.session.hover = None;
                set_pointer_page(app, None, Vec2::ZERO, input.modifiers, effects);
            }
        }
        PointerEventKind::Down {
            button,
            click_count,
        } => on_down(app, input, button, click_count, effects),
        PointerEventKind::Up {
            button,
            click_count,
        } => on_up(app, input, button, click_count, effects),
    }
}

fn on_move(app: &mut App, input: &PointerInput, effects: &mut Vec<Effect>) {
    app.session.pointer = Some(input.screen);
    if app.session.gesture.is_some() {
        gesture::drag(app, input);
        return;
    }
    let hit = hit::hit_test(app, input.screen);
    app.session.hover = hit::entity_of(&hit).cloned();
    // A page that got a press keeps the pointer until the release, so a drag
    // inside it (a text selection, a slider) carries on past its edge.
    let held = app.session.captured.holder().and_then(|page| {
        let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
        let local = app.page_placement(page)?.page_local(world).as_vec2();
        Some((page.clone(), local))
    });
    match held.or_else(|| entered_page(app, hit)) {
        Some((page, local)) => {
            set_pointer_page(app, Some(page.clone()), local, input.modifiers, effects);
            effects.push(pointer_to(
                page,
                PointerEventKind::Move,
                local,
                input.modifiers,
            ));
        }
        None => set_pointer_page(app, None, Vec2::ZERO, input.modifiers, effects),
    }
}

fn on_down(
    app: &mut App,
    input: &PointerInput,
    button: PointerButton,
    click_count: u8,
    effects: &mut Vec<Effect>,
) {
    app.session.pointer = Some(input.screen);
    if button == PointerButton::Left && app.session.editing.is_some() {
        // A press in the text being edited is the editor's. Anywhere else it
        // ends the edit before it does what it would have done.
        if let Some(drag) = edit::press(app, input, click_count) {
            app.session.gesture = Some(drag.into());
            return;
        }
        edit::end(app, effects);
    }
    if button == PointerButton::Left && tool_takes_press(app, input, click_count, effects) {
        return;
    }
    if let Some((page, local)) = entered_page(app, hit::hit_test(app, input.screen)) {
        app.session.captured.press(button, page.clone());
        effects.push(pointer_to(
            page,
            PointerEventKind::Down {
                button,
                click_count,
            },
            local,
            input.modifiers,
        ));
    }
}

/// The entered page and the point in its CSS pixels, when `hit` is its body.
fn entered_page(app: &App, hit: Hit) -> Option<(EntityId, Vec2)> {
    match hit {
        Hit::PageContent { page, local } if app.session.focus.page() == Some(&page) => {
            Some((page, local))
        }
        Hit::PageContent { .. }
        | Hit::GroupLabel { .. }
        | Hit::Handle { .. }
        | Hit::Anchor { .. }
        | Hit::EntityBody { .. }
        | Hit::GroupBorder { .. }
        | Hit::Edge { .. }
        | Hit::Empty => None,
    }
}

fn on_up(
    app: &mut App,
    input: &PointerInput,
    button: PointerButton,
    click_count: u8,
    effects: &mut Vec<Effect>,
) {
    app.session.pointer = Some(input.screen);
    if button == PointerButton::Left && app.session.gesture.is_some() {
        // The release counts as the drag's last frame.
        gesture::drag(app, input);
        let Some(gesture) = app.session.gesture.take() else {
            return;
        };
        gesture::finish(app, gesture, input, effects);
        return;
    }
    // The release goes to the page that got the press even if the pointer
    // has left it, so drags that end outside complete and no page is left
    // thinking a button is held.
    let Some(page) = app.session.captured.release(button) else {
        return;
    };
    let Some(placement) = app.page_placement(&page) else {
        return;
    };
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    effects.push(pointer_to(
        page,
        PointerEventKind::Up {
            button,
            click_count,
        },
        placement.page_local(world).as_vec2(),
        input.modifiers,
    ));
}

/// Offers a left press to the active tool. Returns whether the tool took it,
/// in which case nothing is forwarded.
fn tool_takes_press(
    app: &mut App,
    input: &PointerInput,
    click_count: u8,
    effects: &mut Vec<Effect>,
) -> bool {
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    match app.session.tool {
        Tool::Comment => {
            app.session.gesture = Some(Gesture::CommentRegion {
                start: world,
                start_screen: input.screen,
                current: world,
                page: hit::page_at(app, world).map(|(page, _)| page),
            });
            true
        }
        Tool::Select => select::press(app, input, click_count, effects),
        tool @ (Tool::AddPage
        | Tool::AddText
        | Tool::AddSticky
        | Tool::AddShape
        | Tool::AddDocument) => {
            app.session.gesture = place::begin(tool, world).map(Gesture::Place);
            true
        }
        Tool::Draw => {
            let stroke = draw::begin(app, world);
            app.session.gesture = Some(Gesture::Draw(stroke));
            true
        }
    }
}
