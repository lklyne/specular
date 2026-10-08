//! The viewport presets a page's `presetIndex` names (the table lives in
//! `specular-doc`), and the orientation an API patch turns them to.

pub(crate) use specular_doc::LAPTOP;

/// The size of preset `index`, or `None` when the table has no such row.
pub(crate) fn size(index: u64) -> Option<(f64, f64)> {
    specular_doc::preset(index).map(|preset| (preset.width, preset.height))
}

/// `size` turned to `orientation`: `landscape` puts the long side across,
/// `portrait` the short side. Anything else leaves it as the table has it.
pub(crate) fn oriented(size: (f64, f64), orientation: Option<&str>) -> (f64, f64) {
    let (long, short) = (size.0.max(size.1), size.0.min(size.1));
    match orientation {
        Some("landscape") => (long, short),
        Some("portrait") => (short, long),
        _ => size,
    }
}
