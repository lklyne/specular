//! The tab row mod against `panels/strip`.

use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_doc::Command;
use specular_doc::EntityId;
use specular_interact::panel::TAB_ROW;
use specular_interact::{Action, ApiCall, ApiRun, Event, Showing, ThreadId, state};

use crate::support::{Loaded, along, timed_load, window_app};

const SIZE: (f32, f32) = (1200.0, TAB_ROW);

/// An app with its page open in a tab beside the canvas.
fn app_with_a_tab() -> specular_testkit::TestApp {
    let mut app = window_app();
    app.act(Action::OpenTab(EntityId::from("p1")));
    app.act(Action::Show(Showing::Canvas));
    app
}

/// Every control id the tab row's model lists: each tab and the close
/// button of each item's, the add button, the right panel's toggle and the
/// zoom readout when it is there, and the Share slot.
fn listed_ids(strip: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    for tab in strip["tabs"].as_array().into_iter().flatten() {
        let id = tab["id"].as_str().expect("a tab id").to_owned();
        if id != "view.canvas" {
            ids.push(format!("{id}.close"));
        }
        ids.push(id);
    }
    for key in ["add", "chat", "zoom"] {
        ids.extend(strip[key]["id"].as_str().map(str::to_owned));
    }
    ids.push("share".to_owned());
    ids
}

fn shows(tree: &str, id: &str) -> bool {
    match id {
        "zoom" => tree.contains("module_component specular.Zoom \"zoom\""),
        "share" => tree.contains("module_component specular.Share \"share\""),
        _ => tree.contains(&format!("Button \"{id}\"")),
    }
}

#[gpui::test]
fn the_tab_row_shows_every_control_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    let app = app_with_a_tab();
    let listed = listed_ids(&state(app.app())["panels/strip"]);
    assert!(
        listed.contains(&"view.item.p1.close".to_owned())
            && listed.contains(&"chat.toggle".to_owned()),
        "{listed:?}"
    );
    for hidden in [
        &[][..],
        &["view.add", "share"],
        &["chat.toggle", "view.item.p1"],
    ] {
        let tree = Loaded::new(&app, "tabs", json!({ "hidden": hidden }), SIZE, cx).tree();
        for id in &listed {
            let hide = hidden
                .iter()
                .any(|h| id == h || id.starts_with(&format!("{h}.")));
            assert_eq!(
                shows(&tree, id),
                !hide,
                "`{id}` with {hidden:?} hidden:\n{tree}"
            );
        }
    }
}

#[gpui::test]
fn a_click_on_a_tab_presses_it_through_the_host(cx: &mut TestAppContext) {
    let app = app_with_a_tab();
    let mut loaded = Loaded::new(&app, "tabs", json!({}), SIZE, cx);
    let wanted = r#"control.press {"id":"view.item.p1"}"#;
    let runs = loaded.click_until(along(TAB_ROW / 2.0, (90, 1200), 6), wanted);
    assert!(runs.contains(&wanted.to_owned()), "{runs:?}");
    assert_eq!(
        runs.first().map(String::as_str),
        Some(r#"control.press {"id":"view.canvas"}"#),
        "the first tab along the row is the canvas"
    );
}

#[gpui::test]
fn the_tab_row_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app_with_a_tab();
    let mut loaded = timed_load(&app, "tabs", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("tabs"),
        ["agents", "look", "mods/tabs/props", "panels/strip"]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    loaded.assert_camera_moves_nothing(&mut app);
    crate::support::assert_renders_inside_a_frame(&mut loaded, &mut app, "tabs", 300, |app, i| {
        let showing = if i % 2 == 0 {
            Showing::Item(EntityId::from("p1"))
        } else {
            Showing::Canvas
        };
        app.act(Action::Show(showing));
    });
}

/// `app` with `count` agents, each having just renamed the page.
fn with_agents(app: &mut specular_testkit::TestApp, count: usize) {
    for n in 0..count {
        let label = Command::SetLabel {
            id: EntityId::from("p1"),
            label: Some(format!("by {n}")),
        };
        app.send(Event::Api(ApiCall {
            ticket: 0,
            canvas: None,
            thread: Some(ThreadId(format!("agent-{n}"))),
            run: ApiRun::Apply {
                command: label,
                select: None,
            },
        }));
    }
    app.tick(1_000_000).tick(1_000_100);
}

#[gpui::test]
fn agents_on_the_canvas_show_as_badges_unless_hidden(cx: &mut TestAppContext) {
    let mut app = app_with_a_tab();
    let tree = Loaded::new(&app, "tabs", json!({}), SIZE, cx).tree();
    assert!(!shows(&tree, "agents"), "no agents, no badges:\n{tree}");
    with_agents(&mut app, 5);
    let tree = Loaded::new(&app, "tabs", json!({}), SIZE, cx).tree();
    assert!(shows(&tree, "agents"), "{tree}");
    assert!(
        tree.contains("+2"),
        "three badges and a count for the rest:\n{tree}"
    );
    let hidden = Loaded::new(&app, "tabs", json!({ "hidden": ["agents"] }), SIZE, cx).tree();
    assert!(!shows(&hidden, "agents"), "{hidden}");
}
