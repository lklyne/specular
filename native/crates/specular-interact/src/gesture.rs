//! [`Gesture`]: the drag in flight, and what moving, releasing and
//! cancelling it does.

use glam::{DVec2, Vec2};
use specular_doc::{Command, EntityId};

use crate::comment::{self, CommentDrag};
use crate::draw::{self, DrawStroke};
use crate::edge_drag::{self, EdgeDrag};
use crate::edit::{self, TextSelectDrag};
use crate::layout::drag::{self as line_drag, LineDrag};
use crate::marquee::MarqueeMode;
use crate::move_drag::{self, MoveDrag};
use crate::place::{self, PlaceDrag};
use crate::resize_drag::{self, ResizeDrag};
use crate::{App, Effect, PointerInput, group_fit, marquee};

/// A pointer drag between a press and its release. It owns the pointer: no
/// page sees the moves or the release.
///
/// Match on this without a wildcard arm, so a new gesture makes the compiler
/// list every place that must handle it.
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    /// Pressed on a body: a move of the selection once the pointer has
    /// travelled, a click until then.
    Move(MoveDrag),
    /// Dragging a resize handle.
    Resize(ResizeDrag),
    /// Pressed on empty canvas, or through a body with Command or Control
    /// held: a marquee once the pointer has travelled, a click until then.
    Marquee {
        /// The canvas point the press landed on.
        start: DVec2,
        /// The same point on screen, to tell a drag from a click.
        start_screen: Vec2,
        /// The canvas point the pointer is at.
        current: DVec2,
        /// The entity the press went through. The marquee leaves it out, and
        /// a click selects it.
        origin: Option<EntityId>,
        /// Whether the pointer has travelled far enough to be a drag.
        dragged: bool,
        /// What the rect takes, from the modifiers at the latest move.
        mode: MarqueeMode,
    },
    /// Pressed with the comment tool: a region once the pointer has
    /// travelled, a click on a point or an element until then.
    Comment(CommentDrag),
    /// Pressed with a one-shot creation tool: the release places a page, a
    /// text, a sticky or a shape.
    Place(PlaceDrag),
    /// Drawing a freehand stroke.
    Draw(DrawStroke),
    /// Pressed inside the text being edited: placing the caret, and
    /// selecting once the pointer has travelled.
    TextSelect(TextSelectDrag),
    /// Dragging an edge out of an anchor, or one end of an existing edge off
    /// its anchor.
    EdgeDrag(EdgeDrag),
    /// Dragging a reorder dot or a gap strip of a line (ADR 0015).
    Line(LineDrag),
}

/// The pointer moved mid-drag, or a modifier changed under it.
pub(crate) fn drag(app: &mut App, input: &PointerInput) {
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    // Taken out while it runs, so it can change the document it sits beside.
    match app.session.gesture.take() {
        None => {}
        Some(Gesture::Move(mut drag)) => {
            move_drag::drag(app, &mut drag, input);
            app.session.gesture = Some(Gesture::Move(drag));
        }
        Some(Gesture::Resize(drag)) => {
            resize_drag::drag(app, &drag, world, input.modifiers);
            app.session.gesture = Some(Gesture::Resize(drag));
        }
        Some(gesture @ Gesture::Marquee { .. }) => {
            app.session.gesture = Some(gesture);
            marquee::drag(app, input);
        }
        Some(Gesture::Comment(mut drag)) => {
            comment::drag(app, &mut drag, input.screen);
            app.session.gesture = Some(Gesture::Comment(drag));
        }
        Some(Gesture::Place(mut drag)) => {
            place::drag(app, &mut drag, world, input.modifiers);
            app.session.gesture = Some(Gesture::Place(drag));
        }
        Some(Gesture::Draw(mut stroke)) => {
            draw::drag(app, &mut stroke, world, input.modifiers.shift);
            app.session.gesture = Some(Gesture::Draw(stroke));
        }
        Some(Gesture::TextSelect(drag)) => {
            edit::drag(app, &drag, world);
            app.session.gesture = Some(Gesture::TextSelect(drag));
        }
        Some(Gesture::EdgeDrag(mut drag)) => {
            edge_drag::drag(app, &mut drag, input.screen);
            app.session.gesture = Some(Gesture::EdgeDrag(drag));
        }
        Some(Gesture::Line(mut drag)) => {
            line_drag::drag(app, &mut drag, world);
            app.session.gesture = Some(Gesture::Line(drag));
        }
    }
}

/// The button came up, ending `gesture`. A move, a copy or a resize becomes
/// one undo step; a resized page is re-laid-out; a marquee changes the
/// selection; a comment press opens a draft or asks the shell what is under
/// it; a placement or a stroke creates its entity.
pub(crate) fn finish(
    app: &mut App,
    gesture: Gesture,
    input: &PointerInput,
    effects: &mut Vec<Effect>,
) {
    match gesture {
        Gesture::Marquee {
            start,
            start_screen,
            origin,
            dragged,
            ..
        } => marquee::finish(app, start, start_screen, origin, dragged, input),
        Gesture::Move(drag) => move_drag::finish(app, drag, input.modifiers, effects),
        Gesture::Resize(drag) => resize_drag::finish(app, &drag),
        Gesture::Comment(drag) => comment::finish(app, &drag, effects),
        Gesture::Place(drag) => place::finish(app, drag, effects),
        Gesture::Draw(stroke) => draw::finish(app, &stroke, effects),
        // The selection is already where the drag left it.
        Gesture::TextSelect(_) => {}
        Gesture::EdgeDrag(drag) => edge_drag::finish(app, &drag, effects),
        Gesture::Line(drag) => line_drag::finish(app, &drag, effects),
    }
}

/// Abandons the gesture in flight: a move or resize snaps back, a marquee or
/// a comment press is dropped, what a placement or a stroke was making is
/// taken back, and an edge end being dragged takes its edge with it.
pub(crate) fn cancel(app: &mut App, effects: &mut Vec<Effect>) {
    match app.session.gesture.take() {
        None | Some(Gesture::Marquee { .. } | Gesture::Comment(_) | Gesture::TextSelect(_)) => {}
        Some(Gesture::Move(drag)) => move_drag::cancel(app, &drag),
        Some(Gesture::Resize(drag)) => {
            crate::live::restore(&mut app.document, drag.starts());
            crate::live::restore(&mut app.document, drag.followers());
        }
        Some(Gesture::Place(drag)) => place::cancel(app, &drag),
        Some(Gesture::Draw(stroke)) => draw::cancel(app, &stroke),
        Some(Gesture::EdgeDrag(drag)) => edge_drag::cancel(app, &drag, effects),
        Some(Gesture::Line(drag)) => line_drag::cancel(app, &drag),
    }
}

/// Runs `command` as one undo step. Undoing it brings back the selection as
/// it stands now, so a caller selects what the step made after this, not
/// before.
pub(crate) fn apply_step(app: &mut App, command: Command) {
    apply_fitted(app, command, &[]);
}

/// Runs `command` as one undo step with the refit of the groups it touched
/// (see `group_fit`), leaving `moved` groups, which the step moves or
/// resizes itself, as they are.
pub(crate) fn apply_fitted(app: &mut App, command: Command, moved: &[EntityId]) {
    let command = group_fit::then_fit(&mut app.document, command, moved);
    let selection = app.session.selection.clone();
    if let Err(error) = (app.history).apply_from(&mut app.document, command, selection) {
        tracing::warn!("command refused: {error}");
    }
}
