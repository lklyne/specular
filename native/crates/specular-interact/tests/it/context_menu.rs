//! A canvas's menu in the sidebar: `menu.open` opens it at a point,
//! `panels/menu` holds its items, and pressing one by its name does what
//! the row's own control does.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "a test fails by panicking"
)]

use serde_json::{Value, json};
use specular_core::Camera;
use specular_doc::Document;
use specular_interact::{
    Action, App, CanvasId, Event, OpenedCanvas, OpenedSpace, Project, SidebarAction, run_named,
    state,
};
use specular_testkit::TestApp;

fn canvas(id: &str, name: &str) -> OpenedCanvas {
    OpenedCanvas {
        id: CanvasId::new(id),
        name: name.to_owned(),
        file: format!("{name}.canvas"),
        document: Document::default(),
        camera: Camera::default(),
        tabs: Vec::new(),
    }
}

/// A project with three canvases, the sidebar open.
fn three_canvases() -> TestApp {
    let mut app = TestApp::empty();
    app.send(Event::SpaceOpened(Box::new(OpenedSpace {
        project: Some(Project::folder("/work/site")),
        canvases: vec![
            canvas("c1", "Home"),
            canvas("c2", "Two"),
            canvas("c3", "Ideas"),
        ],
        active: None,
    })));
    app.act(Action::Sidebar(SidebarAction::Toggle));
    app.take_effects();
    app
}

fn run(app: &mut TestApp, name: &str, args: &Value) {
    let run = run_named(app.app(), name, args).unwrap_or_else(|error| panic!("{name}: {error}"));
    app.send(run.into_event().expect("the app's own"));
}

/// The ids of the open menu's items, in order.
fn items(app: &TestApp) -> Vec<String> {
    let open = &state(app.app())["panels/menu"];
    let mut ids = Vec::new();
    for control in open["menu"]["controls"].as_array().into_iter().flatten() {
        for section in control["content"].as_array().into_iter().flatten() {
            for option in section["options"]["options"]
                .as_array()
                .into_iter()
                .flatten()
            {
                ids.push(option["id"].as_str().expect("an id").to_owned());
            }
        }
    }
    ids
}

fn canvas_ids(app: &App) -> Vec<String> {
    (state(app)["projects"].as_array().into_iter().flatten())
        .flat_map(|project| project["canvases"].as_array().cloned().unwrap_or_default())
        .filter_map(|canvas| canvas["id"].as_str().map(str::to_owned))
        .collect()
}

/// What a row checks once its item ran.
type Check = fn(&App);

#[test]
fn a_canvas_menu_opens_at_the_point_and_its_items_do_what_the_row_does() {
    let rows: [(&str, Check); 2] = [
        ("sidebar.canvas.2.menu.delete", |app| {
            assert_eq!(canvas_ids(app), ["c1", "c3"], "Delete takes it away");
        }),
        ("sidebar.canvas.2.menu.rename", |app| {
            let row = &state(app)["panels/sidebar"]["sections"][0]["rows"][1];
            assert_eq!(row["renaming"], true, "Rename types the name in its row");
        }),
    ];
    for (press, check) in rows {
        let mut app = three_canvases();
        let open = json!({ "target": "canvas", "id": "c2", "at": [40, 120] });
        run(&mut app, "menu.open", &open);
        let menu = &state(app.app())["panels/menu"];
        assert_eq!(menu["at"], json!([40.0, 120.0]), "at the point");
        assert_eq!(
            items(&app),
            [
                "sidebar.canvas.2.menu.rename",
                "sidebar.canvas.2.menu.delete"
            ]
        );
        run(&mut app, "control.press", &json!({ "id": press }));
        assert!(
            state(app.app())["panels/menu"].is_null(),
            "{press}: a pick closes it"
        );
        check(app.app());
    }
    // A canvas there is not opens nothing.
    let mut app = three_canvases();
    run(
        &mut app,
        "menu.open",
        &json!({ "target": "canvas", "id": "gone", "at": [0, 0] }),
    );
    assert!(state(app.app())["panels/menu"].is_null());
}
