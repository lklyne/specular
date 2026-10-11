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
    /// A flat bezel, the same for every device of a kind.
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
}

const fn geometry(
    frame: (f64, f64),
    screen: (f64, f64, f64, f64),
    screen_corner_radius: f64,
) -> ArtGeometry {
    ArtGeometry {
        frame,
        screen_origin: (screen.0, screen.1),
        screen_size: (screen.2, screen.3),
        screen_corner_radius,
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

    /// The name the drawing is asked for by, where a file entity's image is
    /// asked for by its path. It is no path: the shell holds the drawing.
    pub const fn asset(self) -> &'static str {
        match self {
            Self::IphoneSe => "specular-device-frame:iphone-se.svg",
            Self::Iphone15Pro => "specular-device-frame:iphone-15-pro.svg",
            Self::Iphone15ProMax => "specular-device-frame:iphone-15-pro-max.svg",
            Self::IpadPro11 => "specular-device-frame:ipad-pro-11inches.svg",
        }
    }

    /// The drawing [`asset`](Self::asset) names.
    pub fn from_asset(asset: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|art| art.asset() == asset)
    }

    /// Where the screen sits in the drawing: the manifest's `frame`,
    /// `screen` and `screenCornerRadius`.
    pub const fn geometry(self) -> ArtGeometry {
        match self {
            Self::IphoneSe => geometry((891.0, 1790.0), (54.253, 198.193, 782.494, 1391.527), 0.0),
            Self::Iphone15Pro => {
                geometry((873.0, 1792.0), (38.737, 33.735, 795.526, 1724.535), 111.33)
            }
            Self::Iphone15ProMax => {
                geometry((874.0, 1802.0), (35.157, 31.112, 802.687, 1739.778), 102.67)
            }
            Self::IpadPro11 => geometry(
                (1741.0, 2412.0),
                (90.928, 96.276, 1553.792, 2225.053),
                32.09,
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
