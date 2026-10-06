//! The per-frame shape list for the canvas chrome: page borders, annotations,
//! and the selection outline with its resize handles.
//!
//! Everything is drawn through the compositor's shape layer, so the cost of
//! chrome is the cost of those shapes and nothing else. Paint order: page
//! borders, annotations (region, then pin), the comment tool's live preview,
//! then the selection on top.

use glam::Vec2;
use specular_compositor::{PAGE_CORNER_RADIUS, ShapeDraw, ShapeExtent};
use specular_core::{CanvasRect, PageId};

use crate::annotation::Annotation;
use crate::handles::{Corner, HANDLE_SIZE};
use crate::placement::PlacedPage;

/// Linear RGBA, straight alpha (the compositor's convention).
type Rgba = [f32; 4];

const CLEAR: Rgba = [0.0; 4];
const WHITE: Rgba = [1.0, 1.0, 1.0, 1.0];
/// `#c4c4c4`.
const BORDER: Rgba = [0.552, 0.552, 0.552, 1.0];
/// `#8e8e8e`.
const BORDER_HOVER: Rgba = [0.270, 0.270, 0.270, 1.0];
/// `#2f7df4`.
const SELECTION: Rgba = [0.028, 0.205, 0.905, 1.0];
/// `#f5a524`.
const COMMENT: Rgba = [0.913, 0.376, 0.018, 1.0];
const COMMENT_FILL: Rgba = [0.913, 0.376, 0.018, 0.14];

const BORDER_WIDTH: f32 = 1.0;
const BORDER_HOVER_WIDTH: f32 = 1.5;
const SELECTION_WIDTH: f32 = 1.5;
const HANDLE_STROKE: f32 = 1.0;
const HANDLE_RADIUS: f32 = 1.0;
const REGION_RADIUS: f32 = 4.0;
const REGION_STROKE: f32 = 1.5;
/// Pin diameter in logical pixels.
const PIN_SIZE: f32 = 20.0;
const PIN_STROKE: f32 = 1.5;

/// What the chrome layer draws this frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChromeScene<'a> {
    pub(crate) placed: &'a [PlacedPage],
    pub(crate) selected: Option<PageId>,
    pub(crate) hovered: Option<PageId>,
    pub(crate) annotations: &'a [Annotation],
    /// The comment tool's region while it is being dragged.
    pub(crate) preview: Option<CanvasRect>,
}

/// Replaces the contents of `out` with the shapes for `scene`; `None` (chrome
/// off) leaves it empty. `out` keeps its capacity across frames.
pub(crate) fn build_shapes(scene: Option<&ChromeScene<'_>>, out: &mut Vec<ShapeDraw>) {
    out.clear();
    let Some(scene) = scene else {
        return;
    };
    for page in scene.placed {
        let hovered = scene.hovered == Some(page.page);
        let (stroke, stroke_width) = if hovered {
            (BORDER_HOVER, BORDER_HOVER_WIDTH)
        } else {
            (BORDER, BORDER_WIDTH)
        };
        out.push(page_outline(page.rect, stroke, stroke_width));
    }
    for annotation in scene.annotations {
        if let Some(region) = annotation.rect(scene.placed) {
            push_annotation(region, out);
        }
    }
    if let Some(region) = scene.preview {
        push_annotation(region, out);
    }
    let selected = scene
        .selected
        .and_then(|id| scene.placed.iter().find(|page| page.page == id));
    if let Some(page) = selected {
        out.push(page_outline(page.rect, SELECTION, SELECTION_WIDTH));
        out.extend(Corner::ALL.map(|corner| handle(corner.point(page.rect))));
    }
}

/// An unfilled outline hugging `rect` from outside, following the page's
/// rounded corners.
fn page_outline(rect: CanvasRect, stroke: Rgba, stroke_width: f32) -> ShapeDraw {
    ShapeDraw {
        extent: ShapeExtent::Canvas(rect),
        corner_radius: PAGE_CORNER_RADIUS,
        fill: CLEAR,
        stroke,
        stroke_width,
    }
}

/// A resize handle centred on `corner`.
fn handle(corner: Vec2) -> ShapeDraw {
    ShapeDraw {
        extent: ShapeExtent::Screen {
            anchor: corner,
            size: Vec2::splat(HANDLE_SIZE),
        },
        corner_radius: HANDLE_RADIUS,
        fill: WHITE,
        stroke: SELECTION,
        stroke_width: HANDLE_STROKE,
    }
}

/// A translucent region and its pin at the top-left corner.
fn push_annotation(region: CanvasRect, out: &mut Vec<ShapeDraw>) {
    out.push(ShapeDraw {
        extent: ShapeExtent::Canvas(region),
        corner_radius: REGION_RADIUS,
        fill: COMMENT_FILL,
        stroke: COMMENT,
        stroke_width: REGION_STROKE,
    });
    out.push(ShapeDraw {
        extent: ShapeExtent::Screen {
            anchor: region.origin(),
            size: Vec2::splat(PIN_SIZE),
        },
        corner_radius: PIN_SIZE / 2.0,
        fill: COMMENT,
        stroke: WHITE,
        stroke_width: PIN_STROKE,
    });
}

#[cfg(test)]
mod tests {
    use specular_core::CssSize;

    use super::*;

    fn pages() -> Vec<PlacedPage> {
        vec![
            PlacedPage::new(
                PageId(1),
                CanvasRect::new(0.0, 0.0, 400.0, 300.0),
                CssSize::new(400, 300),
            ),
            PlacedPage::new(
                PageId(2),
                CanvasRect::new(500.0, 0.0, 400.0, 300.0),
                CssSize::new(400, 300),
            ),
        ]
    }

