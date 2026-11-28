//! Mod items, tools and commands (ADR 0057) as the app keeps them: an item
//! placed, changed and drawn, a canvas that needs a mod it lacks, a
//! shipped mod set aside as an edited copy, a tool's
//! stroke in the `tool` key, and a mod's own commands in the palette, on
//! their keys and in the dock. Each ends where undo returns to the start.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "a test fails by panicking"
)]

use std::path::Path;

use serde_json::{Value, json};
use specular_core::Modifiers;
use specular_doc::Kind;
use specular_interact::mods::{InstalledMod, ModCatalog, ModManifest, SetAside};
use specular_interact::{Control, Event, Key, PaletteRun, Tool, dock, palette, run_named, state};
use specular_testkit::TestApp;

const STAMP: &str = r##"{
  "api": "1.6", "title": "Stamp",
  "items": { "size": [100, 80] },
  "tools": [{ "name": "pen", "title": "Stamp pen",
              "stroke": [{ "type": "path", "stroke": "#112233", "line": 4, "taper": true }] }],
  "commands": [
    { "name": "spin", "title": "Spin the stamp", "key": "alt+shift+s", "dock": true },
    { "name": "pin", "title": "Pin the thread", "chat": true }
  ]
}"##;

fn stamp() -> InstalledMod {
    InstalledMod {
        name: "stamp".to_owned(),
        dir: "/mods/stamp".to_owned(),
        manifest: ModManifest::parse(STAMP, Path::new("mod.json")).expect("manifest"),
    }
}

fn with_stamp(mut app: TestApp) -> TestApp {
    app.send(Event::ModCatalog(ModCatalog {
        mods: vec![stamp()],
    }));
    app
}

fn run(app: &mut TestApp, name: &str, args: &Value) {
    let run = run_named(app.app(), name, args).unwrap_or_else(|error| panic!("{name}: {error}"));
    app.send(
        run.into_event()
            .unwrap_or_else(|_| panic!("{name} is the app's")),
    );
}

fn place(app: &mut TestApp) -> String {
    let args = json!({ "kind": "mod", "mod": "stamp", "x": 10, "y": 20, "props": { "n": 1 } });
    run(app, "item.add", &args);
    app.selected_ids()[0].to_owned()
}

