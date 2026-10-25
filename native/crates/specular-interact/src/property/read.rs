//! What the selection holds for a property: the value every item it applies
//! to shares, or `None` when they differ or none applies. A field a file left
//! out reads as the value the renderer draws it with.

use specular_doc::{
    BorderStyle, BrushType, Color, ColorScheme, DeviceFrameColor, DeviceFrameStyle, Edge, EdgeEnd,
    Entity, FillStyle, ItemId, Kind, LineStyle, Page, ShapeKind, TextAlign, TextFont,
};

use super::Orientation;
use super::page;
use crate::App;

/// The size text is drawn at when it names none.
pub(crate) const DEFAULT_TEXT_SIZE: f64 = 14.0;
/// A shape's border width when it names none.
pub(crate) const DEFAULT_BORDER_WIDTH: f64 = 2.0;
/// An edge's line width when it names none.
pub(crate) const DEFAULT_EDGE_WIDTH: f64 = 1.5;

/// What `entity` and `edge` give for each selected item, gathered in
/// selection order.
fn gather<T>(
    app: &App,
    mut entity: impl FnMut(&Entity, &mut Vec<T>),
    mut edge: impl FnMut(&Edge, &mut Vec<T>),
) -> Vec<T> {
    let mut values = Vec::new();
    for item in app.session.selection.items() {
        match item {
            ItemId::Entity(id) => {
                if let Some(found) = app.document.entity(id) {
                    entity(found, &mut values);
                }
            }
            ItemId::Edge(id) => {
                if let Some(found) = app.document.edge(id) {
                    edge(found, &mut values);
                }
            }
        }
    }
    values
}

/// The one value in `values`, if they all agree.
fn common<T: PartialEq>(values: Vec<T>) -> Option<T> {
    let mut values = values.into_iter();
    let first = values.next()?;
    values.all(|value| value == first).then_some(first)
}

fn of_entities<T: PartialEq>(app: &App, read: impl Fn(&Entity, &mut Vec<T>)) -> Option<T> {
    common(gather(app, read, |_, _| {}))
}

fn of_edges<T: PartialEq>(app: &App, read: impl Fn(&Edge) -> T) -> Option<T> {
    common(gather(app, |_, _| {}, |edge, out| out.push(read(edge))))
}

/// The color of texts, shapes, groups, drawings' strokes and edges, as stored.
/// An item with no stored color is left out of the comparison.
pub(crate) fn color(app: &App) -> Option<Color> {
    common(gather(
        app,
        |entity, out| match &entity.kind {
            Kind::Text(text) => out.extend(text.color.clone()),
            Kind::Shape(shape) => out.extend(shape.color.clone()),
            Kind::Group(group) => out.extend(group.color.clone()),
            Kind::Drawing(drawing) => out.extend(drawing.strokes.iter().map(|s| s.color.clone())),
            Kind::Page(_) | Kind::File(_) => {}
        },
        |edge, out| out.extend(edge.color.clone()),
    ))
}

/// A shape's border color, as stored.
pub(crate) fn border_color(app: &App) -> Option<Color> {
    of_entities(app, |entity, out| {
        if let Kind::Shape(shape) = &entity.kind {
            out.extend(shape.border_color.clone());
        }
    })
}

/// The size of texts and shape labels.
pub(crate) fn text_size(app: &App) -> Option<f64> {
    of_entities(app, |entity, out| match &entity.kind {
        Kind::Text(text) => out.push(text.size.unwrap_or(DEFAULT_TEXT_SIZE)),
        Kind::Shape(shape) => out.push(shape.text_size.unwrap_or(DEFAULT_TEXT_SIZE)),
        Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => {}
    })
}

/// The typeface of texts.
pub(crate) fn text_font(app: &App) -> Option<TextFont> {
    of_entities(app, |entity, out| {
        if let Kind::Text(text) = &entity.kind {
            out.push(text.font.unwrap_or(TextFont::Sans));
        }
    })
}

/// The horizontal alignment of shape labels.
pub(crate) fn text_align(app: &App) -> Option<TextAlign> {
    of_entities(app, |entity, out| {
        if let Kind::Shape(shape) = &entity.kind {
            out.push(shape.text_align.unwrap_or(TextAlign::Center));
        }
    })
}

/// The silhouette of shapes.
pub(crate) fn shape_kind(app: &App) -> Option<ShapeKind> {
    of_entities(app, |entity, out| {
        if let Kind::Shape(shape) = &entity.kind {
            out.push(shape.shape);
        }
    })
}

/// Whether shapes are painted.
pub(crate) fn fill_style(app: &App) -> Option<FillStyle> {
    of_entities(app, |entity, out| {
        if let Kind::Shape(shape) = &entity.kind {
            out.push(shape.fill_style.unwrap_or(FillStyle::Solid));
        }
    })
}

/// The border style of shapes.
pub(crate) fn border_style(app: &App) -> Option<BorderStyle> {
    of_entities(app, |entity, out| {
        if let Kind::Shape(shape) = &entity.kind {
            out.push(shape.border_style.unwrap_or(BorderStyle::Solid));
        }
    })
}

