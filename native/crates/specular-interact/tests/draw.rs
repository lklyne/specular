//! The draw tool: a stroke per press, drag and release, each its own drawing
//! and its own undo step.

use specular_doc::{BrushType, Color, ColorPreset, Drawing, Entity, EntityId, Kind, Point, Rect};
use specular_interact::{Action, Gesture, Key, Tool, ToolDefaultPatch};
use specular_testkit::{SHIFT, TestApp, assert_doc_snapshot};

/// The page of `TestApp::with_pages(1)`.
const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

/// The drawings, back-to-front.
fn drawings(app: &TestApp) -> Vec<(&Entity, &Drawing)> {
    app.document()
        .entities()
        .filter_map(|entity| match &entity.kind {
            Kind::Drawing(drawing) => Some((entity, drawing)),
            _ => None,
        })
        .collect()
}

fn points(drawing: &Drawing) -> Vec<(f64, f64)> {
    drawing.strokes[0]
        .points
        .iter()
        .map(|point| (point.x, point.y))
        .collect()
}

#[test]
fn a_press_drag_release_with_the_draw_tool_is_one_stroke_in_one_drawing() {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw)
        .press((100.0, 100.0))
        .drag_to((140.0, 120.0))
        .drag_to((180.0, 90.0))
        .release();
    assert_doc_snapshot!(app);
    let drawn = drawings(&app);
    assert_eq!(drawn.len(), 1);
    let (entity, drawing) = drawn[0];
    assert_eq!(
        points(drawing),
        [(100.0, 100.0), (140.0, 120.0), (180.0, 90.0)],
        "points are in canvas space"
    );
    // The points, grown by half the 2-unit stroke.
    assert_eq!(entity.rect, Rect::new(99.0, 89.0, 82.0, 32.0));
    let stroke = &drawing.strokes[0];
    assert_eq!(
        (&stroke.color, stroke.width, stroke.brush),
        (&Color::Preset(ColorPreset::Red), 2.0, Some(BrushType::Pen))
    );
    assert_eq!(app.session().tool, Tool::Draw, "the tool is persistent");
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
    app.assert_undo_returns_to_start();
}

#[test]
fn each_stroke_is_its_own_drawing_and_its_own_undo_step() {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw)
        .drag((100.0, 100.0), (150.0, 150.0))
        .drag((300.0, 100.0), (350.0, 150.0));
    assert_eq!(drawings(&app).len(), 2);
    let ids: Vec<&EntityId> = drawings(&app)
        .iter()
        .map(|(entity, _)| &entity.id)
        .collect();
    assert_ne!(ids[0], ids[1]);
    app.undo();
    let left = drawings(&app);
    assert_eq!(left.len(), 1);
    assert_eq!(points(left[0].1), [(100.0, 100.0), (150.0, 150.0)]);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn the_stroke_in_flight_is_in_the_document_and_named_by_the_app() {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw)
        .press((100.0, 100.0))
        .drag_to((120.0, 130.0));
    let live = app.app().creating().cloned().expect("a stroke in flight");
    assert!(matches!(app.session().gesture, Some(Gesture::Draw(_))));
    let drawn = drawings(&app);
    assert_eq!(drawn[0].0.id, live);
    assert_eq!(points(drawn[0].1), [(100.0, 100.0), (120.0, 130.0)]);
    assert!(!app.app().can_undo(), "nothing is recorded until release");
    app.release();
    assert_eq!(drawings(&app)[0].0.id, live, "the release keeps the id");
    app.assert_undo_returns_to_start();
}

#[test]
fn a_click_with_the_draw_tool_leaves_a_dot() {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw).click((50.0, 60.0));
    let drawn = drawings(&app);
    assert_eq!(points(drawn[0].1), [(50.0, 60.0)]);
    assert_eq!(drawn[0].0.rect, Rect::new(49.0, 59.0, 2.0, 2.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn the_brush_color_and_width_come_from_the_tool_defaults() {
    let mut app = TestApp::empty();
    app.chord(SHIFT, Key::Char('m'))
        .act(Action::SetToolDefault(ToolDefaultPatch::DrawColor(
            Color::Preset(ColorPreset::Cyan),
        )))
        .act(Action::SetToolDefault(ToolDefaultPatch::DrawStrokeWidth(
            12.0,
        )))
        .drag((100.0, 100.0), (200.0, 100.0));
    let drawn = drawings(&app);
    let stroke = &drawn[0].1.strokes[0];
    assert_eq!(
        (&stroke.color, stroke.width, stroke.brush),
        (
            &Color::Preset(ColorPreset::Cyan),
            12.0,
            Some(BrushType::Highlight)
        )
    );
    assert_eq!(drawn[0].0.rect, Rect::new(94.0, 94.0, 112.0, 12.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn shift_keeps_new_points_on_a_45_degree_line_from_the_first() {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw)
        .press((100.0, 100.0))
        .hold(SHIFT)
        .drag_to((200.0, 108.0))
        .release()
        .let_go();
    let drawn = drawings(&app);
    let Point { x, y } = drawn[0].1.strokes[0].points[1];
    assert!((y - 100.0).abs() < 1e-9, "flat, not {y}");
    assert!(x > 200.0, "as far from the start as the pointer, not {x}");
    app.assert_undo_returns_to_start();
}

#[test]
fn drawing_clears_the_selection_and_ignores_what_is_under_the_pointer() {
    let mut app = TestApp::with_pages(1);
    app.select(&["p1"])
        .tool(Tool::Draw)
        .drag((150.0, 150.0), (250.0, 250.0));
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
    assert_eq!(app.rect("p1"), P1, "the page did not move");
    assert_eq!(drawings(&app).len(), 1);
    app.assert_undo_returns_to_start();
}

#[test]
fn escape_mid_stroke_drops_it_and_returns_to_select() {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw)
        .press((100.0, 100.0))
        .drag_to((200.0, 200.0))
        .key(Key::Escape);
    assert_eq!(app.document().entities().count(), 0);
    assert_eq!(app.session().tool, Tool::Select);
    assert!(!app.app().can_undo());
    app.release().assert_undo_returns_to_start();
}
