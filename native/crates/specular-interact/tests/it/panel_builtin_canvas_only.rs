//! The built-in panels for a shell whose UI library draws the toolbar: only
//! a popup beside a canvas item is laid out, hit and clicked.

use glam::Vec2;
use specular_doc::Rect;
use specular_interact::{Event, Hit, Tool, hit_test};
use specular_testkit::{TestApp, sticky};

const A: Rect = Rect::new(300.0, 300.0, 200.0, 200.0);

fn app() -> TestApp {
    let mut app = TestApp::with_entities([sticky("t", A, "note")]);
    app.viewport(Vec2::new(1600.0, 1000.0));
    app.send(Event::BuiltinCanvasPopups);
    app
}

#[test]
fn there_is_no_toolbar_and_the_strip_it_would_cover_is_canvas() {
    let app = app();
    assert!(app.panel_layout().toolbar.is_none());
    let hit = hit_test(app.app(), Vec2::new(800.0, 20.0));
    assert!(!matches!(hit, Hit::Panel { .. }), "{hit:?}");
}

#[test]
fn a_selection_still_has_its_popup() {
    let mut app = app();
    app.select(&["t"]);
    let layout = app.panel_layout();
    assert!(layout.popup.is_some());
    assert!(layout.controls().count() > 0);
}

#[test]
fn a_tool_popup_hangs_from_the_toolbar_so_it_is_not_built_in() {
    let mut app = app();
    app.tool(Tool::AddSticky);
    assert!(app.panel_layout().popup.is_none());
}
