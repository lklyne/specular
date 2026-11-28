//! The shipped canvas mods (ADR 0057) over a real app: `feed` draws its
//! items from `canvas/mod_items` with `item.draw`, only when an item's props
//! or size move; `watercolor`'s brush is drawn by the app, so its script
//! sends nothing during a drag and places the stroke on release. Each
//! renders once a step and never while idle, fast enough for a frame.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use std::path::Path;

use serde_json::{Value, json};
use specular_doc::{Color, ColorPreset, Kind, ShapeKind};
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
    let mods = ["feed", "watercolor"].map(installed).to_vec();
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
fn the_watercolor_brush_is_drawn_by_the_app_and_placed_on_release(cx: &mut gpui::TestAppContext) {
    let mut app = app_with_mods();
    run(&mut app, "tool.set", &json!({ "tool": "watercolor/brush" }));
    let mut loaded = timed_load(&app, "watercolor", json!({}), NONE, cx);
    app.press((100.0, 100.0));
    loaded.step(&app);
    assert_renders_inside_a_frame(&mut loaded, &mut app, "watercolor", 200, |app, i| {
        let x = 100.0 + i as f32;
        app.drag_to((x, 100.0 + (x / 10.0).sin() * 20.0));
    });
    assert!(
        !app.app().mod_trail().is_empty(),
        "the app draws the live stroke"
    );
    app.release();
    assert!(loaded.step(&app), "the release is a `tool` change");
    let runs = loaded.runs();
    assert_eq!(
        runs_of(&runs, "tool.preview"),
        Vec::<&String>::new(),
        "no preview from the script"
    );
    let placed = runs_of(&runs, "item.add");
    assert_eq!(placed.len(), 1, "{placed:?}");
    assert!(placed[0].contains("\"mod\":\"watercolor\""), "{placed:?}");
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
}

#[gpui::test]
fn a_star_is_a_shape_in_the_tools_style_and_its_mod_reshapes_it(cx: &mut gpui::TestAppContext) {
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
    run(
        &mut app,
        "control.press",
        &json!({ "id": "shape.color.cyan" }),
    );
    let mut loaded = timed_load(&app, "star", json!({}), NONE, cx);
    app.press((100.0, 100.0)).drag_to((220.0, 200.0)).release();
    assert!(!loaded.step(&app), "the app places the star, not the mod");
    let star = app.selected_ids()[0].to_owned();
    let shape = |app: &TestApp| match &app
        .document()
        .entity(&star.as_str().into())
        .expect("star")
        .kind
    {
        Kind::Shape(shape) => shape.clone(),
        other => panic!("a star is a shape, not {other:?}"),
    };
    let placed = shape(&app);
    assert_eq!(placed.shape, ShapeKind::Custom);
    assert_eq!(
        placed.color,
        Some(Color::Preset(ColorPreset::Cyan)),
        "the tool's colour"
    );
    let custom = placed.custom.expect("its outline");
    assert_eq!(
        (custom.name.as_str(), &custom.params["points"]),
        ("star/star", &json!(5))
    );

    run(
        &mut app,
        "control.press",
        &json!({ "id": "mod.star/points" }),
    );
    assert!(loaded.step(&app), "the dock button is a `call`");
    let runs = loaded.runs();
    let asked = runs_of(&runs, "shape.set");
    assert_eq!(asked.len(), 1, "{asked:?}");
    let args: Value = serde_json::from_str(&asked[0]["shape.set ".len()..]).expect("json");
    run(&mut app, "shape.set", &args);
    let reshaped = shape(&app);
    let custom = reshaped.custom.expect("its outline");
    assert_eq!(custom.params["points"], json!(6));
    assert_eq!(
        custom.path().matches('L').count(),
        11,
        "six points, twelve corners"
    );
    assert_eq!(reshaped.color, placed.color, "the style stays");
    app.assert_undo_returns_to_start();
}
