//! The context menu in the built-in renderer: where a right press opens it,
//! what the press selects, how it is laid out, and what closes it.

#![expect(
    clippy::expect_used,
    reason = "a helper fails the test it is called from"
)]

use specular_doc::Rect;
use specular_interact::panel::builtin::Surface;
use specular_interact::{Action, ClipboardContent, Effect, Event, Key, PageNotice, Tool};
use specular_testkit::{TestApp, group, inside, page, shape, sticky};

const A: Rect = Rect::new(100.0, 300.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 300.0, 200.0, 100.0);

fn opened(entities: impl IntoIterator<Item = specular_doc::Entity>) -> TestApp {
    let mut app = TestApp::with_entities(entities);
    app.with_panels();
    app
}

fn menu_rect(app: &TestApp) -> specular_interact::panel::builtin::PanelRect {
    app.panel_layout().dropdown.expect("a menu is open").rect
}

#[test]
fn a_right_press_on_an_unselected_item_selects_it_and_opens_the_menu() {
    let mut app = opened([shape("a", A), shape("b", B)]);
    app.select(&["b"]);
    app.right_click((150.0, 350.0));
    assert_eq!(app.selected_ids(), ["a"]);
    assert!(app.menu_open());
    let menu = app.panel_layout().dropdown.expect("the menu is laid out");
    assert_eq!(menu.surface, Surface::Dropdown);
    assert_eq!((menu.rect.x, menu.rect.y), (150.0, 350.0), "at the pointer");
}

#[test]
fn a_right_press_on_one_of_several_selected_keeps_them_all() {
    let mut app = opened([shape("a", A), shape("b", B)]);
    app.select(&["a", "b"]);
    app.right_click((450.0, 350.0));
    assert_eq!(app.selected_ids(), ["a", "b"]);
    assert!(app.menu_open());
}

#[test]
fn a_right_press_on_empty_canvas_clears_the_selection_and_offers_the_clipboard() {
    let mut app = opened([shape("a", A)]);
    app.select(&["a"]);
    app.right_click((900.0, 600.0));
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
    assert!(
        app.panel_layout()
            .controls()
            .any(|id| id.as_str() == "menu.paste")
    );
    assert!(
        app.panel_layout()
            .controls()
            .any(|id| id.as_str() == "menu.select-all")
    );
}

#[test]
fn the_menu_is_kept_inside_the_viewport() {
    let mut app = opened([shape("a", A)]);
    app.right_click((1590.0, 990.0));
    let menu = menu_rect(&app);
    assert!(
        menu.right() <= 1600.0 - 8.0 && menu.bottom() <= 1000.0 - 8.0,
        "{menu:?}"
    );
    assert!(
        menu.x > 1000.0 && menu.y > 700.0,
        "pulled back, not moved far"
    );
}

#[test]
fn the_menu_stays_clear_of_the_sidebar() {
    let mut app = opened([shape("a", Rect::new(500.0, 300.0, 200.0, 100.0))]);
    app.show_sidebar(true);
    let covered = app.app().covered_left();
    assert!(covered > 100.0);
    app.right_click((covered + 2.0, 600.0));
    assert!(menu_rect(&app).x >= covered + 8.0);
}

#[test]
fn choosing_an_item_runs_it_closes_the_menu_and_is_one_undo_step() {
    let mut app = opened([shape("a", A), shape("b", B)]);
    app.right_click((150.0, 350.0));
    app.click_control("menu.duplicate");
    assert!(!app.menu_open());
    assert_eq!(app.document().entities().count(), 3);
    app.undo();
    assert_eq!(app.document().entities().count(), 2);
    app.redo();
    app.assert_undo_returns_to_start();
}

#[test]
fn delete_and_the_stack_order_come_from_the_menu() {
    let mut app = opened([
        shape("a", A),
        sticky("n", Rect::new(120.0, 320.0, 100.0, 100.0), "x"),
    ]);
    app.right_click((130.0, 330.0));
    assert_eq!(app.selected_ids(), ["n"]);
    app.click_control("menu.send-to-back");
    assert_eq!(app.document().order()[0].as_str(), "n");
    app.right_click((150.0, 410.0));
    assert_eq!(app.selected_ids(), ["n"], "the part of it a is not over");
    app.click_control("menu.delete");
    assert!(app.document().entity(&"n".into()).is_none());
    app.assert_undo_returns_to_start();
}

