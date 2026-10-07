//! [`Gesture`]: the drag in flight, and what moving, releasing and
//! cancelling it does.

use glam::{DVec2, Vec2};
use specular_doc::{Command, EntityId};

use crate::draw::{self, DrawStroke};
use crate::marquee::MarqueeMode;
use crate::move_drag::{self, MoveDrag};
use crate::place::{self, PlaceDrag};
use crate::resize_drag::{self, ResizeDrag};
use crate::{App, Effect, PointerInput, comment, geometry, marquee};

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
    /// Dragging out a comment region.
    CommentRegion {
        /// The canvas point the drag started at.
        start: DVec2,
        /// The same point on screen, to tell a drag from a click.
        start_screen: Vec2,
        /// The canvas point the pointer is at.
        current: DVec2,
        /// The page the drag started over, which the comment binds to.
        page: Option<EntityId>,
    },
    /// Pressed with a one-shot creation tool: the release places a page, a
    /// text, a sticky or a shape.
    Place(PlaceDrag),
    /// Drawing a freehand stroke.
    Draw(DrawStroke),
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
        Some(Gesture::CommentRegion {
            start,
            start_screen,
            page,
            ..
        }) => {
            app.session.gesture = Some(Gesture::CommentRegion {
                start,
                start_screen,
                current: world,
                page,
            });
        }
        Some(Gesture::Place(mut drag)) => {
            place::drag(app, &mut drag, world, input.modifiers);
            app.session.gesture = Some(Gesture::Place(drag));
        }
        Some(Gesture::Draw(mut stroke)) => {
            draw::drag(app, &mut stroke, world, input.modifiers.shift);
            app.session.gesture = Some(Gesture::Draw(stroke));
        }
    }
}

/// The button came up, ending `gesture`. A move, a copy or a resize becomes
/// one undo step; a resized page is re-laid-out; a marquee changes the
/// selection; a comment drag long enough to not be a click creates its
/// annotation; a placement or a stroke creates its entity.
pub(crate) fn finish(
    app: &mut App,
    gesture: Gesture,
    input: &PointerInput,
    effects: &mut Vec<Effect>,
) {
    let screen = input.screen;
    match gesture {
        Gesture::Marquee {
            start,
            start_screen,
            origin,
            dragged,
            ..
        } => marquee::finish(app, start, start_screen, origin, dragged, input),
        Gesture::Move(drag) => move_drag::finish(app, drag, effects),
        Gesture::Resize(drag) => resize_drag::finish(app, &drag, effects),
        Gesture::CommentRegion {
            start,
            start_screen,
            page,
            ..
        } => {
            if (screen - start_screen).length() >= comment::MIN_COMMENT_DRAG {
                let end = app.session.camera.screen_to_world(screen).as_dvec2();
                comment::create_region(app, geometry::spanning(start, end), page);
            }
        }
        Gesture::Place(drag) => place::finish(app, drag, effects),
        Gesture::Draw(stroke) => draw::finish(app, &stroke, effects),
    }
}

/// Abandons the gesture in flight: a move or resize snaps back, a marquee or
/// a comment region is dropped, and what a placement or a stroke was making
/// is taken back.
pub(crate) fn cancel(app: &mut App) {
    match app.session.gesture.take() {
        None | Some(Gesture::Marquee { .. } | Gesture::CommentRegion { .. }) => {}
        Some(Gesture::Move(drag)) => move_drag::cancel(app, &drag),
        Some(Gesture::Resize(drag)) => crate::live::restore(&mut app.document, drag.starts()),
        Some(Gesture::Place(drag)) => place::cancel(app, &drag),
        Some(Gesture::Draw(stroke)) => draw::cancel(app, &stroke),
    }
}

/// Runs `command` as one undo step.
pub(crate) fn apply_step(app: &mut App, command: Command) {
    if let Err(error) = app.history.apply(&mut app.document, command) {
        tracing::warn!("command refused: {error}");
    }
}
