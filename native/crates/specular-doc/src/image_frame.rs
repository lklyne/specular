//! Device frames round images: a screenshot shown on the screen of the
//! device it was taken on.
//!
//! An image keeps its frame in its `metadata` under the keys a page uses
//! (`showDeviceFrame`, `deviceId`, `deviceFrameStyle`, `deviceFrameColor`),
//! and is drawn in the same shells and drawings. Two things differ. The
//! image's rect is its screen at whatever size it was placed, so the shell
//! is scaled to it; and the image lies on its side when it is wider than
//! tall, so it keeps no orientation. The node is placed by its screen in
//! the `.canvas`, as every file node is.

use serde_json::Value;

use crate::{
    DeviceArt, DeviceFrameColor, DeviceFrameStyle, DeviceShell, Entity, FileRef, JsonMap, Kind,
    PRESET_MENU, Rect, ShellInsets, VIEWPORT_PRESETS, ViewportPreset, device_shell,
};

/// How near an image's shape must be to a device's screen for the image
/// to be taken as that device's screenshot, as a share of the screen's
/// proportions.
const NEAR_SHAPE: f64 = 0.05;
/// How near it must be for the device's drawing to fit round it: the
/// drawing's screen is the preset's shape, and the image fills its hole.
const DRAWING_SHAPE: f64 = 0.01;
/// The screen width a frame with no device is drawn for, so its bezel is
/// as thick on a large image as on a phone-sized one.
const UNNAMED_SCREEN: f64 = 390.0;

impl FileRef {
    /// Whether the file is drawn as an image: Electron's
    /// `IMAGE_EXTENSIONS`, and an iPhone's photos, which macOS decodes.
    pub fn is_image(&self) -> bool {
        is_image_path(&self.file)
    }

    /// The device frame an image file is drawn in round `rect`, its
    /// screen, or `None` when its `showDeviceFrame` is off or it is no
    /// image. A realistic frame names its drawing only when the image is
    /// the shape of the drawing's screen.
    pub fn image_shell(&self, rect: Rect) -> Option<DeviceShell> {
        let meta = self.metadata.as_ref().filter(|_| self.is_image())?;
        if meta.get("showDeviceFrame") != Some(&Value::Bool(true)) {
            return None;
        }
        let landscape = rect.width > rect.height;
        let Some(preset) = preset_of(meta) else {
            let scale = rect.width.min(rect.height) / UNNAMED_SCREEN;
            return Some(device_shell(None, landscape, DeviceFrameStyle::Clay).scaled(scale));
        };
        let (across, _) = upright(rect, preset);
        let style = if self.image_art(rect).is_some() {
            frame_style_in(meta)
        } else {
            DeviceFrameStyle::Clay
        };
        let shell = device_shell(Some(preset.device_id), landscape, style);
        Some(shell.scaled(across / preset.width))
    }

    /// The drawing an image file can be framed in round `rect`: its
    /// device's, when the image is the shape of the drawing's screen.
    pub fn image_art(&self, rect: Rect) -> Option<DeviceArt> {
        let meta = self.metadata.as_ref().filter(|_| self.is_image())?;
        let preset = preset_of(meta)?;
        let (across, down) = upright(rect, preset);
        let art = DeviceArt::of_device(preset.device_id)?;
        let off = (across / down) / (preset.width / preset.height) - 1.0;
        (off.abs() <= DRAWING_SHAPE).then_some(art)
    }
}

impl Entity {
    /// The device frame drawn round the entity's rect: a framed page's or
    /// HTML file's, or a framed image's scaled to it. `None` for an entity
    /// with no frame shown.
    pub fn shell(&self) -> Option<DeviceShell> {
        match &self.kind {
            Kind::File(file) if file.is_image() => file.image_shell(self.rect),
            kind @ (Kind::Page(_) | Kind::File(_)) => kind.page_fields()?.shell(),
            Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
        }
    }

