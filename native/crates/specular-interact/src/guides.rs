//! Alignment and distribution guides: the lines that confirm what a move or
//! a resize has lined up with (ADR 0012).
//!
//! Guides are drawn, never pulled to. The grid is the only magnet, and a
//! guide appears when an edge or a centre already sits within
//! [`TOLERANCE`] of a neighbour's after the grid has had its say, so a line
//! on screen is always true.
//!
//! The neighbours are captured once, when the gesture begins, from what is
//! in the viewport. Each frame's guides are then a pure function of that
//! capture and where the dragged rects are now.

mod alignment;
mod distribution;

use glam::DVec2;
use specular_doc::{EdgeSide, EntityId, Kind, Rect};

use crate::{App, Corner, Gesture, Handle, HandleOwner, geometry};

pub(crate) use self::alignment::alignment_guides;
pub(crate) use self::distribution::distribution_guides;

/// How far apart two references may be and still count as aligned, and two
/// gaps as equal, in canvas units.
pub(crate) const TOLERANCE: f64 = 0.5;

/// The direction a guide line runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuideAxis {
    /// A horizontal line: tops, bottoms and vertical middles agree, or the
    /// gaps along x are equal.
    Horizontal,
    /// A vertical line: lefts, rights and horizontal middles agree, or the
    /// gaps along y are equal.
    Vertical,
}

/// The part of a rect a guide lines up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuideReference {
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The middle between top and bottom.
    HCenter,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The middle between left and right.
    VCenter,
}

impl GuideReference {
    const HORIZONTAL: [Self; 3] = [Self::Top, Self::Bottom, Self::HCenter];
    const VERTICAL: [Self; 3] = [Self::Left, Self::Right, Self::VCenter];

    const fn along(axis: GuideAxis) -> [Self; 3] {
        match axis {
            GuideAxis::Horizontal => Self::HORIZONTAL,
            GuideAxis::Vertical => Self::VERTICAL,
        }
    }

    fn of(self, rect: Rect) -> f64 {
        match self {
            Self::Top => rect.y,
            Self::Bottom => rect.y + rect.height,
            Self::HCenter => rect.y + rect.height / 2.0,
            Self::Left => rect.x,
            Self::Right => rect.x + rect.width,
            Self::VCenter => rect.x + rect.width / 2.0,
        }
    }
}

/// An entity whose edges and centres a guide can line up with, or the
/// dragged rect being lined up.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SnapCandidate {
    pub(crate) id: EntityId,
    pub(crate) rect: Rect,
}

/// A line confirming that a dragged rect's edge or centre sits on a
/// neighbour's.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentGuide {
    /// The direction the line runs.
    pub axis: GuideAxis,
    /// Where the line is across its axis: a y for a horizontal line, an x
    /// for a vertical one.
    pub coordinate: f64,
    /// Where the line starts along its axis, the least edge of the two
    /// rects.
    pub start: f64,
    /// Where it ends, their greatest edge.
    pub end: f64,
    /// The entity being dragged.
    pub dragged: EntityId,
    /// The neighbour it lines up with.
    pub candidate: EntityId,
    /// The part of the dragged rect that is aligned.
    pub dragged_reference: GuideReference,
    /// The part of the neighbour it is aligned with.
    pub candidate_reference: GuideReference,
}

/// One gap of an equal-spacing chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistributionGap {
    /// Where the gap starts along the chain's axis.
    pub start: f64,
    /// Where it ends.
    pub end: f64,
    /// Where its mark sits across the axis: the middle of what the two
    /// rects either side share.
    pub cross: f64,
}

/// Marks confirming the dragged rect sits at equal spacing in a run of
/// neighbours.
#[derive(Debug, Clone, PartialEq)]
pub struct DistributionGuide {
    /// The axis the run is spaced along.
    pub axis: GuideAxis,
    /// The spacing.
    pub gap: f64,
    /// The entity being dragged.
    pub dragged: EntityId,
    /// The neighbours in the run, in order along the axis.
    pub candidates: Vec<EntityId>,
    /// Every gap in the run.
    pub gaps: Vec<DistributionGap>,
}

/// The guides one frame shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Guides {
    /// The alignment lines.
    pub alignment: Vec<AlignmentGuide>,
    /// The equal-spacing marks.
    pub distribution: Vec<DistributionGuide>,
}

impl Guides {
    /// Whether there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.alignment.is_empty() && self.distribution.is_empty()
    }
}

/// What a gesture holds to find its guides: who is dragged, and the
/// neighbours as they stood when it began.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct GuideCapture {
    /// The dragged entities that count: a member of a dragged group is
    /// spoken for by the group.
    dragged: Vec<EntityId>,
    candidates: Vec<SnapCandidate>,
}

/// The neighbours of a gesture on `excluded`, of which `dragged` are lined
/// up.
pub(crate) fn capture(app: &App, excluded: &[EntityId], dragged: &[EntityId]) -> GuideCapture {
    let dragged = (dragged.iter())
        .filter(|id| {
            (app.document.entity(id))
                .and_then(|entity| entity.parent.as_ref())
                .is_none_or(|parent| !dragged.contains(parent))
        })
        .cloned()
        .collect();
    GuideCapture {
        dragged,
        candidates: candidates(app, excluded),
    }
}

