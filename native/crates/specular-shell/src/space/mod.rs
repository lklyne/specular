//! The project folders on disk: finding the first, telling a plain folder
//! from a repo, listing their canvases, seeding a new one, and keeping
//! each canvas's file in step with the app.

mod files;
mod listing;
mod locate;
mod project;
mod starter;
#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::Context as _;
use specular_core::Camera;
use specular_doc::Document;
use specular_interact::{GitChoice, OpenedCanvas, OpenedSpace};

pub(crate) use self::files::SpaceFiles;
pub(crate) use self::locate::{
    SpaceChoice, SpaceStart, Startup, electron_space, electron_user_data, scratch_folder, startup,
};
use crate::persist;

pub(crate) use self::project::write_git_choice;

/// Reads the project at `folder`: every canvas it holds, and which to show
/// (ADR 0062). A folder that is not there is made. A plain folder with no
/// canvas gets the starter space; a repo keeps its canvases in
/// `.specular/canvases/` and starts with one empty canvas. `file` names the
/// canvas file to show; without it the project's last active canvas is. A
/// canvas with no saved camera opens at `start_camera`. `git` is the
/// person's choice for a repo's canvases, from the preferences.
pub(crate) fn open(
    folder: &Path,
    file: Option<&str>,
    start_camera: Camera,
    git: Option<GitChoice>,
) -> anyhow::Result<OpenedSpace> {
    std::fs::create_dir_all(folder)
        .with_context(|| format!("making the project folder {}", folder.display()))?;
    let project = project::read(folder, git);
    let canvas_folder = Path::new(&project.canvases).to_owned();
    if project.keeps_canvases_apart() {
        std::fs::create_dir_all(&canvas_folder)
            .with_context(|| format!("making the canvas folder {}", canvas_folder.display()))?;
    } else if let Some(starter) = starter::starter_dir() {
        match starter::seed(folder, &starter) {
            Ok(true) => tracing::info!(folder = %folder.display(), "seeded the starter space"),
            Ok(false) => {}
            // A space that cannot be seeded is still a usable empty one.
            Err(error) => tracing::warn!("the starter space was not copied: {error}"),
        }
    }
    let folder = canvas_folder.as_path();
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
                tabs: persist::tabs_of(&document),
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
        project: Some(project),
        canvases,
        active,
    })
}
