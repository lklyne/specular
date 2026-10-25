//! The viewport presets: the devices a page can be sized as. This table is
//! the list; nothing else owns it.
//!
//! A file names a page's preset twice, as `presetIndex` and as
//! `metadata.deviceId`. So a row never changes its place or its id, and a
//! new device goes at the end. What the size menu offers, and in what order,
//! is [`PRESET_MENU`], which leaves out a row that is kept only so old files
//! open.

use crate::{DeviceArt, DeviceFrameStyle};

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

/// Every preset, by index. A label may change with the device that has
/// that screen now; an index, an id and a size may not.
pub const VIEWPORT_PRESETS: [ViewportPreset; 21] = [
    row("iPhone SE", "iphone-se", 375.0, 667.0),
    row("iPhone 16", "iphone-14-pro", 393.0, 852.0),
    row("iPhone 16 Plus", "iphone-14-pro-max", 430.0, 932.0),
    row("iPad mini", "ipad-mini", 744.0, 1133.0),
    // Retired: the 11-inch iPad Pro before its screen grew.
    row("iPad Pro 11 (2018 to 2022)", "ipad-pro-11", 834.0, 1194.0),
    row("iPad Air 13", "ipad-pro-129", 1024.0, 1366.0),
    row("Laptop", "laptop", 1280.0, 800.0),
    row("Desktop", "desktop", 1440.0, 900.0),
    row("Desktop XL", "desktop-xl", 1920.0, 1080.0),
    row("iPhone Duo (cover)", "iphone-duo-cover", 466.0, 678.0),
    row("iPhone Duo (open)", "iphone-duo-open", 626.0, 890.0),
    row("iPhone 17 Pro Max", "iphone-17-pro-max", 440.0, 956.0),
    row("iPhone 17", "iphone-17", 402.0, 874.0),
    row("iPhone Air", "iphone-air", 420.0, 912.0),
    row("iPhone 16e", "iphone-16e", 390.0, 844.0),
    row("Pixel 10 Pro XL", "pixel-10-pro-xl", 448.0, 997.0),
    row("Pixel 10", "pixel-10", 412.0, 923.0),
    row("Galaxy S25", "galaxy-s25", 360.0, 780.0),
    row("iPad Pro 13", "ipad-pro-13", 1032.0, 1376.0),
    row("iPad Pro 11", "ipad-pro-11-m4", 834.0, 1210.0),
    row("iPad Air 11", "ipad-air-11", 820.0, 1180.0),
];

/// The size menu: its sections in order, each a name and the rows it
/// offers by index. A row in no section is retired.
pub const PRESET_MENU: [(&str, &[u32]); 3] = [
    ("Phones", &[11, 12, 13, 2, 1, 14, 0, 9, 10, 15, 16, 17]),
    ("Tablets", &[18, 19, 5, 20, 3]),
    ("Computers", &[6, 7, 8]),
];

/// The rows the size menu lists outright, the sizes most pages are tried
/// at. The rest of [`PRESET_MENU`] is behind the menu's "More" row.
pub const PRESET_MAIN: [u32; 9] = [11, 12, 0, 16, 18, 3, 6, 7, 8];

/// What a row was called before it was relabelled, for a caller that still
/// asks for it by that name. A name a current row has is not here.
const FORMER_LABELS: [(&str, u32); 3] = [
    ("iPhone 14 Pro", 1),
    ("iPhone 14 Pro Max", 2),
    ("iPad Pro 12.9", 5),
];

/// The preset at `index`, or `None` when the table has no such row.
pub fn preset(index: u64) -> Option<&'static ViewportPreset> {
    VIEWPORT_PRESETS.get(usize::try_from(index).ok()?)
}

/// The index of the preset `name` names: a row's label or its device id,
/// in any case, or a label a row used to have.
pub fn preset_named(name: &str) -> Option<u32> {
    let same = |other: &str| other.eq_ignore_ascii_case(name);
    let row = (VIEWPORT_PRESETS.iter())
        .position(|preset| same(preset.label) || same(preset.device_id))
        .and_then(|index| u32::try_from(index).ok());
    row.or_else(|| {
        (FORMER_LABELS.iter())
            .find(|(label, _)| same(label))
            .map(|&(_, index)| index)
    })
}

