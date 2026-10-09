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
    /// A phone: a home indicator, and a notch when the screen is rounded.
    Phone,
    /// A tablet: a home indicator.
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
    /// Whether the page is on its side.
    pub landscape: bool,
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
    }
}

/// The shell of a page with a size of its own (`CUSTOM_SHELL_*`).
const CUSTOM_SHELL: DeviceShell = shell((12.0, 12.0, 12.0, 12.0), 10.0, 6.0, DeviceKind::Plain);
const EDGE_PHONE: DeviceShell = shell((22.0, 22.0, 26.0, 22.0), 77.0, 55.0, DeviceKind::Phone);
const TABLET: DeviceShell = shell((24.0, 24.0, 24.0, 24.0), 42.0, 18.0, DeviceKind::Tablet);
const COMPUTER: DeviceShell = shell((12.0, 12.0, 12.0, 12.0), 20.0, 8.0, DeviceKind::Plain);

/// The shell a framed page of `device_id` is drawn in, turned on its side
/// when `landscape`. No device is a custom size. A device the table does
/// not know gets the custom shell with no bezel, as in Electron.
pub fn device_shell(device_id: Option<&str>, landscape: bool) -> DeviceShell {
    let portrait = match device_id {
        None => CUSTOM_SHELL,
        Some("iphone-se") => shell((96.0, 28.0, 96.0, 28.0), 30.0, 0.0, DeviceKind::Phone),
        Some("iphone-14-pro" | "iphone-14-pro-max" | "iphone-duo-cover") => EDGE_PHONE,
        // The open Duo is a slab like a tablet but is listed with the phones.
        Some("iphone-duo-open") => DeviceShell {
            kind: DeviceKind::Phone,
            ..TABLET
        },
        Some("ipad-mini" | "ipad-pro-11" | "ipad-pro-129") => TABLET,
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
    };
    if !landscape {
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
