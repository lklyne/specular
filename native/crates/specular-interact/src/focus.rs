//! Keyboard focus and the forwarded pointer: the two places a page is told
//! about keys and the pointer coming or going.

use glam::Vec2;
use specular_core::{InputEvent, Modifiers, PointerEvent, PointerEventKind};
use specular_doc::EntityId;

use crate::{App, Effect, Focus};

/// Gives `page` keyboard focus, or returns it to the canvas. The input
/// method is on only while a page has focus.
pub(crate) fn set_focus(app: &mut App, page: Option<EntityId>, effects: &mut Vec<Effect>) {
    if app.session.focus.page() == page.as_ref() {
        return;
    }
    effects.push(Effect::FocusPage(page.clone()));
    effects.push(Effect::SetImeAllowed(page.is_some()));
    app.session.focus = page.map_or(Focus::Canvas, Focus::Page);
}

/// Leaves the entered page when it is no longer the whole selection.
pub(crate) fn leave_unless_selected(app: &mut App, effects: &mut Vec<Effect>) {
    if let Some(page) = app.session.focus.page()
        && app.session.selection.single_entity() != Some(page)
    {
        set_focus(app, None, effects);
    }
}

/// Tracks which page the pointer is being forwarded to, telling the one it
/// left. `local` is the pointer in the new page's CSS pixels.
pub(crate) fn set_pointer_page(
    app: &mut App,
    page: Option<EntityId>,
    local: Vec2,
    modifiers: Modifiers,
    effects: &mut Vec<Effect>,
) {
    if app.session.pointer_page == page {
        return;
    }
    if let Some(previous) = app.session.pointer_page.take() {
        effects.push(pointer_to(
            previous,
            PointerEventKind::Leave,
            local,
            modifiers,
        ));
    }
    app.session.pointer_page = page;
}

/// A pointer event for `page` at `position` in its CSS pixels.
pub(crate) fn pointer_to(
    page: EntityId,
    kind: PointerEventKind,
    position: Vec2,
    modifiers: Modifiers,
) -> Effect {
    Effect::ForwardInput {
        page,
        event: InputEvent::Pointer(PointerEvent {
            kind,
            position,
            modifiers,
        }),
    }
}