/// The space a device's shell takes around its screen, in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellInsets {
    /// Above the screen.
    pub top: f64,
    /// Right of the screen.
    pub right: f64,
    /// Below the screen.
    pub bottom: f64,
    /// Left of the screen.
    pub left: f64,
}

/// What kind of device a shell is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// A phone: a notch when the screen is rounded, and a home button
    /// when it is square.
    Phone,
    /// A tablet: a plain bezel with a tablet's proportions.
    Tablet,
    /// A laptop, a desktop or a custom size: a plain bezel.
    Plain,
}

/// The device frame around a page's screen: `DeviceDef` in the Electron
/// app's `src/shared/device-catalog.ts`, already turned for the page's
/// orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceShell {
    /// The bezel around the screen.
    pub insets: ShellInsets,
    /// The shell's outer corner radius.
    pub corner_radius: f64,
    /// The screen's corner radius.
    pub screen_corner_radius: f64,
    /// What the shell is decorated as.
    pub kind: DeviceKind,
    /// Whether the device is on its side: turned a quarter turn from how
    /// it stands.
    pub landscape: bool,
    /// The drawing the shell is shown as, or `None` for a clay shell.
    pub art: Option<DeviceArt>,
}

const fn shell(
    insets: (f64, f64, f64, f64),
    corner_radius: f64,
    screen_corner_radius: f64,
    kind: DeviceKind,
) -> DeviceShell {
    DeviceShell {
        insets: ShellInsets {
            top: insets.0,
            right: insets.1,
            bottom: insets.2,
            left: insets.3,
        },
        corner_radius,
        screen_corner_radius,
        kind,
        landscape: false,
        art: None,
    }
}

/// The shell of a page with a size of its own.
const CUSTOM_SHELL: DeviceShell = shell((12.0, 12.0, 12.0, 12.0), 10.0, 6.0, DeviceKind::Plain);
/// The folding phone closed, an edge-to-edge phone.
const DUO_COVER: DeviceShell = shell((22.0, 22.0, 26.0, 22.0), 77.0, 55.0, DeviceKind::Phone);
/// The folding phone open: a slab like a tablet, listed with the phones.
const DUO_OPEN: DeviceShell = shell((24.0, 24.0, 24.0, 24.0), 42.0, 18.0, DeviceKind::Phone);
const COMPUTER: DeviceShell = shell((12.0, 12.0, 12.0, 12.0), 20.0, 8.0, DeviceKind::Plain);

/// The shell a framed page of `device_id` is drawn in, turned on its side
/// when `landscape` is not how the device stands: a phone stands tall and a
/// laptop wide. No device is a custom size. A device the table does
/// not know gets the custom shell with no bezel.
///
/// A device with a drawing has one shell in both styles, its drawing's
/// body, so changing the style never moves or resizes it. Only a realistic
/// shell names the drawing.
pub fn device_shell(
    device_id: Option<&str>,
    landscape: bool,
    style: DeviceFrameStyle,
) -> DeviceShell {
    let portrait = match device_id.and_then(body) {
        Some(body) => DeviceShell {
            art: body.art.filter(|_| style == DeviceFrameStyle::Realistic),
            ..body
        },
        None => clay(device_id),
    };
    let stands_wide = (VIEWPORT_PRESETS.iter())
        .find(|preset| Some(preset.device_id) == device_id)
        .is_some_and(|preset| preset.width > preset.height);
    if landscape == stands_wide {
        return portrait;
    }
    let ShellInsets {
        top,
        right,
        bottom,
        left,
    } = portrait.insets;
    DeviceShell {
        insets: ShellInsets {
            top: left,
            right: top,
            bottom: right,
            left: bottom,
        },
        landscape: true,
        ..portrait
    }
}

