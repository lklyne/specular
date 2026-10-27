//! The clipboard effects, through the system clipboard. Nothing is decided
//! here: the shell hands over whatever the clipboard holds and `update`
//! picks what a paste makes of it.

use std::io::Cursor;

use arboard::{Clipboard, ImageData};
use image::{ImageFormat, RgbaImage};
use specular_interact::{AssetBytes, ClipboardContent, ClipboardImage, Event};

use super::runtime::{Runtime, ShellWindow};

impl<W: ShellWindow> Runtime<W> {
    /// The system clipboard, opened on first use.
    fn clipboard(&mut self) -> Option<&mut Clipboard> {
        if self.clipboard.is_none() {
            match Clipboard::new() {
                Ok(clipboard) => self.clipboard = Some(clipboard),
                Err(error) => tracing::warn!("the clipboard cannot be opened: {error}"),
            }
        }
        self.clipboard.as_mut()
    }

    pub(super) fn write_clipboard(&mut self, text: String) {
        if let Some(held) = self.scripted_clipboard.as_mut() {
            held.0 = Some(text);
            return;
        }
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        if let Err(error) = clipboard.set_text(text) {
            tracing::warn!("the clipboard was not written: {error}");
        }
    }

    /// Reads the clipboard as text and as an image and sends both on.
    pub(super) fn read_clipboard(&mut self) {
        if let Some(held) = self.scripted_clipboard.clone() {
            let content = ClipboardContent {
                text: held.0,
                image: None,
            };
            self.dispatch(Event::Clipboard(content));
            return;
        }
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        let text = available(clipboard.get_text(), "text");
        let image = available(clipboard.get_image(), "image").and_then(|image| encode(&image));
        self.dispatch(Event::Clipboard(ClipboardContent { text, image }));
    }
}

/// What a clipboard read gave. Holding nothing of that kind is not an error.
fn available<T>(read: Result<T, arboard::Error>, kind: &str) -> Option<T> {
    match read {
        Ok(value) => Some(value),
        Err(arboard::Error::ContentNotAvailable) => None,
        Err(error) => {
            tracing::warn!("the clipboard's {kind} was not read: {error}");
            None
        }
    }
}

/// The clipboard's RGBA pixels as a PNG file.
fn encode(image: &ImageData<'_>) -> Option<ClipboardImage> {
    let (width, height) = (
        u32::try_from(image.width).ok()?,
        u32::try_from(image.height).ok()?,
    );
    let pixels = RgbaImage::from_raw(width, height, image.bytes.to_vec())?;
    let mut png = Cursor::new(Vec::new());
    if let Err(error) = pixels.write_to(&mut png, ImageFormat::Png) {
        tracing::warn!("the clipboard's image was not encoded: {error}");
        return None;
    }
    Some(ClipboardImage {
        width,
        height,
        png: AssetBytes::from(png.into_inner()),
    })
}
