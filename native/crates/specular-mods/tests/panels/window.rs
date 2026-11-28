//! The window's own panels as mods (ADR 0059) that have no file of their
//! own: the sidebar's Pages section and the status line against the keys
//! they read. Each shows what its model lists, presses through the host,
//! and renders only when a key it read moves, fast enough for a frame.

use gpui::TestAppContext;
use serde_json::json;
use specular_interact::{Action, Event, SidebarAction, state};
use specular_testkit::TestApp;

use crate::support::{Loaded, assert_renders_inside_a_frame, grid, timed_load};

/// An app with two pages, the sidebar open and the first page selected.
fn app() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.send(Event::ShellChrome);
    app.send(Event::ChatPanel(true));
    app.act(Action::Sidebar(SidebarAction::Toggle));
    app.select(&["p1"]);
    app
}

/// The ids of the sidebar's Pages section and its rows.
fn pages_ids(app: &TestApp) -> Vec<String> {
    let model = &state(app.app())["panels/sidebar"];
    let mut ids = Vec::new();
    for section in model["sections"].as_array().into_iter().flatten() {
        if section["kind"] != "pages" {
            continue;
        }
        ids.extend(section["id"].as_str().map(str::to_owned));
        for row in section["rows"].as_array().into_iter().flatten() {
            ids.extend(row["id"].as_str().map(str::to_owned));
        }
    }
    ids
}

#[gpui::test]
fn the_pages_section_shows_what_its_model_lists_and_presses_through_the_host(
    cx: &mut TestAppContext,
) {
    let app = app();
    let size = (256.0, 300.0);
    let ids = pages_ids(&app);
    assert!(ids.len() >= 3, "pages: the model lists {ids:?}");
    let tree = Loaded::new(&app, "pages", json!({}), size, cx).tree();
    for id in &ids {
        assert!(
            tree.contains(&format!("Button \"{id}\"")),
            "pages: `{id}`:\n{tree}"
        );
    }
    let mut loaded = Loaded::new(&app, "pages", json!({}), size, cx);
    let press = r#"control.press {"id":"sidebar.pages.p2"}"#;
    let runs = loaded.click_until(grid((256, 300), 6), press);
    assert!(runs.iter().any(|run| run == press), "pages: {runs:?}");
}

#[gpui::test]
fn the_status_line_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app();
    let mut loaded = timed_load(&app, "status", json!({}), (900.0, 24.0), cx);
    let read = loaded.reads("status");
    for key in ["canvas/info", "layouts", "look", "selection"] {
        assert!(
            read.iter().any(|each| each == key),
            "status reads {key}: {read:?}"
        );
    }
    assert_eq!(
        loaded.idle_renders(),
        0,
        "status: no script render while idle"
    );
    loaded.assert_camera_moves_nothing(&mut app);
    assert_renders_inside_a_frame(&mut loaded, &mut app, "status", 60, |app, i| {
        app.select(&[if i % 2 == 0 { "p2" } else { "p1" }]);
    });
}
