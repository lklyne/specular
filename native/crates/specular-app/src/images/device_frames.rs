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

#[cfg(test)]
mod tests {
    use specular_doc::{
        DeviceFrameColor, DeviceFrameStyle, ShellInsets, VIEWPORT_PRESETS, ViewportPreset,
        device_shell,
    };

    use super::super::svg;
    use super::*;

    /// Every preset that has a drawing, with it.
    fn drawn() -> Vec<(&'static ViewportPreset, DeviceArt)> {
        (VIEWPORT_PRESETS.iter())
            .filter_map(|preset| Some((preset, DeviceArt::of_device(preset.device_id)?)))
            .collect()
    }

    /// Whether the preset's device stands wide, as a laptop does.
    fn wide(preset: &ViewportPreset) -> bool {
        preset.width > preset.height
    }

    const MANIFEST: &str = include_str!("../../assets/device-frames/manifest.json");

    fn manifest_row(art: DeviceArt) -> serde_json::Value {
        let rows: Vec<serde_json::Value> = serde_json::from_str(MANIFEST).unwrap();
        let row = rows.into_iter().find(|row| row["name"] == art.name());
        row.unwrap_or_else(|| panic!("the manifest has no {}", art.name()))
    }

    #[test]
    fn a_shells_insets_are_what_the_manifests_body_leaves_round_the_screen() {
        for (preset, art) in drawn() {
            let row = manifest_row(art);
            let light = row["lightFile"].as_str().unwrap_or(art.name());
            assert!(
                art.asset(DeviceFrameColor::Light).contains(light),
                "{art:?}: the light drawing is the manifest's"
            );
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

            let body = (number("body", "x"), number("body", "y"));
            let body_size = (number("body", "width"), number("body", "height"));
            assert_eq!(geometry.body_origin, body, "{art:?}");
            assert_eq!(geometry.body_size, body_size, "{art:?}");
            assert_eq!(
                Some(geometry.body_corner_radius),
                row["bodyCornerRadius"].as_f64(),
                "{art:?}"
            );

            // The shell is the body, which leaves the buttons outside it.
            let scale = preset.width / size.0;
            // Each inset is within a unit of the body's margin: sides that
            // are all but equal are rounded together.
            let margins = [
                (origin.1 - body.1) * scale,
                (body.0 + body_size.0 - origin.0 - size.0) * scale,
                (body.1 + body_size.1 - origin.1 - size.1) * scale,
                (origin.0 - body.0) * scale,
            ];
            let shell = device_shell(
                Some(preset.device_id),
                wide(preset),
                DeviceFrameStyle::Realistic,
            );
            let ShellInsets {
                top,
                right,
                bottom,
                left,
            } = shell.insets;
            assert_eq!(shell.art, Some(art));
            for (inset, margin) in [top, right, bottom, left].into_iter().zip(margins) {
                assert!(
                    (inset - margin).abs() < 1.0,
                    "{art:?}: {inset} for {margin}"
                );
                assert_eq!(inset.fract(), 0.0, "{art:?}");
            }
            // The drawing's screen is the preset's shape to within a few
            // units, which the dark behind the page and the bezel over it
            // take up.
            let drawn_height = size.1 * scale;
            assert!(
                (drawn_height - preset.height).abs() < 3.0,
                "{art:?}: a screen {drawn_height} tall for a page {}",
                preset.height
            );
            // On its side the shell is turned a quarter turn clockwise.
            let turned = device_shell(
                Some(preset.device_id),
                !wide(preset),
                DeviceFrameStyle::Realistic,
            );
            assert_eq!(turned.insets.right, top, "{art:?}");
            assert_eq!(turned.insets.top, left, "{art:?}");
        }
    }

    #[test]
    fn each_drawing_rasters_at_two_texels_a_unit_or_just_over_with_a_hole_for_the_screen() {
        use DeviceFrameColor::{Dark, Light};
        for (preset, art, color) in drawn()
            .into_iter()
            .flat_map(|(preset, art)| [(preset, art, Dark), (preset, art, Light)])
        {
            // A drawing shared by two presets, or by both colours, is
            // checked once.
            if DeviceArt::of_device(preset.device_id) != Some(art)
                || (color == Light && art.asset(Light) == art.asset(Dark))
            {
                continue;
            }
            let drawing = drawing(art.asset(color)).unwrap();
            let rendered = svg::render(drawing.svg, Some(drawing.want), 8192).unwrap();
            let size = rendered.raster.size;
            // The raster keeps the drawing's shape, so a side may be a few
            // texels over what was asked for.
            let over = |got: u32, want: u32| got.checked_sub(want).is_some_and(|over| over < 4);
            assert!(
                over(size.width, drawing.want.width),
                "{art:?} {color:?}: {size:?}"
            );
            assert!(
                over(size.height, drawing.want.height),
                "{art:?} {color:?}: {size:?}"
            );
            let texel = |x: u32, y: u32| {
                let at = ((y * size.width + x) * 4) as usize;
                &rendered.raster.rgba[at..at + 4]
            };
            // Two thirds down the screen is clear of any island or notch.
            let row = size.height * 2 / 3;
            assert_eq!(
                texel(size.width / 2, row)[3],
                0,
                "{art:?} {color:?}: the screen"
            );
            let shell = device_shell(
                Some(preset.device_id),
                wide(preset),
                DeviceFrameStyle::Realistic,
            );
            let texels_a_unit = art.size().0 / art.geometry().frame.0 * TEXELS_PER_UNIT;
            // The body starts in from the drawing's edge by what sticks out
            // of it: buttons, or a laptop's base.
            let body_left = art.geometry().body_origin.0 * texels_a_unit;
            let in_bezel = (body_left + shell.insets.left * TEXELS_PER_UNIT * 0.75) as u32;
            assert_eq!(texel(in_bezel, row)[3], 255, "{art:?} {color:?}: the bezel");
            // The body's edge, three units in from its outline, is pale on
            // a light device.
            let in_edge = ((art.geometry().body_origin.0 + 3.0) * texels_a_unit) as u32;
            let edge = texel(in_edge, row);
            assert!(
                color == Dark || edge[0] > 180,
                "{art:?} {color:?}: an edge of {edge:?}"
            );
        }
    }
}
