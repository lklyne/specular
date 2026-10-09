//! Files on their way into the space folder's `assets/`, and the file
//! entities that show them.
//!
//! `update` cannot write a file. It names the file, makes the entity that
//! shows it, and returns an [`Effect`] for the shell to put the bytes there.

use std::fmt;
use std::sync::Arc;

use specular_doc::{Command, Entity, EntityId, FileRef, ItemId, Kind, Rect};

use crate::{App, Effect, update};

/// The folder inside the space folder that pasted and dropped files go to.
const ASSETS_FOLDER: &str = "assets";

/// The contents of a file. Cloning shares them.
#[derive(Clone, PartialEq, Eq)]
pub struct AssetBytes(Arc<[u8]>);

impl AssetBytes {
    /// The bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for AssetBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes.into())
    }
}

impl fmt::Debug for AssetBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AssetBytes({} bytes)", self.0.len())
    }
}

/// The path, relative to the space folder, of a new asset named after `id`.
pub(crate) fn asset_file(id: &EntityId, extension: &str) -> String {
    let extension = extension.to_ascii_lowercase();
    format!("{ASSETS_FOLDER}/{}.{extension}", id.as_str())
}

/// The part of `path` after its last dot, if it has one after its last
/// slash.
pub(crate) fn extension(path: &str) -> Option<&str> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').map(|(_, extension)| extension)
}

/// A file entity showing `file` at `rect`.
pub(crate) fn file_entity(id: EntityId, file: String, rect: Rect) -> Entity {
    let file = FileRef {
        file,
        ..FileRef::default()
    };
    Entity::new(id, rect, Kind::File(file))
}

/// Adds `entities` in front of everything, in order, as one undo step, and
/// selects them. Nothing is added when an item view would hide any of them.
pub(crate) fn insert_selected(app: &mut App, entities: Vec<Entity>, effects: &mut Vec<Effect>) {
    let hidden = |entity: &Entity| crate::showing::hides(app, entity);
    if entities.is_empty() || entities.iter().any(hidden) {
        return;
    }
    let ids: Vec<ItemId> = (entities.iter())
        .map(|entity| ItemId::Entity(entity.id.clone()))
        .collect();
    let at = app.document.stack_len();
    let commands = (entities.into_iter().enumerate())
        .map(|(index, entity)| Command::InsertEntity {
            entity: Box::new(entity),
            at: at + index,
        })
        .collect();
    update::document_step(app, Command::Batch(commands), effects);
    app.session.selection.set(ids);
}
