//! The models' glyphs as GPUI images. The path data is the built-in
//! renderer's own, so both renderers draw one set of glyphs.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::{Image, ImageFormat, IntoElement, Styled as _, img, px};
use specular_interact::{Icon, PaintRole, Palette};
use specular_scene::{Color, icon_svg, panel_color};

type Key = (Icon, Color, Option<Color>, bool);

thread_local! {
    /// One image per glyph and colouring: GPUI caches a decoded image by
    /// its identity, so the same `Arc` must come back each frame.
    static IMAGES: RefCell<HashMap<Key, Arc<Image>>> = RefCell::new(HashMap::new());
}

/// A stored colour as the built-in panels would paint it.
pub(super) fn resolved(color: &specular_doc::Color, palette: Palette, role: PaintRole) -> Color {
    panel_color(color, palette, role)
}

/// `0xRRGGBB` as a scene colour.
pub(super) const fn ink(hex: u32) -> Color {
    let [_, r, g, b] = hex.to_be_bytes();
    Color::rgb(r, g, b)
}

/// `icon` at `size` pixels square. `current` is the text colour of the
/// control it is on, `tint` the colour it shows when it shows one.
pub(super) fn glyph(
    icon: Icon,
    current: Color,
    tint: Option<Color>,
    on: bool,
    size: f32,
) -> impl IntoElement {
    let image = IMAGES.with(|images| {
        let mut images = images.borrow_mut();
        let image = images.entry((icon, current, tint, on)).or_insert_with(|| {
            let svg = icon_svg(icon, current, tint, on);
            Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
        });
        Arc::clone(image)
    });
    img(image).size(px(size)).flex_shrink_0()
}
