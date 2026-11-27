//! Mod overlays (ADR 0059): what `overlay.open` opens is in the `overlays`
//! key with its id in its props, Escape closes the top dialog but not a
//! window, and `overlay.close` closes one by its id.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "a test fails by panicking"
)]

use serde_json::{Value, json};
use specular_interact::{Key, run_named, state};
use specular_testkit::TestApp;

fn run(app: &mut TestApp, name: &str, args: &Value) {
    let run = run_named(app.app(), name, args).unwrap_or_else(|error| panic!("{name}: {error}"));
    app.send(run.into_event().expect("the app's own"));
}

fn open_ids(app: &TestApp) -> Vec<String> {
    let open = state(app.app())["overlays"].clone();
    (open.as_array().expect("a list").iter())
        .map(|overlay| overlay["id"].as_str().expect("an id").to_owned())
        .collect()
}

#[test]
fn escape_closes_the_top_dialog_and_close_names_the_rest() {
    let mut app = TestApp::empty();
    let pinned = json!({ "mod": "notes", "kind": "window", "at": [40, 40] });
    run(&mut app, "overlay.open", &pinned);
    let ask = json!({ "mod": "confirm", "id": "ask", "props": { "title": "Delete?" } });
    run(&mut app, "overlay.open", &ask);
    assert_eq!(open_ids(&app), ["notes", "ask"]);
    let top = &state(app.app())["overlays"][1];
    assert_eq!(top["kind"], "dialog", "a dialog unless it says");
    assert_eq!(top["props"]["title"], "Delete?");

    app.key(Key::Escape);
    assert_eq!(open_ids(&app), ["notes"], "Escape closes the dialog");
    app.key(Key::Escape);
    assert_eq!(open_ids(&app), ["notes"], "but leaves a window");
    run(&mut app, "overlay.close", &json!({ "id": "notes" }));
    assert_eq!(open_ids(&app), Vec::<String>::new());
    assert!(run_named(app.app(), "overlay.open", &json!({ "kind": "popup" })).is_err());
}
