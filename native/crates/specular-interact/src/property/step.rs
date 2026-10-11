//! The page settings a key or a button moves to their next value.

use specular_doc::ColorScheme;

use super::{Orientation, Property, read};
use crate::App;

/// A page setting with a short run of values, which one press moves on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageStep {
    /// The device frame: none, clay, realistic, and round again.
    Frame,
    /// The device's colour: the one the framed pages are not drawn in, or
    /// the one the theme is not when they differ.
    FrameColor,
    /// The orientation: landscape, unless the pages are.
    Rotate,
    /// The color scheme: system, light, dark, and round again.
    Scheme,
}

impl PageStep {
    /// What the selected pages are set to next.
    pub fn next(self, app: &App) -> Property {
        match self {
            Self::Frame => Property::DeviceFrame(read::next_device_frame(app)),
            Self::FrameColor => {
                let drawn = read::device_frame_color(app)
                    .unwrap_or_else(|| crate::images::theme_device_color(app));
                Property::DeviceFrameColor(drawn.other())
            }
            Self::Rotate => Property::Orientation(match read::orientation(app) {
                Some(Orientation::Landscape) => Orientation::Portrait,
                Some(Orientation::Portrait) | None => Orientation::Landscape,
            }),
            Self::Scheme => Property::ColorScheme(match read::color_scheme(app) {
                Some(Some(ColorScheme::Light)) => Some(ColorScheme::Dark),
                Some(Some(ColorScheme::Dark)) => None,
                Some(None) | None => Some(ColorScheme::Light),
            }),
        }
    }
}
