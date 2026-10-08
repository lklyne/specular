//! The edge-drag state machine, as pure functions over screen-space
//! geometry: where a drag from an anchor starts, what it snaps to, and what
//! releasing or cancelling it does.
//!
//! Pressing an anchor an edge already ends on grabs that end (an *edit*: the
//! other end stays); any other anchor starts a new edge (a *create*). An edit
//! that ends on nothing, or is cancelled, deletes the edge, which is
//! intended: dragging an end away is how an edge is removed.

use glam::Vec2;
use specular_doc::{Edge, EdgeId, EdgeSide, EntityId};

use crate::anchors::SIDES;
use crate::edge_path::{anchor_point, facing_sides, hit_scale};
use crate::geometry::ScreenRect;

/// How near, in logical pixels at zoom 1, the pointer must be to an anchor's
/// dot to snap to it. It shrinks with the zoom as anchor hit boxes do.
const SNAP_DISTANCE: f32 = 48.0;

/// Which end of an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum End {
    From,
    To,
}

/// An entity an edge could end on, and where it is on screen.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Body<'a> {
    pub(crate) id: &'a EntityId,
    pub(crate) rect: ScreenRect,
}

/// An anchor the drag would end on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Snap {
    pub(crate) entity: EntityId,
    pub(crate) side: EdgeSide,
}

/// A drag in progress.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum State {
    /// Dragging a new edge out of an anchor.
    Create {
        from: EntityId,
        from_side: EdgeSide,
        cursor: Vec2,
        snap: Option<Snap>,
    },
    /// Dragging one end of an existing edge off its anchor.
    Edit {
        edge: EdgeId,
        moving: End,
        fixed: EntityId,
        fixed_side: EdgeSide,
        cursor: Vec2,
        snap: Option<Snap>,
    },
}

/// What a finished or cancelled drag asks the document to do.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Outcome {
    Noop,
    Create {
        from: EntityId,
        from_side: EdgeSide,
        to: EntityId,
        to_side: EdgeSide,
    },
    Edit {
        edge: EdgeId,
        moving: End,
        target: EntityId,
        target_side: EdgeSide,
    },
    Discard(EdgeId),
}

impl State {
    /// The entity and side the drag is pinned to: the grabbed anchor of a
    /// create, the far end of an edit.
    pub(crate) fn origin(&self) -> (&EntityId, EdgeSide) {
        match self {
            Self::Create {
                from, from_side, ..
            } => (from, *from_side),
            Self::Edit {
                fixed, fixed_side, ..
            } => (fixed, *fixed_side),
        }
    }

    /// Where the pointer is.
    pub(crate) const fn cursor(&self) -> Vec2 {
        match self {
            Self::Create { cursor, .. } | Self::Edit { cursor, .. } => *cursor,
        }
    }

    /// The anchor the drag would end on now.
    pub(crate) const fn snap(&self) -> Option<&Snap> {
        match self {
            Self::Create { snap, .. } | Self::Edit { snap, .. } => snap.as_ref(),
        }
    }

    /// The edge an edit is moving the end of.
    pub(crate) const fn edge(&self) -> Option<&EdgeId> {
        match self {
            Self::Create { .. } => None,
            Self::Edit { edge, .. } => Some(edge),
        }
    }

    /// Ends the drag on `snap` when the anchors found none.
    pub(crate) fn snap_if_none(&mut self, found: Snap) {
        let (Self::Create { snap, .. } | Self::Edit { snap, .. }) = self;
        snap.get_or_insert(found);
    }
}

/// Starts a drag from `side` of `entity`, with the pointer at `cursor`.
pub(crate) fn begin<'a>(
    entity: &EntityId,
    side: EdgeSide,
    cursor: Vec2,
    mut edges: impl Iterator<Item = &'a Edge>,
    rect_of: impl Fn(&EntityId) -> Option<ScreenRect>,
) -> State {
    let grabbed = edges.find_map(|edge| end_at(edge, entity, side, &rect_of));
    match grabbed {
        Some((edge, moving, fixed, fixed_side)) => State::Edit {
            edge,
            moving,
            fixed,
            fixed_side,
            cursor,
            snap: None,
        },
        None => State::Create {
            from: entity.clone(),
            from_side: side,
            cursor,
            snap: None,
        },
    }
}

