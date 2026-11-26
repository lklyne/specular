//! The Share mod against `panels/share`.

use gpui::{Modifiers, TestAppContext, point, px};
use serde_json::{Value, json};
use specular_interact::{Action, LinkScope, ShareAction, state};

use crate::support::{Loaded, timed_load, window_app};

const SIZE: (f32, f32) = (400.0, 300.0);

/// Every id the Share model lists: the button, the access dropdown and its
/// choices, and Copy link.
fn listed_ids(share: &Value) -> Vec<String> {
    let mut ids = vec![share["id"].as_str().expect("an id").to_owned()];
    ids.extend(share["access"]["id"].as_str().map(str::to_owned));
    for section in share["access"]["content"].as_array().into_iter().flatten() {
        for option in section["options"]["options"]
            .as_array()
            .into_iter()
            .flatten()
        {
            ids.extend(option["id"].as_str().map(str::to_owned));
        }
    }
    ids.extend(share["copy"]["id"].as_str().map(str::to_owned));
    ids
}

fn shows(tree: &str, id: &str) -> bool {
    tree.contains(&format!("Button \"{id}\""))
}

/// Clicks down the popover's middle, opening it again whenever a click
/// closed it, until `done` says so.
fn probe(loaded: &mut Loaded, mut done: impl FnMut(&mut Loaded) -> bool) {
    for y in (26..280).step_by(3) {
        if loaded.tree().contains(":open[Bool(false)]") {
            loaded
                .vcx
                .simulate_click(point(px(10.0), px(12.0)), Modifiers::default());
            loaded.vcx.run_until_parked();
        }
        loaded
            .vcx
            .simulate_click(point(px(150.0), px(y as f32)), Modifiers::default());
        loaded.vcx.run_until_parked();
        if done(loaded) {
            return;
        }
    }
}

#[gpui::test]
fn the_share_button_shows_every_control_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    let mut app = window_app();
    let listed = listed_ids(&state(app.app())["panels/share"]);
    assert_eq!(listed.len(), 7, "{listed:?}");
    let (choices, plain): (Vec<_>, Vec<_>) =
        (listed.iter()).partition(|id| id.starts_with("share.access."));
    for hidden in [&[][..], &["share.copy"], &["share.access"]] {
        let tree = Loaded::new(&app, "share", json!({ "hidden": hidden }), SIZE, cx).tree();
        for id in &plain {
            let hide = hidden.contains(&id.as_str());
            assert_eq!(
                shows(&tree, id),
                !hide,
                "`{id}` with {hidden:?} hidden:\n{tree}"
            );
        }
    }

    // The choices open in place under their row, each one there.
    let mut loaded = Loaded::new(&app, "share", json!({}), SIZE, cx);
    probe(&mut loaded, |loaded| {
        shows(&loaded.tree(), "share.access.view")
    });
    let tree = loaded.tree();
    for id in &choices {
        assert!(shows(&tree, id), "`{id}` among the choices:\n{tree}");
    }
    loaded.assert_camera_moves_nothing(&mut app);
}

#[gpui::test]
fn copy_link_and_a_choice_run_their_commands_through_the_host(cx: &mut TestAppContext) {
    let app = window_app();
    let mut loaded = Loaded::new(&app, "share", json!({}), SIZE, cx);
    probe(&mut loaded, |loaded| {
        loaded
            .runs()
            .iter()
            .any(|run| run.starts_with("share.copy-link"))
    });
    probe(&mut loaded, |loaded| {
        loaded
            .runs()
            .iter()
            .any(|run| run.starts_with("share.access"))
    });
    let runs = loaded.runs();
    assert!(
        runs.contains(&"share.copy-link null".to_owned()),
        "{runs:?}"
    );
    assert!(
        runs.iter()
            .any(|run| run.starts_with(r#"share.access {"scope":""#)),
        "{runs:?}"
    );
}

#[gpui::test]
fn the_share_button_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = window_app();
    let mut loaded = timed_load(&app, "share", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("share"),
        ["mods/share/props", "panels/share", "theme"]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    let scopes = [Some(LinkScope::View), Some(LinkScope::Edit)];
    crate::support::assert_renders_inside_a_frame(&mut loaded, &mut app, "share", 300, |app, i| {
        app.act(Action::Share(ShareAction::SetAccess(scopes[i % 2])));
    });
}
