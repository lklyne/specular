use specular_doc::EntityId;
use specular_scene::{
    Color, EllipseDraw, Item, PageDraw, PathDraw, PathStroke, RectDraw, Stroke, StrokeAlign,
};

use super::*;

const RED: Color = Color::rgb(255, 0, 0);

pub(crate) fn view(pan: Vec2, zoom: f32) -> ViewTransform {
    ViewTransform {
        camera: Camera::new(pan, zoom),
        viewport: Vec2::new(800.0, 600.0),
        scale_factor: 1.0,
        target: [800, 600],
    }
}

pub(crate) fn rect(x: f32, y: f32, width: f32, height: f32) -> RectDraw {
    RectDraw::filled(Rect::new(x, y, width, height), RED)
}

pub(crate) fn text(x: f32, y: f32, size: f32) -> TextRun {
    TextRun::new("note", Point::new(x, y), size, Color::BLACK)
}

/// Places `items`, measuring every run as 40 by 20 units.
pub(crate) fn placed(view: &ViewTransform, items: Vec<Item>) -> (Vec<Placed>, PlaceCounts) {
    let scene = Scene { items };
    let mut placed = Vec::new();
    let counts = place(&scene, view, |_| Size::new(40.0, 20.0), &mut placed);
    (placed, counts)
}

fn kept(view: &ViewTransform, items: Vec<Item>) -> Vec<usize> {
    placed(view, items).0.iter().map(|p| p.item).collect()
}

#[test]
fn canvas_items_outside_the_viewport_are_culled_in_paint_order() {
    let items = vec![
        Item::canvas(rect(300.0, 0.0, 10.0, 10.0)),
        Item::canvas(rect(5_000.0, 0.0, 10.0, 10.0)),
        Item::canvas(rect(100.0, 0.0, 10.0, 10.0)),
    ];
    assert_eq!(kept(&view(Vec2::ZERO, 1.0), items), [0, 2]);
}

#[test]
fn culling_follows_the_camera_for_canvas_items_only() {
    // At zoom 0.5 the viewport spans 1600 canvas units.
    let items = || {
        vec![
            Item::canvas(rect(1_500.0, 0.0, 50.0, 50.0)),
            Item::screen(rect(1_500.0, 0.0, 50.0, 50.0)),
            Item::screen(rect(700.0, 0.0, 50.0, 50.0)),
        ]
    };
    assert_eq!(
        [
            kept(&view(Vec2::ZERO, 0.5), items()),
            kept(&view(Vec2::ZERO, 1.0), items())
        ],
        [vec![0, 2], vec![2]]
    );
}

#[test]
fn an_outside_stroke_keeps_a_shape_that_is_just_off_screen() {
    let bare = rect(-110.0, 0.0, 100.0, 100.0);
    let stroked = bare.with_stroke(Stroke::new(RED, 20.0, StrokeAlign::Outside));
    let items = vec![Item::canvas(bare), Item::canvas(stroked)];
    assert_eq!(kept(&view(Vec2::ZERO, 1.0), items), [1]);
}

#[test]
fn an_item_whose_clip_misses_the_viewport_is_culled() {
    let items = vec![
        Item::canvas(rect(0.0, 0.0, 100.0, 100.0)).clipped(Rect::new(-50.0, -50.0, 40.0, 40.0)),
        Item::canvas(rect(0.0, 0.0, 100.0, 100.0)).clipped(Rect::new(200.0, 0.0, 40.0, 40.0)),
    ];
    let (placed, counts) = placed(&view(Vec2::ZERO, 1.0), items);
    assert_eq!((placed.len(), counts.culled), (0, 2));
}

#[test]
fn bounds_and_clip_are_cut_to_the_clip_and_the_viewport() {
    let item = Item::canvas(rect(700.0, 100.0, 200.0, 100.0))
        .clipped(Rect::new(750.0, 120.0, 500.0, 500.0));
    let (placed, _) = placed(&view(Vec2::ZERO, 1.0), vec![item]);
    let expected = Rect::new(750.0, 120.0, 50.0, 81.0);
    assert_eq!(
        (placed[0].bounds, placed[0].clip),
        (expected, Some(Rect::new(750.0, 120.0, 50.0, 480.0)))
    );
}

