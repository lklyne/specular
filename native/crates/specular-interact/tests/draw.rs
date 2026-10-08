//! The draw tool: a stroke per press, drag and release, each its own drawing
//! and its own undo step.

use specular_doc::{BrushType, Color, ColorPreset, Drawing, Entity, Kind, Point, Rect};
use specular_interact::{Action, Key, Tool, ToolDefaultPatch};
use specular_testkit::{SHIFT, TestApp, assert_doc_snapshot};

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
