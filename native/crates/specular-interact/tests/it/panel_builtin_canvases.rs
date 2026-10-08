//! The Canvases list of the sidebar in the built-in renderer: switching,
//! adding, renaming in place, and the menu of a right click.

use specular_core::PointerButton;
use specular_doc::Rect;
use specular_interact::panel::builtin::Surface;
use specular_interact::{Effect, Key};
use specular_testkit::{CMD, TestApp, document, sticky};

const BOX: Rect = Rect::new(0.0, 0.0, 200.0, 100.0);

fn at(x: f64, y: f64) -> Rect {
    Rect::new(x, y, 200.0, 100.0)
}

fn shown(app: &TestApp, id: &str) -> bool {
    app.panel_layout()
        .controls()
        .any(|control| control.as_str() == id)
}

fn right_click(app: &mut TestApp, id: &str) {
    let at = app.control_rect(id).centre();
    app.pointer_move(at)
        .press_button(PointerButton::Right, at)
        .release();
}

fn spaced() -> TestApp {
    let mut app = TestApp::with_space([
        ("Home", document([sticky("a", at(600.0, 300.0), "a")])),
        ("Plans", document([sticky("b", at(600.0, 300.0), "b")])),
    ]);
    app.with_panels().show_sidebar(true);
    app.take_effects();
    app
}

#[test]
fn a_click_on_a_canvas_switches_to_it() {
    let mut app = spaced();
    app.click_control("sidebar.canvas.tab_2");
    assert_eq!(app.active_canvas(), "Plans");
    assert!(
        shown(&app, "sidebar.notes.b"),
        "the sections list the canvas now shown"
    );
    assert!(!shown(&app, "sidebar.notes.a"));
}

#[test]
fn the_add_button_makes_a_canvas_and_shows_it() {
    let mut app = spaced();
    app.click_control("sidebar.add");
    assert_eq!(app.canvas_names().len(), 3);
    assert_eq!(app.active_canvas(), app.canvas_names()[2]);
    let writes = app.take_effects();
    assert!(
        writes
            .iter()
            .any(|effect| matches!(effect, Effect::WriteCanvas(_)))
    );
}

#[test]
fn a_double_click_renames_in_place_and_enter_keeps_the_name() {
    let mut app = spaced();
    let row = app.control_rect("sidebar.canvas.tab_2").centre();
    app.double_click(row);
    assert_eq!(
        app.field_edit(),
        Some("Plans"),
        "the whole name is selected"
    );
    assert!(shown(&app, "sidebar.canvas.tab_2.name"));
    app.enter_in_field("sidebar.canvas.tab_2.name", "Roadmap");
    assert_eq!(app.canvas_names(), ["Home", "Roadmap"]);
    assert!(app.field_edit().is_none());
    let effects = app.take_effects();
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::RenameCanvasFile { to, .. } if to.starts_with("Roadmap")
    )));
}

#[test]
fn escape_puts_the_old_name_back_and_an_empty_name_asks_for_nothing() {
    let mut app = spaced();
    let row = app.control_rect("sidebar.canvas.tab_2").centre();
    app.double_click(row)
        .chord(CMD, Key::Char('a'))
        .type_text("Roadmap")
        .key(Key::Escape);
    assert_eq!(app.canvas_names(), ["Home", "Plans"]);
    assert!(app.field_edit().is_none());
    app.double_click(row)
        .chord(CMD, Key::Char('a'))
        .key(Key::Backspace)
        .key(Key::Enter);
    assert_eq!(app.canvas_names(), ["Home", "Plans"]);
}

#[test]
fn a_right_click_opens_a_menu_that_renames_or_deletes() {
    let mut app = spaced();
    right_click(&mut app, "sidebar.canvas.tab_2");
    let menu = app.panel_layout().dropdown.expect("the menu is open");
    assert!(menu.menu && menu.surface == Surface::Dropdown);
    app.click_control("sidebar.canvas.tab_2.menu.rename");
    assert_eq!(app.field_edit(), Some("Plans"));
    app.key(Key::Escape);
    assert!(
        app.panel_layout().dropdown.is_none(),
        "choosing closes the menu"
    );

    right_click(&mut app, "sidebar.canvas.tab_2");
    app.click_control("sidebar.canvas.tab_2.menu.delete");
    assert_eq!(app.canvas_names(), ["Home"]);
    let effects = app.take_effects();
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::TrashCanvasFile { .. }))
    );
}

#[test]
fn the_last_canvas_can_be_deleted_and_leaves_a_fresh_one() {
    let mut app = TestApp::with_space([("Solo", document([sticky("a", BOX, "a")]))]);
    app.with_panels().show_sidebar(true);
    right_click(&mut app, "sidebar.canvas.tab_1");
    app.click_control("sidebar.canvas.tab_1.menu.delete");
    assert_eq!(app.canvas_names(), ["Canvas 1"]);
}

#[test]
fn a_head_folds_its_section_and_a_folded_list_is_titled_by_the_canvas() {
    let mut app = spaced();
    assert!(shown(&app, "sidebar.canvas.tab_1"));
    app.click_control("sidebar.head.canvases");
    assert!(!shown(&app, "sidebar.canvas.tab_1"));
    assert!(
        shown(&app, "sidebar.notes.a"),
        "the sections are not folded with it"
    );
    app.click_control("sidebar.head.notes");
    assert!(!shown(&app, "sidebar.notes.a"));
    app.click_control("sidebar.head.notes")
        .click_control("sidebar.head.canvases");
    assert!(shown(&app, "sidebar.notes.a") && shown(&app, "sidebar.canvas.tab_1"));
}