/// The edge's end on `side` of `entity`, and the end that stays. The sides
/// of an edge that names none are the ones its entities face each other
/// with.
fn end_at(
    edge: &Edge,
    entity: &EntityId,
    side: EdgeSide,
    rect_of: &impl Fn(&EntityId) -> Option<ScreenRect>,
) -> Option<(EdgeId, End, EntityId, EdgeSide)> {
    let (from, to) = (rect_of(&edge.from)?, rect_of(&edge.to)?);
    let (from_side, to_side) = match (edge.from_side, edge.to_side) {
        (Some(from_side), Some(to_side)) => (from_side, to_side),
        (None, _) | (_, None) => facing_sides(from, to),
    };
    if edge.to == *entity && to_side == side {
        Some((edge.id.clone(), End::To, edge.from.clone(), from_side))
    } else if edge.from == *entity && from_side == side {
        Some((edge.id.clone(), End::From, edge.to.clone(), to_side))
    } else {
        None
    }
}

/// Moves the pointer to `cursor` and snaps to the nearest anchor of any
/// other entity within reach.
pub(crate) fn update(state: &mut State, cursor: Vec2, bodies: &[Body<'_>], zoom: f32) {
    let found = nearest_anchor(
        bodies,
        state.origin().0,
        cursor,
        SNAP_DISTANCE * hit_scale(zoom),
    );
    let (State::Create {
        cursor: at, snap, ..
    }
    | State::Edit {
        cursor: at, snap, ..
    }) = state;
    *at = cursor;
    *snap = found;
}

fn nearest_anchor(bodies: &[Body<'_>], skip: &EntityId, cursor: Vec2, reach: f32) -> Option<Snap> {
    let mut best: Option<(f32, Snap)> = None;
    for body in bodies.iter().filter(|body| body.id != skip) {
        for side in SIDES {
            let distance = anchor_point(body.rect, side).distance(cursor);
            if distance < reach && best.as_ref().is_none_or(|(nearest, _)| distance < *nearest) {
                best = Some((
                    distance,
                    Snap {
                        entity: body.id.clone(),
                        side,
                    },
                ));
            }
        }
    }
    best.map(|(_, snap)| snap)
}

/// What releasing the drag does. A create needs an anchor to land on. An
/// edit with none deletes the edge.
pub(crate) fn commit(state: &State) -> Outcome {
    match state {
        State::Create {
            from,
            from_side,
            snap: Some(snap),
            ..
        } => Outcome::Create {
            from: from.clone(),
            from_side: *from_side,
            to: snap.entity.clone(),
            to_side: snap.side,
        },
        State::Create { snap: None, .. } => Outcome::Noop,
        State::Edit {
            edge,
            moving,
            snap: Some(snap),
            ..
        } => Outcome::Edit {
            edge: edge.clone(),
            moving: *moving,
            target: snap.entity.clone(),
            target_side: snap.side,
        },
        State::Edit {
            edge, snap: None, ..
        } => Outcome::Discard(edge.clone()),
    }
}

/// What cancelling the drag does: the edge being edited is deleted, and
/// there is no "put it back". A create leaves nothing behind.
pub(crate) fn cancel(state: &State) -> Outcome {
    match state {
        State::Create { .. } => Outcome::Noop,
        State::Edit { edge, .. } => Outcome::Discard(edge.clone()),
    }
}

/// The side across from `side`: the direction the free end of a preview
/// leaves in.
pub(crate) const fn opposite(side: EdgeSide) -> EdgeSide {
    match side {
        EdgeSide::Top => EdgeSide::Bottom,
        EdgeSide::Bottom => EdgeSide::Top,
        EdgeSide::Left => EdgeSide::Right,
        EdgeSide::Right => EdgeSide::Left,
    }
}

#[cfg(test)]
mod tests;
