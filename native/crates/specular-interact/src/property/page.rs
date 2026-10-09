//! The page properties: the viewport preset and what Electron writes beside
//! it. A page's device, orientation and frame live in `metadata`, next to a
//! custom size.

use serde_json::{Value, json};
use specular_doc::{JsonMap, Page, Rect, preset};

use super::{Orientation, Property};

const DEVICE_ID: &str = "deviceId";
const ORIENTATION: &str = "deviceOrientation";
const FRAME: &str = "showDeviceFrame";
const SIZE_MODE: &str = "pageSizeMode";
const CUSTOM_SIZE: &str = "customSize";
/// The name a custom size had before it was called custom.
const LEGACY_CUSTOM_SIZE: &str = "responsiveSize";

/// Applies `property` to a page and its rect. Returns whether it applies to
/// pages.
pub(super) fn set(property: &Property, page: &mut Page, rect: &mut Rect) -> bool {
    let shell_before = shell_offset(page);
    let applies = apply(property, page, rect);
    // The shell's corner stays where it was, so a frame that appears, goes
    // or changes its bezel moves the screen and not the device.
    let shell_after = shell_offset(page);
    rect.x += shell_after.0 - shell_before.0;
    rect.y += shell_after.1 - shell_before.1;
    applies
}

/// How far the page's screen sits in from its shell's corner.
fn shell_offset(page: &Page) -> (f64, f64) {
    page.shell()
        .map_or((0.0, 0.0), |shell| (shell.insets.left, shell.insets.top))
}

fn apply(property: &Property, page: &mut Page, rect: &mut Rect) -> bool {
    match property {
        Property::ViewportPreset(index) => {
            let Some(chosen) = preset(u64::from(*index)) else {
                return false;
            };
            page.preset_index = Some(*index);
            let meta = page.metadata.get_or_insert_with(JsonMap::new);
            for key in [SIZE_MODE, CUSTOM_SIZE, LEGACY_CUSTOM_SIZE] {
                meta.remove(key);
            }
            meta.insert(DEVICE_ID.to_owned(), json!(chosen.device_id));
            let size = sized_for((chosen.width, chosen.height), orientation_of(meta));
            *rect = Rect::new(rect.x, rect.y, size.0, size.1);
        }
        Property::CustomViewport => resize(page, rect, rect.width, rect.height),
        Property::ViewportWidth(width) => resize(page, rect, *width, rect.height),
        Property::ViewportHeight(height) => resize(page, rect, rect.width, *height),
        Property::Orientation(orientation) => {
            let base = base_size(page);
            let meta = page.metadata.get_or_insert_with(JsonMap::new);
            meta.insert(ORIENTATION.to_owned(), json!(name(*orientation)));
            if !is_custom(meta) {
                let size = sized_for(base, *orientation);
                *rect = Rect::new(rect.x, rect.y, size.0, size.1);
            }
        }
        Property::DeviceFrame(shown) => {
            let meta = page.metadata.get_or_insert_with(JsonMap::new);
            meta.insert(FRAME.to_owned(), json!(*shown));
        }
        Property::ColorScheme(scheme) => page.color_scheme = *scheme,
        Property::Color(_)
        | Property::Label(_)
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
        | Property::FromEnd(_)
        | Property::ToEnd(_) => return false,
    }
    true
}

/// Gives the page the custom size `width` by `height`, which no preset
/// names any more.
fn resize(page: &mut Page, rect: &mut Rect, width: f64, height: f64) {
    let meta = page.metadata.get_or_insert_with(JsonMap::new);
    meta.insert(SIZE_MODE.to_owned(), json!("custom"));
    meta.insert(
        CUSTOM_SIZE.to_owned(),
        json!({"width": number(width), "height": number(height)}),
    );
    meta.remove(LEGACY_CUSTOM_SIZE);
    meta.remove(DEVICE_ID);
    *rect = Rect::new(rect.x, rect.y, width, height);
}

/// The page's orientation. A page that never recorded one is portrait.
pub(super) fn orientation_of(meta: &JsonMap) -> Orientation {
    match meta.get(ORIENTATION).and_then(Value::as_str) {
        Some("landscape") => Orientation::Landscape,
        _ => Orientation::Portrait,
    }
}

/// Whether the page is shown in its device frame.
pub(super) fn framed(meta: &JsonMap) -> bool {
    meta.get(FRAME) == Some(&Value::Bool(true))
}

/// Whether the page keeps a size of its own rather than a preset's.
pub(super) fn is_custom(meta: &JsonMap) -> bool {
    let named = matches!(
        meta.get(SIZE_MODE).and_then(Value::as_str),
        Some("custom" | "responsive")
    );
    let size = meta
        .get(CUSTOM_SIZE)
        .or_else(|| meta.get(LEGACY_CUSTOM_SIZE));
    let is_number = |key: &str| {
        size.and_then(|size| size.get(key))
            .is_some_and(Value::is_number)
    };
    named && is_number("width") && is_number("height")
}

/// The preset size a page is laid out from: its preset's, or Laptop's when it
/// names none.
fn base_size(page: &Page) -> (f64, f64) {
    let index = page.preset_index.unwrap_or(specular_doc::LAPTOP);
    preset(u64::from(index)).map_or((0.0, 0.0), |chosen| (chosen.width, chosen.height))
}

/// `base` turned across when its shape disagrees with `orientation`.
fn sized_for(base: (f64, f64), orientation: Orientation) -> (f64, f64) {
    let wide = base.0 > base.1;
    if wide == (orientation == Orientation::Landscape) {
        base
    } else {
        (base.1, base.0)
    }
}

const fn name(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Portrait => "portrait",
        Orientation::Landscape => "landscape",
    }
}

/// A whole number as one, so the file reads as Electron writes it.
fn number(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        json!(value as i64)
    } else {
        json!(value)
    }
}
