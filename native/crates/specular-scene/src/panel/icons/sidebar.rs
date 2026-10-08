//! Lucide's glyphs for the sidebar and its toggle, in the node lists of
//! `lucide-react`.

use super::lucide::{line, lucide};
use super::{Glyph, Layer, Paint, Shape};

/// `plus`.
pub(super) const PLUS: Glyph = lucide(&[line("M5 12h14M12 5v14")]);
/// `panel-right` mirrored: the frame, and the divider on the left.
pub(super) const PANEL_LEFT: Glyph = lucide(&[
    Layer::stroke(Shape::Rect(3.0, 3.0, 18.0, 18.0, 2.0), Paint::Current, 2.0),
    line("M9 3v18"),
]);
/// `file`.
pub(super) const FILE: Glyph = lucide(&[line(FILE_BODY), line("M14 2v5a1 1 0 0 0 1 1h5")]);
/// `file-text`.
pub(super) const FILE_TEXT: Glyph = lucide(&[
    line(FILE_BODY),
    line("M14 2v5a1 1 0 0 0 1 1h5M10 9H8M16 13H8M16 17H8"),
]);
const FILE_BODY: &str = "M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 \
     3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z";
/// `image`.
pub(super) const IMAGE: Glyph = lucide(&[
    Layer::stroke(Shape::Rect(3.0, 3.0, 18.0, 18.0, 2.0), Paint::Current, 2.0),
    Layer::stroke(Shape::Disc(9.0, 9.0, 2.0), Paint::Current, 2.0),
    line("m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"),
]);
/// `video`.
pub(super) const VIDEO: Glyph = lucide(&[
    line("m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.87a.5.5 0 0 0-.752-.432L16 10.5"),
    Layer::stroke(Shape::Rect(2.0, 6.0, 14.0, 12.0, 2.0), Paint::Current, 2.0),
]);
/// `code`.
pub(super) const CODE: Glyph = lucide(&[line("m16 18 6-6-6-6M8 6l-6 6 6 6")]);
/// `folder`.
pub(super) const FOLDER: Glyph = lucide(&[line(
    "M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 \
     2 0 0 0-2 2v13a2 2 0 0 0 2 2Z",
)]);
/// `folder-open`.
pub(super) const FOLDER_OPEN: Glyph = lucide(&[line(
    "m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 \
     1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2",
)]);
/// `sticky-note`.
pub(super) const STICKY_NOTE: Glyph = lucide(&[
    line(
        "M21 9a2.4 2.4 0 0 0-.706-1.706l-3.588-3.588A2.4 2.4 0 0 0 15 3H5a2 2 0 0 0-2 2v14a2 2 0 \
         0 0 2 2h14a2 2 0 0 0 2-2z",
    ),
    line("M15 3v5a1 1 0 0 0 1 1h5"),
]);
/// `pen-line`.
pub(super) const PEN_LINE: Glyph = lucide(&[
    line("M13 21h8"),
    line(
        "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 \
         .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
    ),
]);
/// `message-square`.
pub(super) const MESSAGE_SQUARE: Glyph = lucide(&[line(
    "M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 \
     0 1 2-2h16a2 2 0 0 1 2 2z",
)]);
/// `tablet`.
pub(super) const TABLET: Glyph = lucide(&[
    Layer::stroke(Shape::Rect(4.0, 2.0, 16.0, 20.0, 2.0), Paint::Current, 2.0),
    line("M12 18h.01"),
]);
/// `laptop`.
pub(super) const LAPTOP: Glyph = lucide(&[
    line(
        "M18 5a2 2 0 0 1 2 2v8.526a2 2 0 0 0 .212.897l1.068 2.127a1 1 0 0 1-.9 1.45H3.62a1 1 0 \
         0 1-.9-1.45l1.068-2.127A2 2 0 0 0 4 15.526V7a2 2 0 0 1 2-2z",
    ),
    line("M20.054 15.987H3.946"),
]);
