//! The images file entities show: which are wanted, and what the shell has
//! said about each.
//!
//! A file entity whose path ends in an image extension gets an [`ImageKey`]
//! and an [`Effect::LoadImage`]. The shell decodes the file, uploads it, and
//! answers with [`Event::Image`](crate::Event::Image). Until then, and after
//! a failure, `view` draws the placeholder card.
//!
//! Three kinds of file need more than one answer:
//!
//! - an svg is rastered by the shell at the size it is drawn at, and again
//!   when the zoom leaves a band around that size ([`vector`]);
//! - a gif arrives as every frame with its delays, and the clock picks the
//!   frame ([`animation`]);
//! - a file changed on disk is asked for again, and the picture on screen
//!   stays until the new one is ready.

mod animation;
mod vector;

use std::collections::BTreeMap;
use std::sync::Arc;

use specular_doc::{Document, Entity, FileRef, Kind};

pub(crate) use self::animation::Animations;

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
    /// For an svg, the width in logical pixels of the raster last asked for,
    /// and so of the one that is or will be on screen.
    pub raster_width: Option<u32>,
}

impl ImageKey {
    /// The texture id of frame `frame`. Frame 0, and every still image, is
    /// the key itself.
    pub fn texture(self, frame: u32) -> u64 {
        self.0 | (u64::from(frame) << 32)
    }
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageNotice {
    /// Decoded and uploaded under the image's key. For an svg the size is
    /// its intrinsic one, not the raster's.
    Ready {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// Every frame of an animated gif is uploaded, frame `n` under
    /// [`ImageKey::texture`]`(n)`.
    Animated {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// How long each frame is shown, in milliseconds.
        delays_ms: Arc<[u32]>,
    },
    /// The file changed on disk. The image is asked for again and what is
    /// on screen stays until the new one is ready.
    Changed,
    /// There is no file at the path.
    Missing,
    /// The file could not be read or decoded, or its type is not drawn yet.
    Failed,
}

/// The images asked for so far, by the `file` path as the document writes it.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Images {
    by_file: BTreeMap<String, Image>,
    next_key: u64,
    animations: Animations,
    /// The zoom when rasters were last considered: a raster is only asked
    /// for once the zoom has held still between two looks.
    zoom_seen: f32,
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
        let raster_width = None;
        images.by_file.insert(
            file.to_owned(),
            Image {
                key,
                state,
                raster_width,
            },
        );
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
///
/// A failure while a picture is on screen is a reload that went wrong, a
/// file caught half written or gone: the picture stays, and the next change
/// on disk tries again.
pub(crate) fn on_notice(
    app: &mut App,
    key: ImageKey,
    notice: ImageNotice,
    effects: &mut Vec<Effect>,
) {
    let now = app.session.now_ms;
    let images = &mut app.session.images;
    let Some((file, image)) = images
        .by_file
        .iter_mut()
        .find(|(_, image)| image.key == key)
    else {
        return;
    };
    let ready = matches!(image.state, ImageState::Ready { .. });
    match notice {
        ImageNotice::Ready { width, height } => {
            image.state = ImageState::Ready { width, height };
            images.animations.stop(key);
        }
        ImageNotice::Animated {
            width,
            height,
            delays_ms,
        } => {
            image.state = ImageState::Ready { width, height };
            images.animations.start(key, delays_ms, now);
        }
        ImageNotice::Changed => {
            image.raster_width = None;
            effects.push(Effect::LoadImage {
                image: key,
                file: file.clone(),
            });
        }
        ImageNotice::Missing if !ready => image.state = ImageState::Missing,
        ImageNotice::Failed if !ready => image.state = ImageState::Failed,
        ImageNotice::Missing | ImageNotice::Failed => {}
    }
    vector::request(app, effects);
}

/// The clock moved: svgs whose raster no longer fits are asked for again,
/// and animated images on screen move to the frame the clock is at.
pub(crate) fn on_tick(app: &mut App, effects: &mut Vec<Effect>) {
    vector::request(app, effects);
    animation::advance(app);
}

/// The file entities that show an image and are in the viewport.
fn shown(app: &App) -> impl Iterator<Item = (&Entity, &FileRef)> {
    let view = app.session.camera.visible_world_rect(app.session.viewport);
    let (left, top) = (f64::from(view.x), f64::from(view.y));
    let (right, bottom) = (left + f64::from(view.width), top + f64::from(view.height));
    app.document.entities().filter_map(move |entity| {
        let Kind::File(file) = &entity.kind else {
            return None;
        };
        let rect = entity.rect;
        let in_view = rect.x < right
            && rect.x + rect.width > left
            && rect.y < bottom
            && rect.y + rect.height > top;
        (in_view && is_image_file(&file.file)).then_some((entity, file))
    })
}

impl App {
    /// The frame of `file`'s animation to draw now: 0 for a still image.
    pub fn image_frame(&self, file: &str) -> u32 {
        let images = &self.session.images;
        images
            .get(file)
            .map_or(0, |image| images.animations.frame(image.key))
    }

    /// Counts the frame changes of animated images on screen, so a shell
    /// can tell the clock alone changed what is drawn.
    pub fn animation_epoch(&self) -> u64 {
        self.session.images.animations.epoch()
    }

    /// Milliseconds until the next animated image on screen changes frame,
    /// or `None` when none is on screen. A shell that has nothing else to
    /// do wakes then and not before.
    pub fn next_frame_in_ms(&self) -> Option<u64> {
        let now = self.session.now_ms;
        let images = &self.session.images;
        let files = shown(self).map(|(_, file)| file.file.as_str());
        files
            .filter_map(|file| images.get(file))
            .filter_map(|image| images.animations.until_next(image.key, now))
            .min()
    }
}
