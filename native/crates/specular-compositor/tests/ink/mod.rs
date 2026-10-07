//! Red rects and finding where a render put its ink, for the tests that
//! draw shapes and text.

use specular_scene::{Color, Rect, RectDraw};

use crate::common::{TARGET_SIZE, pixel};
use crate::scene_harness::BACKGROUND;

pub(crate) const RED: Color = Color::rgb(255, 0, 0);
pub(crate) const RED_TEXEL: [u8; 4] = [255, 0, 0, 255];

pub(crate) fn rect(x: f32, y: f32, width: f32, height: f32, fill: Color) -> RectDraw {
    RectDraw::filled(Rect::new(x, y, width, height), fill)
}

/// The columns and rows holding any pixel that is not the background, as
/// `(left, top, right, bottom)`.
pub(crate) fn ink(pixels: &[[u8; 4]]) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..TARGET_SIZE {
        for x in 0..TARGET_SIZE {
            if pixel(pixels, x, y) != BACKGROUND {
                let (left, top, right, bottom) = bounds.unwrap_or((x, y, x, y));
                bounds = Some((left.min(x), top.min(y), right.max(x), bottom.max(y)));
            }
        }
    }
    bounds
}
