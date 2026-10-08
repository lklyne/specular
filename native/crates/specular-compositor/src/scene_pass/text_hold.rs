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
        }
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
    /// different size, the target shows past what was laid out, or the
    /// batch holds something it did not. Items that have left are no
    /// matter: they were culled because they are out of sight.
    pub(crate) fn placement(&self, frame: &TextFrame, members: &[u64]) -> Option<Placement> {
        let same_raster =
            self.space == frame.space && self.scale.to_bits() == frame.scale.to_bits();
        if !same_raster || !is_subsequence(members, &self.members) {
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

/// Whether every key of `needle` is in `hay`, in the same order.
fn is_subsequence(needle: &[u64], hay: &[u64]) -> bool {
    let mut hay = hay.iter();
    needle.iter().all(|key| hay.any(|held| held == key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(pan: Vec2, zoom: f32) -> TextFrame {
        TextFrame {
            space: Space::Canvas,
            scale: zoom * 2.0,
            stretch: 1.0,
            origin: pan * 2.0,
            target: [3200, 2000],
            limit: 16_384.0,
        }
    }

    /// Where a canvas point lands on the target through `placement`.
    fn drawn(laid: &Laid, placement: &Placement, x: f32, y: f32) -> Vec2 {
        let at = laid.point(x, y);
        let size = Vec2::new(laid.size()[0] as f32, laid.size()[1] as f32);
        Vec2::new(placement.x, placement.y)
            + at * Vec2::new(placement.width, placement.height) / size
    }

    #[test]
    fn a_fresh_layout_is_placed_exactly_and_covers_the_target_with_its_margin() {
        let frame = canvas(Vec2::new(40.0, 40.0), 0.25);
        let laid = Laid::new(&frame, vec![1, 2]);
        let placement = laid.placement(&frame, &[1, 2]).unwrap();
        assert!(placement.exact);
        assert_eq!((placement.x, placement.y), (-MARGIN_PX, -MARGIN_PX));
        assert_eq!(laid.size(), [3200 + 768, 2000 + 768]);
        // Canvas (100, 100) is at (40 + 25) * 2 on the target.
        assert_eq!(drawn(&laid, &placement, 100.0, 100.0), Vec2::splat(130.0));
    }

    #[test]
    fn a_pan_inside_the_margin_moves_the_viewport_and_keeps_the_layout() {
        let laid = Laid::new(&canvas(Vec2::new(40.0, 40.0), 0.25), vec![1, 2]);
        let panned = canvas(Vec2::new(-100.5, 61.0), 0.25);
        let placement = laid.placement(&panned, &[1, 2]).unwrap();
        assert!(placement.exact);
        assert_eq!(
            drawn(&laid, &placement, 100.0, 100.0),
            (Vec2::new(-100.5, 61.0) + 25.0) * 2.0
        );
    }

    #[test]
    fn a_pan_past_the_margin_is_laid_out_again() {
        let laid = Laid::new(&canvas(Vec2::ZERO, 0.25), vec![1]);
        let far = MARGIN_PX / 2.0 + 1.0;
        assert!(
            laid.placement(&canvas(Vec2::new(far, 0.0), 0.25), &[1])
                .is_none()
        );
        assert!(
            laid.placement(&canvas(Vec2::new(0.0, -far), 0.25), &[1])
                .is_none()
        );
        assert!(
            laid.placement(&canvas(Vec2::new(far - 2.0, 0.0), 0.25), &[1])
                .is_some()
        );
    }

    #[test]
    fn a_pan_by_part_of_a_pixel_is_drawn_on_the_nearest_and_marked() {
        let laid = Laid::new(&canvas(Vec2::ZERO, 0.25), vec![1]);
        let placement = laid
            .placement(&canvas(Vec2::new(10.2, 0.0), 0.25), &[1])
            .unwrap();
        assert!(!placement.exact);
        assert_eq!(placement.x, -MARGIN_PX + 20.0);
    }

    #[test]
    fn another_glyph_size_is_laid_out_again() {
        let laid = Laid::new(&canvas(Vec2::ZERO, 0.25), vec![1]);
        assert!(laid.placement(&canvas(Vec2::ZERO, 0.26), &[1]).is_none());
    }

    #[test]
    fn held_glyphs_are_stretched_to_the_camera_s_zoom() {
        // Rasterised for zoom 0.25 and drawn at 0.3: 1.2 target pixels a
        // layout pixel.
        let held = |pan: Vec2, zoom: f32| TextFrame {
            scale: 0.5,
            stretch: zoom / 0.25,
            origin: pan * 2.0,
            ..canvas(pan, zoom)
        };
        let laid = Laid::new(&held(Vec2::ZERO, 0.25), vec![1]);
        let zoomed = held(Vec2::new(-30.0, -20.0), 0.3);
        let placement = laid.placement(&zoomed, &[1]).unwrap();
        assert!(!placement.exact);
        let at = drawn(&laid, &placement, 1000.0, 400.0);
        let wanted = (Vec2::new(1000.0, 400.0) * 0.3 + Vec2::new(-30.0, -20.0)) * 2.0;
        assert!(at.distance(wanted) < 0.01, "{at} against {wanted}");
        // Zoomed out, the target shows more of the canvas than was laid out.
        let out = held(Vec2::ZERO, 0.19);
        assert!(laid.placement(&out, &[1]).is_none());
    }

    #[test]
    fn items_may_leave_a_batch_but_not_join_or_swap() {
        let frame = canvas(Vec2::ZERO, 0.25);
        let laid = Laid::new(&frame, vec![1, 2, 3]);
        assert!(laid.placement(&frame, &[1, 3]).is_some());
        assert!(laid.placement(&frame, &[]).is_some());
        assert!(laid.placement(&frame, &[1, 2, 3, 4]).is_none());
        assert!(laid.placement(&frame, &[3, 1]).is_none());
    }

    #[test]
    fn screen_text_has_no_margin_and_holds_only_while_nothing_moves() {
        let screen = TextFrame {
            space: Space::Screen,
            scale: 2.0,
            stretch: 1.0,
            origin: Vec2::ZERO,
            target: [3200, 2000],
            limit: 16_384.0,
        };
        let laid = Laid::new(&screen, vec![7]);
        assert_eq!(laid.size(), [3200, 2000]);
        assert!(
            laid.placement(&screen, &[7])
                .is_some_and(|placed| placed.exact)
        );
        assert!(laid.placement(&screen, &[8]).is_none());
    }

    #[test]
    fn the_margin_gives_way_when_the_viewport_limit_is_near() {
        let frame = TextFrame {
            limit: 3_300.0,
            ..canvas(Vec2::ZERO, 0.25)
        };
        let laid = Laid::new(&frame, vec![1]);
        assert_eq!(laid.size(), [3300, 2100]);
        assert!(laid.placement(&frame, &[1]).is_some());
    }
}
