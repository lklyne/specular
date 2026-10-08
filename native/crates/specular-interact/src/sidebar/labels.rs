//! The words and glyphs of sidebar rows.

use specular_doc::{Annotation, ShapeKind};

use crate::panel::Icon;

/// How many characters of a comment's text name its row.
const COMMENT_LABEL_CHARS: usize = 60;
/// A page narrower than this is drawn with a phone glyph, and one narrower
/// than [`TABLET_BELOW`] with a tablet's: `viewportIcon` in
/// `shared/pageListItem.tsx`.
const PHONE_BELOW: f64 = 600.0;
const TABLET_BELOW: f64 = 1100.0;

/// The glyph of a page row: the device its width suggests.
pub(super) fn page_icon(width: f64) -> Icon {
    if width < PHONE_BELOW {
        Icon::Device
    } else if width < TABLET_BELOW {
        Icon::Tablet
    } else {
        Icon::Laptop
    }
}

/// A page's size as its row ends: `820×1180`.
pub(super) fn dimensions(width: f64, height: f64) -> String {
    format!("{}\u{d7}{}", width.round(), height.round())
}

/// The glyph of a file row, by extension: `iconForFilePath`.
pub(super) fn file_icon(file: &str) -> Icon {
    let extension = file.rsplit_once('.').map_or("", |(_, extension)| extension);
    let is = |known: &[&str]| {
        known
            .iter()
            .any(|name| extension.eq_ignore_ascii_case(name))
    };
    if is(&["md"]) {
        Icon::FileText
    } else if is(&["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico"]) {
        Icon::Image
    } else if is(&["webm", "mp4", "mov", "ogg"]) {
        Icon::Video
    } else if is(&["html", "htm"]) {
        Icon::Code
    } else {
        Icon::File
    }
}

/// A file's row text: its name without a markdown extension.
pub(super) fn file_label(file: &str) -> String {
    let name = file.rsplit('/').next().unwrap_or(file);
    let stem = name
        .len()
        .checked_sub(3)
        .filter(|&at| name.is_char_boundary(at) && name[at..].eq_ignore_ascii_case(".md"));
    stem.map_or(name, |at| &name[..at]).to_owned()
}

/// A comment's row text: the element it is on, else the start of what it
/// says.
pub(super) fn comment_label(annotation: &Annotation) -> String {
    let element = (annotation.element_name.as_deref().map(str::trim)).filter(|n| !n.is_empty());
    if let Some(element) = element {
        return element.to_owned();
    }
    let text = annotation.text.trim();
    if text.is_empty() {
        return "Comment".to_owned();
    }
    if text.chars().count() <= COMMENT_LABEL_CHARS {
        return text.to_owned();
    }
    let start: String = text.chars().take(COMMENT_LABEL_CHARS - 1).collect();
    format!("{start}\u{2026}")
}

pub(super) fn first_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

/// The host of `url` without a leading `www.`, or `None` when it has none.
pub(super) fn host_label(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?.split(':').next()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    (!host.is_empty()).then(|| host.to_owned())
}

pub(super) const fn shape_label(shape: ShapeKind) -> &'static str {
    match shape {
        ShapeKind::Rectangle => "Rectangle",
        ShapeKind::Rounded => "Rounded rectangle",
        ShapeKind::Ellipse => "Ellipse",
        ShapeKind::Diamond => "Diamond",
        ShapeKind::Triangle => "Triangle",
        ShapeKind::Hexagon => "Hexagon",
        ShapeKind::Pill => "Pill",
        ShapeKind::Parallelogram => "Parallelogram",
        ShapeKind::Chevron => "Chevron",
        ShapeKind::Cylinder => "Cylinder",
    }
}
