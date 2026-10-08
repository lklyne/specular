//! What each property does to one item. Every function returns `None` for an
//! item the property means nothing for.

use specular_doc::{
    Color, Drawing, Edge, Entity, FillStyle, Group, Kind, Rect, Shape, Text, TextStyle, WidthMode,
};

use super::{Property, page};
use crate::tool_defaults::{ToolDefaults, nearest_width};
use crate::{App, edit, strokes};

/// Text sizes the popup offers run from here to there.
const TEXT_SIZE_RANGE: (f64, f64) = (8.0, 256.0);

/// `entity` with `property` set, or `None` if the property does not apply to
/// its kind. The result may equal `entity`.
pub(super) fn entity(
    property: &Property,
    entity: &Entity,
    defaults: &ToolDefaults,
) -> Option<Entity> {
    let mut next = entity.clone();
    let applies = match &mut next.kind {
        Kind::Text(text) => set_text(property, text, defaults),
        Kind::Shape(shape) => set_shape(property, shape),
        Kind::Group(group) => set_group(property, group),
        Kind::Drawing(drawing) => set_drawing(property, drawing, &mut next.rect),
        Kind::Page(page) => page::set(property, page, &mut next.rect),
        Kind::File(_) => false,
    };
    applies.then_some(next)
}

/// `edge` with `property` set, or `None` if the property does not apply to
/// edges.
pub(super) fn edge(property: &Property, edge: &Edge) -> Option<Edge> {
    let mut next = edge.clone();
    match property {
        Property::Color(color) => next.color = Some(color.clone()),
        Property::StrokeWidth(width) => next.stroke_width = Some(*width),
        Property::LineStyle(style) => next.line_style = Some(*style),
        Property::Label(label) => {
            next.label = Some(label.clone()).filter(|label| !label.is_empty());
        }
        Property::FromEnd(end) => next.from_end = Some(*end),
        Property::ToEnd(end) => next.to_end = Some(*end),
        Property::BorderColor(_)
        | Property::TextSize(_)
        | Property::TextFont(_)
        | Property::TextStyle(_)
        | Property::TextAlign(_)
        | Property::TextVerticalAlign(_)
        | Property::ShapeKind(_)
        | Property::FillStyle(_)
        | Property::BorderStyle(_)
        | Property::Brush(_)
        | Property::ViewportPreset(_)
        | Property::CustomViewport
        | Property::ViewportWidth(_)
        | Property::ViewportHeight(_)
        | Property::Orientation(_)
        | Property::DeviceFrame(_)
        | Property::ColorScheme(_) => return None,
    }
    Some(next)
}

/// Gives a text whose size or typeface changed the rect its content now
/// fills: a sticky or a fixed-width text grows downward, an auto-width text
/// hugs its lines.
pub(super) fn refit(app: &App, before: &Entity, next: &mut Entity) {
    if let (Kind::Text(was), Kind::Text(text)) = (&before.kind, &next.kind)
        && (was.size != text.size
            || was.font != text.font
            || was.resolved_style() != text.resolved_style())
    {
        next.rect = edit::fitted(app, next.rect, text);
    }
}

fn set_text(property: &Property, text: &mut Text, defaults: &ToolDefaults) -> bool {
    match property {
        Property::TextStyle(style) => restyle(text, *style, defaults),
        Property::Color(color) => text.color = Some(color.clone()),
        Property::TextSize(size) => text.size = Some(clamped(*size)),
        Property::TextFont(font) => text.font = Some(*font),
        _ => return false,
    }
    true
}

/// Makes `text` a `style` text with the width mode and color the creation
/// tool of that style stamps. A text already of the style is left alone.
fn restyle(text: &mut Text, style: TextStyle, defaults: &ToolDefaults) {
    if text.resolved_style() == style {
        return;
    }
    text.style = Some(style);
    match style {
        TextStyle::Plain => {
            text.width_mode = Some(WidthMode::Auto);
            text.color = Some(defaults.text.color.clone().unwrap_or(Color::Neutral));
        }
        TextStyle::Sticky => {
            text.width_mode = Some(WidthMode::Fixed);
            text.color = Some(defaults.sticky.color.clone());
        }
    }
}

fn set_shape(property: &Property, shape: &mut Shape) -> bool {
    match property {
        Property::Color(color) => {
            shape.color = Some(color.clone());
            if shape.fill_style == Some(FillStyle::None) {
                shape.fill_style = Some(FillStyle::Solid);
            }
        }
        Property::BorderColor(color) => shape.border_color = Some(color.clone()),
        Property::TextSize(size) => shape.text_size = Some(clamped(*size)),
        Property::TextAlign(align) => shape.text_align = Some(*align),
        Property::TextVerticalAlign(align) => shape.text_vertical_align = Some(*align),
        Property::ShapeKind(kind) => shape.shape = *kind,
        Property::FillStyle(style) => shape.fill_style = Some(*style),
        Property::BorderStyle(style) => shape.border_style = Some(*style),
        Property::StrokeWidth(width) => shape.stroke_width = Some(*width),
        _ => return false,
    }
    true
}

fn set_group(property: &Property, group: &mut Group) -> bool {
    match property {
        Property::Color(color) => group.color = Some(color.clone()),
        _ => return false,
    }
    true
}

/// Sets a stroke field on every stroke. The drawing's rect follows the
/// strokes' widths, which pad it.
fn set_drawing(property: &Property, drawing: &mut Drawing, rect: &mut Rect) -> bool {
    if !matches!(
        property,
        Property::Color(_) | Property::StrokeWidth(_) | Property::Brush(_)
    ) {
        return false;
    }
    let before = drawing.strokes.clone();
    for stroke in &mut drawing.strokes {
        match property {
            Property::Color(color) => stroke.color = color.clone(),
            Property::StrokeWidth(width) => stroke.width = *width,
            Property::Brush(brush) => {
                stroke.brush = Some(*brush);
                stroke.width = nearest_width(*brush, stroke.width);
            }
            _ => {}
        }
    }
    // Only a width pads the rect, and a drawing with nothing to change keeps
    // the rect it has.
    if !matches!(property, Property::Color(_)) && drawing.strokes != before {
        *rect = strokes::bounds(&drawing.strokes);
    }
    true
}

fn clamped(size: f64) -> f64 {
    size.round().clamp(TEXT_SIZE_RANGE.0, TEXT_SIZE_RANGE.1)
}
