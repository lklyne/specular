//! The window's own panels as mods (ADR 0059): the sidebar, the dock, the
//! command palette, the agent panel and the status line against the keys
//! they read. Each shows what its model lists but what its props hide,
//! presses through the host, and renders only when a key it read moves,
//! fast enough for a frame.

use gpui::TestAppContext;
use serde_json::json;
use specular_interact::{Action, ChatAction, Event, SidebarAction, Tool, state};
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

fn sidebar_ids(app: &TestApp) -> Vec<String> {
    let model = &state(app.app())["panels/sidebar"];
    let mut ids = Vec::new();
    for section in model["sections"].as_array().into_iter().flatten() {
        ids.extend(section["id"].as_str().map(str::to_owned));
        for row in section["rows"].as_array().into_iter().flatten() {
            ids.extend(row["id"].as_str().map(str::to_owned));
        }
    }
    ids
}

fn dock_ids(app: &TestApp) -> Vec<String> {
    let model = &state(app.app())["panels/dock"];
    (model["controls"].as_array().into_iter().flatten())
        .filter(|control| {
            matches!(
                control["kind"].as_str(),
                Some("button" | "toggle" | "dropdown")
            )
        })
        .filter_map(|control| control["id"].as_str().map(str::to_owned))
        .collect()
}

/// A panel's row of the table: its mod, the box it is drawn in, the ids
/// it must show, and a press it must send.
struct Panel {
    name: &'static str,
    size: (f32, f32),
    ids: fn(&TestApp) -> Vec<String>,
    press: &'static str,
}

const PANELS: [Panel; 2] = [
    Panel {
        name: "sidebar",
        size: (256.0, 600.0),
        ids: sidebar_ids,
        press: r#"control.press {"id":"sidebar.pages.p2"}"#,
    },
    Panel {
        name: "dock",
        size: (900.0, 40.0),
        ids: dock_ids,
        press: "control.press",
    },
];

#[gpui::test]
fn each_panel_shows_what_its_model_lists_but_those_hidden_and_presses_through_the_host(
    cx: &mut TestAppContext,
) {
    let app = app();
    for panel in &PANELS {
        let ids = (panel.ids)(&app);
        assert!(ids.len() >= 3, "{}: the model lists {ids:?}", panel.name);
        let last: Vec<&str> = ids.iter().last().map(String::as_str).into_iter().collect();
        for hidden in [&[][..], &last[..]] {
            let props = json!({ "hidden": hidden });
            let tree = Loaded::new(&app, panel.name, props, panel.size, cx).tree();
            for id in &ids {
                let shown = tree.contains(&format!("Button \"{id}\""));
                assert_eq!(
                    shown,
                    !hidden.contains(&id.as_str()),
                    "{}: `{id}`:\n{tree}",
                    panel.name
                );
            }
        }
        let mut loaded = Loaded::new(&app, panel.name, json!({}), panel.size, cx);
        let size = (panel.size.0 as u32, panel.size.1 as u32);
        let runs = loaded.click_until(grid(size, 6), panel.press);
        assert!(
            runs.iter().any(|run| run.starts_with(panel.press)),
            "{}: {runs:?}",
            panel.name
        );
    }
}

#[gpui::test]
fn each_window_mod_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let reads: [(&str, (f32, f32), &[&str]); 5] = [
        (
            "sidebar",
            (256.0, 600.0),
            &["look", "mods/sidebar/props", "panels/sidebar"],
        ),
        (
            "dock",
            (900.0, 40.0),
            &["look", "mods/dock/props", "panels/dock", "panels/toolbar"],
        ),
        (
            "palette",
            (560.0, 480.0),
            &["look", "mods/palette/props", "panels/palette"],
        ),
        ("chat", (400.0, 600.0), &["look", "panels/chat"]),
        (
            "status",
            (900.0, 24.0),
            &["canvas/info", "layouts", "look", "selection"],
        ),
    ];
    for (name, size, keys) in reads {
        let mut app = app();
        let mut loaded = timed_load(&app, name, json!({}), size, cx);
        let read = loaded.reads(name);
        for key in keys {
            assert!(
                read.iter().any(|each| each == key),
                "{name} reads {key}: {read:?}"
            );
        }
        assert_eq!(
            loaded.idle_renders(),
            0,
            "{name}: no script render while idle"
        );
        loaded.assert_camera_moves_nothing(&mut app);
        let change: fn(&mut TestApp, usize) = match name {
            "palette" => |app, i| {
                let tool = if i % 2 == 0 {
                    Tool::AddText
                } else {
                    Tool::Select
                };
                app.act(Action::SetTool(tool));
            },
            "chat" => |app, i| {
                app.act(Action::Chat(ChatAction::Resize(380.0 + i as f32)));
            },
            _ => |app, i| {
                app.select(&[if i % 2 == 0 { "p2" } else { "p1" }]);
            },
        };
        assert_renders_inside_a_frame(&mut loaded, &mut app, name, 60, change);
    }
}