#[test]
fn transparent_items_are_culled() {
    let items = vec![Item::canvas(rect(0.0, 0.0, 10.0, 10.0)).with_opacity(0.0)];
    assert_eq!(placed(&view(Vec2::ZERO, 1.0), items).1.culled, 1);
}

#[test]
fn canvas_text_under_the_size_floor_is_skipped_and_counted() {
    // 14 units at zoom 0.15 is 2.1 px; at zoom 0.2 it is 2.8 px.
    let items = || vec![Item::canvas(text(10.0, 10.0, 14.0))];
    let small = placed(&view(Vec2::ZERO, 0.15), items());
    let readable = placed(&view(Vec2::ZERO, 0.2), items());
    assert_eq!(
        (small.0.len(), small.1.text_too_small, readable.0.len()),
        (0, 1, 1)
    );
}

#[test]
fn screen_text_ignores_zoom_for_the_size_floor() {
    let items = vec![Item::screen(text(10.0, 10.0, 11.0))];
    assert_eq!(kept(&view(Vec2::ZERO, 0.02), items), [0]);
}

#[test]
fn text_bounds_come_from_the_measured_lines() {
    let (placed, _) = placed(
        &view(Vec2::new(10.0, 0.0), 2.0),
        vec![Item::canvas(text(100.0, 50.0, 14.0))],
    );
    // 40 x 20 units at zoom 2, from (210, 100), plus the fringe.
    assert_eq!(placed[0].bounds, Rect::new(209.0, 99.0, 82.0, 42.0));
}

#[test]
fn off_screen_wrapped_text_is_culled_without_being_measured() {
    let run = TextRun {
        wrap_width: Some(200.0),
        ..text(2_000.0, 10.0, 14.0)
    };
    let scene = Scene {
        items: vec![Item::canvas(run), Item::canvas(text(10.0, 900.0, 14.0))],
    };
    let mut measured = 0;
    let mut placed = Vec::new();
    place(
        &scene,
        &view(Vec2::ZERO, 1.0),
        |_| {
            measured += 1;
            Size::new(40.0, 20.0)
        },
        &mut placed,
    );
    assert_eq!((placed.len(), measured), (0, 0));
}

#[test]
fn each_draw_goes_to_its_pipeline() {
    let stroke = PathStroke::new(RED, 2.0);
    let line = PathDraw::polyline([Point::new(0.0, 0.0), Point::new(50.0, 50.0)], stroke);
    let page = PageDraw {
        page: EntityId::new("p"),
        rect: Rect::new(0.0, 0.0, 100.0, 100.0),
        corner_radius: 8.0,
    };
    let items = vec![
        Item::canvas(page),
        Item::canvas(rect(0.0, 0.0, 10.0, 10.0)),
        Item::canvas(EllipseDraw::filled(Rect::new(0.0, 0.0, 10.0, 10.0), RED)),
        Item::canvas(line),
        Item::canvas(text(0.0, 0.0, 14.0)),
        Item::screen(text(0.0, 0.0, 14.0)),
    ];
    let prims: Vec<_> = placed(&view(Vec2::ZERO, 1.0), items)
        .0
        .iter()
        .map(|p| p.prim)
        .collect();
    assert_eq!(
        prims,
        [
            Prim::Page,
            Prim::Shape,
            Prim::Shape,
            Prim::Mesh(Blend::Normal),
            Prim::Text(Space::Canvas),
            Prim::Text(Space::Screen)
        ]
    );
}

#[test]
fn scissor_is_in_physical_pixels_and_inside_the_target() {
    let view = ViewTransform {
        scale_factor: 2.0,
        target: [1_600, 1_200],
        ..view(Vec2::ZERO, 1.0)
    };
    let scissor = view.scissor(Rect::new(700.0, 10.0, 200.0, 20.0));
    assert_eq!(
        scissor,
        Some(Scissor {
            x: 1_400,
            y: 20,
            width: 200,
            height: 40
        })
    );
}

#[test]
fn a_clip_thinner_than_a_pixel_has_no_scissor() {
    let scissor = view(Vec2::ZERO, 1.0).scissor(Rect::new(10.1, 10.0, 0.2, 50.0));
    assert_eq!(scissor, None);
}
