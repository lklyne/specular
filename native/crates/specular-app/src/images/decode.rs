//! File bytes to pixels: PNG, JPEG, WebP and the first frame of a GIF, and
//! on macOS HEIC and HEIF.

use std::io::Cursor;

use image::{DynamicImage, ImageDecoder as _, ImageReader, imageops::FilterType};
use specular_core::PixelSize;

/// A decoded image: tightly packed sRGB with straight alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decoded {
    pub(crate) size: PixelSize,
    pub(crate) rgba: Vec<u8>,
}

/// Decodes `bytes`, telling the format from its content. The image is
/// turned the way its EXIF orientation says, as a browser shows it, and
/// scaled down to fit `max_dimension` on its longer side when it is larger
/// than a texture can be.
pub(crate) fn decode(bytes: &[u8], max_dimension: u32) -> Result<Decoded, image::ImageError> {
    #[cfg(target_os = "macos")]
    if is_heif(bytes) {
        return super::heif::decode(bytes, max_dimension).ok_or_else(|| {
            image::ImageError::Decoding(image::error::DecodingError::new(
                image::error::ImageFormatHint::Name("HEIF".to_owned()),
                "ImageIO could not decode it",
            ))
        });
    }
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    if image.width() > max_dimension || image.height() > max_dimension {
        image = image.resize(max_dimension, max_dimension, FilterType::Triangle);
    }
    let rgba = image.into_rgba8();
    Ok(Decoded {
        size: PixelSize::new(rgba.width(), rgba.height()),
        rgba: rgba.into_raw(),
    })
}

/// The width and height of the image file at `path`, from its header.
pub(crate) fn dimensions(path: &std::path::Path) -> Option<(u32, u32)> {
    let read = image::image_dimensions(path).ok();
    #[cfg(target_os = "macos")]
    let read = read.or_else(|| {
        let bytes = std::fs::read(path).ok()?;
        is_heif(&bytes)
            .then(|| super::heif::dimensions(&bytes))
            .flatten()
    });
    read
}

/// Whether `bytes` are an HEIC or HEIF file: an ISO media file whose brand
/// is one of HEIF's.
#[cfg(target_os = "macos")]
fn is_heif(bytes: &[u8]) -> bool {
    const BRANDS: [&[u8]; 8] = [
        b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx", b"mif1", b"msf1",
    ];
    bytes.get(4..8) == Some(b"ftyp")
        && (bytes.get(8..12)).is_some_and(|brand| BRANDS.contains(&brand))
}

#[cfg(test)]
pub(crate) mod tests {}
