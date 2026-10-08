//! A title edited in place: today a group's, tomorrow an edge's. It is one
//! line of chrome text above the item at a fixed pixel size, so its frame
//! is derived from the camera, and it ends as a label change.

use glam::DVec2;
use specular_doc::{Command, Entity, Kind, TextAlign, TextFont, VerticalAlign};

use super::buffer::{Target, TextEdit};
use super::frame::TextFrame;
use super::measure::TextSpec;
use crate::{App, Effect, Key, KeyInput, update};

/// A title's text size, in logical pixels.
pub const TITLE_SIZE: f32 = 11.0;
/// The height of a title's line, in logical pixels.
pub const TITLE_LINE: f32 = 16.5;
/// The gap between a title's line and the top of what it names, in logical
/// pixels.
pub const TITLE_GAP: f32 = 4.0;

/// Where `entity`'s title is laid out, for an entity that has one to edit.
///
/// The frame is in canvas space but sized so that the title is
/// [`TITLE_SIZE`] pixels on screen at the camera's zoom: the layout is
/// measured at that size over zoom, and the scene draws it back at the
/// pixel size.
pub(super) fn frame(app: &App, entity: &Entity) -> Option<TextFrame> {
    match &entity.kind {
        Kind::Group(_) => {}
        Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Drawing(_) | Kind::Shape(_) => {
            return None;
        }
    }
    let zoom = app.session.camera.zoom.max(f32::EPSILON);
    let line = TITLE_LINE / zoom;
    Some(TextFrame {
        origin: DVec2::new(
            entity.rect.x,
            entity.rect.y - f64::from(line + TITLE_GAP / zoom),
        ),
        spec: TextSpec {
            font: TextFont::Sans,
            size: TITLE_SIZE / zoom,
            line_height: line,
            wrap_width: None,
            align: TextAlign::Left,
        },
        box_height: Some(line),
        vertical: VerticalAlign::Middle,
    })
}

/// Ends a title edit: the trimmed text becomes the label, unless it is
/// empty or the label already, which keep the label as it was.
pub(super) fn end(app: &mut App, entity: &Entity, edit: &TextEdit, effects: &mut Vec<Effect>) {
    let label = edit.text.trim();
    if label.is_empty() || entity.label.as_deref() == Some(label) {
        return;
    }
    let command = Command::SetLabel {
        id: entity.id.clone(),
        label: Some(label.to_owned()),
    };
    update::document_step(app, command, effects);
}

/// Whether the edit in progress is a title's.
pub(crate) fn is_editing(app: &App) -> bool {
    (app.session.editing.as_ref()).is_some_and(|edit| edit.target == Target::Title)
}

/// Whether the edit in progress is one line, which Enter ends.
fn is_single_line(app: &App) -> bool {
    (app.session.editing.as_ref())
        .is_some_and(|edit| matches!(edit.target, Target::Title | Target::EdgeLabel))
}

/// A title is one line: breaks in pasted text become spaces.
pub(super) fn single_line(target: Target, text: String) -> String {
    match target {
        Target::Title | Target::EdgeLabel => text.replace('\n', " "),
        Target::Text | Target::Label | Target::Note => text,
    }
}

/// The text editor's keys, with Enter ending a one-line edit instead of
/// breaking the line. Returns whether the editor took the key.
pub(crate) fn on_key(app: &mut App, input: &KeyInput, effects: &mut Vec<Effect>) -> bool {
    let composing = (app.session.editing.as_ref()).is_some_and(|edit| edit.composition.is_some());
    if is_single_line(app) && input.key == Key::Enter && !composing {
        if input.pressed {
            super::end(app, effects);
        }
        return true;
    }
    super::keys::on_key(app, input, effects)
}

impl App {
    /// Where the text being edited is laid out, and how it is set.
    pub fn edit_frame(&self) -> Option<TextFrame> {
        let edit = self.session.editing.as_ref()?;
        super::geometry(self, edit).map(|(frame, _)| frame)
    }
}