/// The shell of `device_id` standing up as its drawing's body, if it has a
/// drawing. The drawing is scaled so its screen is the preset's viewport.
/// The insets are what the body leaves round the screen, to whole units so
/// the screen stays on whole pixels when the frame comes and goes. Sides
/// that are all but equal round to the same: all four when the bezel is
/// even all round, or else each opposite pair. Buttons stick out past the
/// shell.
fn body(device_id: &str) -> Option<DeviceShell> {
    let art = DeviceArt::of_device(device_id)?;
    let preset = (VIEWPORT_PRESETS.iter()).find(|preset| preset.device_id == device_id)?;
    let geometry = art.geometry();
    let scale = art.scale(preset.width);
    let left = geometry.screen_origin.0 - geometry.body_origin.0;
    let top = geometry.screen_origin.1 - geometry.body_origin.1;
    let right = geometry.body_size.0 - left - geometry.screen_size.0;
    let bottom = geometry.body_size.1 - top - geometry.screen_size.1;
    let [top, right, bottom, left] = [top, right, bottom, left].map(|inset| inset * scale);
    let even = rounded_together(&[top, right, bottom, left]);
    let across = even.or_else(|| rounded_together(&[left, right]));
    let down = even.or_else(|| rounded_together(&[top, bottom]));
    let [top, right, bottom, left] = [
        down.unwrap_or(top.round()),
        across.unwrap_or(right.round()),
        down.unwrap_or(bottom.round()),
        across.unwrap_or(left.round()),
    ];
    Some(DeviceShell {
        insets: ShellInsets {
            top,
            right,
            bottom,
            left,
        },
        corner_radius: geometry.body_corner_radius * scale,
        screen_corner_radius: geometry.screen_corner_radius * scale,
        kind: if art.is_computer() {
            DeviceKind::Plain
        } else if art.is_tablet() {
            DeviceKind::Tablet
        } else {
            DeviceKind::Phone
        },
        landscape: false,
        art: Some(art),
    })
}

/// The whole number `insets` share when they are within half a unit of
/// each other: their mean, rounded. Such sides are one bezel drawn evenly,
/// and rounded one by one they can come out a unit apart.
fn rounded_together(insets: &[f64]) -> Option<f64> {
    let least = insets.iter().copied().fold(f64::INFINITY, f64::min);
    let most = insets.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean = insets.iter().sum::<f64>() / insets.len() as f64;
    (most - least < 0.5).then(|| mean.round())
}

/// The shell of a device with no drawing, standing up.
fn clay(device_id: Option<&str>) -> DeviceShell {
    match device_id {
        None => CUSTOM_SHELL,
        Some("iphone-duo-cover") => DUO_COVER,
        Some("iphone-duo-open") => DUO_OPEN,
        Some("laptop" | "desktop" | "desktop-xl") => COMPUTER,
        Some(_) => DeviceShell {
            insets: ShellInsets {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            },
            ..CUSTOM_SHELL
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows files were written with before the list grew: their index,
    /// id and size.
    const LEGACY: [(&str, f64, f64); 11] = [
        ("iphone-se", 375.0, 667.0),
        ("iphone-14-pro", 393.0, 852.0),
        ("iphone-14-pro-max", 430.0, 932.0),
        ("ipad-mini", 744.0, 1133.0),
        ("ipad-pro-11", 834.0, 1194.0),
        ("ipad-pro-129", 1024.0, 1366.0),
        ("laptop", 1280.0, 800.0),
        ("desktop", 1440.0, 900.0),
        ("desktop-xl", 1920.0, 1080.0),
        ("iphone-duo-cover", 466.0, 678.0),
        ("iphone-duo-open", 626.0, 890.0),
    ];

    #[test]
    fn every_row_a_file_was_written_with_keeps_its_index_id_and_size() {
        for (index, legacy) in (0..).zip(LEGACY) {
            let row = preset(index).map(|row| (row.device_id, row.width, row.height));
            assert_eq!(row, Some(legacy), "preset {index}");
            assert_eq!(preset_named(legacy.0), u32::try_from(index).ok());
        }
        assert_eq!(
            preset(u64::from(LAPTOP)).map(|row| row.label),
            Some("Laptop")
        );
        assert_eq!(preset(VIEWPORT_PRESETS.len() as u64), None);
        // A name a row used to have still finds it.
        assert_eq!(preset_named("iPhone 14 Pro"), Some(1));
        assert_eq!(preset_named("iPad Pro 12.9"), Some(5));
        assert_eq!(preset_named("iPad Mini"), Some(3));
        assert_eq!(preset_named("Nokia 3310"), None);
    }
}
