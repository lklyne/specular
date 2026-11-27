//! The context menu mod against `panels/menu`.

use glam::Vec2;
use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_interact::{Event, state};
use specular_testkit::TestApp;

use crate::support::{Loaded, grid, timed_load, window_app};

const SIZE: (f32, f32) = (320.0, 640.0);

/// An app with its page selected and the context menu open over it.
fn app_with_a_menu() -> TestApp {
    let mut app = window_app();
    app.select(&["p1"]);
    app.send(Event::ContextMenu(Vec2::new(300.0, 250.0)));
    app
}

/// Every item id the open menu lists.
fn listed_ids(menu: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    for control in menu["menu"]["controls"].as_array().into_iter().flatten() {
        for section in control["content"].as_array().into_iter().flatten() {
            for option in section["options"]["options"]
                .as_array()
                .into_iter()
                .flatten()
            {
                ids.extend(option["id"].as_str().map(str::to_owned));
            }
        }
    }
    ids
}

#[gpui::test]
fn the_menu_shows_every_item_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    let app = app_with_a_menu();
    let listed = listed_ids(&state(app.app())["panels/menu"]);
    assert!(
        listed.len() >= 5,
        "the open menu lists its items: {listed:?}"
    );
    let two: Vec<&str> = listed.iter().take(2).map(String::as_str).collect();
    for hidden in [&[][..], &two[..]] {
        let tree = Loaded::new(&app, "menu", json!({ "hidden": hidden }), SIZE, cx).tree();
        for id in &listed {
            let hide = hidden.contains(&id.as_str());
            let shown = tree.contains(&format!("Button \"{id}\""));
            assert_eq!(shown, !hide, "`{id}` with {hidden:?} hidden:\n{tree}");
        }
    }
    let mut app = window_app();
    let mut closed = Loaded::new(&app, "menu", json!({}), SIZE, cx);
    let tree = closed.tree();
    assert!(
        !tree.contains("Button"),
        "no menu while none is open:\n{tree}"
    );
    closed.assert_camera_moves_nothing(&mut app);
}

#[gpui::test]
fn a_click_on_an_item_presses_it_through_the_host(cx: &mut TestAppContext) {
    let app = app_with_a_menu();
    let menu = state(app.app())["panels/menu"].clone();
    let enabled = (menu["menu"]["controls"][0]["content"]
        .as_array()
        .into_iter()
        .flatten())
    .flat_map(|section| {
        section["options"]["options"]
            .as_array()
            .into_iter()
            .flatten()
    })
    .find(|option| option["enabled"] == true)
    .and_then(|option| option["id"].as_str())
    .expect("an enabled item")
    .to_owned();
    let mut loaded = Loaded::new(&app, "menu", json!({}), SIZE, cx);
    let wanted = format!(r#"control.press {{"id":"{enabled}"}}"#);
    let runs = loaded.click_until(grid((200, 400), 6), &wanted);
    assert!(runs.contains(&wanted), "{runs:?}");
}

#[gpui::test]
fn the_menu_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app_with_a_menu();
    let mut loaded = timed_load(&app, "menu", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("menu"),
        ["look", "mods/menu/props", "panels/menu"]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    crate::support::assert_renders_inside_a_frame(&mut loaded, &mut app, "menu", 300, |app, i| {
        // The menu opens again a little to one side or the other each time.
        app.send(Event::MenuClosed);
        let x = 301.0 + (i % 2) as f32 * 7.0;
        app.send(Event::ContextMenu(Vec2::new(x, 250.0)));
    });
}
