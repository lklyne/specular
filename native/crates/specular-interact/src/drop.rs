//! Files dropped on the canvas: images and markdown become file entities at
//! the drop point, as `useCanvasFileDrop.ts` does.
//!
//! A file inside the space folder is shown where it is. One from anywhere
//! else is copied into `assets/` first, which is an [`Effect`].

use glam::{DVec2, Vec2};
use specular_doc::EntityId;

use crate::asset;
use crate::{App, Effect, clipboard, geometry, grid, is_image_file, is_note_file};

/// The size a dropped image gets when the shell could not read its own.
const DEFAULT_IMAGE_SIZE: DVec2 = DVec2::new(300.0, 300.0);
/// The size a dropped markdown file's Document gets.
const DEFAULT_NOTE_SIZE: DVec2 = DVec2::new(400.0, 400.0);
/// How far each file of a drop sits from the one before, on both axes.
const CASCADE_STEP: f64 = 20.0;

/// One file of a drop, as the shell found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedFile {
    /// The file's absolute path.
    pub path: String,
    /// The path relative to the space folder, with `/` between its parts,
    /// when the file is inside it.
    pub space_path: Option<String>,
    /// An image's width and height in pixels, when the shell could read them.
    pub image_size: Option<(u32, u32)>,
}

/// Makes a file entity for each image and markdown file in `files`, the
/// first at `screen` and each next one a step down and right, as one undo
/// step. Other files are ignored.
pub(crate) fn on_drop(
    app: &mut App,
    files: &[DroppedFile],
    screen: Option<Vec2>,
    effects: &mut Vec<Effect>,
) {
    if app.session.gesture.is_some() {
        return;
    }
    let at = match screen {
        Some(screen) => {
            let world = app.session.camera.screen_to_world(screen).as_dvec2();
            DVec2::new(grid::snap(world.x), grid::snap(world.y))
        }
        None => clipboard::paste_point(app),
    };
    let mut entities = Vec::new();
    for file in files {
        let Some(size) = default_size(file) else {
            continue;
        };
        let id = EntityId::new(app.fresh_id());
        let Some((shown, copy)) = shown_path(&id, file) else {
            continue;
        };
        effects.extend(copy);
        let corner = at + DVec2::splat(CASCADE_STEP * entities.len() as f64);
        entities.push(asset::file_entity(id, shown, geometry::rect(corner, size)));
    }
    asset::insert_selected(app, entities, effects);
}

/// The size a file gets when it lands on the canvas: an image's own pixels,
/// a markdown file's Document size, and `None` for any other file, which a
/// drop ignores.
pub fn default_size(file: &DroppedFile) -> Option<DVec2> {
    if is_image_file(&file.path) {
        Some(
            (file.image_size).map_or(DEFAULT_IMAGE_SIZE, |(width, height)| {
                DVec2::new(f64::from(width), f64::from(height))
            }),
        )
    } else if is_note_file(&file.path) {
        Some(DEFAULT_NOTE_SIZE)
    } else {
        None
    }
}

/// What the entity for `file` points at, and the copy to make first when
/// the file is outside the space folder. The copy is named after `id`.
/// `None` for a file outside the folder that has no extension to keep.
pub fn shown_path(id: &EntityId, file: &DroppedFile) -> Option<(String, Option<Effect>)> {
    match (&file.space_path, asset::extension(&file.path)) {
        (Some(inside), _) => Some((inside.clone(), None)),
        (None, Some(extension)) => {
            let copy = asset::asset_file(id, extension);
            let effect = Effect::CopyAsset {
                from: file.path.clone(),
                file: copy.clone(),
            };
            Some((copy, Some(effect)))
        }
        (None, None) => None,
    }
}
