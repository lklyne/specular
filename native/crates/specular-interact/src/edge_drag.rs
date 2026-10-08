//! Dragging an edge: out of an anchor to make one, or off the end of an
//! existing one to move it.
//!
//! Nothing is written to the document while the pointer is down: the drag
//! is a preview in the session, and the release makes one undo step (or
//! none). The state machine is in [`controller`].

mod controller;

use glam::Vec2;
use specular_doc::{Command, Edge, EdgeEnd, EdgeId, EdgeKind, EdgeSide, EntityId, ItemId};

use self::controller::{Body, End, Outcome, Snap, State};
use crate::edge_path::{EdgeCurve, anchor_point, facing_sides};
use crate::geometry::ScreenRect;
use crate::{App, Effect, Gesture, caps, hit, update};

/// A drag from an anchor, up to its release.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeDrag {
    state: State,
}

/// What the scene draws while an edge is dragged. All of it is in logical
/// screen pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgePreview {
    /// The rubber band: from the end that stays to the anchor it would snap
    /// to, or to the pointer.
    pub curve: EdgeCurve,
    /// Where the band is pinned: the dot at its fixed end.
    pub origin: Vec2,
    /// The anchor the release would land on, if the pointer is near one.
    pub snap: Option<Vec2>,
}

impl App {
    /// The edge being dragged, or `None` when none is.
    pub fn edge_preview(&self) -> Option<EdgePreview> {
        let Some(Gesture::EdgeDrag(drag)) = &self.session.gesture else {
            return None;
        };
        let (fixed, side) = drag.state.origin();
        let origin = anchor_point(self.screen_rect_of(fixed)?, side);
        let end = match drag.state.snap() {
            Some(snap) => (
                anchor_point(self.screen_rect_of(&snap.entity)?, snap.side),
                snap.side,
            ),
            None => (drag.state.cursor(), controller::opposite(side)),
        };
        Some(EdgePreview {
            curve: EdgeCurve::joining((origin, side), end, self.session.camera.zoom),
            origin,
            snap: drag.state.snap().map(|_| end.0),
        })
    }

    /// The edge whose end is being dragged, which the scene leaves out: the
    /// rubber band stands in for it.
    pub fn rerouting(&self) -> Option<&EdgeId> {
        match &self.session.gesture {
            Some(Gesture::EdgeDrag(drag)) => drag.state.edge(),
            Some(
                Gesture::Move(_)
                | Gesture::Resize(_)
                | Gesture::Marquee { .. }
                | Gesture::Comment(_)
                | Gesture::Place(_)
                | Gesture::Draw(_)
                | Gesture::TextSelect(_),
            )
            | None => None,
        }
    }

    fn screen_rect_of(&self, id: &EntityId) -> Option<ScreenRect> {
        Some(ScreenRect::of(
            &self.session.camera,
            self.document.entity(id)?.rect,
        ))
    }
}

/// Starts a drag from `side` of `entity`, with the pointer at `screen`.
pub(crate) fn begin(app: &App, entity: &EntityId, side: EdgeSide, screen: Vec2) -> EdgeDrag {
    let state = controller::begin(entity, side, screen, app.document.edges(), |id| {
        app.screen_rect_of(id)
    });
    EdgeDrag { state }
}

/// The pointer moved to `screen`.
pub(crate) fn drag(app: &App, drag: &mut EdgeDrag, screen: Vec2) {
    let camera = &app.session.camera;
    let rects: Vec<(EntityId, ScreenRect)> = (app.document.entities())
        .filter(|entity| caps::has_anchors(&entity.kind))
        .map(|entity| (entity.id.clone(), ScreenRect::of(camera, entity.rect)))
        .collect();
    let bodies: Vec<Body<'_>> = (rects.iter())
        .map(|(id, rect)| Body { id, rect: *rect })
        .collect();
    controller::update(&mut drag.state, screen, &bodies, camera.zoom);
    if drag.state.snap().is_none()
        && let Some(snap) = body_snap(app, &drag.state, screen)
    {
        drag.state.snap_if_none(snap);
    }
}

/// The side of the entity under `screen` that faces the end that stays, for
/// a release over a body and not near any anchor.
fn body_snap(app: &App, state: &State, screen: Vec2) -> Option<Snap> {
    let (fixed, _) = state.origin();
    let body = hit::body_at(app, screen);
    let target = hit::entity_of(&body)?;
    let entity = app.document.entity(target)?;
    // Dropping into the group the edge starts in is not a connection.
    if target == fixed
        || !caps::has_anchors(&entity.kind)
        || app
            .document
            .ancestors(fixed)
            .any(|group| group.id == *target)
    {
        return None;
    }
    let (_, side) = facing_sides(app.screen_rect_of(fixed)?, app.screen_rect_of(target)?);
    Some(Snap {
        entity: target.clone(),
        side,
    })
}

/// The button came up: makes the edge, moves its end, or deletes it.
pub(crate) fn finish(app: &mut App, drag: &EdgeDrag, effects: &mut Vec<Effect>) {
    run(app, controller::commit(&drag.state), effects);
}

/// The drag was abandoned.
pub(crate) fn cancel(app: &mut App, drag: &EdgeDrag, effects: &mut Vec<Effect>) {
    run(app, controller::cancel(&drag.state), effects);
}

fn run(app: &mut App, outcome: Outcome, effects: &mut Vec<Effect>) {
    match outcome {
        Outcome::Noop => {}
        Outcome::Create {
            from,
            from_side,
            to,
            to_side,
        } => {
            let edge = Edge {
                from_side: Some(from_side),
                to_side: Some(to_side),
                to_end: Some(EdgeEnd::Arrow),
                kind: Some(EdgeKind::Connection),
                ..Edge::new(app.fresh_id(), from.clone(), to)
            };
            let command = Command::InsertEdge {
                edge: Box::new(edge),
                at: app.document.stack_len(),
            };
            update::document_step(app, command, effects);
            // What was selected stays if the edge starts at it; anything
            // else is let go.
            if !app.session.selection.contains(&ItemId::Entity(from)) {
                app.session.selection.set([]);
            }
        }
        Outcome::Edit {
            edge,
            moving,
            target,
            target_side,
        } => {
            let Some(mut moved) = app.document.edge(&edge).cloned() else {
                return;
            };
            match moving {
                End::From => (moved.from, moved.from_side) = (target, Some(target_side)),
                End::To => (moved.to, moved.to_side) = (target, Some(target_side)),
            }
            if app.document.edge(&edge) != Some(&moved) {
                update::document_step(app, Command::ReplaceEdge(Box::new(moved)), effects);
            }
        }
        Outcome::Discard(edge) => {
            update::document_step(app, Command::RemoveEdge(edge), effects);
        }
    }
}

impl From<EdgeDrag> for Gesture {
    fn from(drag: EdgeDrag) -> Self {
        Self::EdgeDrag(drag)
    }
}
