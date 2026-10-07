//! Items whose pixels come from elsewhere: live pages and decoded images.

use specular_doc::EntityId;

use crate::Rect;

/// A live page's latest frame, with its popup layer over it when one is
/// showing.
///
/// The page is named by its entity id, as events and effects name it. Whoever
/// renders the scene knows which page host backs each entity.
#[derive(Debug, Clone, PartialEq)]
pub struct PageDraw {
    /// Which page.
    pub page: EntityId,
    /// Where the page's viewport is drawn.
    pub rect: Rect,
    /// Corner radius the frame is clipped to.
    pub corner_radius: f32,
}

/// A handle to pixels the renderer was given earlier: a decoded image, a
/// video frame or a rendered document preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(pub u64);

/// An image stretched over a rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageDraw {
    /// Which image.
    pub image: ImageId,
    /// Where it is drawn.
    pub rect: Rect,
    /// The part of the image to show, as fractions of its width and height.
    /// [`ImageDraw::WHOLE`] shows all of it; `cover` fits crop with this.
    pub source: Rect,
    /// Corner radius the image is clipped to.
    pub corner_radius: f32,
}

impl ImageDraw {
    /// The `source` rect that shows the whole image.
    pub const WHOLE: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

    /// The whole image over `rect`, with square corners.
    pub const fn new(image: ImageId, rect: Rect) -> Self {
        Self {
            image,
            rect,
            source: Self::WHOLE,
            corner_radius: 0.0,
        }
    }
}
