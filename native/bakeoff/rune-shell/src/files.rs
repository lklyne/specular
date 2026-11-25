//! Image files to pixels, as `specular-shell/src/images/` does it, without
//! its decode thread or file watching: PNG, JPEG, WebP, gif frames and svg.
//!
//! Copied in outline because the shell's loader is private to it. Nothing
//! here needs a GPU: the pixels wait in the [`Shell`](crate::shell::Shell)
//! until a compositor exists to upload them.

use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder as _, DynamicImage, ImageDecoder as _, ImageReader};
use resvg::{tiny_skia, usvg};
use specular_core::PixelSize;
use specular_interact::ImageNotice;

/// The longest side an svg is drawn at, as in the shell.
const SVG_MAX_SIDE: u32 = 4096;
/// What a gif frame with a delay of 10 ms or less is shown for.
const DEFAULT_DELAY_MS: u32 = 100;

/// Straight-alpha sRGB pixels, tightly packed.
#[derive(Debug, Clone)]
pub struct Pixels {
    /// Their size.
    pub size: PixelSize,
    /// Four bytes a pixel.
    pub rgba: Vec<u8>,
}

/// A decoded image: its frames (one unless it is an animated gif) and what
/// the app is told about it.
#[derive(Debug, Clone)]
pub struct Decoded {
    /// The frames, in order.
    pub frames: Arc<[Pixels]>,
    /// What to tell the app.
    pub notice: ImageNotice,
}

/// Why a file gave no image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// There is no such file.
    Missing,
    /// It could not be read or decoded.
    Failed,
}

impl Failure {
    /// What to tell the app.
    pub const fn notice(self) -> ImageNotice {
        match self {
            Self::Missing => ImageNotice::Missing,
            Self::Failed => ImageNotice::Failed,
        }
    }
}

/// Reads and decodes the image at `path`. An svg is drawn to fill `want`
/// pixels, or at its own size with none.
pub fn load(path: &Path, want: Option<PixelSize>) -> Result<Decoded, Failure> {
    let bytes = std::fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Failure::Missing
        } else {
            Failure::Failed
        }
    })?;
    let is_svg = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));
    if is_svg {
        return svg(&bytes, want).ok_or(Failure::Failed);
    }
    if bytes.starts_with(b"GIF8")
        && let Some(animated) = gif(&bytes)
    {
        return Ok(animated);
    }
    still(&bytes).ok_or(Failure::Failed)
}

fn still(bytes: &[u8]) -> Option<Decoded> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    let orientation = decoder.orientation().ok()?;
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    let rgba = image.into_rgba8();
    let size = PixelSize::new(rgba.width(), rgba.height());
    Some(Decoded {
        frames: Arc::from([Pixels {
            size,
            rgba: rgba.into_raw(),
        }]),
        notice: ImageNotice::Ready {
            width: size.width,
            height: size.height,
        },
    })
}

/// Every frame of an animated gif. `None` for one with a single frame,
/// which is a still.
fn gif(bytes: &[u8]) -> Option<Decoded> {
    let decoder = GifDecoder::new(Cursor::new(bytes)).ok()?;
    let (width, height) = decoder.dimensions();
    let mut frames = Vec::new();
    let mut delays_ms = Vec::new();
    for frame in decoder.into_frames() {
        let frame = frame.ok()?;
        let (millis, per) = frame.delay().numer_denom_ms();
        let delay = millis / per.max(1);
        delays_ms.push(if delay <= 10 { DEFAULT_DELAY_MS } else { delay });
        frames.push(Pixels {
            size: PixelSize::new(width, height),
            rgba: frame.into_buffer().into_raw(),
        });
    }
    if frames.len() < 2 {
        return None;
    }
    Some(Decoded {
        frames: frames.into(),
        notice: ImageNotice::Animated {
            width,
            height,
            delays_ms: delays_ms.into(),
        },
    })
}

fn svg_fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

fn svg(bytes: &[u8], want: Option<PixelSize>) -> Option<Decoded> {
    let options = usvg::Options {
        fontdb: svg_fonts(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options).ok()?;
    let size = tree.size();
    let intrinsic = PixelSize::new(
        (size.width().round() as u32).max(1),
        (size.height().round() as u32).max(1),
    );
    let want = want.unwrap_or(intrinsic);
    let mut scale = (want.width as f32 / size.width()).max(want.height as f32 / size.height());
    let longest = size.width().max(size.height()) * scale;
    if longest > SVG_MAX_SIDE as f32 {
        scale *= SVG_MAX_SIDE as f32 / longest;
    }
    let width = ((size.width() * scale).ceil() as u32).max(1);
    let height = ((size.height() * scale).ceil() as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let rgba = (pixmap.pixels().iter())
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Some(Decoded {
        frames: Arc::from([Pixels {
            size: PixelSize::new(width, height),
            rgba,
        }]),
        notice: ImageNotice::Ready {
            width: intrinsic.width,
            height: intrinsic.height,
        },
    })
}
