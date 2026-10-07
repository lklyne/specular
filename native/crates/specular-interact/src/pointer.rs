//! Pointer routing.
//!
//! A left press goes to the active tool first. A gesture it starts owns the
//! pointer until the button comes up, and the page under it sees none of it.
//! Every other press focuses the page under it and is forwarded, and a
//! release goes to the page that got the press.

use glam::Vec2;
use specular_core::{PointerButton, PointerEventKind};
use specular_doc::{EntityId, ItemId};

use crate::focus::{pointer_to, set_focus, set_hover};
use crate::{App, Effect, Gesture, Hit, PointerInput, Tool, gesture, handles, hit};

pub(crate) fn on_pointer(app: &mut App, input: &PointerInput, effects: &mut Vec<Effect>) {
    match input.kind {
        PointerEventKind::Move => on_move(app, input, effects),
        PointerEventKind::Leave => {
            // A drag that leaves the window still ends at its release.
            if app.session.gesture.is_none() {
                app.session.pointer = None;
                set_hover(app, None, Vec2::ZERO, input.modifiers, effects);
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
        gesture::drag(app, input.screen);
        return;
    }
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    match hit::page_at(app, world) {
        Some((page, placement)) => {
            let local = placement.page_local(world).as_vec2();
            set_hover(app, Some(page.clone()), local, input.modifiers, effects);
            effects.push(pointer_to(
                page,
                PointerEventKind::Move,
                local,
                input.modifiers,
            ));
        }
        None => set_hover(app, None, Vec2::ZERO, input.modifiers, effects),
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
    if button == PointerButton::Left && tool_takes_press(app, input) {
        return;
    }
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    let under = hit::page_at(app, world);
    if button == PointerButton::Left {
        let page = under.as_ref().map(|(page, _)| page.clone());
        set_focus(app, page, effects);
    }
    if let Some((page, placement)) = under {
        app.session.captured.press(button, page.clone());
        effects.push(pointer_to(
            page,
            PointerEventKind::Down {
                button,
                click_count,
            },
            placement.page_local(world).as_vec2(),
            input.modifiers,
        ));
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
    if button == PointerButton::Left
        && let Some(gesture) = app.session.gesture.take()
    {
        gesture::finish(app, gesture, input.screen, effects);
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
/// in which case nothing is focused or forwarded.
///
/// With the select tool: a handle of the selected entity starts a resize,
/// Alt+press on a page starts a move, and any other press selects the page
/// under it (or clears the selection) and carries on to the page.
fn tool_takes_press(app: &mut App, input: &PointerInput) -> bool {
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
        Tool::Select => match hit::hit_test(app, input.screen) {
            Hit::Handle { corner, .. } => {
                let Some(target) = handles::resize_target(app) else {
                    return false;
                };
                app.session.gesture = Some(Gesture::Resize {
                    entity: target.entity.clone(),
                    corner,
                    grab: corner.point(target.rect) - world,
                    start: target.rect,
                    min_size: target.min_size,
                });
                true
            }
            Hit::PageContent { page, .. } if input.modifiers.alt => {
                let Some(start) = app.document.entity(&page).map(|entity| entity.rect) else {
                    return false;
                };
                select_only(app, Some(page.clone()));
                app.session.gesture = Some(Gesture::Move {
                    origin: world,
                    items: vec![(page, start)],
                });
                true
            }
            Hit::PageContent { page, .. } => {
                select_only(app, Some(page));
                false
            }
            Hit::Empty => {
                select_only(app, None);
                false
            }
        },
        // Placement and drawing arrive with each kind's slice. Until then
        // these tools hold the press so it does not reach a page.
        Tool::AddPage
        | Tool::AddText
        | Tool::AddSticky
        | Tool::AddDocument
        | Tool::AddShape
        | Tool::Draw => true,
    }
}

fn select_only(app: &mut App, entity: Option<EntityId>) {
    app.session.selection.set(entity.map(ItemId::Entity));
}
