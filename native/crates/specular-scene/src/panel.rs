//! What the Kit draws its panels with from here: the glyphs as SVG and the
//! colors a swatch or a tinted glyph paints.

mod icons;

use crate::Colors;

pub use self::icons::{arrow_cursor_svg, icon_svg};

/// The color a panel paints `color` in: the stored color resolved against
/// the hues of `palette` for how `role` uses it. The resolution the
/// swatches and tinted glyphs get.
pub fn panel_color(
    color: &specular_doc::Color,
    palette: specular_interact::Palette,
    role: specular_interact::PaintRole,
    appearance: specular_interact::Appearance,
) -> crate::Color {
    use crate::view::palette;
    let hues = match palette {
        specular_interact::Palette::Soft => palette::Palette::Soft,
        specular_interact::Palette::Vivid => palette::Palette::Vivid,
    };
    let role = match role {
        specular_interact::PaintRole::Fill => palette::Role::Fill,
        specular_interact::PaintRole::Ink => palette::Role::Ink,
    };
    palette::resolve(color, hues, role, Colors::of(appearance))
}
