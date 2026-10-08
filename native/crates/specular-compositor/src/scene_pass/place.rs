//! Resolves a [`Scene`] against the camera: which items show, where, and
//! clipped to what. Pure, so culling is tested without a GPU.

use glam::Vec2;
use specular_core::Camera;
use specular_scene::{
    Blend, ColumnDraw, Draw, Point, Rect, Scene, Size, Space, TextRun, VerticalAlign,
};

use super::column;
use super::text_layout::text_rect;

/// Text whose font size is under this many logical pixels on screen is not
/// drawn: it cannot be read, and it is the most expensive thing to draw
/// (ADR 0039).
pub(crate) const MIN_TEXT_PX: f32 = 2.5;

/// Room for the antialiased fringe around an item's geometric bounds.
const FRINGE_PX: f32 = 1.0;

/// The camera and the target it projects onto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ViewTransform {
    pub(crate) camera: Camera,
    /// Viewport size in logical pixels.
    pub(crate) viewport: Vec2,
    /// Physical pixels per logical pixel.
    pub(crate) scale_factor: f32,
    /// Target size in physical pixels.
    pub(crate) target: [u32; 2],
}

impl ViewTransform {
    /// Logical pixels per unit of `space`.
    pub(crate) fn scale(&self, space: Space) -> f32 {
        match space {
            Space::Canvas => self.camera.zoom,
            Space::Screen => 1.0,
        }
    }

    /// `point` in logical screen pixels.
    pub(crate) fn point(&self, space: Space, point: Point) -> Vec2 {
        let point = Vec2::new(point.x, point.y);
        match space {
            Space::Canvas => self.camera.world_to_screen(point),
            Space::Screen => point,
        }
    }

    /// `rect` in logical screen pixels.
    pub(crate) fn rect(&self, space: Space, rect: Rect) -> Rect {
        let origin = self.point(space, rect.origin());
        let scale = self.scale(space);
        Rect::new(origin.x, origin.y, rect.width * scale, rect.height * scale)
    }

    /// The whole viewport in logical pixels.
    /// A rect in logical pixels, back in `space`.
    pub(crate) fn rect_to(&self, space: Space, rect: Rect) -> Rect {
        match space {
            Space::Canvas => {
                let origin = (self.camera).screen_to_world(Vec2::new(rect.x, rect.y));
                let zoom = self.camera.zoom;
                Rect::new(origin.x, origin.y, rect.width / zoom, rect.height / zoom)
            }
            Space::Screen => rect,
        }
    }

    pub(crate) fn viewport_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.viewport.x, self.viewport.y)
    }

    /// A logical-pixel clip as a scissor rect on the target, or `None` when
    /// it covers no pixel.
    pub(crate) fn scissor(&self, clip: Rect) -> Option<Scissor> {
        let [width, height] = self.target;
        let edge = |logical: f32, limit: u32| {
            (logical * self.scale_factor)
                .round()
                .clamp(0.0, limit as f32) as u32
        };
        let (left, right) = (edge(clip.x, width), edge(clip.right(), width));
        let (top, bottom) = (edge(clip.y, height), edge(clip.bottom(), height));
        (right > left && bottom > top).then(|| Scissor {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }
}

/// A scissor rect in physical pixels, inside the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Scissor {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Which pipeline draws an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Prim {
    /// A page's textured quads.
    Page,
    /// An image's textured quad.
    Image,
    /// An SDF rect or ellipse, or a rect's shadow.
    Shape,
    /// A tessellated polygon or path. Each blend is its own pipeline.
    Mesh(Blend),
    /// Glyphs. Canvas and screen text rasterise at different scales.
    Text(Space),
}

/// A scene item that survived culling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placed {
    /// Index into [`Scene::items`].
    pub(crate) item: usize,
    pub(crate) prim: Prim,
    /// What the item can touch, in logical pixels, inside its clip and the
    /// viewport.
    pub(crate) bounds: Rect,
    /// The item's clip in logical pixels, inside the viewport.
    pub(crate) clip: Option<Rect>,
}

