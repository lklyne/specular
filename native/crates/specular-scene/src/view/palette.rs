//! Resolving a stored [`specular_doc::Color`] to the colour that is drawn,
//! and the fixed colours of the chrome.
//!
//! A stored colour names a slot, not a hue. The surface being painted picks
//! the [`Palette`]: the same preset is a muted pastel on a sticky note and a
//! saturated ink on a pen stroke (ADR 0013). The neutral slot depends on the
//! [`Role`] as well. Everything here is the light theme.

use specular_doc::{Color as Stored, ColorPreset};

use crate::Color;

/// Which hue set a surface paints in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Palette {
    /// Muted pastels: sticky notes, shapes, the highlighter.
    Soft,
    /// Saturated inks: plain text, edges, the pen, groups.
    Vivid,
}

/// How a colour is used, which decides what the neutral slot means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// A background: the neutral is a warm off-white.
    Fill,
    /// Glyphs and strokes: the neutral is near-black.
    Ink,
}

/// The neutral ink: default text, and text on a sticky note.
pub(crate) const INK: Color = Color::rgb(0x1c, 0x19, 0x17);
/// The neutral fill: an uncoloured sticky note or shape.
pub(crate) const NEUTRAL_FILL: Color = Color::rgb(0xfd, 0xf8, 0xf5);

/// The accent blue of selection outlines, handles and the marquee.
pub(crate) const SELECTION: Color = Color::rgb(0x3b, 0x82, 0xf6);
/// A page's resting border.
pub(crate) const PAGE_BORDER: Color = Color::rgb(0xa6, 0xa0, 0x9b);
/// Secondary chrome text: page titles, group labels, file names.
pub(crate) const MUTED_TEXT: Color = Color::rgb(0x6b, 0x6b, 0x6b);
/// The inside of a card that stands in for content: a file.
pub(crate) const CARD: Color = Color::rgb(0xfa, 0xfa, 0xf9);

/// The blue slot's stored value. JSON Canvas numbers six presets; blue is
/// this app's seventh.
const BLUE_PRESET: &str = "7";

/// A hue slot's two colours.
struct Hue {
    soft: Color,
    vivid: Color,
}

/// The light-theme hues. The vivid column is each saturated hue with its
/// OKLCH lightness pinned to 0.5, which is what the light canvas paints.
const fn hue(preset: ColorPreset) -> Hue {
    match preset {
        ColorPreset::Red => Hue {
            soft: Color::rgb(0xe8, 0xb4, 0xb8),
            vivid: Color::rgb(0xcd, 0x00, 0x00),
        },
        ColorPreset::Orange => Hue {
            soft: Color::rgb(0xe8, 0xcc, 0xb0),
            vivid: Color::rgb(0xa8, 0x3d, 0x00),
        },
        ColorPreset::Yellow => Hue {
            soft: Color::rgb(0xff, 0xe1, 0x8e),
            vivid: Color::rgb(0x85, 0x5c, 0x00),
        },
        ColorPreset::Green => Hue {
            soft: Color::rgb(0xb8, 0xd8, 0xc8),
            vivid: Color::rgb(0x00, 0x80, 0x00),
        },
        ColorPreset::Cyan => Hue {
            soft: Color::rgb(0xb0, 0xd0, 0xd8),
            vivid: Color::rgb(0x00, 0x71, 0xa2),
        },
        ColorPreset::Purple => Hue {
            soft: Color::rgb(0xc8, 0xb8, 0xd8),
            vivid: Color::rgb(0x94, 0x14, 0xb9),
        },
    }
}

const BLUE: Hue = Hue {
    soft: Color::rgb(0xb0, 0xc4, 0xd8),
    vivid: Color::rgb(0x00, 0x5c, 0xd4),
};

