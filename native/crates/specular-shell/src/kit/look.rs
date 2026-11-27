//! The layout's look in the window (ADR 0059): the colours a layout or its
//! theme renames, the chrome's font and text size, and colours named in a
//! layout's paint (`"background": "sidebar"`).
//!
//! The overrides are held for the thread, as the appearance is in
//! [`theme`](crate::theme): every colour function there asks [`over`]
//! first, so the app's own panels draw in the layout's colours too.

use std::cell::RefCell;
use std::collections::BTreeMap;

use specular_interact::Resolved;

/// What the window draws with now.
#[derive(Debug, Clone, PartialEq, Default)]
struct Held {
    colors: BTreeMap<String, u32>,
    font: Option<String>,
    font_size: Option<f32>,
    radius: Option<f32>,
}

thread_local! {
    static HELD: RefCell<Held> = RefCell::new(Held::default());
}

/// `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` as `0xRRGGBBAA`.
pub(crate) fn hex(text: &str) -> Option<u32> {
    let digits = text.strip_prefix('#')?;
    if !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    // A short form doubles each digit: `#0af` is `#00aaff`.
    let long: String = match digits.len() {
        3 | 4 => digits.chars().flat_map(|digit| [digit, digit]).collect(),
        6 | 8 => digits.to_owned(),
        _ => return None,
    };
    let value = u32::from_str_radix(&long, 16).ok()?;
    Some(if long.len() == 6 {
        value << 8 | 0xff
    } else {
        value
    })
}

/// Puts the look `resolved` in place. Returns whether it
/// changed, so the Kit's theme is applied again.
pub(crate) fn set(resolved: &Resolved) -> bool {
    let held = Held {
        colors: (resolved.colors.iter())
            .filter_map(|(name, value)| Some((name.clone(), hex(value)?)))
            .collect(),
        font: resolved.font.clone(),
        font_size: resolved.font_size,
        radius: resolved.radius,
    };
    HELD.with_borrow_mut(|now| {
        if *now == held {
            return false;
        }
        *now = held;
        true
    })
}

/// The layout's colour for `name`, when it renames it.
pub(crate) fn over(name: &str) -> Option<u32> {
    HELD.with_borrow(|held| held.colors.get(name).copied())
}

/// The chrome's font family, when the layout names one.
pub(crate) fn font() -> Option<String> {
    HELD.with_borrow(|held| held.font.clone())
}

/// The chrome's text size, 12 points unless the layout says.
pub(crate) fn font_size() -> f32 {
    HELD.with_borrow(|held| held.font_size).unwrap_or(12.0)
}

/// The corner radius of the Kit's controls and popups, 6 points unless the
/// layout says.
pub(crate) fn radius() -> f32 {
    HELD.with_borrow(|held| held.radius).unwrap_or(6.0).max(0.0)
}

/// The corner radius of panels that float over the window, such as dialogs:
/// [`radius`] scaled as the Kit scales its large radius.
pub(crate) fn radius_large() -> f32 {
    radius() * 10.0 / 6.0
}

/// A colour a layout writes: `#rrggbb[aa]`, or a colour's name as a mod's
/// `colors()` has it (`sidebar`, `toolbarBorder`, `accent`).
pub(crate) fn color(text: &str) -> Option<u32> {
    hex(text)
        .or_else(|| over(text))
        .or_else(|| crate::theme::by_name(text))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_colour_reads_in_its_short_and_long_forms() {
        assert_eq!(super::hex("#000"), Some(0x0000_00ff));
        assert_eq!(super::hex("#0af8"), Some(0x00aa_ff88));
        assert_eq!(super::hex("#00aaff"), Some(0x00aa_ffff));
        assert_eq!(super::hex("#00aaff80"), Some(0x00aa_ff80));
        assert_eq!(super::hex("#00aaf"), None);
        assert_eq!(super::hex("#00g"), None);
    }
}