/// Every entity in the viewport that is not `excluded`, back to front. A
/// group in view stands for its members, so they do not count twice.
fn candidates(app: &App, excluded: &[EntityId]) -> Vec<SnapCandidate> {
    let camera = app.session.camera;
    let low = camera.screen_to_world(glam::Vec2::ZERO).as_dvec2();
    let high = camera.screen_to_world(app.session.viewport).as_dvec2();
    let viewport = Rect::new(low.x, low.y, high.x - low.x, high.y - low.y);
    let shown: Vec<SnapCandidate> = (app.document.entities())
        .filter(|entity| !excluded.contains(&entity.id))
        .filter_map(|entity| {
            let rect = crate::shown_rect(app, entity)?;
            geometry::intersection(rect, viewport)
                .is_some()
                .then(|| SnapCandidate {
                    id: entity.id.clone(),
                    rect,
                })
        })
        .collect();
    let stands_for = |parent: &EntityId| {
        shown.iter().any(|candidate| candidate.id == *parent)
            && (app.document.entity(parent))
                .is_some_and(|entity| matches!(entity.kind, Kind::Group(_)))
    };
    shown
        .iter()
        .filter(|candidate| {
            (app.document.entity(&candidate.id))
                .and_then(|entity| entity.parent.as_ref())
                .is_none_or(|parent| !stands_for(parent))
        })
        .cloned()
        .collect()
}

/// The references a resize handle moves, which are the only ones its guides
/// confirm.
pub(crate) fn references_for(handle: Handle) -> Vec<GuideReference> {
    use GuideReference::{Bottom, Left, Right, Top};
    match handle {
        Handle::Side(EdgeSide::Top) => vec![Top],
        Handle::Side(EdgeSide::Bottom) => vec![Bottom],
        Handle::Side(EdgeSide::Right) => vec![Right],
        Handle::Side(EdgeSide::Left) => vec![Left],
        Handle::Corner(Corner::TopRight) => vec![Top, Right],
        Handle::Corner(Corner::TopLeft) => vec![Top, Left],
        Handle::Corner(Corner::BottomRight) => vec![Bottom, Right],
        Handle::Corner(Corner::BottomLeft) => vec![Bottom, Left],
    }
}

impl GuideCapture {
    /// The dragged rects as the document has them now.
    fn dragged_now(&self, app: &App) -> Vec<SnapCandidate> {
        (self.dragged.iter())
            .filter_map(|id| {
                let rect = crate::shown_rect(app, app.document.entity(id)?)?;
                Some(SnapCandidate {
                    id: id.clone(),
                    rect,
                })
            })
            .collect()
    }

    /// The guides for the dragged rects where they are.
    pub(crate) fn guides(&self, app: &App, references: Option<&[GuideReference]>) -> Guides {
        detect(&self.dragged_now(app), &self.candidates, references)
    }

    /// The guides for copies `delta` away from the dragged rects, which
    /// have not moved. Each original is a neighbour of its own copy.
    pub(crate) fn copy_guides(&self, app: &App, delta: DVec2) -> Guides {
        let origins = self.dragged_now(app);
        let copies: Vec<SnapCandidate> = (origins.iter())
            .map(|origin| SnapCandidate {
                id: origin.id.clone(),
                rect: origin.rect.translated(delta.x, delta.y),
            })
            .collect();
        let candidates: Vec<SnapCandidate> = (self.candidates.iter().cloned())
            .chain(origins.into_iter().map(|origin| SnapCandidate {
                id: EntityId::new(format!("{}:origin", origin.id)),
                rect: origin.rect,
            }))
            .collect();
        detect(&copies, &candidates, None)
    }
}

fn detect(
    dragged: &[SnapCandidate],
    candidates: &[SnapCandidate],
    references: Option<&[GuideReference]>,
) -> Guides {
    Guides {
        alignment: alignment_guides(dragged, candidates, references, TOLERANCE),
        distribution: (dragged.iter())
            .flat_map(|rect| {
                [GuideAxis::Horizontal, GuideAxis::Vertical]
                    .into_iter()
                    .flat_map(|axis| distribution_guides(rect, candidates, axis, TOLERANCE))
            })
            .collect(),
    }
}

impl App {
    /// The alignment and distribution guides the move or resize in flight
    /// shows. Empty with no such gesture, and before a press has become a
    /// drag.
    pub fn guides(&self) -> Guides {
        match &self.session.gesture {
            Some(Gesture::Move(drag)) => drag.guides(self),
            Some(Gesture::Resize(drag)) => match drag.owner() {
                HandleOwner::Entity(_) => {
                    (drag.guide_capture()).guides(self, Some(&references_for(drag.handle())))
                }
                HandleOwner::Selection => Guides::default(),
            },
            Some(
                Gesture::Marquee { .. }
                | Gesture::Comment(_)
                | Gesture::Place(_)
                | Gesture::Draw(_)
                | Gesture::TextSelect(_)
                | Gesture::EdgeDrag(_)
                | Gesture::Line(_),
            )
            | None => Guides::default(),
        }
    }
}
