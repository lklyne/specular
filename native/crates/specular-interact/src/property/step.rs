//! The page settings a key or a button moves to their next value.

use specular_doc::ColorScheme;

use super::{Orientation, Property, read};
use crate::App;

/// A page setting with a short run of values, which one press moves on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageStep {
    /// The device frame: none, clay, realistic, and round again.
    Frame,
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
