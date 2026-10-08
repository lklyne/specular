//! SVG files to pixels, drawn at the size they are wanted at.

use std::sync::{Arc, OnceLock};

use resvg::{tiny_skia, usvg};
use specular_core::PixelSize;

use super::decode::Decoded;

/// The longest side of a raster, in pixels. Zoomed in further than this the
/// raster is stretched, not made larger.
const MAX_SIDE: u32 = 4096;

/// An svg drawn into pixels.
#[derive(Debug)]
pub(crate) struct Rendered {
    /// The size the svg's `width`, `height` and `viewBox` give it.
    pub(crate) intrinsic: PixelSize,
    pub(crate) raster: Decoded,
}

/// Whether `path` names an svg.
pub(crate) fn is_svg(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
}

/// The system's fonts, loaded the first time an svg is drawn.
fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = usvg::fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

/// Draws `bytes` to fill `want` pixels, or at its intrinsic size when there
/// is none. The raster keeps the svg's proportions and is at most
/// `max_dimension` (and [`MAX_SIDE`]) on its longer side.
pub(crate) fn render(
    bytes: &[u8],
    want: Option<PixelSize>,
    max_dimension: u32,
) -> Result<Rendered, String> {
    let options = usvg::Options {
        fontdb: fonts(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options).map_err(|error| error.to_string())?;
    let size = tree.size();
    let intrinsic = PixelSize::new(
        (size.width().round() as u32).max(1),
        (size.height().round() as u32).max(1),
    );
    let want = want.unwrap_or(intrinsic);
    let mut scale = (want.width as f32 / size.width()).max(want.height as f32 / size.height());
    let longest = size.width().max(size.height()) * scale;
    let cap = MAX_SIDE.min(max_dimension) as f32;
    if longest > cap {
        scale *= cap / longest;
    }
    let width = ((size.width() * scale).ceil() as u32).max(1);
    let height = ((size.height() * scale).ceil() as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("the raster has no area")?;
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / size.width(),
        height as f32 / size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Ok(Rendered {
        intrinsic,
        raster: Decoded {
            size: PixelSize::new(width, height),
            rgba,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40" viewBox="0 0 80 40"><rect width="80" height="40" fill="#f00"/></svg>"##;

    #[test]
    fn the_intrinsic_size_is_the_svgs_and_the_raster_is_the_size_wanted() {
        let at_home = render(SVG, None, 8192).unwrap();
        assert_eq!(at_home.intrinsic, PixelSize::new(80, 40));
        assert_eq!(at_home.raster.size, PixelSize::new(80, 40));
        let zoomed = render(SVG, Some(PixelSize::new(400, 200)), 8192).unwrap();
        assert_eq!(zoomed.intrinsic, PixelSize::new(80, 40));
        assert_eq!(zoomed.raster.size, PixelSize::new(400, 200));
        assert_eq!(&zoomed.raster.rgba[..4], &[255, 0, 0, 255]);
        let capped = render(SVG, Some(PixelSize::new(80_000, 40_000)), 8192).unwrap();
        assert_eq!(capped.raster.size, PixelSize::new(MAX_SIDE, MAX_SIDE / 2));
    }
}
