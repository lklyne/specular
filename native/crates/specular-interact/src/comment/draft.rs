//! The draft: the comment being written, which is not in the document yet,
//! and the composer its text is typed in.
//!
//! The composer is chrome: a card of a fixed pixel size beside what the
//! draft is on, whatever the zoom. Its text is an edit session keyed by an
//! [`EntityId`] holding the draft's id, which no entity or edge has.

use glam::Vec2;
use specular_core::text::{TextAlign, TextFont};
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, EntityId, Rect, VerticalAlign,
};

use crate::edit::{self, Origin, Target, TextEdit};
use crate::focus::set_focus;
use crate::geometry::ScreenRect;
use crate::gesture;
use crate::{App, Effect, TextFrame, TextSpec, Tool, chat, geometry};

/// The composer's text size and line height, in logical pixels.
const TEXT_SIZE: f32 = 11.0;
const LINE: f32 = 15.4;
/// The width the text wraps at, and the card around it.
const WRAP_WIDTH: f32 = 240.0;
const CARD_WIDTH: f32 = 260.0;
/// The room between the card's edge and its text: at each side, and above
/// and below.
const PADDING: Vec2 = Vec2::new(10.0, 8.0);
/// Where the card's corner sits from the point a draft is on.
const POINT_OFFSET: Vec2 = Vec2::new(14.0, -13.0);
/// The gap between the bottom of a region or an element and the card.
const GAP: f32 = 8.0;

/// Makes `draft` the comment being written. Whatever was being edited ends
/// first, so an earlier draft is committed or dropped, and an entered page
/// gives up the keys.
///
/// With a right panel the draft is finished in the panel's field: it opens,
/// and the canvas keeps only the marker. Otherwise the draft's text is
/// edited in a card on the canvas.
pub(super) fn open(app: &mut App, draft: Annotation, effects: &mut Vec<Effect>) {
    edit::end(app, effects);
    set_focus(app, None, effects);
    // The comment just committed by the line above gives up the focus: the
    // new draft is the one thing being worked on.
    app.session.focused_comment = None;
    if app.session.chat.available() {
        app.session.comment_draft = Some(draft);
        app.session.chat.show();
        return;
    }
    let origin = Origin {
        text: String::new(),
        rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        created: false,
    };
    let key = EntityId::from(draft.id.as_str());
    let mut edit = TextEdit::new(key, Target::Comment, "", origin);
    edit.active_ms = app.session.now_ms;
    app.session.comment_draft = Some(draft);
    app.session.editing = Some(edit);
    effects.push(Effect::SetImeAllowed(true));
    edit::place_candidates(app, effects);
}

/// Ends the draft's edit: the trimmed text makes it a comment in the
/// document, as one undo step, and that comment takes the focus. The comment
/// is queued into a thread. A draft with nothing written is dropped and
/// leaves no step.
pub(crate) fn end(app: &mut App, edit: &TextEdit, effects: &mut Vec<Effect>) {
    let Some(draft) = app.session.comment_draft.take() else {
        return;
    };
    chat::commit_comment(app, draft, &edit.text, Vec::new(), effects);
}

/// The comment stages of Escape, each of which is all that one press does:
/// an open draft is dropped with what was typed in it, and with no draft a
/// focused comment loses the focus. Returns whether either happened.
///
/// A drag in flight is not a stage: with no draft open Escape is left to
/// cancel it, focused comment or not.
pub(crate) fn cancel(app: &mut App, effects: &mut Vec<Effect>) -> bool {
    if app.session.comment_draft.take().is_some() {
        // The only drag a draft can have under it is a selection in its own
        // text, and a draft in the panel has no text here.
        if app
            .session
            .editing
            .as_ref()
            .is_some_and(TextEdit::is_comment)
        {
            gesture::cancel(app, effects);
            app.session.editing = None;
            effects.push(Effect::SetImeAllowed(false));
        }
        return true;
    }
    app.session.gesture.is_none() && app.session.focused_comment.take().is_some()
}

