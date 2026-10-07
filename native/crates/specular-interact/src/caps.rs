//! What each [`Kind`] allows: its smallest size, its aspect rule and whether
//! edges can connect to it. One `match` per rule, so a new kind has to answer
//! each.

use glam::DVec2;
use specular_doc::Kind;

/// How a resize treats an entity's aspect ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AspectMode {
    /// Width and height change independently.
    Free,
    /// Holding Shift keeps the ratio the drag started with.
    ShiftLocks,
    /// The ratio is kept unless Shift is held.
    ShiftUnlocks,
}

impl AspectMode {
    /// Whether a resize keeps the ratio, given whether Shift is held.
    pub const fn locks(self, shift: bool) -> bool {
        match self {
            Self::Free => false,
            Self::ShiftLocks => shift,
            Self::ShiftUnlocks => !shift,
        }
    }
}

/// File extensions shown as a picture or a video, which keep their ratio.
const MEDIA_EXTENSIONS: [&str; 12] = [
    "png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico", "webm", "mp4", "mov", "ogg",
];

/// The smallest size, in canvas units, a resize may give an entity of this
/// kind.
pub const fn min_size(kind: &Kind) -> DVec2 {
    match kind {
        Kind::Page(_) => DVec2::new(320.0, 200.0),
        Kind::Text(_) => DVec2::new(100.0, 60.0),
        Kind::File(_) => DVec2::new(80.0, 80.0),
        Kind::Group(_) => DVec2::new(120.0, 80.0),
        Kind::Drawing(_) => DVec2::new(16.0, 16.0),
        Kind::Shape(_) => DVec2::new(24.0, 24.0),
    }
}

/// The aspect rule a resize of this kind follows.
pub fn aspect_mode(kind: &Kind) -> AspectMode {
    match kind {
        // A text's height follows its content, so a free vertical drag would
        // do nothing. Keeping the ratio makes that drag change the width.
        Kind::Text(_) => AspectMode::ShiftUnlocks,
        Kind::File(file) => {
            let media = file.file.rsplit_once('.').is_some_and(|(_, extension)| {
                MEDIA_EXTENSIONS
                    .iter()
                    .any(|known| extension.eq_ignore_ascii_case(known))
            });
            if media {
                AspectMode::ShiftUnlocks
            } else {
                AspectMode::ShiftLocks
            }
        }
        Kind::Shape(_) => AspectMode::ShiftLocks,
        Kind::Page(_) | Kind::Group(_) | Kind::Drawing(_) => AspectMode::Free,
    }
}

/// Whether the kind shows edge anchors when selected. Drawings do not: the
/// dots crowd a selected stroke and make it awkward to grab.
pub const fn has_anchors(kind: &Kind) -> bool {
    match kind {
        Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Shape(_) => true,
        Kind::Drawing(_) => false,
    }
}
