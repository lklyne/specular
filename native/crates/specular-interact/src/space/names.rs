//! Canvas names and the file names made from them, as the Electron app
//! makes them, so both apps find each other's files.

use super::{Canvas, CanvasId};

/// The name of the canvas an empty space starts with.
pub const DEFAULT_CANVAS_NAME: &str = "Canvas 1";

/// The file a canvas named `name` with id `id` is kept in:
/// `<name>-<first four characters of the id>.canvas`. The suffix keeps two
/// canvases whose names differ only in characters a file name cannot hold
/// from sharing a file.
pub fn canvas_file_name(name: &str, id: &CanvasId) -> String {
    let unprefixed = id.as_str().strip_prefix("tab_").unwrap_or(id.as_str());
    let suffix: String = unprefixed.chars().take(4).collect();
    format!("{}-{suffix}.canvas", file_stem(name))
}

/// The file a canvas named `name` was kept in before file names carried
/// part of the id: `<name>.canvas`. Read, never made.
pub fn legacy_canvas_file_name(name: &str) -> String {
    format!("{}.canvas", file_stem(name))
}

/// `name` with the characters no file name can hold replaced.
fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            other => other,
        })
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "Untitled".to_owned()
    } else {
        stem.to_owned()
    }
}

/// Whether a canvas other than `except` has `name`, once trimmed.
pub(super) fn taken(canvases: &[Canvas], name: &str, except: Option<&CanvasId>) -> bool {
    (canvases.iter())
        .filter(|canvas| Some(&canvas.id) != except)
        .any(|canvas| canvas.name.trim() == name.trim())
}

/// The name a canvas made with none gets: `Canvas N`, counting from one
/// more than there are, skipping any that is taken.
pub(super) fn next_default(canvases: &[Canvas]) -> String {
    // Among one more name than there are canvases, one is free.
    (canvases.len() + 1..=canvases.len() * 2 + 2)
        .map(|number| format!("Canvas {number}"))
        .find(|name| !taken(canvases, name, None))
        .unwrap_or_else(|| DEFAULT_CANVAS_NAME.to_owned())
}

/// The name a copy of the canvas `name` gets: `name Copy`, then
/// `name Copy 2` and on while that is taken.
pub(super) fn copy_of(canvases: &[Canvas], name: &str) -> String {
    let first = format!("{} Copy", name.trim());
    if !taken(canvases, &first, None) {
        return first;
    }
    (2..=canvases.len() + 2)
        .map(|number| format!("{first} {number}"))
        .find(|name| !taken(canvases, name, None))
        .unwrap_or(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_named_after_the_canvas_and_four_characters_of_its_id() {
        let id = CanvasId::new("tab_9f3c1b2a-0000");
        assert_eq!(canvas_file_name("Home page", &id), "Home page-9f3c.canvas");
    }

    #[test]
    fn characters_a_file_name_cannot_hold_become_underscores() {
        let id = CanvasId::new("tab_abcd");
        assert_eq!(canvas_file_name(" a/b:c? ", &id), "a_b_c_-abcd.canvas");
        assert_eq!(canvas_file_name("   ", &id), "Untitled-abcd.canvas");
    }
}
