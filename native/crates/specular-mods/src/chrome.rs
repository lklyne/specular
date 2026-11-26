//! The chrome's colours as a mod reads them (`colors()` in the `specular`
//! module): the panel colours the canvas and the Kit window draw with,
//! light or dark as the `theme` key says, as `#rrggbbaa` strings.
//!
//! The Kit's theme carries only GPUI Kit's generic tokens (`background`,
//! `primary`, ...); the toolbar's own strip, its text and the raised face of
//! the control that is on have no token there, so a mod that means to look
//! like the app's own panels reads them here.

use serde_json::{Map, Value};
use specular_interact::Appearance;
use specular_scene::{Color, Colors};

/// `color` as a script writes a colour literal.
fn hex(color: Color) -> String {
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        color.r, color.g, color.b, color.a
    )
}

fn packed(rgba: u32) -> String {
    format!("#{rgba:08x}")
}

/// The appearance a `theme` key's value names; light when it names none.
pub(crate) fn appearance_of(theme: Option<&Value>) -> Appearance {
    match theme
        .and_then(|theme| theme.get("appearance"))
        .and_then(Value::as_str)
    {
        Some("dark") => Appearance::Dark,
        _ => Appearance::Light,
    }
}

/// The raised face of the control that is on, which only the Kit draws:
/// the same values as `specular-shell/src/kit/theme.rs` (`raised_fill`,
/// `raised_rim`, `raised_shadow`), light then dark.
const RAISED: [(&str, u32, u32); 5] = [
    ("raisedFill", 0xede9_e7ff, 0x5450_4bff),
    ("raisedRim", 0x0000_0014, 0xffff_ff26),
    ("raisedShadowNear", 0x0000_001a, 0x0000_0040),
    ("raisedShadowFar", 0x0000_000d, 0x0000_0026),
    ("tabHover", 0xdfdc_daff, 0x4b47_42ff),
];

/// Every colour a mod may draw the chrome with, by name.
pub fn chrome_colors(appearance: Appearance) -> Value {
    let panel = &Colors::of(appearance).panel;
    let named = [
        ("toolbar", panel.toolbar),
        ("toolbarBorder", panel.toolbar_border),
        ("toolbarText", panel.toolbar_text),
        ("toolbarTextStrong", panel.toolbar_text_strong),
        ("popup", panel.popup),
        ("popupBorder", panel.popup_border),
        ("text", panel.text),
        ("textMuted", panel.text_muted),
        ("hover", panel.hover),
        ("on", panel.on),
        ("divider", panel.divider),
        ("keyText", panel.key_text),
        ("sidebar", panel.sidebar),
        ("sidebarRule", panel.sidebar_rule),
        ("interactive", panel.interactive),
        ("interactiveHover", panel.interactive_hover),
    ];
    let mut colors: Map<String, Value> = (named.into_iter())
        .map(|(name, color)| (name.to_owned(), Value::String(hex(color))))
        .collect();
    for (name, light, dark) in RAISED {
        let rgba = match appearance {
            Appearance::Light => light,
            Appearance::Dark => dark,
        };
        colors.insert(name.to_owned(), Value::String(packed(rgba)));
    }
    Value::Object(colors)
}
