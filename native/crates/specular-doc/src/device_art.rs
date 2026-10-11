//! The realistic device frames: which devices have art, and where the
//! screen sits in each drawing.
//!
//! The drawings are the SVG files under `specular-app/assets/device-frames`,
//! converted from the Flutter `device_frame` package. [`DeviceArt`] and its
//! numbers are in `device_art/table.rs`, which
//! `native/scripts/device-art-table.py` writes from that folder's
//! `manifest.json`. A test in `specular-app` holds the two together.

mod table;

pub use table::DeviceArt;

/// How a framed page's device is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceFrameStyle {
    /// A flat bezel. A device with a drawing gets its body's outline, and
    /// every other device the outline of its kind.
    Clay,
    /// The device's own drawing, for the devices that have one. A device
    /// without one is drawn in clay.
    Realistic,
}

impl DeviceFrameStyle {
    /// The style as `metadata.deviceFrameStyle` names it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clay => "clay",
            Self::Realistic => "realistic",
        }
    }

    /// The style `metadata.deviceFrameStyle` names. No name, and one this
    /// build does not know, is clay.
    pub fn from_name(name: Option<&str>) -> Self {
        match name {
            Some("realistic") => Self::Realistic,
            _ => Self::Clay,
        }
    }
}

/// The colour a framed page's device is drawn in, in either style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeviceFrameColor {
    /// A light device: a pale clay bezel, or silver and white.
    Light,
    /// A dark device: a dark clay bezel, or graphite and black.
    Dark,
}

impl DeviceFrameColor {
    /// The colour as `metadata.deviceFrameColor` names it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// The colour `metadata.deviceFrameColor` names. No name, and one this
    /// build does not know, is none: the device follows the app's theme.
    pub fn from_name(name: Option<&str>) -> Option<Self> {
        match name {
            Some("light") => Some(Self::Light),
            Some("dark") => Some(Self::Dark),
            _ => None,
        }
    }

    /// The other colour.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

/// Where the screen sits in a device's drawing, in the drawing's own units
/// with the device standing up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtGeometry {
    /// The drawing's width and height.
    pub frame: (f64, f64),
    /// The screen's left and top edge.
    pub screen_origin: (f64, f64),
    /// The screen's width and height.
    pub screen_size: (f64, f64),
    /// The screen's corner radius.
    pub screen_corner_radius: f64,
    /// The left and top edge of the body: the device's main outline,
    /// without the buttons that stick out of it.
    pub body_origin: (f64, f64),
    /// The body's width and height.
    pub body_size: (f64, f64),
    /// The body's corner radius.
    pub body_corner_radius: f64,
}

impl DeviceArt {
    /// The drawing and colour [`asset`](Self::asset) names. A device with no
    /// light drawing is named dark.
    pub fn from_asset(asset: &str) -> Option<(Self, DeviceFrameColor)> {
        Self::ALL.into_iter().find_map(|art| {
            [DeviceFrameColor::Dark, DeviceFrameColor::Light]
                .into_iter()
                .find(|color| art.asset(*color) == asset)
                .map(|color| (art, color))
        })
    }

    /// How many canvas units one unit of the drawing is, for a screen
    /// `screen_width` canvas units wide standing up.
    pub fn scale(self, screen_width: f64) -> f64 {
        screen_width / self.geometry().screen_size.0
    }

    /// The drawing's width and height in canvas units round the widest
    /// preset viewport it is drawn at.
    pub fn size(self) -> (f64, f64) {
        let width = (crate::VIEWPORT_PRESETS.iter())
            .filter(|preset| Self::of_device(preset.device_id) == Some(self))
            .map(|preset| preset.width)
            .fold(0.0, f64::max);
        let (scale, frame) = (self.scale(width), self.geometry().frame);
        (frame.0 * scale, frame.1 * scale)
    }
}
