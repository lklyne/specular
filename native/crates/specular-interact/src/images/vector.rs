//! SVG files: the shell rasters one at the size it is drawn at, so it stays
//! sharp at any zoom, and rasters it again when the zoom has left a band
//! around that size. Raster sizes are in logical pixels; the shell scales
//! them to the window's pixels.

use specular_core::PixelSize;
use specular_doc::{FileRef, ObjectFit};

use super::{ImageState, shown};
use crate::{App, Effect};

/// A raster is kept while the size drawn is between these fractions of it.
/// Below the band it is sharper than it needs to be, above it soft.
const KEEP_FROM: f32 = 0.75;
const KEEP_TO: f32 = 1.5;
/// The widest raster asked for, in logical pixels.
const MAX_WIDTH: f32 = 4096.0;

/// Whether `file` is an svg.
pub(super) fn is_svg(file: &str) -> bool {
    file.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("svg"))
}

/// The width the intrinsic image `width` by `height` is drawn at in a
/// canvas rect of `rect_width` by `rect_height`, at `zoom`.
fn drawn_width(file: &FileRef, rect: (f64, f64), image: (u32, u32), zoom: f32) -> f32 {
    let (width, height) = (image.0 as f32, image.1 as f32);
    let (across, down) = (rect.0 as f32 * zoom / width, rect.1 as f32 * zoom / height);
    let scale = match file.object_fit.unwrap_or(ObjectFit::Contain) {
        ObjectFit::Contain => across.min(down),
        ObjectFit::Cover | ObjectFit::Fill => across.max(down),
    };
    width * scale
}

/// Asks for a new raster of each svg on screen whose drawn size is outside
/// the band of its last one.
///
/// Nothing is asked for while the zoom is moving: it has to be the same at
/// two looks in a row, so a pinch or a wheel gesture rasters once, when it
/// ends.
pub(super) fn request(app: &mut App, effects: &mut Vec<Effect>) {
    let zoom = app.session.camera.zoom;
    let seen = std::mem::replace(&mut app.session.images.zoom_seen, zoom);
    let held = (seen - zoom).abs() <= f32::EPSILON;
    if !held {
        return;
    }
    // The widest each svg is drawn, by file.
    let mut wanted: Vec<(String, f32)> = Vec::new();
    for (entity, file) in shown(app) {
        if !is_svg(&file.file) {
            continue;
        }
        let Some(image) = app.session.images.get(&file.file) else {
            continue;
        };
        let ImageState::Ready { width, height } = image.state else {
            continue;
        };
        let rect = (entity.rect.width, entity.rect.height);
        let width = drawn_width(file, rect, (width, height), zoom).clamp(1.0, MAX_WIDTH);
        match wanted.iter_mut().find(|(known, _)| *known == file.file) {
            Some((_, widest)) => *widest = widest.max(width),
            None => wanted.push((file.file.clone(), width)),
        }
    }
    for (file, width) in wanted {
        let Some(image) = app.session.images.by_file.get_mut(&file) else {
            continue;
        };
        let ImageState::Ready {
            width: intrinsic_width,
            height: intrinsic_height,
        } = image.state
        else {
            continue;
        };
        let width = width.ceil() as u32;
        let kept = image.raster_width.is_some_and(|raster| {
            let ratio = width as f32 / raster as f32;
            (KEEP_FROM..=KEEP_TO).contains(&ratio)
        });
        if kept {
            continue;
        }
        image.raster_width = Some(width);
        let height = (width as f32 * intrinsic_height as f32 / intrinsic_width as f32).ceil();
        effects.push(Effect::RasterImage {
            image: image.key,
            file,
            size: PixelSize::new(width, height.max(1.0) as u32),
        });
    }
}
