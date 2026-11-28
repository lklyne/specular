//! Mod items (ADR 0057): web content when the mod's items are pages, else
//! the display list the mod sent or the item was saved with, else a card
//! with the mod's name. And the armed mod tool's preview.
//!
//! A display list is in item space, so it is drawn at the item's corner,
//! clipped to its rect: the canvas moves and zooms it with no word from the
//! mod.

use specular_doc::{DrawBox, Entity, ModDraw, ModItem, ObjectFit, Paint};
use specular_interact::ImageState;

use super::frame::{Frame, canvas_rect};
use super::{file, image, page, palette};
use crate::{
    Color, EllipseDraw, ImageId, Item, PathDraw, PathStroke, Point, PolygonDraw, Rect, RectDraw,
    Scene, Stroke, StrokeAlign, TextAlign, TextRun, brush_outline,
};

/// The size of a mod's text when it names none.
const TEXT_SIZE: f64 = 14.0;

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, item: &ModItem, scene: &mut Scene) {
    if let Some(live) = frame.app.page(&entity.id) {
        page::draw(frame, entity, &live, scene);
        return;
    }
    let rect = canvas_rect(entity.rect);
    match frame.app.mod_draws(&entity.id) {
        Some(draws) => {
            let origin = Point::new(rect.x, rect.y);
            let items = draws.iter().filter_map(|draw| item_of(frame, origin, draw));
            scene.extend(items.map(|item| item.clipped(rect)));
        }
        None => file::card(frame, rect, &item.name, scene),
    }
}

/// The armed mod tool's live stroke, drawn with its brush, and the preview
/// its mod sent, in canvas units.
pub(crate) fn preview(frame: &Frame<'_>, scene: &mut Scene) {
    let origin = Point::new(0.0, 0.0);
    let trail = frame.app.mod_trail();
    let draws = trail.iter().chain(frame.app.mod_preview());
    scene.extend(draws.filter_map(|draw| item_of(frame, origin, draw)));
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the scene is f32, and a mod's numbers are canvas units"
)]
fn at(origin: Point, frame: DrawBox) -> Rect {
    Rect::new(
        origin.x + frame.x as f32,
        origin.y + frame.y as f32,
        frame.width as f32,
        frame.height as f32,
    )
}

fn color(value: Option<&String>) -> Option<Color> {
    palette::parse_hex(value?)
}

#[expect(clippy::cast_possible_truncation, reason = "the scene is f32")]
fn width(paint: &Paint) -> f32 {
    paint.line.unwrap_or(1.0) as f32
}

/// One entry of a display list as a scene item, its item space's corner at
/// `origin`. `None` for an image that is not ready and a draw with neither
/// fill nor outline.
#[expect(clippy::cast_possible_truncation, reason = "the scene is f32")]
fn item_of(frame: &Frame<'_>, origin: Point, draw: &ModDraw) -> Option<Item> {
    let point = |[x, y]: [f64; 2]| Point::new(origin.x + x as f32, origin.y + y as f32);
    match draw {
        ModDraw::Rect {
            frame: rect,
            paint,
            radius,
        } => {
            let (fill, outline) = (color(paint.fill.as_ref()), color(paint.stroke.as_ref()));
            (fill.is_some() || outline.is_some()).then(|| {
                Item::canvas(RectDraw {
                    rect: at(origin, *rect),
                    corner_radius: radius.unwrap_or(0.0) as f32,
                    fill,
                    stroke: outline
                        .map(|outline| Stroke::new(outline, width(paint), StrokeAlign::Centre)),
                })
            })
        }
        ModDraw::Ellipse { frame: rect, paint } => {
            let (fill, outline) = (color(paint.fill.as_ref()), color(paint.stroke.as_ref()));
            (fill.is_some() || outline.is_some()).then(|| {
                Item::canvas(EllipseDraw {
                    rect: at(origin, *rect),
                    fill,
                    stroke: outline
                        .map(|outline| Stroke::new(outline, width(paint), StrokeAlign::Centre)),
                })
            })
        }
        ModDraw::Path {
            points,
            closed,
            paint,
            taper,
            speed,
        } => {
            if !*closed && (*taper || *speed > 0.0) {
                return brush(origin, points, paint, *taper, *speed);
            }
            let outline =
                color(paint.stroke.as_ref()).map(|outline| PathStroke::new(outline, width(paint)));
            let fill = color(paint.fill.as_ref());
            let points = points.iter().copied().map(point);
            if *closed {
                (fill.is_some() || outline.is_some()).then(|| {
                    Item::canvas(PolygonDraw {
                        points: points.collect(),
                        fill,
                        stroke: outline,
                    })
                })
            } else {
                outline.map(|outline| Item::canvas(PathDraw::polyline(points, outline)))
            }
        }
        ModDraw::Text {
            x,
            y,
            text,
            size,
            color: ink,
            width: wrap,
            align,
        } => {
            let ink = color(ink.as_ref()).unwrap_or(frame.colors.ink);
            let size = size.unwrap_or(TEXT_SIZE) as f32;
            let align = match align.as_deref() {
                Some("center") => TextAlign::Centre,
                Some("right") => TextAlign::Right,
                _ => TextAlign::Left,
            };
            Some(Item::canvas(TextRun {
                wrap_width: wrap.map(|wrap| wrap as f32),
                align,
                ..TextRun::new(text.as_str(), point([*x, *y]), size, ink)
            }))
        }
        ModDraw::Image {
            frame: rect,
            src,
            fit,
            radius,
        } => {
            let loaded = frame.app.image(src)?;
            let ImageState::Ready { width, height } = loaded.state else {
                return None;
            };
            let fit = fit.unwrap_or(ObjectFit::Cover);
            let drawn = image::fitted(ImageId(loaded.key.0), at(origin, *rect), width, height, fit);
            let drawn = crate::ImageDraw {
                corner_radius: radius.unwrap_or(0.0) as f32,
                ..drawn
            };
            Some(Item::canvas(drawn).clipped(at(origin, *rect)))
        }
    }
}

/// An open path with `taper` or `speed`, filled as its brush outline in its
/// `stroke` colour.
#[expect(clippy::cast_possible_truncation, reason = "the scene is f32")]
fn brush(
    origin: Point,
    points: &[[f64; 2]],
    paint: &Paint,
    taper: bool,
    speed: f64,
) -> Option<Item> {
    let ink = color(paint.stroke.as_ref())?;
    let outline = brush_outline(points, paint.line.unwrap_or(1.0), taper, speed);
    (!outline.is_empty()).then(|| {
        Item::canvas(PolygonDraw {
            points: (outline.into_iter())
                .map(|[x, y]| Point::new(origin.x + x as f32, origin.y + y as f32))
                .collect(),
            fill: Some(ink),
            stroke: None,
        })
    })
}
