//! A box's paint as a layout or a mod names it (ADR 0059): a colour, a
//! gradient, or a repeating pattern over a base colour.
//!
//! ```json
//! "toolbar"
//! { "gradient": ["#fff", "#ddd"], "angle": 180 }
//! { "from": "#fff", "to": "#ddd", "angle": 180 }
//! { "color": "#ddd", "pattern": "stripes", "ink": "#bbb", "width": 1, "gap": 1 }
//! { "pattern": "hatch", "ink": "#0002", "width": 1, "gap": 4 }
//! { "pattern": "checker", "ink": "#0001", "size": 2 }
//! ```
//!
//! Hatching and checks are drawn by the GPU in one quad. Stripes, which it
//! has no fill for, are one thin box each, so a box keeps at most
//! [`MOST_STRIPES`] of them.

use gpui_kit::{
    AnyElement, Div, Hsla, IntoElement as _, ParentElement as _, Styled as _, checkerboard, div,
    linear_color_stop, linear_gradient, pattern_slash, px,
};
use serde_json::{Map, Value};

use crate::{look, theme};

/// The most stripes one box draws.
const MOST_STRIPES: usize = 400;

/// A colour by hex or by name.
pub(crate) fn hsla(text: &str) -> Option<Hsla> {
    look::color(text).map(theme::solid)
}

/// `element`, `extent` points across and down, painted with `paint`.
pub(crate) fn paint(element: Div, paint: Option<&Value>, extent: (f32, f32)) -> Div {
    match paint {
        Some(Value::String(color)) => match hsla(color) {
            Some(color) => element.bg(color),
            None => element,
        },
        Some(Value::Object(fields)) => {
            let element = base(element, fields);
            match pattern(fields, extent) {
                Some(layer) => element.relative().overflow_hidden().child(layer),
                None => element,
            }
        }
        _ => element,
    }
}

fn number(fields: &Map<String, Value>, key: &str, default: f32) -> f32 {
    fields
        .get(key)
        .and_then(Value::as_f64)
        .map_or(default, |value| value as f32)
}

fn color_at(fields: &Map<String, Value>, key: &str) -> Option<Hsla> {
    hsla(fields.get(key)?.as_str()?)
}

/// The colour or gradient under any pattern.
fn base(element: Div, fields: &Map<String, Value>) -> Div {
    let stops: Vec<Hsla> = match fields.get("gradient").and_then(Value::as_array) {
        Some(stops) => stops
            .iter()
            .filter_map(|stop| hsla(stop.as_str()?))
            .collect(),
        None => ["from", "to"]
            .iter()
            .filter_map(|key| color_at(fields, key))
            .collect(),
    };
    if let [from, to, ..] = stops.as_slice() {
        let angle = number(fields, "angle", 180.0);
        return element.bg(linear_gradient(
            angle,
            linear_color_stop(*from, 0.0),
            linear_color_stop(*to, 1.0),
        ));
    }
    match color_at(fields, "color") {
        Some(color) => element.bg(color),
        None => element,
    }
}

/// The pattern's layer, filling the box.
fn pattern(fields: &Map<String, Value>, extent: (f32, f32)) -> Option<AnyElement> {
    let ink = color_at(fields, "ink")?;
    let layer = div().absolute().left_0().top_0().size_full();
    let width = number(fields, "width", 1.0).max(0.5);
    let gap = number(fields, "gap", 1.0).max(0.0);
    match fields.get("pattern")?.as_str()? {
        "hatch" => Some(
            layer
                .bg(pattern_slash(ink, width, width + gap))
                .into_any_element(),
        ),
        "checker" => {
            let size = number(fields, "size", 2.0).max(1.0);
            Some(layer.bg(checkerboard(ink, size)).into_any_element())
        }
        "stripes" => {
            let vertical = fields.get("direction").and_then(Value::as_str) == Some("vertical");
            let across = if vertical { extent.0 } else { extent.1 };
            let period = width + gap;
            let count = ((across / period).ceil().max(0.0) as usize).min(MOST_STRIPES);
            let stripes = (0..count).map(|index| {
                let at = px(index as f32 * period);
                let stripe = div().absolute().bg(ink);
                let stripe = if vertical {
                    stripe.top_0().bottom_0().left(at).w(px(width))
                } else {
                    stripe.left_0().right_0().top(at).h(px(width))
                };
                stripe.into_any_element()
            });
            Some(layer.children(stripes).into_any_element())
        }
        _ => None,
    }
}
