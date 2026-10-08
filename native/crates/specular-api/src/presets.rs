//! The viewport presets a page's `presetIndex` names: the table in the
//! Electron app's `src/shared/device-catalog.ts`.

/// The preset `add page` uses when none is given: Laptop.
pub(crate) const LAPTOP: u32 = 6;

/// Width and height in CSS pixels, portrait or default orientation, by
/// preset index.
const PRESETS: [(f64, f64); 11] = [
    (375.0, 667.0),   // iPhone SE
    (393.0, 852.0),   // iPhone 14 Pro
    (430.0, 932.0),   // iPhone 14 Pro Max
    (744.0, 1133.0),  // iPad Mini
    (834.0, 1194.0),  // iPad Pro 11
    (1024.0, 1366.0), // iPad Pro 12.9
    (1280.0, 800.0),  // Laptop
    (1440.0, 900.0),  // Desktop
    (1920.0, 1080.0), // Desktop XL
    (466.0, 678.0),   // iPhone Duo (cover)
    (626.0, 890.0),   // iPhone Duo (open)
];

/// The size of preset `index`, or `None` when the table has no such row.
pub(crate) fn size(index: u64) -> Option<(f64, f64)> {
    PRESETS.get(usize::try_from(index).ok()?).copied()
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