#[gpui::test]
fn the_palette_runs_the_command_typed_on_return(cx: &mut TestAppContext) {
    let app = app();
    let mut loaded = Loaded::new(
        &app,
        "palette",
        json!({ "overlay": "palette" }),
        (560.0, 480.0),
        cx,
    );
    let field = (loaded.reads("palette").into_iter())
        .find_map(|key| {
            key.strip_prefix("shared/field/")
                .and_then(|rest| rest.strip_suffix("/submit"))
                .map(str::to_owned)
        })
        .expect("the palette reads its field's submit");
    let publish = |loaded: &Loaded, key: String, value: serde_json::Value| {
        loaded.hosting.revisions.borrow_mut().publish(&key, value);
    };
    publish(&loaded, format!("shared/field/{field}"), json!("settings"));
    loaded.step(&app);
    publish(&loaded, format!("shared/field/{field}/submit"), json!(1));
    loaded.step(&app);
    let runs = loaded.runs();
    assert!(
        runs.iter().any(|run| run.starts_with("app.settings")),
        "{runs:?}"
    );
}

#[gpui::test]
fn the_dock_draws_a_drawings_colours_by_their_stored_names(cx: &mut TestAppContext) {
    use specular_testkit::drawing;
    let mut app = TestApp::with_entities([drawing(
        "d1",
        specular_doc::Rect::new(100.0, 100.0, 200.0, 200.0),
    )]);
    app.send(Event::ShellChrome);
    app.select(&["d1"]);
    let colours = &state(app.app())["panels/dock"];
    assert!(
        colours.to_string().contains("\"neutral\""),
        "the drawing's swatches name neutral: {colours}"
    );
    let tree = Loaded::new(&app, "dock", json!({}), (900.0, 40.0), cx).tree();
    assert!(tree.contains("Swatch"), "{tree}");
}

#[gpui::test]
fn a_chat_pane_starts_a_thread_of_its_own_and_sends_to_it(cx: &mut TestAppContext) {
    let app = app();
    let mut loaded = Loaded::new(&app, "chat", json!({ "pane": "a" }), (400.0, 600.0), cx);
    let read = loaded.reads("chat");
    assert!(read.iter().any(|key| key == "chat/threads"), "{read:?}");
    assert!(
        !read.iter().any(|key| key == "panels/chat"),
        "a pane does not follow the panel's thread: {read:?}"
    );
    let field = (read.into_iter())
        .find_map(|key| {
            key.strip_prefix("shared/field/")
                .and_then(|rest| rest.strip_suffix("/submit"))
                .map(str::to_owned)
        })
        .expect("the pane reads its field's submit");
    let publish = |loaded: &Loaded, key: String, value: serde_json::Value| {
        loaded.hosting.revisions.borrow_mut().publish(&key, value);
    };
    publish(&loaded, format!("shared/field/{field}"), json!("hello"));
    loaded.step(&app);
    publish(&loaded, format!("shared/field/{field}/submit"), json!(1));
    loaded.step(&app);
    let runs = loaded.runs();
    let made = (runs.iter())
        .find(|run| run.starts_with("chat.new-thread") && run.contains(r#""open":false"#))
        .expect("the pane starts a thread beside the panel's");
    let sent = (runs.iter())
        .find(|run| run.starts_with("chat.send"))
        .expect("the pane sends what was typed");
    let id = made
        .split(r#""id":""#)
        .nth(1)
        .and_then(|rest| rest.split('"').next());
    let id = id.expect("the new thread's id");
    assert!(sent.contains(&format!(r#""thread":"{id}""#)), "{sent}");
    assert!(sent.contains("hello"), "{sent}");
}
