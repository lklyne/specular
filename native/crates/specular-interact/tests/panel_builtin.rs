//! The built-in panels under the pointer: a click on a control runs its
//! action, and nothing pressed on a panel reaches the canvas.

use specular_doc::{BrushType, Rect};
use specular_interact::{
    Action, ControlId, Cursor, Hit, Key, Tool, ToolDefaultPatch, hit_test,
    panel::builtin::TOOLBAR_HEIGHT,
};
use specular_testkit::{CMD, TestApp, assert_doc_snapshot, shape, sticky};

const A: Rect = Rect::new(300.0, 300.0, 200.0, 200.0);
const B: Rect = Rect::new(700.0, 300.0, 200.0, 200.0);

/// A sticky and a shape, with the built-in panels on.
fn app() -> TestApp {
    let mut app = TestApp::with_entities([sticky("t", A, "note"), shape("s", B)]);
    app.with_panels();
    app
}

fn open(app: &TestApp) -> Option<&str> {
    app.session().panel.open.as_ref().map(ControlId::as_str)
}

#[test]
fn each_tool_button_arms_its_tool() {
    let mut app = app();
    let tools = [
        ("tool.draw", Tool::Draw),
        ("tool.sticky", Tool::AddSticky),
        ("tool.shape", Tool::AddShape),
        ("tool.page", Tool::AddPage),
        ("tool.text", Tool::AddText),
        ("tool.document", Tool::AddDocument),
        ("tool.comment", Tool::Comment),
        ("tool.select", Tool::Select),
    ];
    for (control, tool) in tools {
        app.click_control(control);
        assert_eq!(app.session().tool, tool, "{control}");
    }
    assert!(app.session().gesture.is_none());
}

#[test]
fn a_second_click_puts_the_draw_and_comment_tools_down_and_no_other() {
    let mut app = app();
    app.click_control("tool.draw").click_control("tool.draw");
    assert_eq!(app.session().tool, Tool::Select);
    app.click_control("tool.comment")
        .click_control("tool.comment");
    assert_eq!(app.session().tool, Tool::Select);
    app.click_control("tool.shape").click_control("tool.shape");
    assert_eq!(app.session().tool, Tool::AddShape);
}

#[test]
fn the_draw_button_keeps_the_brush_the_draw_key_would_reset() {
    let mut app = app();
    app.act(Action::SetToolDefault(ToolDefaultPatch::Brush(
        BrushType::Highlight,
    )));
    app.click_control("tool.draw");
    assert_eq!(
        (app.session().tool, app.app().tool_defaults().draw.brush),
        (Tool::Draw, BrushType::Highlight)
    );
}

#[test]
fn a_click_on_a_panel_leaves_the_selection_the_document_and_the_canvas_alone() {
    let mut app = app();
    app.select(&["t"]);
    let Some(popup) = app.panel_layout().popup.map(|popup| popup.rect) else {
        panic!("no popup");
    };
    // Bare toolbar, then the popup's own padding: on a panel, on no control.
    for at in [(20.0, 20.0), (popup.x + 2.0, popup.y + 2.0)] {
        app.pointer_move(at).press(at);
        assert!(app.session().gesture.is_none(), "{at:?}");
        app.drag_to((at.0 + 60.0, at.1 + 90.0)).release();
        assert_eq!(app.selected_ids(), ["t"], "{at:?}");
    }
    assert!(!app.app().can_undo());
    assert_eq!(app.rect("t"), A);
}

