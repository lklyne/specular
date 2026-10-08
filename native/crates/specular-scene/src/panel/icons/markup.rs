//! A glyph as SVG text, for a UI library that draws the panel models with
//! components of its own and needs the same glyphs the built-in panels use.

use std::fmt::Write as _;

use specular_interact::{Appearance, Icon};

use super::{Glyph, Inks, Layer, Paint, Shape, Turn, glyph, matrix, paint};
use crate::{Color, Colors};

fn attribute(out: &mut String, name: &str, color: Option<Color>) {
    match color {
        None => {
            let _ = write!(out, " {name}=\"none\"");
        }
        Some(color) => {
            let _ = write!(
                out,
                " {name}=\"#{:02x}{:02x}{:02x}\"",
                color.r, color.g, color.b
            );
            if color.a < 255 {
                let _ = write!(out, " {name}-opacity=\"{:.3}\"", f32::from(color.a) / 255.0);
            }
        }
    }
}

fn layer(out: &mut String, layer: &Layer, inks: Inks) {
    match layer.shape {
        Shape::Path(d) => {
            let _ = write!(out, "<path d=\"{d}\"");
        }
        Shape::Rect(x, y, width, height, radius) => {
            let _ = write!(
                out,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"{radius}\""
            );
        }
        Shape::Disc(cx, cy, radius) => {
            let _ = write!(out, "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{radius}\"");
        }
    }
    attribute(out, "fill", paint(layer.fill, inks));
    if layer.stroke != Paint::None {
        attribute(out, "stroke", paint(layer.stroke, inks));
        let _ = write!(
            out,
            " stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
            layer.width
        );
    }
    if layer.turn != Turn::None {
        let terms: Vec<String> = matrix(layer.turn).iter().map(f32::to_string).collect();
        let _ = write!(out, " transform=\"matrix({})\"", terms.join(" "));
    }
    out.push_str("/>");
}

fn markup(glyph: Glyph, inks: Inks) -> String {
    let [x, y, width, height] = glyph.view;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{x} {y} {width} {height}\" \
         width=\"{width}\" height=\"{height}\">"
    );
    // The view box already cuts a glyph off at its frame, which is all
    // `clipped` asks for.
    for one in glyph.layers {
        layer(&mut out, one, inks);
    }
    out.push_str("</svg>");
    out
}

/// `icon` as an SVG document, drawn as the built-in panels draw it:
/// `current` is the text color of the control it is on, `tint` the color
/// the glyph shows when it shows one, `on` whether the control is on and
/// `appearance` the theme it is drawn for.
pub fn icon_svg(
    icon: Icon,
    current: Color,
    tint: Option<Color>,
    on: bool,
    appearance: Appearance,
) -> String {
    let colors = Colors::of(appearance);
    markup(
        glyph(icon),
        Inks {
            current,
            tint,
            on,
            colors,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lucide_glyph_is_stroked_in_the_current_color() {
        let svg = icon_svg(
            Icon::Trash,
            Color::rgb(0x30, 0x30, 0x30),
            None,
            false,
            Appearance::Light,
        );
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\""));
        assert!(svg.contains("stroke=\"#303030\""), "{svg}");
        assert!(svg.ends_with("</svg>"));
    }
}
