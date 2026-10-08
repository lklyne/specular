//! Animated gifs: every frame at full canvas size, with its delay.

use std::io::Cursor;

use image::codecs::gif::GifDecoder;
use image::imageops::{self, FilterType};
use image::{AnimationDecoder as _, ImageDecoder as _, RgbaImage};
use specular_core::PixelSize;

/// The most pixel data one gif's frames may hold, 64 MiB of RGBA. A gif over
/// it is scaled down, every frame alike, as its frames are read, so a long
/// 1080p gif never holds more than this while it decodes.
const MAX_BYTES: u64 = 64 << 20;
/// What a browser shows a frame with a delay of 10 ms or less for: they are
/// written by encoders that mean "as fast as you can".
const DEFAULT_DELAY_MS: u32 = 100;

/// The frames of an animated gif.
#[derive(Debug)]
pub(crate) struct Frames {
    /// The size the gif is drawn at, whatever the frames were scaled to.
    pub(crate) size: PixelSize,
    /// The size of every frame.
    pub(crate) frame_size: PixelSize,
    /// Straight RGBA, one buffer a frame.
    pub(crate) rgba: Vec<Vec<u8>>,
    /// How long each frame is shown.
    pub(crate) delays_ms: Vec<u32>,
}

/// Whether `bytes` start like a gif.
pub(crate) fn is_gif(bytes: &[u8]) -> bool {
    bytes.starts_with(b"GIF8")
}

/// Every frame of `bytes`, or `None` when it has fewer than two and is just
/// a picture.
pub(crate) fn decode(bytes: &[u8]) -> Result<Option<Frames>, image::ImageError> {
    let decoder = GifDecoder::new(Cursor::new(bytes))?;
    let (width, height) = decoder.dimensions();
    let size = PixelSize::new(width, height);
    let mut current = size;
    let mut frames: Vec<RgbaImage> = Vec::new();
    let mut delays_ms = Vec::new();
    for frame in decoder.into_frames() {
        let frame = frame?;
        let (millis, per) = frame.delay().numer_denom_ms();
        let delay = millis / per.max(1);
        delays_ms.push(if delay <= 10 { DEFAULT_DELAY_MS } else { delay });
        frames.push(resized(frame.into_buffer(), current));
        let held: u64 = frames.iter().map(|frame| frame.as_raw().len() as u64).sum();
        if held > MAX_BYTES {
            let shrink = (MAX_BYTES as f64 / held as f64).sqrt() * 0.95;
            current = PixelSize::new(
                ((f64::from(current.width) * shrink) as u32).max(1),
                ((f64::from(current.height) * shrink) as u32).max(1),
            );
            frames = frames.into_iter().map(|f| resized(f, current)).collect();
        }
    }
    if frames.len() < 2 {
        return Ok(None);
    }
    Ok(Some(Frames {
        size,
        frame_size: current,
        rgba: frames.into_iter().map(RgbaImage::into_raw).collect(),
        delays_ms,
    }))
}

fn resized(frame: RgbaImage, to: PixelSize) -> RgbaImage {
    if frame.dimensions() == (to.width, to.height) {
        return frame;
    }
    imageops::resize(&frame, to.width, to.height, FilterType::Triangle)
}

#[cfg(test)]
mod tests {
    use image::codecs::gif::GifEncoder;
    use image::{Delay, Frame, Rgba};

    use super::*;

    fn gif(delays_ms: &[u32]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = GifEncoder::new(&mut bytes);
        for (index, &delay) in delays_ms.iter().enumerate() {
            let shade = (index as u8 + 1) * 60;
            let buffer = RgbaImage::from_pixel(4, 2, Rgba([shade, 0, 0, 255]));
            let delay = Delay::from_numer_denom_ms(delay, 1);
            encoder
                .encode_frame(Frame::from_parts(buffer, 0, 0, delay))
                .unwrap();
        }
        drop(encoder);
        bytes
    }

    #[test]
    fn every_frame_comes_with_its_delay_and_a_quick_one_is_slowed_to_a_browsers() {
        let decoded = decode(&gif(&[200, 0, 50])).unwrap().unwrap();
        assert_eq!(decoded.size, PixelSize::new(4, 2));
        assert_eq!(decoded.delays_ms, [200, DEFAULT_DELAY_MS, 50]);
        let reds: Vec<u8> = decoded.rgba.iter().map(|frame| frame[0]).collect();
        assert_eq!(reds, [60, 120, 180]);
    }

    #[test]
    fn a_gif_of_one_frame_is_a_picture() {
        assert!(decode(&gif(&[100])).unwrap().is_none());
    }
}
