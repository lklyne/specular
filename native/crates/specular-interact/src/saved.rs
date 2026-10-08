//! What a save writes: the document, with every text the session left
//! alone at the size it was read with.
//!
//! A text's rect is its content's size, measured with this renderer's
//! fonts when the file is opened (see `edit::fit_all`). Another app's fonts
//! give other sizes, so writing the measured rects back would change every
//! text in a file the first time anything in it changed. The measured rect
//! is for drawing and hit-testing; the file keeps what it had until the
//! text itself is moved, resized or edited to another size.

use std::collections::HashMap;

use specular_doc::{Command, Document, EntityId, Rect};

use crate::App;

/// The rect each text was read with and the rect it was then given.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LoadedFits(HashMap<EntityId, LoadedFit>);

#[derive(Debug, Clone, Copy, PartialEq)]
struct LoadedFit {
    read: Rect,
    fitted: Rect,
}

impl LoadedFits {
    /// Notes that `id` was read as `read` and measured as `fitted`.
    pub(crate) fn insert(&mut self, id: EntityId, read: Rect, fitted: Rect) {
        if read != fitted {
            self.0.insert(id, LoadedFit { read, fitted });
        }
    }
}

impl App {
    /// The document as a save should write it. Use this, not
    /// [`App::document`], for anything that goes to the `.canvas` file.
    pub fn document_to_save(&self) -> Document {
        to_save(&self.document, &self.session.loaded_fits)
    }
}

/// `document` with every text `fits` lists that still has its measured rect
/// put back to the rect it was read with.
pub(crate) fn to_save(document: &Document, fits: &LoadedFits) -> Document {
    let mut document = document.clone();
    for (id, fit) in &fits.0 {
        let untouched = (document.entity(id)).is_some_and(|entity| entity.rect == fit.fitted);
        if !untouched {
            continue;
        }
        let command = Command::SetRect {
            id: id.clone(),
            rect: fit.read,
        };
        if let Err(error) = document.apply(command) {
            tracing::warn!("a text kept its measured size in the save: {error}");
        }
    }
    document
}
