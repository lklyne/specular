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

/// The ids a dropdown's list shows once open: each choice and its own
/// fields (a page size's width and height on its Custom row), and each
/// control of a row of controls, a dropdown inside it with its own list's.
fn choice_ids(dropdown: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    for section in dropdown["content"].as_array().into_iter().flatten() {
        for option in section["options"]["options"]
            .as_array()
            .into_iter()
            .flatten()
        {
            ids.extend(option["id"].as_str().map(str::to_owned));
            for field in option["fields"].as_array().into_iter().flatten() {
                ids.extend(field["id"].as_str().map(str::to_owned));
            }
        }
        for control in section["controls"].as_array().into_iter().flatten() {
            ids.extend(shown_ids(control));
            if control["kind"] == "dropdown" {
                ids.extend(choice_ids(control));
            }
        }
    }
    ids
}

/// Every choice of `dropdown`'s list and of the lists inside it.
fn options(dropdown: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for section in dropdown["content"].as_array().into_iter().flatten() {
        out.extend(
            section["options"]["options"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
        );
        for control in section["controls"].as_array().into_iter().flatten() {
            if control["kind"] == "dropdown" {
                out.extend(options(control));
            }
        }
    }
    out
}

/// The words drawn inside the element named `id`: the `text` lines of its
/// subtree.
fn words_in(tree: &str, id: &str) -> Vec<String> {
    let head = format!("Button \"{id}\"");
    let mut lines = tree
        .lines()
        .skip_while(|line| !line.trim_start().starts_with(&head));
    let Some(first) = lines.next() else {
        return Vec::new();
    };
    let indent = |line: &str| line.len() - line.trim_start().len();
    let depth = indent(first);
    lines
        .take_while(|line| indent(line) > depth)
        .filter_map(|line| line.trim_start().strip_prefix("text \""))
        .map(|text| text.trim_end_matches('"').to_owned())
        .collect()
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

/// Whether the list of the dropdown `id` is open.
fn is_open(tree: &str, id: &str) -> bool {
    let head = format!("Popover \"{id}.popover\"");
    (tree.lines())
        .any(|line| line.trim_start().starts_with(&head) && line.contains(":open[Bool(true)]"))
}

/// Clicks along the bar until `dropdown`'s list is open. Returns the tree
/// and how far along the bar its button is, where its list opens.
fn open(loaded: &mut Loaded, dropdown: &Value) -> Option<(String, f32)> {
    let id = dropdown["id"].as_str().unwrap_or_default();
    let mut last = 0.0;
    for at in along(SIZE.1 / 2.0, (2, SIZE.0 as u32), 4) {
        let tree = loaded.tree();
        if is_open(&tree, id) {
            return Some((tree, last));
        }
        loaded.vcx.simulate_click(at, gpui::Modifiers::default());
        loaded.vcx.run_until_parked();
        last = f32::from(at.x);
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
            let (tree, _) = open(&mut loaded, dropdown).expect("the dropdown opens");
            for id in choice_ids(dropdown) {
                assert!(
                    shows(&tree, &id),
                    "{name}: `{id}` in {}:\n{tree}",
                    dropdown["id"]
                );
            }
            // A choice shows its face's word once, and not its longer name
            // ("None", not "None Frame: None").
            for option in options(dropdown) {
                let id = option["id"].as_str().unwrap_or_default();
                let words = words_in(&tree, id);
                let label = option["label"].as_str().unwrap_or_default();
                let word = option["face"]["text"].as_str();
                if let Some(word) = word {
                    let times = words.iter().filter(|each| *each == word).count();
                    assert_eq!(times, 1, "{name}: `{id}` shows {words:?}");
                }
                if word != Some(label) {
                    assert!(
                        !words.iter().any(|each| each == label),
                        "{name}: `{id}` shows its name too: {words:?}"
                    );
                }
            }
        }
    }
}

/// Points under the bar from `(x, y)` on, where a list opens.
fn under(x: f32, y: f32) -> impl Iterator<Item = gpui::Point<gpui::Pixels>> {
    let x = x.max(10.0) as u32;
    (y.max(44.0) as u32..790)
        .step_by(6)
        .flat_map(move |y| along(y as f32, (x - 8, x + 240), 20))
}

/// Where the lists of a path of dropdowns open: how far along the bar the
/// first one's button is, and how far down the row that opens the last.
#[derive(Clone, Copy, Default)]
struct Place {
    x: f32,
    y: f32,
}

/// Opens the dropdowns of `path` in turn: the first from the bar, each
/// next from a row of the list before it. `known` is where they opened
/// before, so a list already open need not be found again.
fn open_path(loaded: &mut Loaded, app: &TestApp, path: &[&str], known: Option<Place>) -> Place {
    let first = (controls(app).into_iter())
        .find(|control| control["id"] == path[0])
        .expect("the dropdown is listed");
    let (mut tree, x) = open(loaded, &first).expect("the dropdown opens");
    let mut place = known.unwrap_or(Place { x, y: 44.0 });
    for (depth, inner) in path.iter().enumerate().skip(1) {
        // A row of a list spans it, so one column down it finds the row.
        let column = gpui::px(place.x + 30.0);
        let mut ys = (place.y as u32..790).step_by(4).map(|y| y as f32);
        while !is_open(&tree, inner) {
            let y = ys.next();
            assert!(y.is_some(), "`{inner}` opens:\n{tree}");
            let y = y.unwrap_or_default();
            // A choice pressed on the way closes the lists, so the ones
            // over this one are opened again first.
            if !is_open(&tree, path[depth - 1]) {
                open_path(loaded, app, &path[..depth], Some(place));
            }
            loaded
                .vcx
                .simulate_click(gpui::point(column, gpui::px(y)), gpui::Modifiers::default());
            loaded.vcx.run_until_parked();
            tree = loaded.tree();
            place.y = y;
        }
    }
    place
}

#[gpui::test]
fn a_click_on_a_control_or_a_choice_presses_it_through_the_host(cx: &mut TestAppContext) {
    // The app, the dropdowns to open first, and the control to press.
    let cases: [(Fixture, &[&str], &str); 6] = [
        (|| app_selecting("p1"), &[], "page.reload"),
        (|| app_selecting("d1"), &[], "width.thick"),
        (
            || app_selecting("s1"),
            &["shape.kind"],
            "shape.kind.ellipse",
        ),
        (
            || app_selecting("n1"),
            &["text.size"],
            "text.size.custom.inc",
        ),
        // The more sizes open from a row of the page size's list.
        (
            || app_selecting("p1"),
            &["page.size", "page.size.more"],
            "page.size.13",
        ),
        (app_showing_a_page, &[], "view.lens.fill"),
    ];
    for (app, path, id) in cases {
        let app = app();
        let mut loaded = Loaded::new(&app, "dock", json!({}), SIZE, cx);
        let wanted = format!(r#"control.press {{"id":"{id}"}}"#);
        let runs = if path.is_empty() {
            loaded.click_until(along(SIZE.1 / 2.0, (2, SIZE.0 as u32), 4), &wanted)
        } else {
            // A choice closes the list, so it is opened again before each
            // click under the bar.
            let place = open_path(&mut loaded, &app, path, None);
            for at in under(place.x, place.y) {
                open_path(&mut loaded, &app, path, Some(place));
                if loaded.click_until([at], &wanted).contains(&wanted) {
                    break;
                }
            }
            loaded.runs()
        };
        assert!(runs.contains(&wanted), "{id}: {runs:?}");
    }
}

/// What closes an open list in a case of the table below.
type Close = fn(&mut Loaded, &mut TestApp);

#[gpui::test]
fn a_press_outside_escape_or_a_new_selection_closes_an_open_list(cx: &mut TestAppContext) {
    let at = |x| Rect::new(x, 100.0, 200.0, 150.0);
    let close: [(&str, Close); 4] = [
        ("a press outside", |loaded, _| {
            let far = gpui::point(gpui::px(1000.0), gpui::px(700.0));
            loaded.vcx.simulate_click(far, gpui::Modifiers::default());
            loaded.vcx.run_until_parked();
        }),
        ("Escape", |loaded, _| {
            loaded.vcx.simulate_keystrokes("escape");
            loaded.vcx.run_until_parked();
        }),
        ("another shape selected", |loaded, app| {
            app.select(&["s2"]);
            loaded.step(app);
        }),
        ("nothing selected", |loaded, app| {
            app.select(&[]);
            loaded.step(app);
        }),
    ];
    for (name, close) in close {
        let mut app = TestApp::with_entities([shape("s1", at(0.0)), shape("s2", at(300.0))]);
        app.send(Event::ShellChrome);
        app.select(&["s1"]);
        let mut loaded = Loaded::new(&app, "dock", json!({}), SIZE, cx);
        open_path(&mut loaded, &app, &["shape.color"], None);
        close(&mut loaded, &mut app);
        let tree = loaded.tree();
        assert!(!is_open(&tree, "shape.color"), "{name}:\n{tree}");
    }
}

#[gpui::test]
fn the_dock_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app_selecting("s1");
    let mut loaded = timed_load(&app, "dock", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("dock"),
        [
            "look",
            "mods/dock/props",
            "panels/dock",
            "panels/toolbar",
            "selection"
        ]
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
