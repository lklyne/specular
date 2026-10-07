//! Keyboard focus and hover: the two places a page is told about the
//! pointer and keys coming or going.

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

/// Tracks the hovered page, telling the one the pointer left. `local` is the
/// pointer in the new page's CSS pixels.
pub(crate) fn set_hover(
    app: &mut App,
    page: Option<EntityId>,
    local: Vec2,
    modifiers: Modifiers,
    effects: &mut Vec<Effect>,
) {
    if app.session.hover == page {
        return;
    }
    if let Some(previous) = app.session.hover.take() {
        effects.push(pointer_to(
            previous,
            PointerEventKind::Leave,
            local,
            modifiers,
        ));
    }
    app.session.hover = page;
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
