//! The agent panel mod against `panels/chat`: the canvas's threads, an
//! open thread at work with a mod's button for it, and the composer.

use std::path::Path;

use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_interact::mods::{InstalledMod, ModCatalog, ModManifest};
use specular_interact::{Action, ChatAction, Event, ThreadId, state};
use specular_testkit::TestApp;

use crate::support::{Fixture, Loaded, grid, timed_load, window_app};

const SIZE: (f32, f32) = (400.0, 600.0);

/// A mod with a command for the agent panel's header.
const PIN: &str = r#"{
  "api": "1.3", "title": "Pin",
  "commands": [{ "name": "pin", "title": "Pin the thread", "chat": true }]
}"#;

/// An app with two threads on its canvas and none open, and the pin mod.
fn app_with_threads() -> TestApp {
    let mut app = window_app();
    app.send(Event::ModCatalog(ModCatalog {
        mods: vec![InstalledMod {
            name: "pin".to_owned(),
            dir: "/mods/pin".to_owned(),
            manifest: ModManifest::parse(PIN, Path::new("mod.json")).expect("manifest"),
        }],
    }));
    for id in ["t1", "t2"] {
        app.act(Action::Chat(ChatAction::NewThread {
            id: Some(ThreadId(id.to_owned())),
            open: false,
        }));
    }
    app.take_effects();
    app
}

/// The first thread open, with a message sent and the agent at work on it.
fn app_with_a_thread_at_work() -> TestApp {
    let mut app = app_with_threads();
    app.act(Action::Chat(ChatAction::SelectThread(ThreadId(
        "t1".to_owned(),
    ))));
    app.act(Action::Chat(ChatAction::Send {
        text: "hello".to_owned(),
        images: Vec::new(),
        thread: None,
    }));
    app.take_effects();
    app
}

/// Every control the panel's model lists, by the id the mod gives it: a
/// row for each thread while none is open, Back while one is, each mod's
/// button, New, Stop while the agent works, and the composer.
///
/// GAP: the native panel's Close thread (`ChatModel::close`, archiving the
/// open thread, and the thread rows' context menu) is in neither the
/// `panels/chat` key nor the mod, so a thread cannot be closed from it.
fn listed_ids(chat: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if chat["transcript"].is_null() {
        for thread in chat["threads"].as_array().into_iter().flatten() {
            ids.extend(thread["id"].as_str().map(|id| format!("chat.thread.{id}")));
        }
    }
    if chat["canGoBack"] == true {
        ids.push("chat.back".to_owned());
    }
    let actions = chat["actions"].as_array().map_or(0, Vec::len);
    ids.extend((0..actions).map(|n| format!("chat.action.{n}")));
    ids.push("chat.new".to_owned());
    if chat["composer"]["running"] == true {
        ids.push("chat.stop".to_owned());
    }
    ids.push("chat.composer".to_owned());
    ids
}

fn shows(tree: &str, id: &str) -> bool {
    match id {
        "chat.composer" => tree.contains("module_component specular.Composer \"chat.composer\""),
        _ => tree.contains(&format!("Button \"{id}\"")),
    }
}

#[gpui::test]
fn the_panel_shows_every_control_the_model_lists(cx: &mut TestAppContext) {
    let cases: [(&str, Fixture, &[&str]); 2] = [
        (
            "threads",
            app_with_threads,
            &["chat.thread.t1", "chat.thread.t2", "chat.new"],
        ),
        (
            "a thread at work",
            app_with_a_thread_at_work,
            &["chat.back", "chat.action.0", "chat.stop"],
        ),
    ];
    for (name, app, expected) in cases {
        let app = app();
        let chat = state(app.app())["panels/chat"].clone();
        let listed = listed_ids(&chat);
        for id in expected {
            assert!(
                listed.contains(&(*id).to_owned()),
                "{name}: `{id}` in {listed:?}"
            );
        }
        let tree = Loaded::new(&app, "chat", json!({}), SIZE, cx).tree();
        for id in &listed {
            assert!(shows(&tree, id), "{name}: `{id}`:\n{tree}");
        }
        for message in chat["transcript"]["messages"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let text = message["text"].as_str().unwrap_or_default();
            assert!(
                tree.contains(&format!("text \"{text}\"")),
                "{name}:\n{tree}"
            );
        }
    }
}

#[gpui::test]
fn a_click_on_a_control_runs_its_command_through_the_host(cx: &mut TestAppContext) {
    let cases: [(Fixture, &str); 5] = [
        (app_with_threads, r#"chat.select {"id":"t2"}"#),
        (app_with_threads, "chat.new-thread"),
        (app_with_a_thread_at_work, "chat.back"),
        (app_with_a_thread_at_work, "chat.stop"),
        (app_with_a_thread_at_work, r#"pin/pin {"thread":"t1"}"#),
    ];
    for (app, wanted) in cases {
        let app = app();
        let mut loaded = Loaded::new(&app, "chat", json!({}), SIZE, cx);
        let runs = loaded.click_until_run(grid((400, 300), 4), |run| run.starts_with(wanted));
        assert!(
            runs.iter().any(|run| run.starts_with(wanted)),
            "{wanted}: {runs:?}"
        );
    }
}

#[gpui::test]
fn the_panel_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app_with_a_thread_at_work();
    let mut loaded = timed_load(&app, "chat", json!({}), SIZE, cx);
    assert_eq!(loaded.reads("chat"), ["look", "panels/chat"]);
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    loaded.assert_camera_moves_nothing(&mut app);
    crate::support::assert_renders_inside_a_frame(&mut loaded, &mut app, "chat", 300, |app, i| {
        app.act(Action::Chat(ChatAction::Resize(380.0 + (i % 2) as f32)));
    });
}
