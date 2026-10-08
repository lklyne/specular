//! What the built-in toolbar and popup draw, on their own: `view` never
//! draws them, so these scenes hold the panels and nothing under them.

use specular_doc::{Color, ColorPreset, Edge, EdgeEnd, EntityId, LineStyle, PageAnchor, Rect};
use specular_interact::Key;
use specular_testkit::{
    CMD, TestApp, document, group, inside, insta::assert_snapshot, page, plain_text, shape, sticky,
    with_edge,
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
fn the_toolbar_alone() {
    assert_snapshot!("toolbar", app().panel_scene_snapshot());
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
fn a_list_open_with_its_choice_checked_and_a_row_hovered() {
    let mut app = app();
    app.select(&["t"])
        .click_control("text.size")
        .hover_control("text.size.56");
    assert_snapshot!("size_list", app.panel_scene_snapshot());
}

#[test]
fn a_shape_popup_with_its_border_controls_and_the_ones_that_are_off() {
    let mut app = app();
    app.select(&["s"])
        .click_control("shape.border")
        .click_control("shape.border.none");
    assert_snapshot!("border_dropdown", app.panel_scene_snapshot());
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
fn a_tool_in_hand_hangs_its_popup_under_the_toolbar() {
    let mut app = app();
    app.click_control("tool.sticky");
    assert_snapshot!("tool_popup", app.panel_scene_snapshot());
}

#[test]
fn a_zoom_list_open_with_its_trigger_in_the_popover_color() {
    let mut app = app();
    app.click_control("zoom").hover_control("zoom.150");
    assert_snapshot!("zoom_list", app.panel_scene_snapshot());
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
fn a_page_popup_sits_above_the_page_title() {
    let mut app = TestApp::with_entities([page("p", Rect::new(400.0, 300.0, 375.0, 400.0))]);
    app.viewport((1200.0, 800.0)).with_panels().select(&["p"]);
    assert_snapshot!("page_popup", app.panel_scene_snapshot());
}

#[test]
fn a_field_being_edited_shows_its_text_selection_and_caret_cut_at_its_box() {
    let mut app = TestApp::with_entities([page("p", Rect::new(400.0, 300.0, 800.0, 400.0))]);
    app.viewport((1200.0, 800.0)).with_panels().select(&["p"]);
    app.click_control("page.url")
        .chord(CMD, Key::Char('a'))
        .type_text(&"long-address/".repeat(20));
    assert_snapshot!("page_url_editing", app.panel_scene_snapshot());
}

#[test]
fn the_page_tool_popup_is_a_list_of_presets() {
    let mut app = TestApp::empty();
    app.viewport((1200.0, 800.0))
        .with_panels()
        .click_control("tool.page");
    assert_snapshot!("page_tool_popup", app.panel_scene_snapshot());
}

#[test]
fn formatting_buttons_show_on_where_the_caret_is_in_bold_text() {
    let mut app = TestApp::with_entities([sticky(
        "t",
        Rect::new(500.0, 300.0, 200.0, 200.0),
        "a **bold** word",
    )]);
    app.viewport((1200.0, 800.0)).with_panels();
    app.double_click((550.0, 350.0))
        .chord(CMD, Key::ArrowUp)
        .key(Key::ArrowRight)
        .key(Key::ArrowRight)
        .key(Key::ArrowRight)
        .key(Key::ArrowRight);
    assert_snapshot!("formatting_on", app.panel_scene_snapshot());
}

#[test]
fn the_context_menu_of_a_page_has_a_disabled_row_and_keys() {
    let mut app = TestApp::with_entities([page("p", Rect::new(300.0, 200.0, 375.0, 400.0))]);
    app.viewport((1200.0, 800.0)).with_panels();
    app.right_click((400.0, 400.0)).hover_control("menu.reload");
    assert_snapshot!("page_menu", app.panel_scene_snapshot());
}

#[test]
fn the_context_menu_of_empty_canvas_is_drawn_as_a_menu_of_words() {
    let mut app = app();
    app.right_click((100.0, 600.0));
    assert_snapshot!("empty_menu", app.panel_scene_snapshot());
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

/// A canvas with a group, a page with an item hooked to it, and notes, in a
/// space of two canvases, the sidebar shown in a window short enough to
/// scroll.
fn sidebar_app() -> TestApp {
    let hooked = specular_doc::Entity {
        anchor: Some(PageAnchor::new(EntityId::from("p"))),
        ..sticky("h", Rect::new(0.0, 0.0, 100.0, 100.0), "hooked note")
    };
    let canvas = document([
        group("g", Rect::new(0.0, 0.0, 600.0, 400.0)),
        inside(
            "g",
            sticky("m", Rect::new(40.0, 40.0, 100.0, 100.0), "in the group"),
        ),
        page("p", Rect::new(800.0, 0.0, 375.0, 667.0)),
        hooked,
        shape("s", Rect::new(0.0, 500.0, 100.0, 100.0)),
        sticky(
            "t",
            Rect::new(200.0, 500.0, 100.0, 100.0),
            "a sticky with a rather long line of text",
        ),
    ]);
    let mut app = TestApp::with_space([("Home", canvas), ("Plans", document([]))]);
    app.viewport((1200.0, 420.0))
        .with_panels()
        .show_sidebar(true);
    app
}

#[test]
fn the_sidebar_with_its_canvases_groups_and_pages() {
    let mut app = sidebar_app();
    app.click_control("sidebar.notes.g.toggle")
        .select(&["s"])
        .hover_control("sidebar.canvas.tab_2");
    assert_snapshot!("sidebar", app.panel_scene_snapshot());
}

#[test]
fn the_sidebar_scrolled_with_a_name_being_edited() {
    let mut app = sidebar_app();
    let row = app.control_rect("sidebar.canvas.tab_2").centre();
    app.double_click(row).type_text("Roadmap");
    assert_snapshot!("sidebar_rename", app.panel_scene_snapshot());
    app.key(specular_interact::Key::Escape)
        .pointer_move((100.0, 300.0))
        .wheel((0.0, -40.0));
    assert_snapshot!("sidebar_scrolled", app.panel_scene_snapshot());
}
