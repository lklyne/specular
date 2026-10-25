//! How much room a draw takes, for culling and for deciding which items
//! overlap.

use crate::{Draw, PathStroke, Rect, Stroke};

impl Draw {
    /// The rect that holds everything this draws, strokes included, in the
    /// item's space.
    ///
    /// `None` when the extent is not known from the geometry alone: text and
    /// columns of it, whose size depends on shaping, and paths and polygons with no points.
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Self::Page(page) => Some(page.rect),
            Self::Image(image) => Some(image.rect),
            // A blur's tail runs a little past its radius.
            Self::Shadow(shadow) => Some(shadow.rect.outset(shadow.blur.max(0.0) * 1.5)),
            Self::Rect(rect) => Some(rect.rect.outset(shape_outset(rect.stroke))),
            Self::Ellipse(ellipse) => Some(ellipse.rect.outset(shape_outset(ellipse.stroke))),
            Self::Polygon(polygon) => Rect::bounding(polygon.points.iter().copied())
                .map(|rect| rect.outset(path_outset(polygon.stroke))),
            Self::Path(path) => {
                Rect::bounding(path.commands.iter().flat_map(|command| command.points()))
                    .map(|rect| rect.outset(path_outset(path.stroke)))
            }
            Self::Text(_) | Self::Column(_) => None,
        }
    }
}

fn shape_outset(stroke: Option<Stroke>) -> f32 {
    stroke.map_or(0.0, Stroke::outset)
}

fn path_outset(stroke: Option<PathStroke>) -> f32 {
    stroke.map_or(0.0, PathStroke::outset)
}
