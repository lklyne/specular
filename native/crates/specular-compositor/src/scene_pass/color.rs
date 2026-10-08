//! Scene colours to what the shaders take.

use specular_scene::Color;

/// `color` as linear RGBA with straight alpha, its alpha scaled by `opacity`.
pub(crate) fn linear(color: Color, opacity: f32) -> [f32; 4] {
    [
        srgb_to_linear(color.r),
        srgb_to_linear(color.g),
        srgb_to_linear(color.b),
        f32::from(color.a) / 255.0 * opacity.clamp(0.0, 1.0),
    ]
}

/// An absent colour: nothing is drawn.
pub(crate) const CLEAR: [f32; 4] = [0.0; 4];

fn srgb_to_linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}