#[test]
fn a_paste_lands_where_the_menu_was_opened() {
    let mut app = opened([shape("a", Rect::new(0.0, 0.0, 100.0, 100.0))]);
    app.select(&["a"]).act(Action::Copy);
    let copied = app
        .take_effects()
        .into_iter()
        .find_map(|effect| match effect {
            Effect::WriteClipboard(text) => Some(text),
            _ => None,
        })
        .expect("the copy wrote the clipboard");
    app.right_click((905.0, 705.0));
    app.click_control("menu.paste");
    assert!(!app.menu_open());
    assert_eq!(app.session().pointer, Some(glam::Vec2::new(905.0, 705.0)));
    app.take_effects();
    app.send(Event::Clipboard(ClipboardContent {
        text: Some(copied),
        image: None,
    }));
    let pasted = app.selected().expect("the paste is selected").to_owned();
    assert_eq!(app.rect(&pasted).x, 900.0);
    assert_eq!(app.rect(&pasted).y, 700.0);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_page_menu_has_back_forward_and_reload_that_follow_its_history() {
    let mut app = opened([page("p", Rect::new(100.0, 300.0, 375.0, 400.0))]);
    app.right_click((200.0, 500.0));
    let layout = app.panel_layout();
    let back = layout
        .node(&"menu.back".to_owned().into())
        .expect("back is shown");
    assert!(!back.state.enabled, "no history yet");
    app.page_reports(
        "p",
        PageNotice::Loading {
            loading: false,
            can_go_back: true,
            can_go_forward: false,
        },
    );
    let layout = app.panel_layout();
    let back = layout
        .node(&"menu.back".to_owned().into())
        .expect("back is shown");
    assert!(back.state.enabled);
    app.take_effects();
    app.click_control("menu.back");
    assert!(
        !app.take_effects().is_empty(),
        "the page is asked to go back"
    );
}

#[test]
fn a_disabled_item_does_nothing_and_the_menu_stays() {
    let mut app = opened([page("p", Rect::new(100.0, 300.0, 375.0, 400.0))]);
    app.right_click((200.0, 500.0));
    app.take_effects();
    app.click_control("menu.back");
    assert!(app.menu_open());
    assert_eq!(app.take_effects(), []);
}

#[test]
fn escape_closes_the_menu_and_keeps_the_selection() {
    let mut app = opened([shape("a", A)]);
    app.right_click((150.0, 350.0));
    app.key(Key::Escape);
    assert!(!app.menu_open());
    assert_eq!(app.selected_ids(), ["a"]);
}

#[test]
fn a_press_elsewhere_closes_the_menu_and_goes_no_further() {
    let mut app = opened([shape("a", A), shape("b", B)]);
    app.right_click((150.0, 350.0));
    app.click((450.0, 350.0));
    assert!(!app.menu_open());
    assert_eq!(app.selected_ids(), ["a"], "the click did not select b");
    app.right_click((150.0, 350.0));
    app.right_click((450.0, 350.0));
    assert!(!app.menu_open(), "a second right press only closes it");
    assert_eq!(app.selected_ids(), ["a"]);
}

#[test]
fn a_change_of_selection_closes_the_menu() {
    let mut app = opened([shape("a", A), shape("b", B)]);
    app.right_click((150.0, 350.0));
    app.select(&["b"]);
    assert!(!app.menu_open());
}

#[test]
fn a_right_press_inside_the_entered_page_goes_to_the_page() {
    let mut app = opened([page("p", Rect::new(100.0, 300.0, 375.0, 400.0))]);
    app.click((200.0, 500.0)).click((200.0, 500.0));
    assert_eq!(
        app.session()
            .focus
            .page()
            .map(specular_doc::EntityId::as_str),
        Some("p")
    );
    app.take_effects();
    app.right_click((220.0, 520.0));
    assert!(!app.menu_open());
    assert!(!app.take_effects().is_empty(), "the page heard the press");
}

#[test]
fn a_tool_in_hand_has_no_menu() {
    let mut app = opened([shape("a", A)]);
    app.tool(Tool::AddShape);
    app.right_click((150.0, 350.0));
    assert!(!app.menu_open());
}

#[test]
fn a_grouped_member_is_selected_through_its_group() {
    let mut app = opened([
        group("g", Rect::new(100.0, 300.0, 400.0, 200.0)),
        inside("g", shape("m", Rect::new(120.0, 320.0, 100.0, 100.0))),
    ]);
    app.right_click((110.0, 480.0));
    assert_eq!(app.selected_ids(), ["g"]);
    app.click_control("menu.ungroup");
    assert_eq!(app.selected_ids(), ["m"]);
    app.assert_undo_returns_to_start();
}
