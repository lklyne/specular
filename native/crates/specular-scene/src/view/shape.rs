//! Shape entities: a silhouette from the catalog, its border and its label.

use specular_doc::{BorderStyle, Entity, FillStyle, Shape};

use super::editing;
use super::frame::{Frame, canvas_rect};
use super::palette::{self, Palette, Role};
use super::shape_path::{self, Silhouette};
use crate::{
    Color, Dash, EllipseDraw, Item, PathDraw, PathStroke, PolygonDraw, Rect, RectDraw, Scene,
    Stroke, StrokeAlign, TextRun,
};

/// The hue of a shape with no colour.
const DEFAULT_BASE: Color = Color::rgb(0x6b, 0x72, 0x80);
/// The fill is the base hue this far towards white, and the border the base
/// hue this far towards black.
const FILL_LIGHTEN: f32 = 0.5;
const BORDER_DARKEN: f32 = 0.35;
const DEFAULT_BORDER_WIDTH: f32 = 2.0;
/// A dashed border's dash and gap, as multiples of its width.
const DASH_ON: f32 = 2.0;
const DASH_OFF: f32 = 1.5;
const LABEL_COLOR: Color = Color::rgb(20, 20, 20);

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, shape: &Shape, scene: &mut Scene) {
    let rect = canvas_rect(entity.rect);
    let base = shape.color.as_ref().map_or(DEFAULT_BASE, |color| {
        palette::resolve(color, Palette::Soft, Role::Fill)
    });
    let fill = match shape.fill_style.unwrap_or(FillStyle::Solid) {
        FillStyle::Solid => Some(palette::lighten(base, FILL_LIGHTEN)),
        FillStyle::None => None,
    };
    let border_base = shape.border_color.as_ref().map_or(base, |color| {
        palette::resolve(color, Palette::Soft, Role::Fill)
    });
    let border_color = palette::darken(border_base, BORDER_DARKEN);
    let width = shape
        .stroke_width
        .map_or(DEFAULT_BORDER_WIDTH, |width| width as f32);
    let style = match shape.border_style.unwrap_or(BorderStyle::Solid) {
        BorderStyle::Solid | BorderStyle::Dashed if width <= 0.0 => BorderStyle::None,
        style => style,
    };
    let stroke = match style {
        BorderStyle::None => None,
        BorderStyle::Solid => Some(PathStroke::new(border_color, width)),
        BorderStyle::Dashed => Some(PathStroke::new(border_color, width).dashed(Dash {
            on: width * DASH_ON,
            off: width * DASH_OFF,
        })),
    };

    let silhouette = shape_path::silhouette(shape.shape, rect);
    scene.push(Item::canvas(body(silhouette, rect, fill, stroke)));
    if let Some(commands) = shape_path::overlay(shape.shape, rect) {
        // With no border the rim is a hairline in the fill's colour.
        let rim = stroke.or_else(|| fill.map(|fill| PathStroke::new(fill, 1.0)));
        if rim.is_some() {
            scene.push(Item::canvas(PathDraw {
                commands,
                fill: None,
                stroke: rim,
            }));
        }
    }
    // The label is set in the editor's frame, so the caret is measured on
    // the run that is drawn.
    let Some(label) = frame.app.text_frame(&entity.id) else {
        return;
    };
    let within = shape_path::label_box(shape.shape, rect);
    let shown = frame.app.editing_text(&entity.id).unwrap_or(&shape.text);
    editing::selection(frame, &entity.id, Some(within), scene);
    if !shown.is_empty() {
        let run = TextRun::framed(shown, &label, LABEL_COLOR);
        scene.push(Item::canvas(run).clipped(within));
    }
    editing::caret(frame, &entity.id, Some(within), LABEL_COLOR, scene);
}

/// The silhouette as the cheapest draw that can show it. A solid rect or
/// ellipse is a distance-field shape; a dashed one has to be a path.
fn body(
    silhouette: Silhouette,
    rect: Rect,
    fill: Option<Color>,
    stroke: Option<PathStroke>,
) -> crate::Draw {
    let dashed = stroke.is_some_and(|stroke| stroke.dash.is_some());
    // A path's stroke straddles its outline, so the field shapes do too.
    let centred = stroke.map(|stroke| Stroke::new(stroke.color, stroke.width, StrokeAlign::Centre));
    match silhouette {
        Silhouette::Rect(corner_radius) if !dashed => RectDraw {
            rect,
            corner_radius,
            fill,
            stroke: centred,
        }
        .into(),
        Silhouette::Ellipse if !dashed => EllipseDraw {
            rect,
            fill,
            stroke: centred,
        }
        .into(),
        Silhouette::Polygon(points) => PolygonDraw {
            points,
            fill,
            stroke,
        }
        .into(),
        Silhouette::Rect(_) | Silhouette::Ellipse | Silhouette::Path(_) => PathDraw {
            commands: silhouette.into_path(rect),
            fill,
            stroke,
        }
        .into(),
    }
}