/// The colour `stored` paints as on a surface of `palette`, used as `role`.
/// A literal `#RRGGBB` passes through; anything unreadable is the neutral.
pub(crate) fn resolve(stored: &Stored, palette: Palette, role: Role) -> Color {
    let neutral = match role {
        Role::Fill => NEUTRAL_FILL,
        Role::Ink => INK,
    };
    let pick = |hue: Hue| match palette {
        Palette::Soft => hue.soft,
        Palette::Vivid => hue.vivid,
    };
    match stored {
        Stored::Neutral => neutral,
        Stored::Preset(preset) => pick(hue(*preset)),
        Stored::Custom(value) if value == BLUE_PRESET => pick(BLUE),
        Stored::Custom(value) => parse_hex(value).unwrap_or(neutral),
    }
}

/// [`resolve`] for an optional colour, where absent means the neutral.
pub(crate) fn resolve_or_neutral(stored: Option<&Stored>, palette: Palette, role: Role) -> Color {
    resolve(stored.unwrap_or(&Stored::Neutral), palette, role)
}

/// `#RRGGBB` or `#RGB` as a colour.
fn parse_hex(value: &str) -> Option<Color> {
    let digits = value.strip_prefix('#')?;
    let channel = |at: usize, len: usize| {
        let part = digits.get(at..at + len)?;
        let value = u8::from_str_radix(part, 16).ok()?;
        Some(if len == 1 { value * 17 } else { value })
    };
    match digits.len() {
        6 => Some(Color::rgb(channel(0, 2)?, channel(2, 2)?, channel(4, 2)?)),
        3 => Some(Color::rgb(channel(0, 1)?, channel(1, 1)?, channel(2, 1)?)),
        _ => None,
    }
}

/// `color` moved `amount` of the way to white.
pub(crate) fn lighten(color: Color, amount: f32) -> Color {
    let mix = |channel: u8| {
        let channel = f32::from(channel);
        (channel + (255.0 - channel) * amount).round() as u8
    };
    Color::rgba(mix(color.r), mix(color.g), mix(color.b), color.a)
}

/// `color` moved `amount` of the way to black.
pub(crate) fn darken(color: Color, amount: f32) -> Color {
    let mix = |channel: u8| (f32::from(channel) * (1.0 - amount)).round() as u8;
    Color::rgba(mix(color.r), mix(color.g), mix(color.b), color.a)
}

/// `color` at `alpha`, 0 to 1.
pub(crate) fn with_alpha(color: Color, alpha: f32) -> Color {
    color.with_alpha((alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preset_is_a_pastel_on_a_fill_and_an_ink_on_a_stroke() {
        let yellow = Stored::Preset(ColorPreset::Yellow);
        assert_eq!(
            (
                resolve(&yellow, Palette::Soft, Role::Fill),
                resolve(&yellow, Palette::Vivid, Role::Ink)
            ),
            (Color::rgb(0xff, 0xe1, 0x8e), Color::rgb(0x85, 0x5c, 0x00))
        );
    }

    #[test]
    fn neutral_follows_the_role() {
        assert_eq!(
            (
                resolve(&Stored::Neutral, Palette::Soft, Role::Fill),
                resolve(&Stored::Neutral, Palette::Soft, Role::Ink)
            ),
            (NEUTRAL_FILL, INK)
        );
    }

    #[test]
    fn the_seventh_preset_is_blue() {
        assert_eq!(
            resolve(&Stored::parse("7"), Palette::Soft, Role::Fill),
            Color::rgb(0xb0, 0xc4, 0xd8)
        );
    }

    #[test]
    fn a_hex_passes_through_and_nonsense_is_neutral() {
        assert_eq!(
            [
                resolve(&Stored::parse("#ff00aa"), Palette::Vivid, Role::Ink),
                resolve(&Stored::parse("#f0a"), Palette::Vivid, Role::Ink),
                resolve(&Stored::parse("tomato"), Palette::Vivid, Role::Ink),
            ],
            [
                Color::rgb(0xff, 0x00, 0xaa),
                Color::rgb(0xff, 0x00, 0xaa),
                INK
            ]
        );
    }

    #[test]
    fn lighten_and_darken_interpolate_each_channel() {
        let colour = Color::rgb(100, 200, 0);
        assert_eq!(
            (lighten(colour, 0.5), darken(colour, 0.5)),
            (Color::rgb(178, 228, 128), Color::rgb(50, 100, 0))
        );
    }
}
