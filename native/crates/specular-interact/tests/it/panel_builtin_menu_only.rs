//! The built-in panels for a shell whose UI library draws the chrome and
//! the sidebar: only the context menu is laid out, hit and clicked.

use glam::Vec2;
use specular_doc::Rect;
use specular_interact::{Event, Hit, Tool, hit_test};
use specular_testkit::{TestApp, sticky};

const A: Rect = Rect::new(300.0, 300.0, 200.0, 200.0);

fn app() -> TestApp {
    let mut app = TestApp::with_entities([sticky("t", A, "note")]);
    app.viewport(Vec2::new(1600.0, 1000.0));
    app.send(Event::BuiltinMenu);
    app
}

#[test]
fn no_chrome_or_sidebar_is_laid_out_whatever_the_app_shows() {
    let mut app = app();
    app.show_sidebar(true).select(&["t"]);
    assert_eq!(app.panel_layout().panels().count(), 0);
    app.tool(Tool::AddSticky);
    assert_eq!(app.panel_layout().panels().count(), 0);
    // The strip the chrome would cover is not hit as a panel: whoever draws
    // the chrome takes the pointer there.
    let hit = hit_test(app.app(), Vec2::new(800.0, 20.0));
    assert!(!matches!(hit, Hit::Panel { .. }), "{hit:?}");
}

#[test]
fn a_right_press_still_opens_the_menu_and_an_item_of_it_runs() {
    let mut app = app();
    app.right_click((400.0, 400.0));
    assert!(app.menu_open());
    app.click_control("menu.delete");
    assert!(app.document().entity(&"t".into()).is_none());
    app.assert_undo_returns_to_start();
}
