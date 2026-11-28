//! The dock mod against `panels/dock` and the tab's lens in
//! `panels/toolbar`: the controls of each kind of selection, the choices
//! each dropdown opens, and the lens of an item tab.

use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_doc::{EntityId, Rect};
use specular_interact::{Action, Event, Showing, state};
use specular_testkit::{TestApp, drawing, page, plain_text, shape, sticky};

use crate::support::{Fixture, Loaded, along, timed_load};

const SIZE: (f32, f32) = (900.0, 40.0);

/// One of each kind on the canvas, `selected` selected.
fn app_selecting(selected: &str) -> TestApp {
    let at = |x| Rect::new(x, 100.0, 200.0, 150.0);
    let mut app = TestApp::with_entities([
        page("p1", at(0.0)),
        shape("s1", at(300.0)),
        sticky("n1", at(600.0), "hi"),
        plain_text("t1", at(900.0), "hey"),
        drawing("d1", at(1200.0)),
    ]);
    app.send(Event::ShellChrome);
    app.select(&[selected]);
    app
}

/// The page shown in a tab of its own, which puts the lens in the dock.
fn app_showing_a_page() -> TestApp {
    let mut app = app_selecting("p1");
    app.act(Action::OpenTab(EntityId::from("p1")));
    app.act(Action::Show(Showing::Item(EntityId::from("p1"))));
    app
}

/// The ids a control shows in the bar: its own, a swatch's for each
/// choice, a stepper's two buttons.
fn shown_ids(control: &Value) -> Vec<String> {
    let id = control["id"].as_str().unwrap_or_default();
    match control["kind"].as_str() {
        // Choices are a context menu's, never the dock's: listed so a dock
        // that had them would fail here.
        Some("button" | "toggle" | "dropdown" | "field" | "choices") => vec![id.to_owned()],
        Some("swatches") => (control["options"].as_array().into_iter().flatten())
            .filter_map(|option| option["id"].as_str().map(str::to_owned))
            .collect(),
        Some("stepper") => vec![format!("{id}.dec"), format!("{id}.inc")],
        _ => Vec::new(),
    }
}

/// The ids a dropdown's list shows once open: each choice, and each
/// control of a row of controls.
///
/// GAP: a choice's own fields (`page.size.width`, `page.size.height` on
/// the page size's Custom row) are not drawn, and a dropdown inside a
/// dropdown (`page.size.more`) shows its button but never opens its
/// choices (`page.size.13`, `page.size.2`, ... the more sizes), since the
/// mod opens only a dropdown of the bar.
fn choice_ids(dropdown: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    for section in dropdown["content"].as_array().into_iter().flatten() {
        for option in section["options"]["options"]
            .as_array()
            .into_iter()
            .flatten()
        {
            ids.extend(option["id"].as_str().map(str::to_owned));
        }
        for control in section["controls"].as_array().into_iter().flatten() {
            ids.extend(shown_ids(control));
        }
    }
    ids
}

/// The dock's controls and then the lens's, as the model lists them.
fn controls(app: &TestApp) -> Vec<Value> {
    let state = state(app.app());
    let dock = (state["panels/dock"]["controls"].as_array().cloned()).unwrap_or_default();
    let lens = (state["panels/toolbar"]["view"].as_array().cloned()).unwrap_or_default();
    dock.into_iter().chain(lens).collect()
}

fn shows(tree: &str, id: &str) -> bool {
    tree.contains(&format!("Button \"{id}\""))
        || tree.contains(&format!("module_component specular.Field \"{id}\""))
}

/// Clicks along the bar until `dropdown`'s list shows its first choice.
fn open(loaded: &mut Loaded, dropdown: &Value) -> Option<String> {
    let first = choice_ids(dropdown).remove(0);
    for at in along(SIZE.1 / 2.0, (2, SIZE.0 as u32), 4) {
        let tree = loaded.tree();
        if shows(&tree, &first) {
            return Some(tree);
        }
        loaded.vcx.simulate_click(at, gpui::Modifiers::default());
        loaded.vcx.run_until_parked();
    }
    None
}

const SELECTIONS: [&str; 5] = ["p1", "s1", "n1", "t1", "d1"];