/// Leaving the comment tool drops a draft made by one of its gestures: a
/// point, an element or a region. A draft on the selection came from a popup
/// that sits under any tool, so it stays. Only a panel's draft is dropped
/// here: a card on the canvas is committed by the edit ending.
pub(crate) fn on_tool_change(app: &mut App) {
    if app.session.tool == Tool::Comment || !app.session.chat.available() {
        return;
    }
    let selection = app.session.comment_draft.as_ref().is_some_and(|draft| {
        draft
            .metadata
            .as_ref()
            .is_some_and(|metadata| metadata.contains_key("selectionEntityIds"))
    });
    if !selection {
        app.session.comment_draft = None;
    }
}

/// Drops the draft and the focus: the document they were in has been
/// replaced.
pub(crate) fn forget(app: &mut App) {
    app.session.comment_draft = None;
    app.session.focused_comment = None;
}

/// The top-left corner of the composer's card on screen: beside a point,
/// centred under a region, and under an element from its left edge.
fn card_corner(app: &App, draft: &Annotation) -> Option<Vec2> {
    let camera = &app.session.camera;
    match &draft.anchor {
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => {
            let point = Vec2::new(*canvas_x as f32, *canvas_y as f32);
            Some(camera.world_to_screen(point) + POINT_OFFSET)
        }
        AnnotationAnchor::Region(_) => {
            let region = ScreenRect::of(camera, super::region_on_canvas(app, draft)?);
            Some(Vec2::new(
                region.centre().x - CARD_WIDTH / 2.0,
                region.max().y + GAP,
            ))
        }
        AnnotationAnchor::Element { .. } => {
            let element = ScreenRect::of(camera, super::element_on_canvas(app, draft)?);
            Some(Vec2::new(element.min.x, element.max().y + GAP))
        }
        // No draft is made on a page point.
        AnnotationAnchor::Page { .. } => None,
    }
}

/// Where the draft's text is laid out.
///
/// The frame is in canvas space but sized so that the text is
/// [`TEXT_SIZE`] pixels on screen at the camera's zoom, as a title's is.
pub(crate) fn frame(app: &App) -> Option<TextFrame> {
    let camera = &app.session.camera;
    let zoom = camera.zoom.max(f32::EPSILON);
    let corner = card_corner(app, app.session.comment_draft.as_ref()?)? + PADDING;
    Some(TextFrame {
        origin: camera.screen_to_world(corner).as_dvec2(),
        spec: TextSpec {
            font: TextFont::Sans,
            size: TEXT_SIZE / zoom,
            line_height: LINE / zoom,
            wrap_width: Some(WRAP_WIDTH / zoom),
            align: TextAlign::Left,
        },
        box_height: None,
        vertical: VerticalAlign::Top,
    })
}

/// Whether `screen` is on the composer, where a press is the text's.
pub(crate) fn is_over_composer(app: &App, screen: Vec2) -> bool {
    app.comment_composer()
        .is_some_and(|card| ScreenRect::of(&app.session.camera, card).contains(screen))
}

impl App {
    /// The comment being written. It is not in the document: committing its
    /// text puts it there.
    pub fn comment_draft(&self) -> Option<&Annotation> {
        self.session.comment_draft.as_ref()
    }

    /// The card the draft's text is typed in, in canvas space: as wide as
    /// the composer is at this zoom and as tall as its lines of text. `None`
    /// when the right panel takes the draft.
    pub fn comment_composer(&self) -> Option<Rect> {
        // A shell with a right panel finishes the draft there.
        if self.session.chat.available() {
            return None;
        }
        let camera = &self.session.camera;
        let corner = card_corner(self, self.session.comment_draft.as_ref()?)?;
        let lines = (self.editing_layout()).map_or(1, |layout| layout.lines.len().max(1));
        let size = Vec2::new(CARD_WIDTH, PADDING.y * 2.0 + lines as f32 * LINE);
        Some(geometry::rect(
            camera.screen_to_world(corner).as_dvec2(),
            (size / camera.zoom.max(f32::EPSILON)).as_dvec2(),
        ))
    }

    /// The comment the keys act on: the one last committed or picked.
    pub fn focused_comment(&self) -> Option<&AnnotationId> {
        self.session.focused_comment.as_ref()
    }
}
