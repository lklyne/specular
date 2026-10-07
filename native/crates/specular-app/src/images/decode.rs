//! File bytes to pixels: PNG, JPEG, WebP and the first frame of a GIF.

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

#[cfg(test)]
pub(crate) mod tests {
    use image::{ExtendedColorType, ImageEncoder as _, ImageFormat, RgbaImage};

    use super::*;

    /// A `width` by `height` image, red on its left half and blue on its
    /// right, in `format`.
    pub(crate) fn encoded(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let image = RgbaImage::from_fn(width, height, |x, _| {
            if x < width / 2 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 0, 255, 255])
            }
        });
        let mut bytes = Vec::new();
        match format {
            // JPEG has no alpha channel to encode.
            ImageFormat::Jpeg => {
                let rgb = DynamicImage::ImageRgba8(image).into_rgb8();
                image::codecs::jpeg::JpegEncoder::new(&mut bytes)
                    .write_image(&rgb, width, height, ExtendedColorType::Rgb8)
                    .unwrap();
            }
            _ => DynamicImage::ImageRgba8(image)
                .write_to(&mut Cursor::new(&mut bytes), format)
                .unwrap(),
        }
        bytes
    }

    fn texel(decoded: &Decoded, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * decoded.size.width + x) * 4) as usize;
        [0, 1, 2, 3].map(|channel| decoded.rgba[at + channel])
    }

    #[test]
    fn lossless_formats_decode_to_their_exact_pixels() {
        for format in [ImageFormat::Png, ImageFormat::WebP, ImageFormat::Gif] {
            let decoded = decode(&encoded(8, 4, format), 8192).unwrap();
            assert_eq!(decoded.size, PixelSize::new(8, 4), "{format:?}");
            assert_eq!(decoded.rgba.len(), 8 * 4 * 4, "{format:?}");
            assert_eq!(texel(&decoded, 1, 1), [255, 0, 0, 255], "{format:?}");
            assert_eq!(texel(&decoded, 6, 2), [0, 0, 255, 255], "{format:?}");
        }
    }

    #[test]
    fn a_jpeg_decodes_to_nearly_its_pixels() {
        let decoded = decode(&encoded(16, 8, ImageFormat::Jpeg), 8192).unwrap();
        assert_eq!(decoded.size, PixelSize::new(16, 8));
        let [red, _, blue, alpha] = texel(&decoded, 2, 4);
        assert!(red > 200 && blue < 60 && alpha == 255);
    }

    #[test]
    fn the_format_comes_from_the_content_not_the_name() {
        // Nothing here says PNG but the bytes.
        assert!(decode(&encoded(2, 2, ImageFormat::Png), 8192).is_ok());
    }

    #[test]
    fn an_image_larger_than_a_texture_is_scaled_down_in_proportion() {
        let decoded = decode(&encoded(64, 16, ImageFormat::Png), 32).unwrap();
        assert_eq!(decoded.size, PixelSize::new(32, 8));
        assert_eq!(decoded.rgba.len(), 32 * 8 * 4);
    }

    #[test]
    fn bytes_that_are_no_image_fail() {
        assert!(decode(b"<svg xmlns='http://www.w3.org/2000/svg'/>", 8192).is_err());
        assert!(decode(b"", 8192).is_err());
        let png = encoded(8, 8, ImageFormat::Png);
        assert!(decode(&png[..png.len() / 2], 8192).is_err());
    }
}