#[gpui::test]
fn the_dock_shows_every_control_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    let apps = (SELECTIONS.iter().map(|id| (*id, app_selecting(id))))
        .chain([("a page's tab", app_showing_a_page())]);
    for (name, app) in apps {
        let controls = controls(&app);
        let listed: Vec<String> = controls.iter().flat_map(shown_ids).collect();
        assert!(listed.len() >= 3, "{name}: the model lists {listed:?}");
        // The mod hides a control of the dock by its id: the first and the
        // last here.
        let dock_ids: Vec<String> = (state(app.app())["panels/dock"]["controls"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|control| control["id"].as_str().map(str::to_owned))
            .collect();
        let some: Vec<String> = [dock_ids.first(), dock_ids.last()]
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        for hidden in [Vec::new(), some] {
            let tree = Loaded::new(&app, "dock", json!({ "hidden": hidden }), SIZE, cx).tree();
            for control in &controls {
                let hide = hidden.iter().any(|id| control["id"] == id.as_str());
                for id in shown_ids(control) {
                    assert_eq!(
                        shows(&tree, &id),
                        !hide,
                        "{name}: `{id}` with {hidden:?} hidden:\n{tree}"
                    );
                }
            }
        }
        let dropdowns = controls
            .iter()
            .filter(|control| control["kind"] == "dropdown");
        for dropdown in dropdowns {
            let mut loaded = Loaded::new(&app, "dock", json!({}), SIZE, cx);
            let tree = open(&mut loaded, dropdown).expect("the dropdown opens");
            for id in choice_ids(dropdown) {
                assert!(
                    shows(&tree, &id),
                    "{name}: `{id}` in {}:\n{tree}",
                    dropdown["id"]
                );
            }
        }
    }
}

#[gpui::test]
fn a_click_on_a_control_or_a_choice_presses_it_through_the_host(cx: &mut TestAppContext) {
    // The app, the dropdown to open first, and the control to press.
    let cases: [(Fixture, Option<&str>, &str); 5] = [
        (|| app_selecting("p1"), None, "page.reload"),
        (|| app_selecting("d1"), None, "width.thick"),
        (
            || app_selecting("s1"),
            Some("shape.kind"),
            "shape.kind.ellipse",
        ),
        (
            || app_selecting("n1"),
            Some("text.size"),
            "text.size.custom.inc",
        ),
        (app_showing_a_page, None, "view.lens.fill"),
    ];
    for (app, dropdown, id) in cases {
        let app = app();
        let mut loaded = Loaded::new(&app, "dock", json!({}), SIZE, cx);
        let wanted = format!(r#"control.press {{"id":"{id}"}}"#);
        let runs = match dropdown {
            None => loaded.click_until(along(SIZE.1 / 2.0, (2, SIZE.0 as u32), 4), &wanted),
            Some(dropdown) => {
                let control = (controls(&app).into_iter())
                    .find(|control| control["id"] == dropdown)
                    .expect("the dropdown is listed");
                // A choice closes the list, so it is opened again before
                // each click under the bar, where it opens.
                let under = (44..480)
                    .step_by(6)
                    .flat_map(|y| along(y as f32, (10, 400), 20));
                for at in under {
                    open(&mut loaded, &control).expect("the dropdown opens");
                    if loaded.click_until([at], &wanted).contains(&wanted) {
                        break;
                    }
                }
                loaded.runs()
            }
        };
        assert!(runs.contains(&wanted), "{id}: {runs:?}");
    }
}

#[gpui::test]
fn the_dock_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app_selecting("s1");
    let mut loaded = timed_load(&app, "dock", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("dock"),
        ["look", "mods/dock/props", "panels/dock", "panels/toolbar"]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    loaded.assert_camera_moves_nothing(&mut app);
    crate::support::assert_renders_inside_a_frame(&mut loaded, &mut app, "dock", 300, |app, i| {
        app.select(&[if i % 2 == 0 { "n1" } else { "s1" }]);
    });
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
    let tree = Loaded::new(&app, "dock", json!({}), SIZE, cx).tree();
    assert!(tree.contains("Swatch"), "{tree}");
}
