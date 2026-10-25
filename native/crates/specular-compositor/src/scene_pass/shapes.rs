//! Scene rects and ellipses as SDF shape instances. Pure.

use glam::Vec2;
use specular_scene::{Color, Draw, Item, Rect, ShadowDraw, Space, Stroke, StrokeAlign};

use super::color::{CLEAR, linear};
use super::place::ViewTransform;
use crate::gpu_types::ShapeInstance;

/// The instance for a rect or ellipse item; `None` for any other draw.
pub(crate) fn shape_instance(item: &Item, view: &ViewTransform) -> Option<ShapeInstance> {
    if let Draw::Shadow(shadow) = &item.draw {
        return Some(shadow_instance(item, shadow, view));
    }
    let (rect, corner_radius, fill, stroke, kind) = match &item.draw {
        Draw::Rect(draw) => (
            draw.rect,
            draw.corner_radius,
            draw.fill,
            draw.stroke,
            ShapeInstance::RECT,
        ),
        Draw::Ellipse(draw) => (
            draw.rect,
            0.0,
            draw.fill,
            draw.stroke,
            ShapeInstance::ELLIPSE,
        ),
        Draw::Page(_)
        | Draw::Shadow(_)
        | Draw::Polygon(_)
        | Draw::Path(_)
        | Draw::Text(_)
        | Draw::Column(_)
        | Draw::Image(_) => {
            return None;
        }
    };
    let scale = view.scale(item.space);
    let half_size = Vec2::new(rect.width, rect.height) * (0.5 * scale);
    let stroke = resolve_stroke(stroke, scale, view.scale_factor, item.opacity);
    Some(ShapeInstance {
        centre: canvas_centre(rect, item.space, view).to_array(),
        half_size: half_size.to_array(),
        fill: fill.map_or(CLEAR, |fill| linear(fill, item.opacity)),
        stroke: stroke.color,
        corner_radius: (corner_radius.max(0.0) * scale).min(half_size.min_element()),
        stroke_width: stroke.width,
        stroke_offset: stroke.offset,
        kind,
        turn: item.turn,
    })
}

fn shadow_instance(item: &Item, shadow: &ShadowDraw, view: &ViewTransform) -> ShapeInstance {
    let scale = view.scale(item.space);
    let half_size = Vec2::new(shadow.rect.width, shadow.rect.height) * (0.5 * scale);
    ShapeInstance {
        centre: canvas_centre(shadow.rect, item.space, view).to_array(),
        half_size: half_size.to_array(),
        fill: linear(shadow.color, item.opacity),
        stroke: CLEAR,
        corner_radius: (shadow.corner_radius.max(0.0) * scale).min(half_size.min_element()),
        stroke_width: shadow.blur.max(0.0) * scale,
        stroke_offset: 0.0,
        kind: ShapeInstance::SHADOW,
        turn: item.turn,
    }
}

/// The shader projects a shape's centre from canvas space, so a screen-space
/// shape's centre is unprojected first.
fn canvas_centre(rect: Rect, space: Space, view: &ViewTransform) -> Vec2 {
    let centre = rect.centre();
    let centre = Vec2::new(centre.x, centre.y);
    match space {
        Space::Canvas => centre,
        Space::Screen => view.camera.screen_to_world(centre),
    }
}

struct ResolvedStroke {
    color: [f32; 4],
    /// Logical pixels.
    width: f32,
    /// Start of the band, in logical pixels outwards from the edge.
    offset: f32,
}

fn resolve_stroke(
    stroke: Option<Stroke>,
    scale: f32,
    scale_factor: f32,
    opacity: f32,
) -> ResolvedStroke {
    let Some(stroke) = stroke.filter(|stroke| stroke.width > 0.0) else {
        return ResolvedStroke {
            color: CLEAR,
            width: 0.0,
            offset: 0.0,
        };
    };
    let (width, color) = hairline(stroke.width * scale, stroke.color, scale_factor, opacity);
    let offset = match stroke.align {
        StrokeAlign::Inside => -width,
        StrokeAlign::Centre => -width * 0.5,
        StrokeAlign::Outside => 0.0,
    };
    ResolvedStroke {
        color,
        width,
        offset,
    }
}

/// A stroke of `width` logical pixels as what is drawn: one thinner than a
/// device pixel is held at one device pixel and faded by the same ratio, so
/// it keeps its weight instead of breaking up.
pub(crate) fn hairline(
    width: f32,
    color: Color,
    scale_factor: f32,
    opacity: f32,
) -> (f32, [f32; 4]) {
    let floor = 1.0 / scale_factor.max(1.0);
    if width >= floor {
        (width, linear(color, opacity))
    } else {
        (floor, linear(color, opacity * (width / floor)))
    }
}
