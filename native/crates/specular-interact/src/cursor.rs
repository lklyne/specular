//! The pointer cursor: what the tool, the gesture in flight and whatever is
//! under the pointer ask for.

use specular_doc::EdgeSide;

use crate::{App, Corner, Cursor, Effect, Gesture, Handle, Hit, Tool, edit, hit};

/// Asks the shell for a different cursor when the one wanted has changed.
pub(crate) fn refresh(app: &mut App, effects: &mut Vec<Effect>) {
    let cursor = wanted(app);
    if app.session.cursor != cursor {
        app.session.cursor = cursor;
        effects.push(Effect::SetCursor(cursor));
    }
}

fn wanted(app: &App) -> Cursor {
    let session = &app.session;
    match &session.gesture {
        // The handle's cursor stays for the whole drag, wherever the pointer
        // goes.
        Some(Gesture::Resize(drag)) => return of_handle(drag.handle()),
        Some(Gesture::TextSelect(_)) => return Cursor::Text,
        Some(Gesture::EdgeDrag(_)) => return Cursor::Crosshair,
        Some(Gesture::Line(_)) => return Cursor::Grabbing,
        Some(
            Gesture::Move(_)
            | Gesture::Marquee { .. }
            | Gesture::Comment(_)
            | Gesture::Place(_)
            | Gesture::Draw(_),
        ) => {
            return session.tool.cursor();
        }
        None => {}
    }
    // A panel is over whatever the canvas would ask a cursor for.
    if crate::panel::builtin::over_field(app) {
        return Cursor::Text;
    }
    if crate::panel::builtin::over(app) {
        return Cursor::Default;
    }
    if edit::is_over_text(app) {
        return Cursor::Text;
    }
    match (session.tool, session.pointer) {
        (Tool::Select, Some(pointer)) => match hit::hit_test(app, pointer) {
            Hit::Handle { handle, .. } => of_handle(handle),
            Hit::Anchor { .. } => Cursor::Crosshair,
            // `Cursor` has no column or row resize arrow for a gap strip.
            Hit::Layout(_) => Cursor::Grab,
            Hit::Comment { .. }
            | Hit::GroupLabel { .. }
            | Hit::PageContent { .. }
            | Hit::EntityBody { .. }
            | Hit::GroupBorder { .. }
            | Hit::Edge { .. }
            | Hit::Empty
            | Hit::Panel { .. } => Cursor::Default,
        },
        (
            Tool::Select
            | Tool::AddPage
            | Tool::AddText
            | Tool::AddSticky
            | Tool::AddDocument
            | Tool::AddShape
            | Tool::Draw
            | Tool::Comment
            | Tool::Inspect,
            _,
        ) => session.tool.cursor(),
    }
}

const fn of_handle(handle: Handle) -> Cursor {
    match handle {
        Handle::Corner(Corner::TopLeft | Corner::BottomRight) => Cursor::ResizeNwse,
        Handle::Corner(Corner::TopRight | Corner::BottomLeft) => Cursor::ResizeNesw,
        // `Cursor` has no vertical or horizontal resize arrow, and a
        // diagonal one would point the wrong way.
        Handle::Side(EdgeSide::Top | EdgeSide::Bottom | EdgeSide::Left | EdgeSide::Right) => {
            Cursor::Default
        }
    }
}
