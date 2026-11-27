//! The layout's look in the window (ADR 0059): the colours a layout or its
//! theme renames, the chrome's font and text size, and colours named in a
//! layout's paint (`"background": "sidebar"`).
//!
//! The overrides are held for the thread, as the appearance is in
//! [`theme`](crate::theme): every colour function there asks [`over`]
//! first, so the app's own panels draw in the layout's colours too.

use std::cell::RefCell;
use std::collections::BTreeMap;

use specular_interact::{Look, Resolved};

/// What the window draws with now.
#[derive(Debug, Clone, PartialEq, Default)]
struct Held {
    colors: BTreeMap<String, u32>,
    font: Option<String>,
    font_size: Option<f32>,
}

thread_local! {
    static HELD: RefCell<Held> = RefCell::new(Held::default());
}

/// `#rrggbb` or `#rrggbbaa` as `0xRRGGBBAA`.
pub(crate) fn hex(text: &str) -> Option<u32> {
    let digits = text.strip_prefix('#')?;
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        6 => Some(value << 8 | 0xff),
        8 => Some(value),
        _ => None,
    }
}

/// Puts the look `resolved` and `look` in place. Returns whether it
/// changed, so the Kit's theme is applied again.
pub(crate) fn set(look: &Look, resolved: &Resolved) -> bool {
    let held = Held {
        colors: (resolved.colors.iter())
            .filter_map(|(name, value)| Some((name.clone(), hex(value)?)))
            .collect(),
        font: look.font.clone(),
        font_size: look.font_size,
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

/// A colour a layout writes: `#rrggbb[aa]`, or a colour's name as a mod's
/// `colors()` has it (`sidebar`, `toolbarBorder`, `accent`).
pub(crate) fn color(text: &str) -> Option<u32> {
    hex(text)
        .or_else(|| over(text))
        .or_else(|| crate::theme::by_name(text))
}