#[test]
fn a_swatch_from_the_color_dropdown_recolors_the_sticky_in_one_undo_step() {
    let mut app = app();
    app.select(&["t"]).click_control("text.color");
    assert_eq!(open(&app), Some("text.color"));
    app.click_control("text.color.swatches.green");
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"t","type":"text","x":300,"y":300,"width":200,"height":200,"text":"note","color":"4","specular":{"textStyle":"sticky"}}
      {"id":"s","type":"shape","x":700,"y":300,"width":200,"height":200,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["t","s"]}
    "#);
    // A color is tried, then another: the list stays open.
    assert_eq!(open(&app), Some("text.color"));
    assert_eq!(app.selected_ids(), ["t"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_choice_from_a_list_closes_it_and_a_stepper_under_it_does_not() {
    let mut app = app();
    app.select(&["t"]).click_control("text.size");
    app.click_control("text.size.custom.inc");
    assert_eq!(open(&app), Some("text.size"));
    app.click_control("text.size.custom.inc");
    app.click_control("text.size.56");
    assert_eq!(open(&app), None);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"t","type":"text","x":300,"y":300,"width":200,"height":800,"text":"note","specular":{"textStyle":"sticky","textSize":56}}
      {"id":"s","type":"shape","x":700,"y":300,"width":200,"height":200,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["t","s"]}
    "#);
    app.assert_undo_returns_to_start();
}

#[test]
fn escape_closes_the_dropdown_first_and_then_does_what_it_did() {
    let mut app = app();
    app.select(&["t"])
        .tool(Tool::Select)
        .click_control("text.font");
    assert_eq!(open(&app), Some("text.font"));
    app.key(Key::Escape);
    assert_eq!(open(&app), None);
    assert_eq!(app.selected_ids(), ["t"]);
    app.key(Key::Escape);
    assert!(app.selection().is_empty());
}

#[test]
fn escape_with_a_tool_in_hand_closes_the_dropdown_and_keeps_the_tool() {
    let mut app = app();
    app.click_control("tool.shape").click_control("shape.kind");
    app.key(Key::Escape);
    assert_eq!((open(&app), app.session().tool), (None, Tool::AddShape));
    app.key(Key::Escape);
    assert_eq!(app.session().tool, Tool::Select);
}

#[test]
fn a_press_released_off_its_control_does_nothing() {
    let mut app = app();
    app.press_control("tool.shape")
        .drag_to((800.0, 600.0))
        .release();
    assert_eq!(app.session().tool, Tool::Select);
    assert!(app.session().panel.pressed.is_none());
    // Nor does one that slides onto another control.
    app.press_control("tool.shape");
    let other = app.control_rect("tool.draw").centre();
    app.drag_to(other).release();
    assert_eq!(app.session().tool, Tool::Select);
    assert!(app.selection().is_empty() && !app.app().can_undo());
}

#[test]
fn a_press_outside_an_open_dropdown_closes_it_and_goes_no_further() {
    let mut app = app();
    app.select(&["t"]).click_control("text.color");
    // On the shape, which a press would otherwise select and start to move.
    app.pointer_move((800.0, 400.0)).press((800.0, 400.0));
    assert_eq!(open(&app), None);
    assert!(app.session().gesture.is_none());
    app.drag_to((850.0, 450.0)).release();
    assert_eq!(app.selected_ids(), ["t"]);
    assert_eq!(app.rect("s"), B);
    // With it closed the same press is the canvas's again.
    app.click((800.0, 400.0));
    assert_eq!(app.selected_ids(), ["s"]);
}

#[test]
fn a_press_on_the_trigger_of_the_open_dropdown_closes_it() {
    let mut app = app();
    app.select(&["t"])
        .click_control("text.color")
        .click_control("text.color");
    assert_eq!(open(&app), None);
}

#[test]
fn a_dropdown_closes_when_its_popup_goes() {
    let mut app = app();
    app.select(&["t"]).click_control("text.color");
    app.select(&["s"]);
    assert_eq!(open(&app), None);
    app.click_control("shape.kind");
    app.select(&[]);
    assert_eq!(open(&app), None);
}

#[test]
fn opening_the_zoom_levels_puts_the_tool_down() {
    let mut app = app();
    app.click_control("tool.shape").click_control("zoom");
    assert_eq!(
        (open(&app), app.session().tool),
        (Some("zoom"), Tool::Select)
    );
    app.click_control("zoom.50");
    assert_eq!(open(&app), None);
    assert!((app.session().camera.zoom - 0.5).abs() < 1e-6);
}

#[test]
fn a_format_button_keeps_the_text_edit_open() {
    let mut app = app();
    app.double_click((400.0, 400.0));
    app.chord(CMD, Key::Char('a')).click_control("format.bold");
    assert_eq!(app.editing_text(), "**note**");
    // So does a property of the text being edited.
    app.click_control("text.size").click_control("text.size.32");
    assert!(app.session().editing.is_some());
}

#[test]
fn a_control_that_is_off_takes_no_press() {
    let mut app = app();
    app.select(&["s"])
        .click_control("shape.border")
        .click_control("shape.border.none");
    let before = app.doc_snapshot();
    app.click_control("shape.border.w3");
    assert_eq!(app.doc_snapshot(), before);
    assert!(app.session().panel.pressed.is_none());
    app.assert_undo_returns_to_start();
}

#[test]
fn the_wheel_pans_through_the_popup_and_not_through_the_toolbar_or_a_list() {
    let mut app = app();
    app.select(&["t"]);
    let pan = |app: &TestApp| app.session().camera.pan;
    let start = pan(&app);
    app.hover_control("tool.shape").wheel((0.0, 40.0));
    assert_eq!(pan(&app), start);
    app.click_control("text.size")
        .hover_control("text.size.32")
        .wheel((0.0, 40.0))
        .pinch(0.2);
    assert_eq!(app.session().camera, specular_core::Camera::new(start, 1.0));
    app.key(Key::Escape)
        .hover_control("text.size")
        .wheel((0.0, 40.0));
    assert_ne!(pan(&app), start);
}

#[test]
fn the_pointer_over_a_panel_is_an_arrow_and_hovers_nothing_under_it() {
    let mut app = app();
    app.tool(Tool::AddShape).pointer_move((800.0, 600.0));
    assert_eq!(app.session().cursor, Cursor::Crosshair);
    app.hover_control("tool.draw");
    assert_eq!(app.session().cursor, Cursor::Default);
    assert_eq!(
        app.session().panel.hover.as_ref().map(ControlId::as_str),
        Some("tool.draw")
    );
    app.pointer_move((800.0, 600.0));
    assert!(app.session().panel.hover.is_none());
}

#[test]
fn a_point_on_a_panel_hits_the_panel_and_not_what_is_under_it() {
    let mut app = TestApp::with_entities([shape("s", Rect::new(700.0, 0.0, 200.0, 200.0))]);
    let on_bar = glam::Vec2::new(800.0, TOOLBAR_HEIGHT / 2.0);
    assert!(matches!(
        hit_test(app.app(), on_bar),
        Hit::EntityBody { .. }
    ));
    app.with_panels();
    assert_eq!(hit_test(app.app(), on_bar), Hit::Panel { control: None });
    let on_tool = app.control_rect("tool.draw").centre();
    let Hit::Panel { control } = hit_test(app.app(), on_tool) else {
        panic!("the toolbar is over the canvas");
    };
    assert_eq!(control.as_ref().map(ControlId::as_str), Some("tool.draw"));
}

#[test]
fn with_the_built_in_panels_off_nothing_is_a_panel() {
    let mut app = TestApp::with_entities([sticky("t", A, "note")]);
    app.viewport((1600.0, 1000.0)).select(&["t"]);
    assert_eq!(app.panel_snapshot(), "");
    for x in (0..1600).step_by(40) {
        for y in (0..400).step_by(10) {
            let hit = hit_test(app.app(), glam::Vec2::new(x as f32, y as f32));
            assert!(!matches!(hit, Hit::Panel { .. }), "{x},{y}");
        }
    }
    // Where the toolbar would be is canvas: a drag there is a marquee.
    app.press((800.0, 20.0));
    assert!(app.session().gesture.is_some());
}
