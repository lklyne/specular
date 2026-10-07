//! The images file entities show: which are wanted, and what the shell has
//! said about each.
//!
//! A file entity whose path ends in an image extension gets an [`ImageKey`]
//! and an [`Effect::LoadImage`]. The shell decodes the file, uploads it, and
//! answers with [`Event::Image`](crate::Event::Image). Until then, and after
//! a failure, `view` draws the placeholder card.

use std::collections::BTreeMap;

use specular_doc::{Document, Kind};

use crate::{App, Effect};

/// Names one image between `update`, the shell and the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageKey(pub u64);

/// What is known about one image file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Image {
    /// The key the shell and the renderer know it by.
    pub key: ImageKey,
    /// How far loading has got.
    pub state: ImageState,
}

/// How far loading an image has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageState {
    /// Asked for; no answer yet.
    Loading,
    /// Decoded and uploaded.
    Ready {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// There is no file at the path.
    Missing,
    /// The file could not be read or decoded, or its type is not drawn yet.
    Failed,
}

/// What the shell reports about an image it was asked to load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageNotice {
    /// Decoded and uploaded under the image's key.
    Ready {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// There is no file at the path.
    Missing,
    /// The file could not be read or decoded, or its type is not drawn yet.
    Failed,
}

/// The images asked for so far, by the `file` path as the document writes it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Images {
    by_file: BTreeMap<String, Image>,
    next_key: u64,
}

impl Images {
    pub(crate) fn get(&self, file: &str) -> Option<&Image> {
        self.by_file.get(file)
    }
}

/// Whether `file` is drawn as an image: Electron's `IMAGE_EXTENSIONS`.
pub fn is_image_file(file: &str) -> bool {
    const EXTENSIONS: [&str; 8] = ["png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico"];
    file.rsplit_once('.').is_some_and(|(_, extension)| {
        (EXTENSIONS.iter()).any(|known| extension.eq_ignore_ascii_case(known))
    })
}

/// The image files `document` shows, each once.
fn wanted(document: &Document) -> impl Iterator<Item = &str> {
    document.entities().filter_map(|entity| match &entity.kind {
        Kind::File(file) if is_image_file(&file.file) => Some(file.file.as_str()),
        Kind::File(_)
        | Kind::Page(_)
        | Kind::Text(_)
        | Kind::Group(_)
        | Kind::Drawing(_)
        | Kind::Shape(_) => None,
    })
}

/// Asks for every image the document shows that has not been asked for.
/// Images nothing shows any more are kept: an undo can bring them back.
pub(crate) fn request_new(app: &mut App, effects: &mut Vec<Effect>) {
    let images = &mut app.session.images;
    for file in wanted(&app.document) {
        if images.by_file.contains_key(file) {
            continue;
        }
        let key = ImageKey(images.next_key);
        images.next_key += 1;
        let state = ImageState::Loading;
        images.by_file.insert(file.to_owned(), Image { key, state });
        effects.push(Effect::LoadImage {
            image: key,
            file: file.to_owned(),
        });
    }
}

/// A different document was opened: lets go of the images it does not show,
/// which no undo can bring back, and asks for the ones that are new.
pub(crate) fn reopen(app: &mut App, effects: &mut Vec<Effect>) {
    let document = &app.document;
    app.session.images.by_file.retain(|file, image| {
        let shown = wanted(document).any(|wanted| wanted == file);
        if !shown {
            effects.push(Effect::DropImage(image.key));
        }
        shown
    });
    request_new(app, effects);
}

/// The shell answered for `key`. An answer for an image since let go of is
/// ignored.
pub(crate) fn on_notice(app: &mut App, key: ImageKey, notice: ImageNotice) {
    let images = app.session.images.by_file.values_mut();
    if let Some(image) = images.into_iter().find(|image| image.key == key) {
        image.state = match notice {
            ImageNotice::Ready { width, height } => ImageState::Ready { width, height },
            ImageNotice::Missing => ImageState::Missing,
            ImageNotice::Failed => ImageState::Failed,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::is_image_file;

    #[test]
    fn image_files_are_told_by_their_extension_in_any_case() {
        for file in [
            "a.png",
            "assets/b.JPG",
            "c.jpeg",
            "d.gif",
            "e.webp",
            "f.svg",
            "g.BMP",
            "h.ico",
        ] {
            assert!(is_image_file(file), "{file}");
        }
        for file in [
            "notes.md",
            "png",
            "clip.mp4",
            "archive.png.zip",
            "Button.tsx",
            "",
        ] {
            assert!(!is_image_file(file), "{file}");
        }
    }
}