    /// The realistic drawing the entity can be framed in, whether or not
    /// its frame is shown or realistic.
    pub fn device_art(&self) -> Option<DeviceArt> {
        match &self.kind {
            Kind::File(file) if file.is_image() => file.image_art(self.rect),
            kind @ (Kind::Page(_) | Kind::File(_)) => kind.page_fields()?.device_art(),
            Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
        }
    }

    /// The colour the entity keeps its device in, or `None` for one that
    /// follows the app's theme or has no frame to colour.
    pub fn frame_color(&self) -> Option<DeviceFrameColor> {
        match &self.kind {
            Kind::File(file) if file.is_image() => frame_color_in(file.metadata.as_ref()?),
            kind @ (Kind::Page(_) | Kind::File(_)) => kind.page_fields()?.frame_color(),
            Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
        }
    }
}

impl DeviceShell {
    /// The shell made `scale` times as large: its bezel and both corners.
    #[must_use]
    pub fn scaled(self, scale: f64) -> Self {
        let ShellInsets {
            top,
            right,
            bottom,
            left,
        } = self.insets;
        Self {
            insets: ShellInsets {
                top: top * scale,
                right: right * scale,
                bottom: bottom * scale,
                left: left * scale,
            },
            corner_radius: self.corner_radius * scale,
            screen_corner_radius: self.screen_corner_radius * scale,
            ..self
        }
    }
}

/// Whether `file` names an image by its extension.
pub fn is_image_path(file: &str) -> bool {
    const EXTENSIONS: [&str; 10] = [
        "png", "jpg", "jpeg", "gif", "svg", "webp", "bmp", "ico", "heic", "heif",
    ];
    file.rsplit_once('.').is_some_and(|(_, extension)| {
        (EXTENSIONS.iter()).any(|known| extension.eq_ignore_ascii_case(known))
    })
}

/// The device a screenshot `width` by `height` pixels was most likely taken
/// on, as its preset, or `None` when it is no device's shape.
///
/// A screenshot is its screen's points at one, two or three pixels a
/// point, so a size that is a preset's at one of those scales, either way
/// up, is that device. Any other is the device whose screen is nearest its
/// shape, if one is within a few percent. Only the presets the size menu
/// offers are tried.
pub fn screenshot_device(width: f64, height: f64) -> Option<&'static ViewportPreset> {
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let (short, long) = (width.min(height), width.max(height));
    let offered = || {
        (PRESET_MENU.iter())
            .flat_map(|(_, rows)| rows.iter())
            .filter_map(|&index| VIEWPORT_PRESETS.get(usize::try_from(index).ok()?))
    };
    let sides = |preset: &ViewportPreset| {
        (
            preset.width.min(preset.height),
            preset.width.max(preset.height),
        )
    };
    let exact = offered().find(|preset| {
        let (preset_short, preset_long) = sides(preset);
        [1.0, 2.0, 3.0].into_iter().any(|scale| {
            (short / scale - preset_short).abs() < 1.0 && (long / scale - preset_long).abs() < 1.0
        })
    });
    let off = |preset: &ViewportPreset| {
        let (preset_short, preset_long) = sides(preset);
        ((short / long) / (preset_short / preset_long) - 1.0).abs()
    };
    exact.or_else(|| {
        offered()
            .filter(|preset| off(preset) <= NEAR_SHAPE)
            .min_by(|a, b| off(a).total_cmp(&off(b)))
    })
}

/// The style `meta` keeps a frame in.
pub(crate) fn frame_style_in(meta: &JsonMap) -> DeviceFrameStyle {
    DeviceFrameStyle::from_name(meta.get("deviceFrameStyle").and_then(Value::as_str))
}

/// The colour `meta` keeps a device in, or `None` to follow the theme.
pub(crate) fn frame_color_in(meta: &JsonMap) -> Option<DeviceFrameColor> {
    DeviceFrameColor::from_name(meta.get("deviceFrameColor").and_then(Value::as_str))
}

/// The preset `meta`'s `deviceId` names.
fn preset_of(meta: &JsonMap) -> Option<&'static ViewportPreset> {
    let device = meta.get("deviceId").and_then(Value::as_str)?;
    (VIEWPORT_PRESETS.iter()).find(|preset| preset.device_id == device)
}

