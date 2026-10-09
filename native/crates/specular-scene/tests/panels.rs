//! What the built-in toolbar and dock draw, on their own: `view` never
//! draws them, so these scenes hold the panels and nothing under them.

use specular_doc::{EntityId, PageAnchor, Rect};
use specular_interact::Key;
use specular_testkit::{
    CMD, SHIFT, TestApp, document, group, inside, insta::assert_snapshot, page, shape, sticky,
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
fn the_tab_row_with_the_canvas_two_pages_and_a_document() {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(0.0, 0.0, 1280.0, 800.0)),
        specular_testkit::note("n", Rect::new(1400.0, 0.0, 400.0, 500.0), "plan.md"),
        page("p2", Rect::new(1900.0, 0.0, 390.0, 844.0)),
        shape("s", Rect::new(0.0, 900.0, 200.0, 200.0)),
    ]);
    app.viewport((1200.0, 800.0)).with_panels();
    // A title too long for its tab is cut short.
    let title = "A page whose title is far too long to fit".to_owned();
    app.page_reports("p1", specular_interact::PageNotice::Title(title))
        .click_control("view.item.p2")
        .hover_control("view.canvas");
    assert_snapshot!("tab_row", app.panel_scene_snapshot());
}

#[test]
fn the_toolbar_and_a_sticky_in_the_dock() {
    let mut app = app();
    app.select(&["t"]);
    assert_snapshot!("sticky_dock", app.panel_scene_snapshot());
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
fn a_field_being_edited_shows_its_text_selection_and_caret_cut_at_its_box() {
    let mut app = TestApp::with_entities([page("p", Rect::new(400.0, 300.0, 800.0, 400.0))]);
    app.viewport((1200.0, 800.0)).with_panels().select(&["p"]);
    app.click_control("page.url")
        .chord(CMD, Key::Char('a'))
        .type_text(&"long-address/".repeat(20));
    assert_snapshot!("page_url_editing", app.panel_scene_snapshot());
    app.chord(SHIFT, Key::ArrowLeft)
        .chord(SHIFT, Key::ArrowLeft)
        .chord(SHIFT, Key::ArrowLeft);
    assert_snapshot!("page_url_selected", app.panel_scene_snapshot());
    app.key(Key::ArrowRight).compose("にほ");
    assert_snapshot!("page_url_composing", app.panel_scene_snapshot());
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
        anchor: Some(PageAnchor {
            page_url: Some("https://elsewhere.test/".to_owned()),
            ..PageAnchor::new(EntityId::from("p"))
        }),
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

#[test]
fn a_dropdown_of_plain_words_is_a_white_menu() {
    let mut app = app();
    app.select(&["t"]).click_control("text.size");
    assert_snapshot!("size_menu", app.panel_scene_snapshot());
}
