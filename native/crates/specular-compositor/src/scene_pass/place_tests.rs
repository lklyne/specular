use specular_scene::{Color, Item, RectDraw};

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
    let scene = Scene::from_iter(items);
    let mut placed = Vec::new();
    let counts = place(&scene, view, |_| Size::new(40.0, 20.0), &mut placed);
    (placed, counts)
}

#[test]
fn off_screen_wrapped_text_is_culled_without_being_measured() {
    let run = TextRun {
        wrap_width: Some(200.0),
        ..text(2_000.0, 10.0, 14.0)
    };
    let scene = Scene::from_iter([Item::canvas(run), Item::canvas(text(10.0, 900.0, 14.0))]);
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