/// The width of shape borders, drawings' strokes and edges.
pub(crate) fn stroke_width(app: &App) -> Option<f64> {
    common(gather(
        app,
        |entity, out| match &entity.kind {
            Kind::Shape(shape) => out.push(shape.stroke_width.unwrap_or(DEFAULT_BORDER_WIDTH)),
            Kind::Drawing(drawing) => out.extend(drawing.strokes.iter().map(|s| s.width)),
            Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Text(_) => {}
        },
        |edge, out| out.push(edge.stroke_width.unwrap_or(DEFAULT_EDGE_WIDTH)),
    ))
}

/// The brush of drawings' strokes.
pub(crate) fn brush(app: &App) -> Option<BrushType> {
    of_entities(app, |entity, out| {
        if let Kind::Drawing(drawing) = &entity.kind {
            out.extend(
                drawing
                    .strokes
                    .iter()
                    .map(|s| s.brush.unwrap_or(BrushType::Pen)),
            );
        }
    })
}

/// The line style of edges.
pub(crate) fn line_style(app: &App) -> Option<LineStyle> {
    of_edges(app, |edge| edge.line_style.unwrap_or(LineStyle::Solid))
}

/// The label of edges, empty when they have none.
pub(crate) fn edge_label(app: &App) -> Option<String> {
    of_edges(app, |edge| edge.label.clone().unwrap_or_default())
}

/// The endpoint shape at the start of edges.
pub(crate) fn from_end(app: &App) -> Option<EdgeEnd> {
    of_edges(app, |edge| edge.from_end.unwrap_or(EdgeEnd::None))
}

/// The endpoint shape at the end of edges.
pub(crate) fn to_end(app: &App) -> Option<EdgeEnd> {
    of_edges(app, |edge| edge.to_end.unwrap_or(EdgeEnd::Arrow))
}

/// The viewport preset of pages. A page with a custom size has none.
pub(crate) fn viewport_preset(app: &App) -> Option<u32> {
    of_entities(app, |entity, out| {
        if let Some(page) = entity.kind.page_fields() {
            let custom = page.metadata.as_ref().is_some_and(page::is_custom);
            out.extend(page.preset_index.filter(|_| !custom));
        }
    })
}

/// The orientation of pages.
pub(crate) fn orientation(app: &App) -> Option<Orientation> {
    of_entities(app, |entity, out| {
        if let Some(page) = entity.kind.page_fields() {
            let meta = page.metadata.as_ref();
            out.push(meta.map_or(Orientation::Portrait, page::orientation_of));
        }
    })
}

/// The color scheme a page with none of its own is shown in: the app's.
pub(crate) fn followed_color_scheme(app: &App) -> ColorScheme {
    match app.appearance() {
        crate::Appearance::Light => ColorScheme::Light,
        crate::Appearance::Dark => ColorScheme::Dark,
    }
}

/// The device frame pages are drawn in, which is clay for a page kept as
/// realistic that has no drawing to show; the inner `None` is no frame.
#[expect(
    clippy::option_option,
    reason = "the outer None is no page or a mixed selection, the inner one no frame"
)]
pub(crate) fn device_frame(app: &App) -> Option<Option<DeviceFrameStyle>> {
    of_entities(app, |entity, out| {
        if let Some(page) = entity.kind.page_fields() {
            out.push(frame_shown(&page));
        }
    })
}

fn frame_shown(page: &Page) -> Option<DeviceFrameStyle> {
    page.shell().map(|shell| match shell.art {
        Some(_) => DeviceFrameStyle::Realistic,
        None => DeviceFrameStyle::Clay,
    })
}

/// The frame one more press gives the selected pages: none, clay,
/// realistic, and round again. Any page without a frame gets one first.
/// Realistic is left out when no page has a drawing for it, so a press
/// always changes something.
pub(super) fn next_device_frame(app: &App) -> Option<DeviceFrameStyle> {
    let pages = gather(
        app,
        |entity, out| {
            if let Some(page) = entity.kind.page_fields() {
                out.push((frame_shown(&page), page.device_art().is_some()));
            }
        },
        |_, _| {},
    );
    let shown = |style| pages.iter().any(|(shown, _)| *shown == style);
    if shown(None) {
        Some(DeviceFrameStyle::Clay)
    } else if shown(Some(DeviceFrameStyle::Realistic)) {
        None
    } else {
        (pages.iter().any(|(_, drawn)| *drawn)).then_some(DeviceFrameStyle::Realistic)
    }
}

/// The colour the devices of framed pages are drawn in: each page's own,
/// or the app theme's for a page that keeps none. A page with no frame is
/// left out.
pub(crate) fn device_frame_color(app: &App) -> Option<DeviceFrameColor> {
    of_entities(app, |entity, out| {
        if let Some(page) = entity.kind.page_fields()
            && page.shell().is_some()
        {
            out.push(app.device_frame_color(&page));
        }
    })
}

/// Whether a selected page shows a device frame, and so has a colour to set.
pub(crate) fn any_device_frame(app: &App) -> bool {
    let framed = gather(
        app,
        |entity, out| {
            if let Some(page) = entity.kind.page_fields()
                && page.shell().is_some()
            {
                out.push(());
            }
        },
        |_, _| {},
    );
    !framed.is_empty()
}

/// The color-scheme override of pages; the inner `None` follows the system.
#[expect(
    clippy::option_option,
    reason = "the outer None is no page or a mixed selection, the inner one the system's"
)]
pub(crate) fn color_scheme(app: &App) -> Option<Option<ColorScheme>> {
    of_entities(app, |entity, out| {
        if let Some(page) = entity.kind.page_fields() {
            out.push(page.color_scheme);
        }
    })
}
