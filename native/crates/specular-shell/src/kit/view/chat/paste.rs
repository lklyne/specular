//! Images pasted into the composer, held until the turn is sent
//! (`usePastedImages.ts`).
//!
//! The Kit's field hands a paste to a hook before it inserts text. An image
//! on the clipboard, or copied image files, are kept here with a thumbnail
//! and the paste is consumed. Anything else is left for the field.

use std::path::Path;
use std::sync::Arc;

use gpui_kit::{ClipboardEntry, ClipboardItem, Context, Image, ImageFormat};
use specular_interact::{ImageUpload, MediaType};

use crate::view::ShellView;

/// One pasted image, waiting to be sent.
pub(super) struct Pasted {
    /// What Send carries.
    pub(super) upload: ImageUpload,
    /// The same bytes as GPUI draws them. The `Arc` is kept so its decoded
    /// form is cached across frames.
    pub(super) thumbnail: Arc<Image>,
}

/// The formats a thread takes, as GPUI names them.
const fn format_of(media_type: MediaType) -> ImageFormat {
    match media_type {
        MediaType::Png => ImageFormat::Png,
        MediaType::Jpeg => ImageFormat::Jpeg,
        MediaType::Gif => ImageFormat::Gif,
        MediaType::Webp => ImageFormat::Webp,
    }
}

/// The thread's name for a clipboard image's format, if it takes it.
const fn media_type_of(format: ImageFormat) -> Option<MediaType> {
    match format {
        ImageFormat::Png => Some(MediaType::Png),
        ImageFormat::Jpeg => Some(MediaType::Jpeg),
        ImageFormat::Gif => Some(MediaType::Gif),
        ImageFormat::Webp => Some(MediaType::Webp),
        ImageFormat::Svg
        | ImageFormat::Bmp
        | ImageFormat::Tiff
        | ImageFormat::Ico
        | ImageFormat::Pnm => None,
    }
}

/// The format a copied file's name says it is.
fn media_type_of_file(path: &Path) -> Option<MediaType> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some(MediaType::Png),
        "jpg" | "jpeg" => Some(MediaType::Jpeg),
        "gif" => Some(MediaType::Gif),
        "webp" => Some(MediaType::Webp),
        _ => None,
    }
}

impl Pasted {
    fn new(media_type: MediaType, bytes: Vec<u8>) -> Self {
        Self {
            thumbnail: Arc::new(Image::from_bytes(format_of(media_type), bytes.clone())),
            upload: ImageUpload {
                media_type,
                bytes: bytes.into(),
            },
        }
    }

    /// A copied image file, read now.
    fn from_file(path: &Path) -> Option<Self> {
        let media_type = media_type_of_file(path)?;
        match std::fs::read(path) {
            Ok(bytes) => Some(Self::new(media_type, bytes)),
            Err(error) => {
                tracing::warn!("reading the pasted {}: {error}", path.display());
                None
            }
        }
    }
}

/// The images `item` carries, in the formats a thread takes.
fn images_of(item: &ClipboardItem) -> Vec<Pasted> {
    let mut images = Vec::new();
    for entry in item.entries() {
        match entry {
            ClipboardEntry::Image(image) => {
                if let Some(media_type) = media_type_of(image.format) {
                    images.push(Pasted::new(media_type, image.bytes.clone()));
                }
            }
            ClipboardEntry::ExternalPaths(paths) => {
                images.extend(
                    paths
                        .paths()
                        .iter()
                        .filter_map(|path| Pasted::from_file(path)),
                );
            }
            ClipboardEntry::String(_) => {}
        }
    }
    images
}

impl ShellView {
    /// Keeps the images of a paste. `true` means the paste was all images
    /// and is spent; `false` leaves it to the field as text.
    pub(crate) fn take_pasted(&mut self, item: &ClipboardItem, cx: &mut Context<'_, Self>) -> bool {
        let images = images_of(item);
        if images.is_empty() {
            return false;
        }
        self.chat.images.extend(images);
        cx.notify();
        true
    }

    /// Pastes the image file at `path` as if it were on the clipboard, for
    /// the scripted-input driver: the system pasteboard is left alone.
    pub(crate) fn paste_image_file(&mut self, path: &Path, cx: &mut Context<'_, Self>) -> bool {
        let Some(media_type) = media_type_of_file(path) else {
            return false;
        };
        let Ok(bytes) = std::fs::read(path) else {
            return false;
        };
        let image = Image::from_bytes(format_of(media_type), bytes);
        self.take_pasted(&ClipboardItem::new_image(&image), cx)
    }
}
