//! The shipped canvas mods (ADR 0057) over a real app: `feed` draws its
//! items from `canvas/mod_items` with `item.draw`, only when an item's props
//! or size move; `ink` follows its tool's drag in the `tool` key, previews
//! it and places the stroke on release. Each renders once a step and never
//! while idle, fast enough for a frame.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use std::path::Path;

use serde_json::{Value, json};
use specular_interact::mods::{InstalledMod, ModCatalog, ModManifest};
use specular_interact::{Event, run_named};
use specular_mods::shipped;
use specular_testkit::TestApp;

use crate::support::{assert_renders_inside_a_frame, timed_load, window_app};

/// Out of sight, a canvas mod's box is of no size.
const NONE: (f32, f32) = (0.0, 0.0);

/// The shipped mod `name` as the app lists it once installed.
fn installed(name: &str) -> InstalledMod {
    let files = shipped::named(name).expect("shipped").files;
    let source = files
        .iter()
        .find(|file| file.name == "mod.json")
        .expect("mod.json");
    let manifest = ModManifest::parse(source.text, Path::new("mod.json")).expect("manifest");
    InstalledMod {
        name: name.to_owned(),
        dir: format!("/mods/{name}"),
        manifest,
    }
}

/// An app with the shipped canvas mods installed.
fn app_with_mods() -> TestApp {
    let mut app = window_app();
    let mods = ["feed", "ink", "watercolor"].map(installed).to_vec();
    app.send(Event::ModCatalog(ModCatalog { mods }));
    app
}

/// Runs the command `name` as the palette or a mod would.
fn run(app: &mut TestApp, name: &str, args: &Value) {
    let event = (run_named(app.app(), name, args))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .into_event()
        .unwrap_or_else(|_| panic!("{name} is the app's"));
    app.send(event);
}

fn runs_of<'a>(runs: &'a [String], name: &str) -> Vec<&'a String> {
    let prefix = format!("{name} ");
    runs.iter().filter(|run| run.starts_with(&prefix)).collect()
}

#[gpui::test]
fn a_feed_draws_each_item_when_its_props_move(cx: &mut gpui::TestAppContext) {
    let mut app = app_with_mods();
    let props = json!({ "title": "Refs", "urls": ["a.png", "https://example.com/b.png"] });
    run(
        &mut app,
        "item.add",
        &json!({ "kind": "mod", "mod": "feed", "props": props }),
    );
    let id = app.selected_ids()[0].to_owned();
    let mut loaded = timed_load(&app, "feed", json!({}), NONE, cx);
    let draws = loaded.runs();
    assert_eq!(runs_of(&draws, "item.draw").len(), 1, "{draws:?}");
    assert!(
        draws[0].contains("\"type\":\"image\"") && draws[0].contains("Refs"),
        "{draws:?}"
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    loaded.assert_camera_moves_nothing(&mut app);

    // Moving the item reads no key the feed reads.
    app.select(&[&id]);
    app.act(specular_interact::Action::Nudge { dx: 10.0, dy: 0.0 });
    assert!(!loaded.step(&app), "a move wakes no script");

    assert_renders_inside_a_frame(&mut loaded, &mut app, "feed", 200, |app, i| {
        run(
            app,
            "item.set",
            &json!({ "id": id, "props": { "title": format!("Refs {i}") } }),
        );
    });
    assert_eq!(
        runs_of(&loaded.runs(), "item.draw").len(),
        201,
        "one draw a change"
    );
}

#[gpui::test]
fn the_ink_pen_previews_its_stroke_and_places_it_on_release(cx: &mut gpui::TestAppContext) {
    let mut app = app_with_mods();
    run(&mut app, "tool.set", &json!({ "tool": "ink/pen" }));
    let mut loaded = timed_load(&app, "ink", json!({}), NONE, cx);
    app.press((100.0, 100.0));
    assert!(loaded.step(&app), "the press is a `tool` change");
    assert_renders_inside_a_frame(&mut loaded, &mut app, "ink", 200, |app, i| {
        let x = 100.0 + i as f32;
        app.drag_to((x, 100.0 + (x / 10.0).sin() * 20.0));
    });
    app.release();
    assert!(loaded.step(&app), "the release is a `tool` change");
    let runs = loaded.runs();
    assert!(
        runs_of(&runs, "tool.preview").len() >= 200,
        "a preview a move"
    );
    let placed = runs_of(&runs, "item.add");
    assert_eq!(placed.len(), 1, "{placed:?}");
    assert!(placed[0].contains("\"mod\":\"ink\""), "{placed:?}");
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
}

#[gpui::test]
fn a_star_from_the_shape_picker_is_drawn_placed_and_undone(cx: &mut gpui::TestAppContext) {
    let mut app = window_app();
    app.send(Event::ModCatalog(ModCatalog {
        mods: vec![installed("star")],
    }));
    run(&mut app, "tool.set", &json!({ "tool": "shape" }));
    run(
        &mut app,
        "control.press",
        &json!({ "id": "shape.kind.star/star" }),
    );
    let mut loaded = timed_load(&app, "star", json!({}), NONE, cx);
    app.press((100.0, 100.0)).drag_to((220.0, 200.0));
    assert!(loaded.step(&app), "the drag is a `tool` change");
    app.release();
    assert!(loaded.step(&app), "the release is a `tool` change");
    let runs = loaded.runs();
    assert!(
        runs_of(&runs, "tool.preview")[0].contains("\"closed\":true"),
        "{runs:?}"
    );
    let placed = runs_of(&runs, "item.add");
    assert_eq!(placed.len(), 1, "{placed:?}");
    // What the mod asked for, run as the window would run it.
    let args: Value = serde_json::from_str(&placed[0]["item.add ".len()..]).expect("json");
    run(&mut app, "item.add", &args);
    assert!(
        loaded.step(&app),
        "the new item is a `canvas/mod_items` change"
    );
    let draws = runs_of(&loaded.runs(), "item.draw").len();
    assert_eq!(draws, 1, "the star draws its item");
    app.assert_undo_returns_to_start();
}
