//! An edge's label edited in place: one line centred on the middle of the
//! curve, at the size and place the scene draws the committed label. Enter
//! and a click elsewhere keep it, and so does Escape; an emptied label
//! removes the label.
//!
//! The edit session is keyed by an [`EntityId`] holding the edge's id: edge
//! and entity ids share one namespace, so the key names the edge alone.

use glam::{DVec2, Vec2};
use specular_doc::{Command, EdgeId, EntityId, ItemId, Rect, TextAlign, TextFont, VerticalAlign};

use super::buffer::{Origin, Target, TextEdit};
use super::frame::TextFrame;
use super::measure::TextSpec;
use crate::{App, Effect, update};

/// The label's text size in canvas units, which the scene draws at the
/// zoom.
pub const LABEL_SIZE: f32 = 16.0;
/// A line's height as a multiple of the size, which the scene's labels
/// share.
const LINE_HEIGHT: f32 = 1.4;
/// How far past the text a press still counts as on it, in logical pixels.
const SLACK: f32 = 4.0;

/// Where the label of the edge named by `key` is laid out: centred, in
/// canvas space, on the middle of its curve.
pub(super) fn frame(app: &App, key: &EntityId) -> Option<TextFrame> {
    let middle = app.edge_curve(&EdgeId::from(key.as_str()))?.point(0.5);
    let middle = app.session.camera.screen_to_world(middle).as_dvec2();
    let line = LABEL_SIZE * LINE_HEIGHT;
    Some(TextFrame {
        origin: DVec2::new(middle.x, middle.y - f64::from(line) / 2.0),
        spec: TextSpec {
            font: TextFont::Sans,
            size: LABEL_SIZE,
            line_height: line,
            wrap_width: None,
            align: TextAlign::Center,
        },
        box_height: Some(line),
        vertical: VerticalAlign::Middle,
    })
}

/// Starts editing the label of `id` with all of it selected, and makes the
/// edge the selection.
pub(crate) fn begin(app: &mut App, id: &EdgeId, effects: &mut Vec<Effect>) {
    super::end(app, effects);
    let Some(edge) = app.document.edge(id) else {
        return;
    };
    let text = edge.label.clone().unwrap_or_default();
    let origin = Origin {
        text: text.clone(),
        rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        created: false,
    };
    let key = EntityId::from(id.as_str());
    let mut edit = TextEdit::new(key, Target::EdgeLabel, &text, origin);
    edit.active_ms = app.session.now_ms;
    app.session.editing = Some(edit);
    app.session.selection.set([ItemId::Edge(id.clone())]);
    effects.push(Effect::SetImeAllowed(true));
    super::place_candidates(app, effects);
}

/// Ends a label edit: the trimmed text becomes the label, and nothing left
/// of it removes the label. No step when the label is as it was.
pub(super) fn end(app: &mut App, edit: &TextEdit, effects: &mut Vec<Effect>) {
    let id = EdgeId::from(edit.entity.as_str());
    let Some(edge) = app.document.edge(&id) else {
        return;
    };
    let text = edit.text.trim();
    let label = (!text.is_empty()).then(|| text.to_owned());
    if edge.label == label {
        return;
    }
    let mut edge = edge.clone();
    edge.label = label;
    update::document_step(app, Command::ReplaceEdge(Box::new(edge)), effects);
}

/// Whether `screen` is on the label being edited: its text, a little past
/// it, or the line it sits on.
pub(crate) fn is_over(app: &App, screen: Vec2) -> bool {
    let Some(edit) = &app.session.editing else {
        return false;
    };
    if edit.target != Target::EdgeLabel {
        return false;
    }
    let Some((frame, layout)) = super::geometry(app, edit) else {
        return false;
    };
    let camera = &app.session.camera;
    let on_text = layout
        .range_boxes(&(0..edit.text.len()))
        .into_iter()
        .any(|line| {
            let rect = frame.rect_of(&layout, line);
            let min = camera.world_to_screen(Vec2::new(rect.x as f32, rect.y as f32));
            let size = Vec2::new(rect.width as f32, rect.height as f32) * camera.zoom;
            let (min, max) = (min - Vec2::splat(SLACK), min + size + Vec2::splat(SLACK));
            screen.cmpge(min).all() && screen.cmple(max).all()
        });
    on_text
        || app
            .edge_curve(&EdgeId::from(edit.entity.as_str()))
            .is_some_and(|curve| curve.hit(screen, camera.zoom))
}

/// The edge whose label is the whole selection, named by the key an edit of
/// it uses.
pub(crate) fn selected_key(app: &App) -> Option<EntityId> {
    match app.session.selection.items() {
        [ItemId::Edge(id)] => Some(EntityId::from(id.as_str())),
        _ => None,
    }
}

impl App {
    /// The text to draw for the label of `id` in place of the document's,
    /// when `id` is the edge being edited.
    pub fn editing_edge_label(&self, id: &EdgeId) -> Option<&str> {
        let edit = self.session.editing.as_ref()?;
        (edit.target == Target::EdgeLabel && edit.entity.as_str() == id.as_str())
            .then_some(edit.text.as_str())
    }
}
