//! The safe area of a device's screen: how far in from each edge a page has
//! to stay to clear what the hardware puts over it, a Dynamic Island, a
//! notch, a camera hole, a home indicator and the status bar they sit in.
//!
//! The values are what the devices report, in CSS pixels. A phone on its
//! side keeps the top inset on both short edges, as iOS does, since it can
//! be turned either way. A device with nothing over its screen has none: a
//! tablet, a computer and a phone with a square screen.

/// How far a page stays in from each edge of its device's screen, in CSS
/// pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SafeArea {
    /// From the top edge.
    pub top: f64,
    /// From the right edge.
    pub right: f64,
    /// From the bottom edge.
    pub bottom: f64,
    /// From the left edge.
    pub left: f64,
}

impl SafeArea {
    /// A screen with nothing over it.
    pub const NONE: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    /// Whether the whole screen is safe.
    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// One device's insets: the top and bottom standing up, and the bottom on
/// its side, where the top inset moves to the two short edges.
struct Insets {
    top: f64,
    bottom: f64,
    bottom_on_side: f64,
}

/// An iPhone with a home indicator: 34 under it standing up, 21 on its
/// side.
const fn iphone(top: f64) -> Insets {
    Insets {
        top,
        bottom: 34.0,
        bottom_on_side: 21.0,
    }
}

/// An Android phone with a camera hole in its status bar and a gesture bar
/// 24 high.
const fn android(top: f64) -> Insets {
    Insets {
        top,
        bottom: 24.0,
        bottom_on_side: 24.0,
    }
}

fn insets(device_id: &str) -> Option<Insets> {
    Some(match device_id {
        // The Dynamic Island of the iPhone 14 Pro to the iPhone 16. The
        // folding phone is drawn as one of them, open or closed.
        "iphone-14-pro" | "iphone-14-pro-max" | "iphone-duo-cover" | "iphone-duo-open" => {
            iphone(59.0)
        }
        // The island sits lower from the iPhone 16 Pro on.
        "iphone-17" | "iphone-17-pro-max" => iphone(62.0),
        "iphone-air" => iphone(68.0),
        // A notch.
        "iphone-16e" => iphone(47.0),
        "pixel-10-pro-xl" => android(49.0),
        "pixel-10" => android(52.0),
        "galaxy-s25" => android(40.0),
        _ => return None,
    })
}

/// The safe area of the screen of `device_id`, on its side when
/// `landscape`. A device with nothing over its screen, and one the table
/// does not know, has none.
pub fn safe_area(device_id: Option<&str>, landscape: bool) -> SafeArea {
    let Some(insets) = device_id.and_then(insets) else {
        return SafeArea::NONE;
    };
    if landscape {
        SafeArea {
            top: 0.0,
            right: insets.top,
            bottom: insets.bottom_on_side,
            left: insets.top,
        }
    } else {
        SafeArea {
            top: insets.top,
            right: 0.0,
            bottom: insets.bottom,
            left: 0.0,
        }
    }
}
