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

#[cfg(test)]
mod tests {
    use specular_scene::RectDraw;

    use super::super::place::tests::view;
    use super::*;

    const RED: Color = Color::rgb(255, 0, 0);
    const RECT: Rect = Rect::new(100.0, 40.0, 200.0, 100.0);

    fn instance(item: &Item, zoom: f32) -> ShapeInstance {
        shape_instance(item, &view(Vec2::new(10.0, 20.0), zoom)).unwrap()
    }

    #[test]
    fn a_canvas_rect_scales_its_size_radius_and_stroke_with_zoom() {
        let draw = RectDraw::filled(RECT, RED)
            .with_corner_radius(8.0)
            .with_stroke(Stroke::new(RED, 3.0, StrokeAlign::Outside));
        let instance = instance(&Item::canvas(draw), 2.0);
        assert_eq!(
            (
                instance.centre,
                instance.half_size,
                instance.corner_radius,
                instance.stroke_width
            ),
            ([200.0, 90.0], [200.0, 100.0], 16.0, 6.0)
        );
    }

    #[test]
    fn a_screen_rect_keeps_its_pixel_size_and_lands_on_its_pixels() {
        let item = Item::screen(RectDraw::filled(RECT, RED).with_corner_radius(8.0));
        let view = view(Vec2::new(10.0, 20.0), 2.0);
        let instance = shape_instance(&item, &view).unwrap();
        let on_screen = view.camera.world_to_screen(Vec2::from(instance.centre));
        assert_eq!(
            (on_screen, instance.half_size, instance.corner_radius),
            (Vec2::new(200.0, 90.0), [100.0, 50.0], 8.0)
        );
    }

    #[test]
    fn stroke_alignment_sets_where_the_band_starts() {
        let offset = |align| {
            let draw = RectDraw::filled(RECT, RED).with_stroke(Stroke::new(RED, 4.0, align));
            instance(&Item::canvas(draw), 1.0).stroke_offset
        };
        assert_eq!(
            [
                offset(StrokeAlign::Inside),
                offset(StrokeAlign::Centre),
                offset(StrokeAlign::Outside)
            ],
            [-4.0, -2.0, 0.0]
        );
    }

    #[test]
    fn a_shadow_is_one_instance_whose_blur_scales_with_zoom() {
        let shadow = specular_scene::ShadowDraw {
            rect: RECT,
            corner_radius: 4.0,
            blur: 8.0,
            color: Color::rgba(0, 0, 0, 20),
        };
        let instance = instance(&Item::canvas(shadow), 0.5);
        assert_eq!(
            (
                instance.kind,
                instance.half_size,
                instance.corner_radius,
                instance.stroke_width,
                instance.stroke,
            ),
            (ShapeInstance::SHADOW, [50.0, 25.0], 2.0, 4.0, CLEAR)
        );
    }

    #[test]
    fn a_stroke_thinner_than_a_pixel_is_held_at_one_and_faded() {
        // 2 units at zoom 0.25 is half a pixel.
        let draw = RectDraw::outlined(RECT, Stroke::new(RED, 2.0, StrokeAlign::Outside));
        let instance = instance(&Item::canvas(draw), 0.25);
        assert_eq!((instance.stroke_width, instance.stroke[3]), (1.0, 0.5));
    }
}
