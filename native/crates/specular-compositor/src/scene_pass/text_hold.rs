//! A text batch laid out once and drawn under later cameras.
//!
//! Laying glyphs out is the dearest thing a frame of text does, and a pan
//! changes none of it: every glyph moves by the same amount. So a batch is
//! laid out in its own pixel space, with a margin beyond the target, and a
//! frame under a later camera draws the same quads through a pass viewport
//! that puts that space where the camera now has it. The layout is kept
//! until what the batch holds changes, the target shows past the margin, or
//! the glyphs would be the wrong size.
//!
//! An item may leave a kept batch only by going out of sight. One that is
//! gone from the scene while the target still shows where it was has its
//! glyphs in the layout, so the batch is laid out again without them.
//!
//! A viewport lands on whole pixels only when the pan is a whole number of
//! them. When it is not, the text is drawn within half a pixel of where it
//! belongs while the camera moves and laid out again once it stops.

use glam::Vec2;
use specular_scene::{Rect, Space};

/// How far past the target a batch is laid out, in layout pixels: how far a
/// pan can go before glyphs that were cut off at the edge are needed.
pub(crate) const MARGIN_PX: f32 = 384.0;

/// How a frame wants a space's text laid out and placed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextFrame {
    pub(crate) space: Space,
    /// Layout pixels per unit of the space: the size glyphs are rasterised
    /// at.
    pub(crate) scale: f32,
    /// Target pixels per layout pixel. Not one only while canvas text is
    /// held at an older zoom's glyph size.
    pub(crate) stretch: f32,
    /// Where the space's origin is on the target, in physical pixels.
    pub(crate) origin: Vec2,
    /// Target size in physical pixels.
    pub(crate) target: [u32; 2],
    /// The longest side a pass viewport may have.
    pub(crate) limit: f32,
}

/// Where a batch was laid out, and what was in it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Laid {
    space: Space,
    scale: f32,
    /// Where the space's origin is in the layout.
    origin: Vec2,
    /// The layout's size, which glyphs are culled to.
    size: [u32; 2],
    /// A key for each item, in paint order.
    members: Vec<u64>,
    /// Where each item is, in the space's units. An item with none recorded
    /// counts as in sight.
    extents: Vec<Rect>,
}

/// The pass viewport that draws a layout where a frame wants it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    /// Whether every glyph is on the pixel a fresh layout would put it on.
    pub(crate) exact: bool,
}

impl Laid {
    /// A layout for `frame` with as much margin as the viewport limit
    /// leaves room for. Screen text never moves under a camera and gets
    /// none.
    pub(crate) fn new(frame: &TextFrame, members: Vec<u64>) -> Self {
        let visible = frame
            .target
            .map(|side| (side as f32 / frame.stretch).ceil().max(1.0));
        let longest = visible[0].max(visible[1]);
        let room = ((frame.limit / frame.stretch - longest) / 2.0).floor();
        let margin = match frame.space {
            Space::Canvas => MARGIN_PX.min(room).max(0.0),
            Space::Screen => 0.0,
        };
        Self {
            space: frame.space,
            scale: frame.scale,
            origin: frame.origin / frame.stretch + Vec2::splat(margin),
            size: visible.map(|side| (side + 2.0 * margin) as u32),
            members,
            extents: Vec::new(),
        }
    }

    /// This layout knowing where each of its items is, in the order of the
    /// keys it was made with.
    pub(crate) fn with_extents(mut self, extents: Vec<Rect>) -> Self {
        self.extents = extents;
        self
    }

    /// The layout's size in layout pixels.
    pub(crate) fn size(&self) -> [u32; 2] {
        self.size
    }

    /// Layout pixels per unit of the space.
    pub(crate) fn scale(&self) -> f32 {
        self.scale
    }

    /// `point`, in the space's units, in layout pixels.
    pub(crate) fn point(&self, x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y) * self.scale + self.origin
    }

    /// The part of the space the layout covers, in the space's units.
    pub(crate) fn covers(&self) -> Rect {
        let origin = -self.origin / self.scale;
        Rect::new(
            origin.x,
            origin.y,
            self.size[0] as f32 / self.scale,
            self.size[1] as f32 / self.scale,
        )
    }

    /// The viewport that draws this layout for `frame` holding `members`,
    /// or `None` when it has to be laid out again: the glyphs would be a
    /// different size, the target shows past what was laid out, the batch
    /// holds something it did not, or something it held is gone from where
    /// the target still shows. An item that left by going out of sight is
    /// no matter: its glyphs are out of sight too.
    pub(crate) fn placement(&self, frame: &TextFrame, members: &[u64]) -> Option<Placement> {
        let same_raster =
            self.space == frame.space && self.scale.to_bits() == frame.scale.to_bits();
        if !same_raster || !self.still_holds(frame, members) {
            return None;
        }
        let stretch = frame.stretch;
        let mut at = frame.origin - self.origin * stretch;
        let size = Vec2::new(self.size[0] as f32, self.size[1] as f32) * stretch;
        let mut exact = false;
        if (stretch - 1.0).abs() < f32::EPSILON {
            let whole = at.round();
            exact = at.distance(whole) < 0.01;
            at = whole;
        }
        let target = Vec2::new(frame.target[0] as f32, frame.target[1] as f32);
        let covered =
            at.x <= 0.0 && at.y <= 0.0 && at.x + size.x >= target.x && at.y + size.y >= target.y;
        let allowed = size.x <= frame.limit && size.y <= frame.limit;
        (covered && allowed).then_some(Placement {
            x: at.x,
            y: at.y,
            width: size.x,
            height: size.y,
            exact,
        })
    }
}

impl Laid {
    /// Where the frame this layout was made for draws it.
    pub(crate) fn own_placement(&self, frame: &TextFrame) -> Option<Placement> {
        self.placement(frame, &self.members)
    }

    /// Whether `members` are items of this layout in the same order, and
    /// every item of it they leave out is out of `frame`'s sight.
    fn still_holds(&self, frame: &TextFrame, members: &[u64]) -> bool {
        let mut held = self.members.iter().enumerate();
        for key in members {
            loop {
                match held.next() {
                    Some((_, other)) if other == key => break,
                    Some((index, _)) if self.shows(frame, index) => return false,
                    Some(_) => {}
                    None => return false,
                }
            }
        }
        held.all(|(index, _)| !self.shows(frame, index))
    }

    /// Whether `frame`'s target shows any of where item `index` is.
    fn shows(&self, frame: &TextFrame, index: usize) -> bool {
        let Some(extent) = self.extents.get(index) else {
            return true;
        };
        let scale = frame.scale * frame.stretch;
        let (left, top) = (
            extent.x * scale + frame.origin.x,
            extent.y * scale + frame.origin.y,
        );
        let (right, bottom) = (left + extent.width * scale, top + extent.height * scale);
        let [width, height] = frame.target.map(|side| side as f32);
        right > 0.0 && bottom > 0.0 && left < width && top < height
    }
}
