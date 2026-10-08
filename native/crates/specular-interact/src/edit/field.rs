//! A text field of a panel edited in place: the address of a page, a custom
//! size.
//!
//! The field is chrome: a box of fixed pixel size laid out by the panel
//! renderer. Its text is an edit session keyed by an [`EntityId`] holding
//! the control's name, which no entity has, so the editor's caret, selection,
//! clipboard and input method work on it as they do anywhere.
//!
//! Enter keeps what was typed, and so does a press anywhere else, as a
//! field's blur does. Escape puts the old value back. Keeping a value runs
//! the action its [`FieldSubmit`](crate::FieldSubmit) makes of the text;
//! text that asks for nothing leaves the field as the model has it.

use glam::Vec2;
use specular_doc::{EntityId, Rect, TextAlign, TextFont, VerticalAlign};

use super::buffer::{Origin, Target, TextEdit};
use super::frame::TextFrame;
use super::measure::TextSpec;
use crate::focus::set_focus;
use crate::panel::builtin::{FIELD_LINE, FIELD_TEXT, field_box, text_area};
use crate::panel::{ControlId, field_named};
use crate::update::run_action;
use crate::{App, Effect, gesture};

/// Room kept right of the caret when the text is wider than the field, in
/// logical pixels, so the caret is never cut by the field's edge.
const CARET_ROOM: f32 = 2.0;

fn control_of(edit: &TextEdit) -> ControlId {
    ControlId::from(edit.entity.as_str().to_owned())
}

/// The field being edited, if one is.
fn editing(app: &App) -> Option<&TextEdit> {
    app.session.editing.as_ref().filter(|edit| edit.is_field())
}

/// Starts editing the field named `id` with all of its value selected.
/// Whatever was being edited ends first, and an entered page gives up the
/// keys. Does nothing for a field the model does not show.
pub(crate) fn begin(app: &mut App, id: &ControlId, effects: &mut Vec<Effect>) {
    if editing(app).is_some_and(|edit| control_of(edit) == *id) {
        return;
    }
    super::end(app, effects);
    let Some(field) = field_named(app, id) else {
        return;
    };
    set_focus(app, None, effects);
    let origin = Origin {
        text: field.value.clone(),
        rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        created: false,
    };
    let key = EntityId::from(id.as_str());
    let mut edit = TextEdit::new(key, Target::Field, &field.value, origin);
    edit.active_ms = app.session.now_ms;
    app.session.editing = Some(edit);
    effects.push(Effect::SetImeAllowed(true));
    super::place_candidates(app, effects);
}

/// Ends the edit keeping what was typed: the action its text asks for is
/// run, unless the text is as it was.
pub(super) fn end(app: &mut App, edit: &TextEdit, effects: &mut Vec<Effect>) {
    if edit.text.trim() == edit.origin.text.trim() {
        return;
    }
    let Some(field) = field_named(app, &control_of(edit)) else {
        return;
    };
    if let Some(action) = field.submit.action(&edit.text) {
        run_action(app, action, effects);
    }
}

/// Escape in a field: the typed text is dropped and the old value shows
/// again. Returns whether a field was being edited.
pub(crate) fn cancel(app: &mut App, effects: &mut Vec<Effect>) -> bool {
    if editing(app).is_none() {
        return false;
    }
    gesture::cancel(app, effects);
    app.session.editing = None;
    effects.push(Effect::SetImeAllowed(false));
    true
}

/// Where the field's line is laid out: its text box, scrolled to keep the
/// caret in view, at the pixel size of panel text whatever the zoom.
pub(super) fn frame(app: &App, edit: &TextEdit) -> Option<TextFrame> {
    let area = text_area(field_box(app, &control_of(edit))?);
    let camera = &app.session.camera;
    let zoom = camera.zoom.max(f32::EPSILON);
    let corner = Vec2::new(area.x - edit.scroll, area.y);
    Some(TextFrame {
        origin: camera.screen_to_world(corner).as_dvec2(),
        spec: TextSpec {
            font: TextFont::Sans,
            size: FIELD_TEXT / zoom,
            line_height: FIELD_LINE / zoom,
            wrap_width: None,
            align: TextAlign::Left,
        },
        box_height: Some(area.height / zoom),
        vertical: VerticalAlign::Middle,
    })
}

/// Whether `screen` is on the field being edited, where a press is the
/// text's.
pub(crate) fn is_over(app: &App, screen: Vec2) -> bool {
    editing(app)
        .and_then(|edit| field_box(app, &control_of(edit)))
        .is_some_and(|rect| rect.contains(screen))
}

/// Scrolls the field being edited the least that keeps its caret in view,
/// and no further than its text reaches.
pub(crate) fn follow_caret(app: &mut App) {
    let Some(edit) = editing(app) else {
        return;
    };
    let Some(area) = field_box(app, &control_of(edit)).map(text_area) else {
        return;
    };
    let spec = TextSpec {
        font: TextFont::Sans,
        size: FIELD_TEXT,
        line_height: FIELD_LINE,
        wrap_width: None,
        align: TextAlign::Left,
    };
    let layout = app.measure.0.layout(&edit.text, &spec);
    let Some(line) = layout.lines.first() else {
        return;
    };
    let at = |offset: usize| {
        (line.stops.iter())
            .find(|stop| stop.offset == offset)
            .map_or(0.0, |stop| stop.x)
    };
    let (caret, end) = (at(edit.caret), line.stops.last().map_or(0.0, |stop| stop.x));
    let room = (area.width - CARET_ROOM).max(0.0);
    let mut scroll = edit.scroll;
    if caret - scroll > room {
        scroll = caret - room;
    }
    scroll = scroll.min(caret);
    scroll = scroll.min((end - room).max(0.0)).max(0.0).round();
    if let Some(edit) = &mut app.session.editing {
        edit.scroll = scroll;
    }
}