    fn scene<'a>(placed: &'a [PlacedPage], annotations: &'a [Annotation]) -> ChromeScene<'a> {
        ChromeScene {
            placed,
            selected: None,
            hovered: None,
            annotations,
            preview: None,
        }
    }

    fn built(scene: &ChromeScene<'_>) -> Vec<ShapeDraw> {
        let mut out = Vec::new();
        build_shapes(Some(scene), &mut out);
        out
    }

    fn canvas_rect(shape: &ShapeDraw) -> Option<CanvasRect> {
        match shape.extent {
            ShapeExtent::Canvas(rect) => Some(rect),
            ShapeExtent::Screen { .. } => None,
        }
    }

    #[test]
    fn every_page_gets_a_border_matching_its_rect_and_corners() {
        let placed = pages();
        let shapes = built(&scene(&placed, &[]));
        assert_eq!(shapes.len(), 2);
        assert_eq!(canvas_rect(&shapes[1]), Some(placed[1].rect));
        assert_eq!(shapes[1].corner_radius, PAGE_CORNER_RADIUS);
        assert_eq!(shapes[1].fill[3], 0.0);
    }

    #[test]
    fn the_hovered_page_has_a_stronger_border() {
        let placed = pages();
        let mut hover = scene(&placed, &[]);
        hover.hovered = Some(PageId(2));
        let shapes = built(&hover);
        assert_eq!((shapes[0].stroke, shapes[1].stroke), (BORDER, BORDER_HOVER));
        assert!(shapes[1].stroke_width > shapes[0].stroke_width);
    }

    #[test]
    fn selection_adds_an_outline_and_four_corner_handles() {
        let placed = pages();
        let mut selecting = scene(&placed, &[]);
        selecting.selected = Some(PageId(1));
        let shapes = built(&selecting);
        // Two borders, one outline, four handles.
        assert_eq!(shapes.len(), 7);
        assert_eq!(shapes[2].stroke, SELECTION);
        let anchors: Vec<Vec2> = shapes[3..]
            .iter()
            .filter_map(|shape| match shape.extent {
                ShapeExtent::Screen { anchor, size } => {
                    assert_eq!(size, Vec2::splat(HANDLE_SIZE));
                    Some(anchor)
                }
                ShapeExtent::Canvas(_) => None,
            })
            .collect();
        assert_eq!(
            anchors,
            [
                Vec2::new(0.0, 0.0),
                Vec2::new(400.0, 0.0),
                Vec2::new(400.0, 300.0),
                Vec2::new(0.0, 300.0)
            ]
        );
    }

    #[test]
    fn selection_paints_above_annotations() {
        let placed = pages();
        let notes = [Annotation::canvas_bound(CanvasRect::new(
            10.0, 10.0, 50.0, 50.0,
        ))];
        let mut selecting = scene(&placed, &notes);
        selecting.selected = Some(PageId(1));
        let shapes = built(&selecting);
        // Borders (2), region, pin, outline, handles (4).
        assert_eq!(shapes.len(), 9);
        assert_eq!(shapes[2].stroke, COMMENT);
        assert_eq!(shapes[4].stroke, SELECTION);
    }

    #[test]
    fn a_page_bound_annotation_follows_its_page() {
        let mut placed = pages();
        let note = Annotation::page_bound(&placed[0], CanvasRect::new(40.0, 30.0, 100.0, 60.0));
        placed[0].rect = CanvasRect::new(1000.0, 500.0, 800.0, 600.0);
        let shapes = built(&scene(&placed, &[note]));
        // Region at 10%/10% of the new rect, twice the size.
        assert_eq!(
            canvas_rect(&shapes[2]),
            Some(CanvasRect::new(1080.0, 560.0, 200.0, 120.0))
        );
    }

    #[test]
    fn the_pin_is_a_screen_sized_circle_at_the_region_corner() {
        let placed = pages();
        let notes = [Annotation::canvas_bound(CanvasRect::new(
            10.0, 20.0, 50.0, 50.0,
        ))];
        let shapes = built(&scene(&placed, &notes));
        let pin = shapes[3];
        assert_eq!(
            pin.extent,
            ShapeExtent::Screen {
                anchor: Vec2::new(10.0, 20.0),
                size: Vec2::splat(PIN_SIZE)
            }
        );
        assert_eq!(pin.corner_radius, PIN_SIZE / 2.0);
    }

    #[test]
    fn the_tool_preview_draws_like_an_annotation() {
        let placed = pages();
        let mut drawing = scene(&placed, &[]);
        drawing.preview = Some(CanvasRect::new(5.0, 5.0, 30.0, 30.0));
        assert_eq!(built(&drawing).len(), 4);
    }

    #[test]
    fn chrome_off_draws_nothing_and_clears_the_previous_frame() {
        let placed = pages();
        let mut out = Vec::new();
        build_shapes(Some(&scene(&placed, &[])), &mut out);
        build_shapes(None, &mut out);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn rebuilding_reuses_the_buffer() {
        let placed = pages();
        let mut out = Vec::new();
        build_shapes(Some(&scene(&placed, &[])), &mut out);
        let capacity = out.capacity();
        build_shapes(Some(&scene(&placed, &[])), &mut out);
        assert_eq!((out.len(), out.capacity()), (2, capacity));
    }
}
