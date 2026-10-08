//! The space folder on disk: finding it, listing its canvases, seeding a
//! new one, and keeping each canvas's file in step with the app.

mod files;
mod listing;
mod locate;
mod starter;
#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::Context as _;
use specular_core::Camera;
use specular_doc::Document;
use specular_interact::{OpenedCanvas, OpenedSpace};

pub(crate) use self::files::SpaceFiles;
pub(crate) use self::locate::{
    SpaceChoice, SpaceStart, electron_space, electron_user_data, scratch_folder, startup,
};
use crate::persist;

/// Reads the space at `folder`: every canvas it holds, and which to show.
/// A folder that is not there is made, and one with no canvas gets the
/// starter space. `file` names the canvas file to show; without it the
/// space's last active canvas is. A canvas with no saved camera opens at
/// `start_camera`.
pub(crate) fn open(
    folder: &Path,
    file: Option<&str>,
    start_camera: Camera,
) -> anyhow::Result<OpenedSpace> {
    std::fs::create_dir_all(folder)
        .with_context(|| format!("making the space folder {}", folder.display()))?;
    if let Some(starter) = starter::starter_dir() {
        match starter::seed(folder, &starter) {
            Ok(true) => tracing::info!(folder = %folder.display(), "seeded the starter space"),
            Ok(false) => {}
            // A space that cannot be seeded is still a usable empty one.
            Err(error) => tracing::warn!("the starter space was not copied: {error}"),
        }
    }
    let listing = listing::list(folder);
    let mut canvases = Vec::new();
    for listed in listing.canvases {
        let path = folder.join(&listed.file);
        let read = std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| Document::from_canvas_str(&text).map_err(|error| error.to_string()));
        match read {
            Ok(document) => canvases.push(OpenedCanvas {
                id: listed.id,
                name: listed.name,
                camera: persist::camera_of(&document).unwrap_or(start_camera),
                file: listed.file,
                document,
            }),
            // Left out, so nothing of ours is ever written over it.
            Err(error) => tracing::warn!(path = %path.display(), "canvas not opened: {error}"),
        }
    }
    let named = file.and_then(|file| canvases.iter().find(|canvas| canvas.file == file));
    let active = named.map(|canvas| canvas.id.clone()).or(listing.active);
    Ok(OpenedSpace {
        folder: Some(folder.to_string_lossy().into_owned()),
        canvases,
        active,
    })
}
