//! The per-frame shape list for the canvas chrome: page borders, annotations,
//! and the selection outline with its resize handles.
//!
//! Everything is drawn through the compositor's shape layer, so the cost of
//! chrome is the cost of those shapes and nothing else. Paint order: page
//! borders, annotations (region, then pin), the comment tool's live preview,
//! then the selection on top.

use glam::Vec2;
use specular_compositor::{PAGE_CORNER_RADIUS, ShapeDraw, ShapeExtent};
use specular_core::CanvasRect;
use specular_interact::{App, Corner, HANDLE_SIZE, region_on_canvas, to_canvas_rect};

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

/// Replaces the contents of `out` with the chrome shapes for `app`; `None`
/// (chrome off) leaves it empty. `out` keeps its capacity across frames.
pub(crate) fn build_shapes(app: Option<&App>, out: &mut Vec<ShapeDraw>) {
    out.clear();
    let Some(app) = app else {
        return;
    };
    let session = app.session();
    for (id, _, placement) in app.pages() {
        let (stroke, stroke_width) = if session.hover.as_ref() == Some(id) {
            (BORDER_HOVER, BORDER_HOVER_WIDTH)
        } else {
            (BORDER, BORDER_WIDTH)
        };
        out.push(page_outline(
            to_canvas_rect(placement.rect),
            stroke,
            stroke_width,
        ));
    }
    for annotation in app.document().annotations() {
        if let Some(region) = region_on_canvas(app, annotation) {
            push_annotation(to_canvas_rect(region), out);
        }
    }
    if let Some(region) = session.comment_preview() {
        push_annotation(to_canvas_rect(region), out);
    }
    for id in session.selection.entities() {
        if let Some(entity) = app.document().entity(id) {
            let rect = to_canvas_rect(entity.rect);
            out.push(page_outline(rect, SELECTION, SELECTION_WIDTH));
        }
    }
    if let Some((_, rect)) = app.handle_target() {
        out.extend(Corner::ALL.map(|corner| handle(corner.point(rect).as_vec2())));
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
    use specular_core::{Modifiers, PointerButton, PointerEventKind};
    use specular_doc::{EntityId, ItemId, Rect};
    use specular_interact::{Action, Event, Key, KeyInput, PointerInput, update};

    use super::*;
    use crate::scene::{document_of, page_entity};

    /// Two 400x300 pages, `p1` at the origin and `p2` 500 to its right.
    fn app() -> App {
        let pages = [("p1", 0.0), ("p2", 500.0)].map(|(id, x)| {
            page_entity(id, "https://example.com/", Rect::new(x, 0.0, 400.0, 300.0))
        });
        let mut app = App::new(0);
        let document = document_of(pages.into()).unwrap();
        update(&mut app, Event::DocumentOpened(Box::new(document)));
        app
    }

    fn pointer(app: &mut App, kind: PointerEventKind, at: (f32, f32)) {
        update(
            app,
            Event::Pointer(PointerInput {
                kind,
                screen: Vec2::new(at.0, at.1),
                modifiers: Modifiers::default(),
            }),
        );
    }

    fn left(pressed: bool) -> PointerEventKind {
        let (button, click_count) = (PointerButton::Left, 1);
        if pressed {
            PointerEventKind::Down {
                button,
                click_count,
            }
        } else {
            PointerEventKind::Up {
                button,
                click_count,
            }
        }
    }

    fn select(app: &mut App, id: &str) {
        let item = ItemId::Entity(EntityId::from(id));
        update(app, Event::Action(Action::Select(vec![item])));
    }

    /// Arms the comment tool and presses at `from`.
    fn start_region(app: &mut App, from: (f32, f32)) {
        let key = KeyInput {
            key: Key::Char('c'),
            pressed: true,
            repeat: false,
            text: None,
            modifiers: Modifiers::default(),
            windows_key_code: 0,
            native_key_code: 0,
        };
        update(app, Event::Key(key));
        pointer(app, left(true), from);
    }

    fn built(app: &App) -> Vec<ShapeDraw> {
        let mut out = Vec::new();
        build_shapes(Some(app), &mut out);
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
        let shapes = built(&app());
        assert_eq!(shapes.len(), 2);
        assert_eq!(
            canvas_rect(&shapes[1]),
            Some(CanvasRect::new(500.0, 0.0, 400.0, 300.0))
        );
        assert_eq!(shapes[1].corner_radius, PAGE_CORNER_RADIUS);
        assert_eq!(shapes[1].fill[3], 0.0);
    }

    #[test]
    fn the_hovered_page_has_a_stronger_border() {
        let mut app = app();
        pointer(&mut app, PointerEventKind::Move, (600.0, 100.0));
        let shapes = built(&app);
        assert_eq!((shapes[0].stroke, shapes[1].stroke), (BORDER, BORDER_HOVER));
        assert!(shapes[1].stroke_width > shapes[0].stroke_width);
    }

    #[test]
    fn selection_adds_an_outline_and_four_corner_handles() {
        let mut app = app();
        select(&mut app, "p1");
        let shapes = built(&app);
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
        let mut app = app();
        start_region(&mut app, (410.0, 310.0));
        pointer(&mut app, left(false), (460.0, 360.0));
        select(&mut app, "p1");
        let shapes = built(&app);
        // Borders (2), region, pin, outline, handles (4).
        assert_eq!(shapes.len(), 9);
        assert_eq!(shapes[2].stroke, COMMENT);
        assert_eq!(shapes[4].stroke, SELECTION);
    }

    #[test]
    fn the_pin_is_a_screen_sized_circle_at_the_region_corner() {
        let mut app = app();
        start_region(&mut app, (410.0, 320.0));
        pointer(&mut app, left(false), (460.0, 370.0));
        let pin = built(&app)[3];
        assert_eq!(
            pin.extent,
            ShapeExtent::Screen {
                anchor: Vec2::new(410.0, 320.0),
                size: Vec2::splat(PIN_SIZE)
            }
        );
        assert_eq!(pin.corner_radius, PIN_SIZE / 2.0);
    }

    #[test]
    fn the_tool_preview_draws_like_an_annotation() {
        let mut app = app();
        start_region(&mut app, (5.0, 5.0));
        pointer(&mut app, PointerEventKind::Move, (35.0, 35.0));
        let shapes = built(&app);
        assert_eq!(shapes.len(), 4);
        assert_eq!(
            canvas_rect(&shapes[2]),
            Some(CanvasRect::new(5.0, 5.0, 30.0, 30.0))
        );
    }

    #[test]
    fn chrome_off_draws_nothing_and_clears_the_previous_frame() {
        let app = app();
        let mut out = built(&app);
        build_shapes(None, &mut out);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn rebuilding_reuses_the_buffer() {
        let app = app();
        let mut out = built(&app);
        let capacity = out.capacity();
        build_shapes(Some(&app), &mut out);
        assert_eq!((out.len(), out.capacity()), (2, capacity));
    }
}
