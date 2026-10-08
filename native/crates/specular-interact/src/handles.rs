//! The resize handles of the selection: where they are, which one the
//! pointer is on, and the rect a drag of one produces.

use glam::{DVec2, Vec2};
use specular_doc::{EdgeSide, EntityId, Rect};

use crate::App;
use crate::geometry::{self, ScreenRect};

/// Handle square size in logical pixels. It does not scale with zoom.
pub const HANDLE_SIZE: f32 = 8.0;
/// Side of the square, and thickness of the strip, where a press counts as a
/// handle. Larger than the drawn handle.
const HANDLE_HIT: f32 = 12.0;
/// How far outside an item's bounds its selection outline is drawn, in
/// logical pixels. Handles centre on the outline, not the bounds.
pub const OUTLINE_PADDING: f32 = 1.0;

/// A corner of an entity rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    /// Low x, low y.
    TopLeft,
    /// High x, low y.
    TopRight,
    /// High x, high y.
    BottomRight,
    /// Low x, high y.
    BottomLeft,
}

impl Corner {
    /// Every corner, clockwise from the top left.
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomRight,
        Self::BottomLeft,
    ];

    /// `-1` on the axes where this corner sits at the low edge, `1` at the
    /// high edge.
    fn sign(self) -> DVec2 {
        match self {
            Self::TopLeft => DVec2::new(-1.0, -1.0),
            Self::TopRight => DVec2::new(1.0, -1.0),
            Self::BottomRight => DVec2::new(1.0, 1.0),
            Self::BottomLeft => DVec2::new(-1.0, 1.0),
        }
    }

    /// Where this corner of `rect` is, in canvas space.
    pub fn point(self, rect: Rect) -> DVec2 {
        geometry::origin(rect) + geometry::size(rect) * (self.sign() * 0.5 + DVec2::splat(0.5))
    }

    /// The corner a resize from this one holds still.
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Self::TopLeft => Self::BottomRight,
            Self::TopRight => Self::BottomLeft,
            Self::BottomRight => Self::TopLeft,
            Self::BottomLeft => Self::TopRight,
        }
    }
}

/// One of the eight resize handles: a corner square or a strip along a side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Handle {
    /// A corner, resizing both axes.
    Corner(Corner),
    /// A side, resizing one axis.
    Side(EdgeSide),
}

impl Handle {
    /// Every handle in hit order. Corners come first, so a press at the very
    /// corner resizes diagonally and not along the side strip under it.
    pub const ALL: [Self; 8] = [
        Self::Corner(Corner::TopLeft),
        Self::Corner(Corner::TopRight),
        Self::Corner(Corner::BottomRight),
        Self::Corner(Corner::BottomLeft),
        Self::Side(EdgeSide::Top),
        Self::Side(EdgeSide::Right),
        Self::Side(EdgeSide::Bottom),
        Self::Side(EdgeSide::Left),
    ];

    /// `-1` on the axes where this handle moves the low edge, `1` the high
    /// edge, `0` where it moves neither.
    pub(crate) fn sign(self) -> DVec2 {
        match self {
            Self::Corner(corner) => corner.sign(),
            Self::Side(EdgeSide::Top) => DVec2::new(0.0, -1.0),
            Self::Side(EdgeSide::Right) => DVec2::new(1.0, 0.0),
            Self::Side(EdgeSide::Bottom) => DVec2::new(0.0, 1.0),
            Self::Side(EdgeSide::Left) => DVec2::new(-1.0, 0.0),
        }
    }

    /// Where this handle sits on `rect`, in canvas space: the corner, or the
    /// middle of the side.
    pub fn point(self, rect: Rect) -> DVec2 {
        geometry::origin(rect) + geometry::size(rect) * (self.sign() * 0.5 + DVec2::splat(0.5))
    }

    /// Where a press counts as this handle, given the selection outline on
    /// screen. A side strip runs the outline's full length.
    fn hit_rect(self, outline: ScreenRect) -> ScreenRect {
        let (min, max, half) = (outline.min, outline.max(), HANDLE_HIT / 2.0);
        match self {
            Self::Corner(corner) => {
                let unit = (corner.sign() * 0.5 + DVec2::splat(0.5)).as_vec2();
                ScreenRect::square(min + outline.size * unit, HANDLE_HIT)
            }
            Self::Side(EdgeSide::Top) => {
                ScreenRect::new(min.x, min.y - half, outline.size.x, HANDLE_HIT)
            }
            Self::Side(EdgeSide::Bottom) => {
                ScreenRect::new(min.x, max.y - half, outline.size.x, HANDLE_HIT)
            }
            Self::Side(EdgeSide::Left) => {
                ScreenRect::new(min.x - half, min.y, HANDLE_HIT, outline.size.y)
            }
            Self::Side(EdgeSide::Right) => {
                ScreenRect::new(max.x - half, min.y, HANDLE_HIT, outline.size.y)
            }
        }
    }
}

/// What a set of handles resizes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HandleOwner {
    /// The one selected entity.
    Entity(EntityId),
    /// Everything selected, scaled together inside the selection bounds.
    Selection,
}

impl App {
    /// The selected entity and its rect, when exactly one entity is selected.
    pub fn handle_target(&self) -> Option<(&EntityId, Rect)> {
        let entity = self
            .document
            .entity(self.session.selection.single_entity()?)?;
        Some((&entity.id, crate::shown_rect(self, entity)?))
    }

    /// What shows resize handles, and the canvas rect they sit around: the
    /// selected entity, or the bounds of a selection of several. A selection
    /// of several whose bounds cannot form (an entity and an edge, say) has
    /// no handles. Nothing has them while text is edited: the entity's size
    /// follows its text, and a press beside it ends the edit.
    pub fn handles(&self) -> Option<(HandleOwner, Rect)> {
        if self.session.editing.is_some() {
            return None;
        }
        if self.session.selection.items().len() > 1 {
            let scope = self.selection_scope();
            return (scope.operands.len() > 1)
                .then_some(scope.shown_bounds)
                .flatten()
                .map(|bounds| (HandleOwner::Selection, bounds));
        }
        self.handle_target()
            .map(|(entity, rect)| (HandleOwner::Entity(entity.clone()), rect))
    }
}

/// The handle of `bounds` under `screen`. Hit-tested in screen space, around
/// the outline the selection draws, so the target size is constant at any
/// zoom.
pub(crate) fn hit(bounds: ScreenRect, screen: Vec2) -> Option<Handle> {
    let outline = bounds.inflated(OUTLINE_PADDING);
    Handle::ALL
        .into_iter()
        .find(|handle| handle.hit_rect(outline).contains(screen))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

    #[test]
    fn corner_points_are_the_rect_corners() {
        let points = Corner::ALL.map(|corner| corner.point(RECT));
        assert_eq!(
            points,
            [
                DVec2::new(100.0, 100.0),
                DVec2::new(500.0, 100.0),
                DVec2::new(500.0, 400.0),
                DVec2::new(100.0, 400.0),
            ]
        );
    }
}