/// `rect`'s sides as `preset`'s screen stands: its width across the
/// preset's width, whichever way up each is.
fn upright(rect: Rect, preset: &ViewportPreset) -> (f64, f64) {
    let same_way = (rect.width > rect.height) == (preset.width > preset.height);
    if same_way {
        (rect.width, rect.height)
    } else {
        (rect.height, rect.width)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn device(width: f64, height: f64) -> Option<&'static str> {
        screenshot_device(width, height).map(|preset| preset.device_id)
    }

    #[test]
    fn a_screenshot_is_the_device_whose_screen_it_is_at_one_to_three_pixels_a_point() {
        let cases = [
            // iPad Pro 13 at 2x, standing and on its side.
            ((2064.0, 2752.0), Some("ipad-pro-13")),
            ((2752.0, 2064.0), Some("ipad-pro-13")),
            // iPhone 17 and iPhone 16 are a third of a percent apart in
            // shape; their pixels tell them apart.
            ((1206.0, 2622.0), Some("iphone-17")),
            ((1179.0, 2556.0), Some("iphone-14-pro")),
            // A laptop at 2x, and an iPad mini at 1x.
            ((2560.0, 1600.0), Some("laptop")),
            ((744.0, 1133.0), Some("ipad-mini")),
            // A screenshot scaled down keeps its device's shape.
            ((1032.0, 1376.0), Some("ipad-pro-13")),
            ((516.0, 688.0), Some("ipad-pro-13")),
            // A square is no device.
            ((800.0, 800.0), None),
            ((0.0, 800.0), None),
        ];
        for ((width, height), want) in cases {
            assert_eq!(device(width, height), want, "{width} by {height}");
        }
    }

    fn image(rect: Rect, meta: &serde_json::Value) -> Entity {
        let file = FileRef {
            file: "shot.png".to_owned(),
            metadata: meta.as_object().cloned(),
            ..FileRef::default()
        };
        Entity::new("img", rect, Kind::File(file))
    }

    #[test]
    fn a_framed_images_shell_is_its_devices_scaled_to_the_image() {
        let pad = |rect, style: &str| {
            image(
                rect,
                &json!({"showDeviceFrame": true, "deviceId": "ipad-pro-13", "deviceFrameStyle": style}),
            )
        };
        let at_one = device_shell(Some("ipad-pro-13"), false, DeviceFrameStyle::Realistic);
        // Twice the preset's screen: twice the bezel, and the drawing.
        let shell = pad(Rect::new(0.0, 0.0, 2064.0, 2752.0), "realistic").shell();
        assert_eq!(shell, Some(at_one.scaled(2.0)));
        assert_eq!(
            shell.and_then(|shell| shell.art),
            Some(DeviceArt::IpadPro13InchesM4)
        );
        // On its side the shell turns, at the same scale.
        let turned = pad(Rect::new(0.0, 0.0, 2752.0, 2064.0), "realistic").shell();
        assert_eq!(
            turned.map(|shell| (shell.landscape, shell.insets.top)),
            Some((true, at_one.insets.left * 2.0))
        );
        // Stretched off the drawing's shape, it is drawn in clay.
        let stretched = pad(Rect::new(0.0, 0.0, 2064.0, 2400.0), "realistic").shell();
        assert_eq!(stretched.map(|shell| shell.art), Some(None));
        // Off, or no image, is no frame.
        let off = image(
            Rect::new(0.0, 0.0, 2064.0, 2752.0),
            &json!({"showDeviceFrame": false, "deviceId": "ipad-pro-13"}),
        );
        assert_eq!(off.shell(), None);
        let mut note = pad(Rect::new(0.0, 0.0, 2064.0, 2752.0), "clay");
        if let Kind::File(file) = &mut note.kind {
            file.file = "notes.md".to_owned();
        }
        assert_eq!(note.shell(), None);
    }
}
