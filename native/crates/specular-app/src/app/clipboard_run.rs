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
        let Some(clipboard) = self.clipboard() else {
            return;
        };
        if let Err(error) = clipboard.set_text(text) {
            tracing::warn!("the clipboard was not written: {error}");
        }
    }

    /// Reads the clipboard as text and as an image and sends both on.
    pub(super) fn read_clipboard(&mut self) {
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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    fn pixels(width: usize, height: usize, bytes: Vec<u8>) -> ImageData<'static> {
        ImageData {
            width,
            height,
            bytes: Cow::Owned(bytes),
        }
    }

    #[test]
    fn clipboard_pixels_become_a_png_of_the_same_size() {
        let red_then_blue = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let encoded = encode(&pixels(2, 1, red_then_blue.clone())).unwrap();
        assert_eq!((encoded.width, encoded.height), (2, 1));
        let decoded = image::load_from_memory(encoded.png.as_slice()).unwrap();
        assert_eq!(decoded.to_rgba8().into_raw(), red_then_blue);
    }

    #[test]
    fn pixels_that_do_not_fill_the_size_are_not_an_image() {
        assert!(encode(&pixels(2, 2, vec![0; 4])).is_none());
    }

    #[test]
    fn an_empty_clipboard_reads_as_nothing() {
        let empty: Result<String, _> = Err(arboard::Error::ContentNotAvailable);
        assert_eq!(available(empty, "text"), None);
        assert_eq!(
            available(Ok("hi".to_owned()), "text").as_deref(),
            Some("hi")
        );
    }
}
