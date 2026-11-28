//! The first-run mod against `panels/welcome`.

use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_interact::{Event, SpaceAsk, state};
use specular_testkit::TestApp;

use crate::support::{Loaded, grid, timed_load, window_app};

const SIZE: (f32, f32) = (800.0, 600.0);

/// An app asking for a space, with the other app's space on offer, or the
/// space it had gone missing.
fn asking(missing: Option<&str>) -> TestApp {
    let mut app = window_app();
    app.send(Event::SpaceNeeded(SpaceAsk {
        missing: missing.map(str::to_owned),
        electron: Some("/Users/me/Specular".to_owned()),
    }));
    app
}

fn listed_ids(welcome: &Value) -> Vec<String> {
    (welcome["choices"].as_array().into_iter().flatten())
        .filter_map(|choice| choice["id"].as_str().map(str::to_owned))
        .collect()
}

#[gpui::test]
fn the_welcome_shows_every_choice_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    for missing in [None, Some("/Volumes/Gone/Space")] {
        let app = asking(missing);
        let listed = listed_ids(&state(app.app())["panels/welcome"]);
        assert!(listed.len() >= 2, "{listed:?}");
        let first: Vec<&str> = listed.iter().take(1).map(String::as_str).collect();
        for hidden in [&[][..], &first[..]] {
            let tree = Loaded::new(&app, "welcome", json!({ "hidden": hidden }), SIZE, cx).tree();
            assert!(tree.contains("module_component specular.Prism"), "{tree}");
            for id in &listed {
                let hide = hidden.contains(&id.as_str());
                let shown = tree.contains(&format!("Button \"{id}\""));
                assert_eq!(shown, !hide, "`{id}` with {hidden:?} hidden:\n{tree}");
            }
        }
    }
    let mut open = window_app();
    let mut loaded = Loaded::new(&open, "welcome", json!({}), SIZE, cx);
    assert!(
        !loaded.tree().contains("Button"),
        "nothing while a space is open"
    );
    loaded.assert_camera_moves_nothing(&mut open);
}

#[gpui::test]
fn a_click_on_a_choice_presses_it_through_the_host(cx: &mut TestAppContext) {
    let app = asking(None);
    let mut loaded = Loaded::new(&app, "welcome", json!({}), SIZE, cx);
    let wanted = r#"control.press {"id":"onboarding.choose"}"#;
    let runs = loaded.click_until(grid((800, 600), 8), wanted);
    assert!(runs.contains(&wanted.to_owned()), "{runs:?}");
    specular_interact::run_named(
        app.app(),
        "control.press",
        &json!({ "id": "onboarding.choose" }),
    )
    .expect("a choice is a control `control.press` takes");
}

#[gpui::test]
fn the_welcome_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = asking(None);
    let mut loaded = timed_load(&app, "welcome", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("welcome"),
        ["look", "mods/welcome/props", "panels/welcome"]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    crate::support::assert_renders_inside_a_frame(
        &mut loaded,
        &mut app,
        "welcome",
        300,
        |app, i| {
            let missing = (i % 2 == 0).then(|| "/Volumes/Gone/Space".to_owned());
            app.send(Event::SpaceNeeded(SpaceAsk {
                missing,
                electron: Some("/Users/me/Specular".to_owned()),
            }));
        },
    );
}
