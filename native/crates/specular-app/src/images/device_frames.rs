//! The drawings of the realistic device frames, which are in the binary.
//!
//! Each is rastered once, large enough for a 2x display at 100%, and drawn
//! smaller from its mip levels. A page of the same device shares the one
//! texture.

use specular_core::PixelSize;
use specular_doc::DeviceArt;

/// Texels to a canvas unit in a raster.
const TEXELS_PER_UNIT: f64 = 2.0;

/// One drawing and the pixels to raster it at.
pub(crate) struct Drawing {
    pub(crate) svg: &'static [u8],
    pub(crate) want: PixelSize,
}

const fn svg(art: DeviceArt) -> &'static [u8] {
    match art {
        DeviceArt::IphoneSe => include_bytes!("../../assets/device-frames/iphone-se.svg"),
        DeviceArt::Iphone15Pro => include_bytes!("../../assets/device-frames/iphone-15-pro.svg"),
        DeviceArt::Iphone15ProMax => {
            include_bytes!("../../assets/device-frames/iphone-15-pro-max.svg")
        }
        DeviceArt::IpadPro11 => {
            include_bytes!("../../assets/device-frames/ipad-pro-11inches.svg")
        }
    }
}

/// The drawing `file` names, if it is a device frame's and not a file's.
pub(crate) fn drawing(file: &str) -> Option<Drawing> {
    let art = DeviceArt::from_asset(file)?;
    let (width, height) = art.size();
    let texels = |units: f64| ((units * TEXELS_PER_UNIT).ceil() as u32).max(1);
    Some(Drawing {
        svg: svg(art),
        want: PixelSize::new(texels(width), texels(height)),
    })
}

#[cfg(test)]
mod tests {
    use specular_doc::{DeviceFrameStyle, ShellInsets, VIEWPORT_PRESETS, device_shell};

    use super::super::svg;
    use super::*;

    const MANIFEST: &str = include_str!("../../assets/device-frames/manifest.json");

    fn manifest_row(art: DeviceArt) -> serde_json::Value {
        let rows: Vec<serde_json::Value> = serde_json::from_str(MANIFEST).unwrap();
        let row = rows.into_iter().find(|row| row["name"] == art.name());
        row.unwrap_or_else(|| panic!("the manifest has no {}", art.name()))
    }

    #[test]
    fn a_realistic_shells_insets_are_the_manifests_margins_at_the_presets_width() {
        for art in DeviceArt::ALL {
            let row = manifest_row(art);
            let number = |of: &str, key: &str| row[of][key].as_f64().unwrap();
            let geometry = art.geometry();
            let frame = (number("frame", "width"), number("frame", "height"));
            let origin = (number("screen", "x"), number("screen", "y"));
            let size = (number("screen", "width"), number("screen", "height"));
            assert_eq!(geometry.frame, frame, "{art:?}");
            assert_eq!(geometry.screen_origin, origin, "{art:?}");
            assert_eq!(geometry.screen_size, size, "{art:?}");
            assert_eq!(
                Some(geometry.screen_corner_radius),
                row["screenCornerRadius"].as_f64(),
                "{art:?}"
            );

            let preset = (VIEWPORT_PRESETS.iter())
                .find(|preset| preset.device_id == art.device_id())
                .unwrap();
            let scale = preset.width / size.0;
            let margins = ShellInsets {
                top: (origin.1 * scale).round(),
                right: ((frame.0 - origin.0 - size.0) * scale).round(),
                bottom: ((frame.1 - origin.1 - size.1) * scale).round(),
                left: (origin.0 * scale).round(),
            };
            let shell = device_shell(Some(art.device_id()), false, DeviceFrameStyle::Realistic);
            assert_eq!((shell.art, shell.insets), (Some(art), margins), "{art:?}");
            // The drawing's screen is the preset's shape, so the page fills it.
            let drawn_height = size.1 * scale;
            assert!(
                (drawn_height - preset.height).abs() < 1.0,
                "{art:?}: a screen {drawn_height} tall for a page {}",
                preset.height
            );
            // On its side the shell is turned a quarter turn clockwise.
            let turned = device_shell(Some(art.device_id()), true, DeviceFrameStyle::Realistic);
            assert_eq!(turned.insets.right, margins.top, "{art:?}");
            assert_eq!(turned.insets.top, margins.left, "{art:?}");
        }
    }

    #[test]
    fn each_drawing_rasters_at_two_texels_a_unit_or_just_over_with_a_hole_for_the_screen() {
        for art in DeviceArt::ALL {
            let drawing = drawing(art.asset()).unwrap();
            let rendered = svg::render(drawing.svg, Some(drawing.want), 8192).unwrap();
            let size = rendered.raster.size;
            // The raster keeps the drawing's shape, so a side may be a few
            // texels over what was asked for.
            let over = |got: u32, want: u32| got.checked_sub(want).is_some_and(|over| over < 4);
            assert!(over(size.width, drawing.want.width), "{art:?}: {size:?}");
            assert!(over(size.height, drawing.want.height), "{art:?}: {size:?}");
            let alpha =
                |x: u32, y: u32| rendered.raster.rgba[((y * size.width + x) * 4 + 3) as usize];
            // Two thirds down the screen is clear of any island or notch.
            let row = size.height * 2 / 3;
            assert_eq!(alpha(size.width / 2, row), 0, "{art:?}: the screen");
            let shell = device_shell(Some(art.device_id()), false, DeviceFrameStyle::Realistic);
            let in_bezel = (shell.insets.left * TEXELS_PER_UNIT * 0.75) as u32;
            assert_eq!(alpha(in_bezel, row), 255, "{art:?}: the bezel");
        }
    }
}
