//! Resolving a stored [`specular_doc::Color`] to the colour that is drawn.
//!
//! A stored colour names a slot, not a hue. The surface being painted picks
//! the [`Palette`]: the same preset is a muted pastel on a sticky note and a
//! saturated ink on a pen stroke (ADR 0013). The neutral slot depends on the
//! [`Role`] and on the theme. The soft pastels are the same in both themes
//! (`canvas-colors.ts`); the vivid hues, the neutral ink and the neutral fill
//! come from the [`Colors`] of the appearance being drawn.

use specular_doc::{Color as Stored, ColorPreset};

use crate::{Color, Colors, Hues, Item, Rect, Shade, ShadowDraw};

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

/// The shadow under a sticky note, a file card and a Document, which is
/// what lifts a pale card off the pale canvas: `0 2px 8px rgba(0, 0, 0,
/// 0.08)` in the light theme, in canvas units so it scales with the zoom.
const CARD_SHADOW_DROP: f32 = 2.0;
const CARD_SHADOW_BLUR: f32 = 8.0;

/// The shadow of a card at `rect`, to push before the card itself.
pub(crate) fn card_shadow(colors: &Colors, rect: Rect, corner_radius: f32) -> Item {
    Item::canvas(ShadowDraw {
        rect: Rect::new(rect.x, rect.y + CARD_SHADOW_DROP, rect.width, rect.height),
        corner_radius,
        blur: CARD_SHADOW_BLUR,
        color: Color::rgba(0, 0, 0, colors.card_shadow),
    })
}

/// The blue slot's stored value. JSON Canvas numbers six presets; blue is
/// this app's seventh.
const BLUE_PRESET: &str = "7";

/// The muted pastel of a hue slot, which fills stickies and shapes and
/// paints the highlighter in either theme.
const fn soft(preset: ColorPreset) -> Color {
    match preset {
        ColorPreset::Red => Color::rgb(0xe8, 0xb4, 0xb8),
        ColorPreset::Orange => Color::rgb(0xe8, 0xcc, 0xb0),
        ColorPreset::Yellow => Color::rgb(0xff, 0xe1, 0x8e),
        ColorPreset::Green => Color::rgb(0xb8, 0xd8, 0xc8),
        ColorPreset::Cyan => Color::rgb(0xb0, 0xd0, 0xd8),
        ColorPreset::Purple => Color::rgb(0xc8, 0xb8, 0xd8),
    }
}

const SOFT_BLUE: Color = Color::rgb(0xb0, 0xc4, 0xd8);

/// The saturated ink of a hue slot in `hues`.
const fn vivid(hues: &Hues, preset: ColorPreset) -> Color {
    match preset {
        ColorPreset::Red => hues.red,
        ColorPreset::Orange => hues.orange,
        ColorPreset::Yellow => hues.yellow,
        ColorPreset::Green => hues.green,
        ColorPreset::Cyan => hues.cyan,
        ColorPreset::Purple => hues.purple,
    }
}

/// The colour `stored` paints as on a surface of `palette`, used as `role`,
/// in the theme `colors` is. A literal `#RRGGBB` passes through; anything
/// unreadable is the neutral.
pub(crate) fn resolve(stored: &Stored, palette: Palette, role: Role, colors: &Colors) -> Color {
    let neutral = match role {
        Role::Fill => colors.neutral_fill,
        Role::Ink => colors.ink,
    };
    match stored {
        Stored::Neutral => neutral,
        Stored::Preset(preset) => match palette {
            Palette::Soft => soft(*preset),
            Palette::Vivid => vivid(&colors.hues, *preset),
        },
        Stored::Custom(value) if value == BLUE_PRESET => match palette {
            Palette::Soft => SOFT_BLUE,
            Palette::Vivid => colors.hues.blue,
        },
        Stored::Custom(value) => parse_hex(value).unwrap_or(neutral),
    }
}

/// [`resolve`] for an optional colour, where absent means the neutral.
pub(crate) fn resolve_or_neutral(
    stored: Option<&Stored>,
    palette: Palette,
    role: Role,
    colors: &Colors,
) -> Color {
    resolve(stored.unwrap_or(&Stored::Neutral), palette, role, colors)
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

/// `color` derived by `shade`.
pub(crate) fn shaded(color: Color, shade: Shade) -> Color {
    match shade {
        Shade::Lighten(amount) => lighten(color, amount),
        Shade::Darken(amount) => darken(color, amount),
        Shade::Keep => color,
    }
}

/// `color` at `alpha`, 0 to 1.
pub(crate) fn with_alpha(color: Color, alpha: f32) -> Color {
    color.with_alpha((alpha.clamp(0.0, 1.0) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use specular_interact::Appearance;

    use super::*;

    #[test]
    fn a_preset_is_a_pastel_on_a_fill_and_an_ink_on_a_stroke() {
        let yellow = Stored::Preset(ColorPreset::Yellow);
        let light = Colors::of(Appearance::Light);
        assert_eq!(
            (
                resolve(&yellow, Palette::Soft, Role::Fill, light),
                resolve(&yellow, Palette::Vivid, Role::Ink, light)
            ),
            (Color::rgb(0xff, 0xe1, 0x8e), Color::rgb(0x85, 0x5c, 0x00))
        );
    }

    #[test]
    fn a_hex_passes_through_and_nonsense_is_neutral() {
        let light = Colors::of(Appearance::Light);
        let ink = |stored: &str| resolve(&Stored::parse(stored), Palette::Vivid, Role::Ink, light);
        assert_eq!(
            [
                ink("#ff00aa"),
                ink("#f0a"),
                ink("tomato"),
                ink("#12345"),
                ink("ff00aa")
            ],
            [
                Color::rgb(0xff, 0x00, 0xaa),
                Color::rgb(0xff, 0x00, 0xaa),
                light.ink,
                light.ink,
                light.ink
            ]
        );
    }
}
