//! The viewport presets a page's `preset_index` names: the table in the
//! Electron app's `src/shared/device-catalog.ts`.

/// One row of the table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportPreset {
    /// The name the size menu shows.
    pub label: &'static str,
    /// The device a page at this preset shows, as `metadata.deviceId`.
    pub device_id: &'static str,
    /// Width in CSS pixels, in the preset's natural orientation.
    pub width: f64,
    /// Height in CSS pixels, in the preset's natural orientation.
    pub height: f64,
}

/// The preset a new page gets when none is asked for: Laptop.
pub const LAPTOP: u32 = 6;

const fn row(
    label: &'static str,
    device_id: &'static str,
    width: f64,
    height: f64,
) -> ViewportPreset {
    ViewportPreset {
        label,
        device_id,
        width,
        height,
    }
}

/// Every preset, by index.
pub const VIEWPORT_PRESETS: [ViewportPreset; 11] = [
    row("iPhone SE", "iphone-se", 375.0, 667.0),
    row("iPhone 14 Pro", "iphone-14-pro", 393.0, 852.0),
    row("iPhone 14 Pro Max", "iphone-14-pro-max", 430.0, 932.0),
    row("iPad Mini", "ipad-mini", 744.0, 1133.0),
    row("iPad Pro 11", "ipad-pro-11", 834.0, 1194.0),
    row("iPad Pro 12.9", "ipad-pro-129", 1024.0, 1366.0),
    row("Laptop", "laptop", 1280.0, 800.0),
    row("Desktop", "desktop", 1440.0, 900.0),
    row("Desktop XL", "desktop-xl", 1920.0, 1080.0),
    row("iPhone Duo (cover)", "iphone-duo-cover", 466.0, 678.0),
    row("iPhone Duo (open)", "iphone-duo-open", 626.0, 890.0),
];

/// The preset at `index`, or `None` when the table has no such row.
pub fn preset(index: u64) -> Option<&'static ViewportPreset> {
    VIEWPORT_PRESETS.get(usize::try_from(index).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_indexed_like_the_electron_catalog() {
        let devices = [
            "iphone-se",
            "iphone-14-pro",
            "iphone-14-pro-max",
            "ipad-mini",
            "ipad-pro-11",
            "ipad-pro-129",
            "laptop",
            "desktop",
            "desktop-xl",
            "iphone-duo-cover",
            "iphone-duo-open",
        ];
        for (index, device) in (0..).zip(devices) {
            assert_eq!(
                preset(index).map(|row| row.device_id),
                Some(device),
                "preset {index}"
            );
        }
        assert_eq!(
            preset(u64::from(LAPTOP)).map(|row| row.label),
            Some("Laptop")
        );
        assert_eq!(preset(devices.len() as u64), None);
    }
}
