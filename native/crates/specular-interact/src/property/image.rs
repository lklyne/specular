//! The device frame of an image: a screenshot shown on its device's
//! screen. It is kept in the image's `metadata` under the page's keys, and
//! the device is the one the image's size says it was taken on.

use serde_json::json;
use specular_doc::{DeviceFrameStyle, FileRef, JsonMap, Rect, screenshot_device};

use super::Property;

const DEVICE_ID: &str = "deviceId";
const FRAME: &str = "showDeviceFrame";
const FRAME_STYLE: &str = "deviceFrameStyle";
const FRAME_COLOR: &str = "deviceFrameColor";

/// Applies `property` to the image `file` shown in `rect`. Returns whether
/// it applies to images. The image stays where it is and the frame goes
/// round it.
pub(super) fn set(property: &Property, file: &mut FileRef, rect: Rect) -> bool {
    if !file.is_image() {
        return false;
    }
    match property {
        Property::DeviceFrame(style) => {
            let shown = file.image_shell(rect).is_some();
            let meta = file.metadata.get_or_insert_with(JsonMap::new);
            meta.insert(FRAME.to_owned(), json!(style.is_some()));
            // A frame put on finds the device again, as the image may have
            // been resized since. One of no device's shape has none.
            if style.is_some() && !shown {
                match screenshot_device(rect.width, rect.height) {
                    Some(device) => {
                        meta.insert(DEVICE_ID.to_owned(), json!(device.device_id));
                    }
                    None => {
                        meta.remove(DEVICE_ID);
                    }
                }
            }
            match style {
                None => {}
                Some(DeviceFrameStyle::Clay) => {
                    meta.remove(FRAME_STYLE);
                }
                Some(style @ DeviceFrameStyle::Realistic) => {
                    meta.insert(FRAME_STYLE.to_owned(), json!(style.name()));
                }
            }
        }
        Property::DeviceFrameColor(color) => {
            let meta = file.metadata.get_or_insert_with(JsonMap::new);
            meta.insert(FRAME_COLOR.to_owned(), json!(color.name()));
        }
        Property::Color(_)
        | Property::BorderColor(_)
        | Property::TextSize(_)
        | Property::TextFont(_)
        | Property::TextStyle(_)
        | Property::TextAlign(_)
        | Property::TextVerticalAlign(_)
        | Property::ShapeKind(_)
        | Property::FillStyle(_)
        | Property::BorderStyle(_)
        | Property::StrokeWidth(_)
        | Property::Brush(_)
        | Property::LineStyle(_)
        | Property::Label(_)
        | Property::FromEnd(_)
        | Property::ToEnd(_)
        | Property::ViewportPreset(_)
        | Property::CustomViewport
        | Property::ViewportWidth(_)
        | Property::ViewportHeight(_)
        | Property::Orientation(_)
        | Property::ColorScheme(_) => return false,
    }
    true
}
