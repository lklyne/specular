//! Turning an installed mod off in the settings: it stops running, its
//! tools go, a layout that places it draws the app's own panel there, and
//! the choice is saved. Turning it on brings it all back. A mod that starts
//! off runs only once turned on.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use std::path::Path;

use specular_interact::mods::{InstalledMod, ModCatalog, ModManifest, SectionPlace};
use specular_interact::{Action, Content, Effect, Event, Overlay, Region, SettingAction, settings};
use specular_testkit::TestApp;

fn installed(name: &str, manifest: &str) -> InstalledMod {
    InstalledMod {
        name: name.to_owned(),
        dir: format!("/mods/{name}"),
        manifest: ModManifest::parse(manifest, Path::new("mod.json")).expect("manifest"),
    }
}

fn app() -> TestApp {
    let mut app = TestApp::empty();
    let panel = r#"{ "api": "1.2", "title": "Panel" }"#;
    let stamp = r#"{ "api": "1.2", "title": "Stamp",
        "tools": [{ "name": "pen", "title": "Stamp pen" }] }"#;
    app.send(Event::ModCatalog(ModCatalog {
        mods: vec![
            installed("dock", panel),
            installed("stamp", stamp),
            installed("toolbar", panel),
        ],
    }));
    app
}

fn turn(app: &mut TestApp, name: &str, on: bool) -> Vec<Effect> {
    app.take_effects();
    app.act(Action::Setting(SettingAction::Mod(name.to_owned(), on)));
    app.take_effects()
}

fn toolbar(app: &TestApp) -> Option<Content> {
    (app.app().layout().node("toolbar")).and_then(|node| node.content.clone())
}

#[test]
fn a_mod_turned_off_stops_running_and_its_panels_are_the_apps_own() {
    let mut app = app();
    assert_eq!(app.app().layout().fill(Region::Toolbar), Some("toolbar"));
    assert!(app.app().mod_catalog().tool_named("stamp/pen").is_some());

    let saved = turn(&mut app, "toolbar", false);
    assert_eq!(toolbar(&app), Some(Content::Panel(Region::Toolbar)));
    assert!(saved.iter().any(|effect| matches!(
        effect,
        Effect::SaveSettings(settings) if settings.mods_off.contains("toolbar")
    )));
    turn(&mut app, "dock", false);
    assert_eq!(app.app().layout().overlay(Overlay::Dock).fill, None);
    turn(&mut app, "stamp", false);
    assert!(app.app().mod_catalog().tool_named("stamp/pen").is_none());
    assert!(app.app().mod_catalog().get("toolbar").is_none());
    assert_eq!(app.app().installed_mods().mods.len(), 3, "still installed");

    let rows = settings(app.app()).mods;
    let row = rows
        .iter()
        .find(|row| row.name == "toolbar")
        .expect("listed");
    assert!(!row.on && row.detail.contains("in this layout"), "{row:?}");

    for name in ["toolbar", "dock", "stamp"] {
        turn(&mut app, name, true);
    }
    assert_eq!(app.app().layout().fill(Region::Toolbar), Some("toolbar"));
    assert_eq!(
        app.app().layout().overlay(Overlay::Dock).fill.as_deref(),
        Some("dock")
    );
    assert!(app.app().mod_catalog().tool_named("stamp/pen").is_some());
    assert!(app.app().settings().mods_off.is_empty());
}

#[test]
fn a_section_mod_that_starts_off_runs_only_once_turned_on() {
    let mut app = app();
    let notes = r#"{ "api": "1.8", "title": "Notes", "section": "sidebar", "starts_off": true }"#;
    let mut mods = app.app().installed_mods().mods.clone();
    mods.push(installed("notes", notes));
    app.send(Event::ModCatalog(ModCatalog { mods }));
    let sections = |app: &TestApp| -> Vec<String> {
        (app.app().mod_catalog())
            .sections(SectionPlace::Sidebar)
            .map(|installed| installed.name.clone())
            .collect()
    };
    assert_eq!(sections(&app), Vec::<String>::new(), "off until turned on");
    let row = |app: &TestApp| {
        (settings(app.app()).mods.into_iter())
            .find(|row| row.name == "notes")
            .expect("listed")
    };
    assert!(!row(&app).on);

    let saved = turn(&mut app, "notes", true);
    assert_eq!(sections(&app), ["notes"]);
    assert!(row(&app).on);
    assert!(saved.iter().any(|effect| matches!(
        effect,
        Effect::SaveSettings(settings) if settings.mods_on.contains("notes")
    )));

    turn(&mut app, "notes", false);
    assert_eq!(sections(&app), Vec::<String>::new());
    assert!(app.app().settings().mods_on.is_empty());
}
