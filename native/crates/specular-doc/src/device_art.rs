//! The realistic device frames: which devices have art, and where the
//! screen sits in each drawing.
//!
//! The drawings are the SVG files under `specular-app/assets/device-frames`,
//! converted from the Flutter `device_frame` package. The numbers here are
//! copied by hand from that folder's `manifest.json`, and a test in
//! `specular-app` holds the two together.

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

/// A device that has a realistic drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DeviceArt {
    /// iPhone SE.
    IphoneSe,
    /// iPhone 15 Pro, which the iPhone 14 Pro preset is drawn as.
    Iphone15Pro,
    /// iPhone 15 Pro Max, which the iPhone 14 Pro Max preset is drawn as.
    Iphone15ProMax,
    /// iPad Pro 11.
    IpadPro11,
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

const fn geometry(
    frame: (f64, f64),
    screen: (f64, f64, f64, f64),
    screen_corner_radius: f64,
    body: (f64, f64, f64, f64),
    body_corner_radius: f64,
) -> ArtGeometry {
    ArtGeometry {
        frame,
        screen_origin: (screen.0, screen.1),
        screen_size: (screen.2, screen.3),
        screen_corner_radius,
        body_origin: (body.0, body.1),
        body_size: (body.2, body.3),
        body_corner_radius,
    }
}

impl DeviceArt {
    /// Every device with a drawing.
    pub const ALL: [Self; 4] = [
        Self::IphoneSe,
        Self::Iphone15Pro,
        Self::Iphone15ProMax,
        Self::IpadPro11,
    ];

    /// The drawing a page of `device_id` is shown in.
    pub fn of_device(device_id: &str) -> Option<Self> {
        (Self::ALL.into_iter()).find(|art| art.device_id() == device_id)
    }

    /// The preset device drawn as this. The presets keep their names and
    /// viewports; the two edge-to-edge phones are drawn as the newer body
    /// with the same screen.
    pub const fn device_id(self) -> &'static str {
        match self {
            Self::IphoneSe => "iphone-se",
            Self::Iphone15Pro => "iphone-14-pro",
            Self::Iphone15ProMax => "iphone-14-pro-max",
            Self::IpadPro11 => "ipad-pro-11",
        }
    }

    /// The drawing's `name` in the manifest, which is its file's stem.
    pub const fn name(self) -> &'static str {
        match self {
            Self::IphoneSe => "iphone-se",
            Self::Iphone15Pro => "iphone-15-pro",
            Self::Iphone15ProMax => "iphone-15-pro-max",
            Self::IpadPro11 => "ipad-pro-11inches",
        }
    }

    /// The name the drawing in `color` is asked for by, where a file
    /// entity's image is asked for by its path. It is no path: the shell
    /// holds the drawings. The two colours are the same shape.
    pub const fn asset(self, color: DeviceFrameColor) -> &'static str {
        match (self, color) {
            (Self::IphoneSe, DeviceFrameColor::Dark) => "specular-device-frame:iphone-se.svg",
            (Self::IphoneSe, DeviceFrameColor::Light) => {
                "specular-device-frame:iphone-se-light.svg"
            }
            (Self::Iphone15Pro, DeviceFrameColor::Dark) => {
                "specular-device-frame:iphone-15-pro.svg"
            }
            (Self::Iphone15Pro, DeviceFrameColor::Light) => {
                "specular-device-frame:iphone-15-pro-light.svg"
            }
            (Self::Iphone15ProMax, DeviceFrameColor::Dark) => {
                "specular-device-frame:iphone-15-pro-max.svg"
            }
            (Self::Iphone15ProMax, DeviceFrameColor::Light) => {
                "specular-device-frame:iphone-15-pro-max-light.svg"
            }
            (Self::IpadPro11, DeviceFrameColor::Dark) => {
                "specular-device-frame:ipad-pro-11inches.svg"
            }
            (Self::IpadPro11, DeviceFrameColor::Light) => {
                "specular-device-frame:ipad-pro-11inches-light.svg"
            }
        }
    }

    /// The drawing and colour [`asset`](Self::asset) names.
    pub fn from_asset(asset: &str) -> Option<(Self, DeviceFrameColor)> {
        Self::ALL.into_iter().find_map(|art| {
            [DeviceFrameColor::Dark, DeviceFrameColor::Light]
                .into_iter()
                .find(|color| art.asset(*color) == asset)
                .map(|color| (art, color))
        })
    }

    /// Where the screen and the body sit in the drawing: the manifest's
    /// `frame`, `screen`, `screenCornerRadius`, `body` and
    /// `bodyCornerRadius`.
    pub const fn geometry(self) -> ArtGeometry {
        match self {
            Self::IphoneSe => geometry(
                (891.0, 1790.0),
                (54.253, 198.193, 782.494, 1391.527),
                0.0,
                (6.26, 2.0, 878.48, 1788.0),
                131.44,
            ),
            Self::Iphone15Pro => geometry(
                (873.0, 1792.0),
                (38.737, 33.735, 795.526, 1724.535),
                111.33,
                (5.0, 0.0, 863.0, 1792.0),
                145.07,
            ),
            Self::Iphone15ProMax => geometry(
                (874.0, 1802.0),
                (35.157, 31.112, 802.687, 1739.778),
                102.67,
                (4.356, 0.0, 864.288, 1802.0),
                133.78,
            ),
            Self::IpadPro11 => geometry(
                (1741.0, 2412.0),
                (90.928, 96.276, 1553.792, 2225.053),
                32.09,
                (0.0, 5.349, 1735.65, 2406.651),
                123.02,
            ),
        }
    }

    /// How many canvas units one unit of the drawing is, for a screen
    /// `screen_width` canvas units wide standing up.
    pub fn scale(self, screen_width: f64) -> f64 {
        screen_width / self.geometry().screen_size.0
    }

    /// The drawing's width and height in canvas units round its preset's
    /// viewport.
    pub fn size(self) -> (f64, f64) {
        let width = (crate::VIEWPORT_PRESETS.iter())
            .find(|preset| preset.device_id == self.device_id())
            .map_or(0.0, |preset| preset.width);
        let (scale, frame) = (self.scale(width), self.geometry().frame);
        (frame.0 * scale, frame.1 * scale)
    }
}
