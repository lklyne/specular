//! The drawings of the realistic device frames, which are in the binary.
//!
//! Each device has a dark drawing and a light one of the same shape. A
//! drawing is rastered once, large enough for a 2x display at 100%, and
//! drawn smaller from its mip levels. Pages of the same device and colour
//! share the one texture.

mod drawings;

use specular_core::PixelSize;
use specular_doc::DeviceArt;

/// Texels to a canvas unit in a raster.
const TEXELS_PER_UNIT: f64 = 2.0;

/// One drawing and the pixels to raster it at.
pub(crate) struct Drawing {
    pub(crate) svg: &'static [u8],
    pub(crate) want: PixelSize,
}

/// The drawing `file` names, if it is a device frame's and not a file's.
pub(crate) fn drawing(file: &str) -> Option<Drawing> {
    let (art, color) = DeviceArt::from_asset(file)?;
    let (width, height) = art.size();
    let texels = |units: f64| ((units * TEXELS_PER_UNIT).ceil() as u32).max(1);
    Some(Drawing {
        svg: drawings::svg(art, color),
        want: PixelSize::new(texels(width), texels(height)),
    })
}