/// What [`place`] left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PlaceCounts {
    /// Items wholly outside the viewport or their clip, or fully transparent.
    pub(crate) culled: u32,
    /// Text runs under [`MIN_TEXT_PX`].
    pub(crate) text_too_small: u32,
}

/// Fills `placed` (cleared first) with the items of `scene` that can show,
/// in paint order. `measure` returns the size of a run's shaped lines in the
/// run's own units; it is only called for runs that pass the cheap checks.
pub(crate) fn place(
    scene: &Scene,
    view: &ViewTransform,
    mut measure: impl FnMut(&TextRun) -> Size,
    placed: &mut Vec<Placed>,
) -> PlaceCounts {
    placed.clear();
    let viewport = view.viewport_rect();
    let mut counts = PlaceCounts::default();
    for (index, item) in scene.items.iter().enumerate() {
        let clip = item.clip.map(|clip| view.rect(item.space, clip));
        let region = match clip {
            Some(clip) => clip.intersection(viewport),
            None => Some(viewport),
        };
        let Some(region) = region.filter(|_| item.opacity > 0.0) else {
            counts.culled += 1;
            continue;
        };
        let (prim, bounds) = match &item.draw {
            Draw::Text(run) => {
                if run.size * view.scale(item.space) < MIN_TEXT_PX {
                    counts.text_too_small += 1;
                    continue;
                }
                let bounds = text_may_show(run, item.space, view, region)
                    .then(|| text_rect(run, measure(run)));
                (Prim::Text(item.space), bounds)
            }
            Draw::Column(column) => {
                let size = (column.rows.iter())
                    .flat_map(|row| &row.cells)
                    .fold(0.0, |size, cell| cell.size.max(size));
                if size * view.scale(item.space) < MIN_TEXT_PX {
                    counts.text_too_small += 1;
                    continue;
                }
                let bounds = column_may_show(column, item.space, view, region)
                    .then(|| column::layout(column, &mut measure).bounds(column));
                (Prim::Text(item.space), bounds)
            }
            Draw::Page(_) => (Prim::Page, item.draw.bounds()),
            Draw::Image(_) => (Prim::Image, item.draw.bounds()),
            Draw::Rect(_) | Draw::Ellipse(_) | Draw::Shadow(_) => (Prim::Shape, item.draw.bounds()),
            Draw::Polygon(_) | Draw::Path(_) => (Prim::Mesh(item.blend), item.draw.bounds()),
        };
        let bounds = bounds.and_then(|bounds| {
            view.rect(item.space, bounds)
                .outset(FRINGE_PX)
                .intersection(region)
        });
        let Some(bounds) = bounds else {
            counts.culled += 1;
            continue;
        };
        placed.push(Placed {
            item: index,
            prim,
            bounds,
            clip: clip.map(|_| region),
        });
    }
    counts
}

/// A check that needs no shaping: `false` only when the run cannot touch
/// `region`. Keeps off-screen text from being shaped just to be culled.
fn text_may_show(run: &TextRun, space: Space, view: &ViewTransform, region: Rect) -> bool {
    let origin = view.point(space, run.origin);
    if let Some(width) = run.wrap_width {
        let right = origin.x + width * view.scale(space);
        if right <= region.x || origin.x >= region.right() {
            return false;
        }
    }
    // Top-aligned lines only ever run downwards from the origin.
    !(run.vertical_align == VerticalAlign::Top && origin.y >= region.bottom())
}

/// Whether a column could reach `region`, told without measuring it: it
/// spans its width and only ever runs downwards from where scrolling leaves
/// its top, which is never below its origin.
fn column_may_show(column: &ColumnDraw, space: Space, view: &ViewTransform, region: Rect) -> bool {
    let origin = view.point(space, column.origin);
    let right = origin.x + column.width * view.scale(space);
    right > region.x && origin.x < region.right() && origin.y < region.bottom()
}

#[cfg(test)]
#[path = "place_tests.rs"]
pub(crate) mod tests;
