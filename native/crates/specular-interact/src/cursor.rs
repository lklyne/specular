//! The pointer cursor: what the tool, the gesture in flight and whatever is
//! under the pointer ask for.

use specular_doc::EdgeSide;

use crate::layout::Axis;
use crate::{App, Corner, Cursor, Effect, Gesture, Handle, Hit, LayoutHandle, Tool, edit, hit};

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
            Hit::Layout(LayoutHandle::Gap { axis: Axis::X, .. }) => Cursor::ResizeEw,
            Hit::Layout(LayoutHandle::Gap { axis: Axis::Y, .. }) => Cursor::ResizeNs,
            Hit::Layout(LayoutHandle::Reorder { .. }) => Cursor::Grab,
            // An entered page is handed the pointer, and says what it is
            // over.
            Hit::PageContent { page, .. } if session.focus.page() == Some(&page) => app
                .page_state(&page)
                .map_or(Cursor::Default, |state| state.cursor),
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
        Handle::Side(EdgeSide::Top | EdgeSide::Bottom) => Cursor::ResizeNs,
        Handle::Side(EdgeSide::Left | EdgeSide::Right) => Cursor::ResizeEw,
    }
}