#[test]
fn a_mod_item_is_placed_changed_and_drawn_and_undo_takes_it_back() {
    let mut app = with_stamp(TestApp::empty());
    let id = place(&mut app);
    assert_eq!(app.rect(&id).width, 100.0, "the size its mod.json gives");
    let items = &state(app.app())["canvas/mod_items"];
    assert_eq!(items[0]["mod"], "stamp");
    assert_eq!(items[0]["props"], json!({ "n": 1 }));
    assert_eq!(app.app().document().mod_needs()[0].from, "mods/stamp");

    run(
        &mut app,
        "item.set",
        &json!({ "id": id, "props": { "n": 2, "gone": null } }),
    );
    let draws =
        json!([{ "type": "rect", "x": 0, "y": 0, "width": 100, "height": 80, "fill": "#ff0000" }]);
    run(&mut app, "item.draw", &json!({ "id": id, "draws": draws }));
    let entity = app.document().entity(&id.as_str().into()).expect("item");
    let Kind::Mod(item) = &entity.kind else {
        panic!("a mod item");
    };
    assert_eq!(item.props["n"], 2);
    assert_eq!(
        item.snapshot.as_ref().map(Vec::len),
        Some(1),
        "the draw is its snapshot"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_canvas_whose_mod_is_missing_offers_to_install_it() {
    let mut app = with_stamp(TestApp::empty());
    place(&mut app);
    app.send(Event::ModCatalog(ModCatalog::default()));
    let notices = &state(app.app())["notices"];
    assert_eq!(notices[0]["id"], "mods.install", "{notices}");
    assert_eq!(notices[0]["action"]["run"], "mods.install");
    run(&mut app, "notice.dismiss", &json!({ "id": "mods.install" }));
    assert_eq!(
        state(app.app())["notices"],
        json!([]),
        "declined for this canvas"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_shipped_mod_set_aside_at_start_says_where_until_dismissed() {
    let mut app = TestApp::empty();
    app.send(Event::ModsSetAside(vec![SetAside {
        name: "sidebar".to_owned(),
        to: "sidebar-edited".to_owned(),
    }]));
    let notices = &state(app.app())["notices"];
    assert_eq!(notices[0]["id"], "mods.set-aside", "{notices}");
    assert!(
        (notices[0]["text"].as_str()).is_some_and(|text| text.contains("`sidebar-edited`")),
        "{notices}"
    );
    run(
        &mut app,
        "notice.dismiss",
        &json!({ "id": "mods.set-aside" }),
    );
    assert_eq!(state(app.app())["notices"], json!([]));
    app.assert_undo_returns_to_start();
}

/// The colour of the live stroke's first brush pass, when it is drawn.
fn trail_color(app: &TestApp) -> Option<Value> {
    let trail = serde_json::to_value(app.app().mod_trail()).expect("draws");
    trail.get(0).map(|pass| pass["stroke"].clone())
}

#[test]
fn a_mod_tool_publishes_its_stroke_and_the_app_draws_it_with_its_brush() {
    let mut app = with_stamp(TestApp::empty());
    run(&mut app, "tool.set", &json!({ "tool": "stamp/pen" }));
    assert!(matches!(app.session().tool, Tool::Mod(_)));
    app.press((100.0, 100.0)).drag_to((140.0, 120.0));
    let stroke = state(app.app())["tool"].clone();
    assert_eq!(stroke["tool"], "stamp/pen");
    assert_eq!(stroke["phase"], "down");
    let trail = serde_json::to_value(app.app().mod_trail()).expect("draws");
    assert_eq!(
        trail[0]["points"], stroke["points"],
        "drawn along the live stroke"
    );
    assert_eq!(trail[0]["taper"], true);

    let brush = json!({ "draws": [{ "type": "path", "stroke": "#445566", "line": 4 }] });
    run(&mut app, "tool.stroke", &brush);
    assert_eq!(
        trail_color(&app),
        Some(json!("#445566")),
        "tool.stroke sets the brush"
    );
    app.release();
    assert_eq!(state(app.app())["tool"]["phase"], "up");
    assert!(trail_color(&app).is_some(), "kept until the mod commits");
    assert!(
        app.document().entities().next().is_none(),
        "the mod places what it makes"
    );
    place(&mut app);
    assert_eq!(trail_color(&app), None, "placing an item ends the trail");
    app.assert_undo_returns_to_start();
}

#[test]
fn a_mod_command_is_in_the_palette_on_its_key_and_in_the_dock() {
    let mut app = with_stamp(TestApp::empty());
    let listed = (palette(app.app()).into_iter())
        .find(|item| item.label == "Spin the stamp")
        .expect("in the palette");
    assert!(matches!(&listed.run, PaletteRun::Named { name, .. } if name == "stamp/spin"));
    assert_eq!(
        listed.chord.map(specular_interact::Chord::text),
        Some("⌥⇧S".to_owned())
    );

    run(&mut app, "stamp/spin", &json!({ "turns": 2 }));
    let call = state(app.app())["call"].clone();
    assert_eq!(
        call,
        json!({ "command": "stamp/spin", "number": 1, "args": { "turns": 2 } })
    );
    let held = Modifiers {
        alt: true,
        shift: true,
        ..Modifiers::default()
    };
    app.chord(held, Key::Char('s'));
    assert_eq!(state(app.app())["call"]["number"], 2, "its key runs it");

    let id = place(&mut app);
    let button = (dock(app.app()).expect("a dock").controls.into_iter())
        .find_map(|control| match control {
            Control::Button(button) if button.id.as_str() == "mod.stamp/spin" => Some(button),
            _ => None,
        })
        .expect("its dock button");
    app.act(button.action);
    assert_eq!(state(app.app())["call"]["args"], json!({ "ids": [id] }));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_mod_command_for_the_chat_is_a_button_on_the_open_thread() {
    let mut app = with_stamp(TestApp::empty());
    assert_eq!(
        state(app.app())["panels/chat"]["actions"],
        json!([]),
        "no thread open"
    );
    run(&mut app, "chat.new-thread", &json!({}));
    let thread = state(app.app())["chat/threads"]["open"].clone();
    assert_eq!(
        state(app.app())["panels/chat"]["actions"],
        json!([{ "command": "stamp/pin", "title": "Pin the thread", "args": { "thread": thread } }])
    );
    let button = specular_interact::chat(app.app()).actions.remove(0);
    app.act(button.action);
    assert_eq!(
        state(app.app())["call"]["args"],
        json!({ "thread": thread })
    );
}

#[test]
fn a_mod_shape_is_one_of_the_shape_tools_shapes() {
    let star = InstalledMod {
        name: "star".to_owned(),
        dir: "/mods/star".to_owned(),
        manifest: ModManifest::parse(
            r#"{ "api": "1.2", "title": "Star", "items": { "size": [120, 120] },
                 "tools": [{ "name": "star", "title": "Star", "icon": "star", "group": "shape" }] }"#,
            Path::new("mod.json"),
        )
        .expect("manifest"),
    };
    let mut app = TestApp::empty();
    app.send(Event::ModCatalog(ModCatalog { mods: vec![star] }));
    let toolbar = state(app.app())["panels/toolbar"].to_string();
    assert!(!toolbar.contains("tool.star/star"), "no button of its own");

    app.act(specular_interact::Action::SetTool(Tool::AddShape));
    let options = state(app.app())["panels/tool_options"].to_string();
    assert!(options.contains("shape.kind.star/star"), "{options}");
    assert!(
        options.contains(r#""icon":"star""#),
        "its own glyph: {options}"
    );
    run(
        &mut app,
        "control.press",
        &json!({ "id": "shape.kind.star/star" }),
    );
    assert!(
        matches!(app.session().tool, Tool::Mod(_)),
        "the star in hand"
    );
    let options = state(app.app())["panels/tool_options"].clone();
    assert!(
        options.to_string().contains("shape.kind.rectangle"),
        "the shapes stay offered"
    );
    let shape_on = state(app.app())["panels/toolbar"].to_string();
    assert!(
        shape_on.contains(r#""active":true"#),
        "the shape button is on"
    );
    run(
        &mut app,
        "control.press",
        &json!({ "id": "shape.kind.rectangle" }),
    );
    assert_eq!(
        app.session().tool,
        Tool::AddShape,
        "a silhouette puts the star down"
    );
    app.assert_undo_returns_to_start();
}
