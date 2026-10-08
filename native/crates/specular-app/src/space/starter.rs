//! The starter space: what a folder with no canvases is given, so a first
//! launch opens on something.

use std::io;
use std::path::{Path, PathBuf};

use super::listing::canvas_files;
use crate::persist::write_atomic;

/// Stands in the starter canvas for the space folder, which a file that
/// ships with the app cannot know.
const SPACE_TOKEN: &str = "__SPECULAR_SPACE__";
const STARTER_CANVAS: &str = "Welcome.canvas";
const STARTER_NOTE: &str = "Welcome.md";

/// Where the starter space's files are: beside the app in a bundle, in the
/// repository's `resources/` in a development build.
pub(crate) fn starter_dir() -> Option<PathBuf> {
    let bundled = std::env::current_exe().ok().and_then(|exe| {
        let contents = exe.parent()?.parent()?;
        Some(contents.join("Resources").join("starter-space"))
    });
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resources/starter-space");
    (bundled.into_iter().chain([repo])).find(|dir| dir.join(STARTER_CANVAS).is_file())
}

/// Copies the starter canvas and its note from `starter` into `space`, if
/// the space holds no canvas at all. A space with one is never touched, so
/// deleting the starter does not bring it back. The note's path in the
/// canvas is a token that becomes the note's absolute path in the space,
/// as the Electron app writes it. Returns whether files were written.
pub(crate) fn seed(space: &Path, starter: &Path) -> io::Result<bool> {
    if !canvas_files(space).is_empty() {
        return Ok(false);
    }
    let canvas = std::fs::read_to_string(starter.join(STARTER_CANVAS))?;
    let note = std::fs::read(starter.join(STARTER_NOTE))?;
    std::fs::create_dir_all(space)?;
    let note_path = space.join(STARTER_NOTE);
    std::fs::write(&note_path, note)?;
    let token = format!("{SPACE_TOKEN}/{STARTER_NOTE}");
    let canvas = canvas.replace(&token, &json_escaped(&note_path.to_string_lossy()));
    write_atomic(&space.join(STARTER_CANVAS), &canvas)?;
    Ok(true)
}

/// `text` as it reads inside a JSON string, without the quotes.
fn json_escaped(text: &str) -> String {
    let quoted = serde_json::Value::from(text).to_string();
    quoted[1..quoted.len() - 1].to_owned()
}
