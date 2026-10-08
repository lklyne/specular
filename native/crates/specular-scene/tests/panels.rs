//! What the built-in toolbar and popup draw, on their own: `view` never
//! draws them, so these scenes hold the panels and nothing under them.

use specular_doc::{Color, ColorPreset, Edge, EdgeEnd, LineStyle, Rect};
use specular_testkit::{
    TestApp, document, insta::assert_snapshot, plain_text, shape, sticky, with_edge,
};

/// A sticky in the middle of a 1200x800 viewport, the built-in panels on.
fn app() -> TestApp {
    let mut app = TestApp::with_entities([
        sticky("t", Rect::new(500.0, 300.0, 200.0, 200.0), "note"),
        shape("s", Rect::new(800.0, 300.0, 200.0, 200.0)),
    ]);
    app.viewport((1200.0, 800.0)).with_panels();
    app
}

#[test]
fn the_toolbar_and_a_sticky_popup() {
    let mut app = app();
    app.select(&["t"]);
    assert_snapshot!("sticky_popup", app.panel_scene_snapshot());
}

#[test]
fn a_color_dropdown_open_with_its_choice_ringed() {
    let mut app = app();
    app.select(&["t"])
        .click_control("text.color")
        .click_control("text.color.swatches.green");
    assert_snapshot!("color_dropdown", app.panel_scene_snapshot());
}

#[test]
fn a_tool_in_hand_with_a_button_hovered_and_one_pressed() {
    let mut app = app();
    app.click_control("tool.draw").hover_control("tool.sticky");
    assert_snapshot!("tool_hover", app.panel_scene_snapshot());
    app.press_control("width.thick");
    assert_snapshot!("control_pressed", app.panel_scene_snapshot());
}

#[test]
fn an_edge_popup_with_its_stroke_list_open() {
    let a = plain_text("a", Rect::new(300.0, 300.0, 200.0, 100.0), "a");
    let b = plain_text("b", Rect::new(700.0, 300.0, 200.0, 100.0), "b");
    let edge = Edge {
        color: Some(Color::Preset(ColorPreset::Red)),
        line_style: Some(LineStyle::Dashed),
        from_end: Some(EdgeEnd::Arrow),
        ..Edge::new("e", "a", "b")
    };
    let mut app = TestApp::from_document(with_edge(document([a, b]), edge));
    app.viewport((1200.0, 800.0)).with_panels().select(&["e"]);
    app.click_control("edge.stroke");
    assert_snapshot!("edge_popup", app.panel_scene_snapshot());
}

#[test]
fn the_panels_are_not_in_the_scene_view_builds() {
    let mut with = app();
    with.select(&["t"]).click_control("text.color");
    let mut without = TestApp::with_entities([
        sticky("t", Rect::new(500.0, 300.0, 200.0, 200.0), "note"),
        shape("s", Rect::new(800.0, 300.0, 200.0, 200.0)),
    ]);
    without.viewport((1200.0, 800.0)).select(&["t"]);
    assert_eq!(with.scene_snapshot(), without.scene_snapshot());
    assert_eq!(without.panel_scene_snapshot(), "");
}
