//! Turns [`ShapeDraw`]s into the shape instances to draw this frame.
//!
//! Pure, like the page draw list: culling and instance resolution are
//! testable without a GPU.

use glam::Vec2;
use specular_core::CanvasRect;

use crate::gpu_types::ShapeInstance;
use crate::scene::{SceneView, ShapeExtent};

/// Fills `instances` (cleared first, capacity reused) with the shapes of
/// `scene` that touch the viewport, in paint order.
pub(crate) fn build_shape_list(scene: &SceneView<'_>, instances: &mut Vec<ShapeInstance>) {
    instances.clear();
    let zoom = scene.camera.zoom;
    let visible = scene.camera.visible_world_rect(scene.viewport);
    for shape in scene.shapes {
        let (centre, half_size) = resolve(shape.extent, zoom);
        // The stroke is a constant number of screen pixels past the edge.
        let reach = (half_size + Vec2::splat(shape.stroke_width.max(0.0))) / zoom;
        let footprint = CanvasRect::new(
            centre.x - reach.x,
            centre.y - reach.y,
            reach.x * 2.0,
            reach.y * 2.0,
        );
        if !footprint.intersects(visible) {
            continue;
        }
        let corner_radius = shape.corner_radius.max(0.0)
            * match shape.extent {
                ShapeExtent::Canvas(_) => zoom,
                ShapeExtent::Screen { .. } => 1.0,
            };
        instances.push(ShapeInstance::new(
            centre,
            half_size,
            corner_radius.min(half_size.min_element()),
            shape.fill,
            shape.stroke,
            shape.stroke_width.max(0.0),
        ));
    }
}

/// Centre in canvas units and half size in logical pixels.
fn resolve(extent: ShapeExtent, zoom: f32) -> (Vec2, Vec2) {
    match extent {
        ShapeExtent::Canvas(rect) => (
            rect.origin() + rect.size() * 0.5,
            rect.size() * (0.5 * zoom),
        ),
        ShapeExtent::Screen { anchor, size } => (anchor, size * 0.5),
    }
}

#[cfg(test)]
mod tests {
    use specular_core::Camera;

    use super::*;
    use crate::scene::{DotGrid, ShapeDraw};

    const FILL: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    const CLEAR: [f32; 4] = [0.0; 4];

    fn canvas_shape(x: f32, y: f32, width: f32, height: f32) -> ShapeDraw {
        ShapeDraw {
            extent: ShapeExtent::Canvas(CanvasRect::new(x, y, width, height)),
            corner_radius: 8.0,
            fill: FILL,
            stroke: CLEAR,
            stroke_width: 0.0,
        }
    }

    fn screen_shape(anchor: Vec2, size: f32) -> ShapeDraw {
        ShapeDraw {
            extent: ShapeExtent::Screen {
                anchor,
                size: Vec2::splat(size),
            },
            corner_radius: size / 2.0,
            fill: FILL,
            stroke: CLEAR,
            stroke_width: 0.0,
        }
    }

    fn build(camera: Camera, shapes: &[ShapeDraw]) -> Vec<ShapeInstance> {
        let scene = SceneView {
            camera,
            viewport: Vec2::new(800.0, 600.0),
            scale_factor: 1.0,
            pages: &[],
            shapes,
            grid: DotGrid::default(),
        };
        let mut instances = Vec::new();
        build_shape_list(&scene, &mut instances);
        instances
    }

    #[test]
    fn canvas_shapes_outside_viewport_are_culled() {
        let shapes = [
            canvas_shape(0.0, 0.0, 100.0, 100.0),
            canvas_shape(5_000.0, 0.0, 100.0, 100.0),
        ];
        assert_eq!(build(Camera::default(), &shapes).len(), 1);
    }

    #[test]
    fn culling_follows_zoom_for_canvas_shapes() {
        // At zoom 0.5 the viewport spans 1600 canvas units.
        let shapes = [canvas_shape(1_500.0, 0.0, 100.0, 100.0)];
        let visible = build(Camera::new(Vec2::ZERO, 0.5), &shapes).len();
        let hidden = build(Camera::default(), &shapes).len();
        assert_eq!((visible, hidden), (1, 0));
    }

    #[test]
    fn screen_shape_past_the_edge_survives_by_its_pixel_size() {
        // A 40 px handle centred 10 world units off the right edge pokes in.
        let near = screen_shape(Vec2::new(810.0, 300.0), 40.0);
        let far = screen_shape(Vec2::new(830.0, 300.0), 40.0);
        assert_eq!(build(Camera::default(), &[near, far]).len(), 1);
    }

    #[test]
    fn screen_shape_footprint_does_not_shrink_with_zoom() {
        // At zoom 0.1, 40 px is 400 canvas units, so an anchor 150 units off
        // the 8000-unit-wide viewport is still touched.
        let shape = screen_shape(Vec2::new(8_150.0, 100.0), 40.0);
        assert_eq!(build(Camera::new(Vec2::ZERO, 0.1), &[shape]).len(), 1);
    }

    #[test]
    fn stroke_width_extends_the_culling_footprint() {
        let mut shape = canvas_shape(-110.0, 0.0, 100.0, 100.0);
        let without = build(Camera::default(), &[shape]).len();
        shape.stroke_width = 20.0;
        let with = build(Camera::default(), &[shape]).len();
        assert_eq!((without, with), (0, 1));
    }

    #[test]
    fn survivors_keep_paint_order() {
        let shapes = [
            canvas_shape(300.0, 0.0, 10.0, 10.0),
            canvas_shape(9_000.0, 0.0, 10.0, 10.0),
            canvas_shape(100.0, 0.0, 10.0, 10.0),
        ];
        let centres: Vec<_> = build(Camera::default(), &shapes)
            .iter()
            .map(|instance| instance.centre[0])
            .collect();
        assert_eq!(centres, [305.0, 105.0]);
    }

    #[test]
    fn canvas_instance_scales_size_and_radius_with_zoom() {
        let mut shape = canvas_shape(100.0, 40.0, 200.0, 100.0);
        shape.stroke_width = 1.5;
        let instances = build(Camera::new(Vec2::ZERO, 2.0), &[shape]);
        let instance = instances[0];
        assert_eq!(
            (
                instance.centre,
                instance.half_size,
                instance.corner_radius,
                instance.stroke_width
            ),
            ([200.0, 90.0], [200.0, 100.0], 16.0, 1.5)
        );
    }

    #[test]
    fn screen_instance_ignores_zoom() {
        let shape = screen_shape(Vec2::new(60.0, 70.0), 10.0);
        let instances = build(Camera::new(Vec2::ZERO, 2.0), &[shape]);
        let instance = instances[0];
        assert_eq!(
            (instance.centre, instance.half_size, instance.corner_radius),
            ([60.0, 70.0], [5.0, 5.0], 5.0)
        );
    }

    #[test]
    fn corner_radius_is_clamped_to_half_the_short_side() {
        let mut shape = canvas_shape(0.0, 0.0, 100.0, 20.0);
        shape.corner_radius = 500.0;
        assert_eq!(build(Camera::default(), &[shape])[0].corner_radius, 10.0);
    }
}
