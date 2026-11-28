//! The sidebar mod against `panels/sidebar`: the projects, their canvases
//! and their details, the sidebar's own toggle and its footer.

use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_core::Camera;
use specular_doc::{Document, Rect};
use specular_interact::{
    Action, CanvasId, Event, OpenedCanvas, OpenedSpace, Project, ProjectAction, SidebarAction,
    state,
};
use specular_testkit::{TestApp, document, page, sticky};

use crate::support::{Fixture, Loaded, grid, timed_load};

const SIZE: (f32, f32) = (256.0, 600.0);

fn canvas(id: &str, name: &str, document: Document) -> OpenedCanvas {
    OpenedCanvas {
        id: CanvasId::new(id),
        name: name.to_owned(),
        file: format!("{name}.canvas"),
        document,
        camera: Camera::default(),
        tabs: Vec::new(),
    }
}

/// Two projects: a repo with two canvases, its details shown and a site
/// bound to it, and a plain folder with one. The sidebar is open and holds
/// the window's top, so its own toggle is on it.
fn app_with_projects() -> TestApp {
    let repo = Project {
        repo: true,
        branch: Some("main".to_owned()),
        canvases: "/work/site/.specular".to_owned(),
        ..Project::folder("/work/site")
    };
    let id = repo.id.clone();
    let home = document([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        sticky("n1", Rect::new(600.0, 100.0, 200.0, 200.0), "hi"),
    ]);
    let mut app = TestApp::empty();
    app.send(Event::SpaceOpened(Box::new(OpenedSpace {
        project: Some(repo),
        canvases: vec![
            canvas("c1", "Home", home),
            canvas("c2", "Two", Document::default()),
        ],
        active: None,
    })));
    app.send(Event::ProjectOpened(Box::new(OpenedSpace {
        project: Some(Project::folder("/work/notes")),
        canvases: vec![canvas("c3", "Ideas", Document::default())],
        active: None,
    })));
    app.send(Event::ShellChrome);
    app.send(Event::ChatPanel(true));
    app.act(Action::Sidebar(SidebarAction::Toggle));
    app.act(Action::Sidebar(SidebarAction::Details(id.clone())));
    app.act(Action::Project(ProjectAction::Bind {
        project: id,
        origin: "http://localhost:3000".to_owned(),
    }));
    app.take_effects();
    app
}

/// Canvases in no folder: one Canvases section.
fn app_without_a_project() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.send(Event::ShellChrome);
    app.act(Action::Sidebar(SidebarAction::Toggle));
    app
}

/// Every control id the sidebar's model lists for the mod: each project's
/// head and its +, its canvases and, while its details
/// show, their controls; the Canvases head and its rows for canvases in no
/// folder; the footer, and the sidebar's own toggle while it holds the top.
/// The Notes and Pages sections are their section mods'.
///
/// GAP: `sidebar.add` (the model's `add`, a new canvas beside the Canvases
/// head) is not drawn by the mod, so canvases in no folder have no new
/// canvas button. A canvas row's context menu (`sidebar.canvas.<n>.menu`,
/// Rename and Delete) is not there either: a double click renames through
/// the prompt, and nothing deletes. A project's details button
/// (`sidebar.project.<n>.details`, in the key and among the named
/// controls) is not drawn, as the native sidebar does not draw it either:
/// the details show only once something else toggles them.
fn listed_ids(sidebar: &Value) -> Vec<String> {
    let text = |value: &Value| value.as_str().map(str::to_owned);
    let mut ids: Vec<String> = Vec::new();
    for section in sidebar["sections"].as_array().into_iter().flatten() {
        match section["kind"].as_str() {
            Some("notes" | "pages") => continue,
            Some("project") => {
                ids.extend(
                    ["id", "add"]
                        .map(|key| text(&section[key]))
                        .into_iter()
                        .flatten(),
                );
                if section["detailsShown"] == true {
                    ids.extend(text(&section["reveal"]));
                    ids.extend(text(&section["close"]));
                    ids.extend(text(&section["git"]["commit"]));
                    ids.extend(text(&section["git"]["ignore"]));
                    for origin in section["origins"].as_array().into_iter().flatten() {
                        ids.extend(text(&origin["auto"]));
                        ids.extend(text(&origin["remove"]));
                    }
                }
            }
            _ => ids.extend(text(&section["id"])),
        }
        for row in section["rows"].as_array().into_iter().flatten() {
            ids.extend(text(&row["id"]));
        }
    }
    ids.extend(text(&sidebar["open"]));
    ids.extend(text(&sidebar["title"]["id"]));
    ids
}

fn shows(tree: &str, id: &str) -> bool {
    tree.contains(&format!("Button \"{id}\""))
}

#[gpui::test]
fn the_sidebar_shows_every_control_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    let fixtures: [(&str, Fixture, &[&str]); 2] = [
        (
            "two projects",
            app_with_projects,
            &[
                "sidebar.project.1",
                "sidebar.project.1.add",
                "sidebar.project.1.reveal",
                "sidebar.project.1.close",
                "sidebar.project.1.git.commit",
                "sidebar.project.1.git.ignore",
                "sidebar.project.1.origin.1.auto",
                "sidebar.project.1.origin.1.remove",
                "sidebar.canvas.1",
                "sidebar.canvas.2",
                "sidebar.project.2",
                "sidebar.canvas.3",
                "sidebar.open",
                "sidebar.toggle",
            ],
        ),
        (
            "no project",
            app_without_a_project,
            &["sidebar.head.canvases", "sidebar.canvas.1", "sidebar.open"],
        ),
    ];
    for (name, app, expected) in fixtures {
        let app = app();
        let listed = listed_ids(&state(app.app())["panels/sidebar"]);
        for id in expected {
            assert!(
                listed.contains(&(*id).to_owned()),
                "{name}: `{id}` in {listed:?}"
            );
        }
        // The mod hides a row, a whole section by its head, or the footer.
        for hidden in [
            &[][..],
            &["sidebar.canvas.1", "sidebar.open"],
            &["sidebar.project.2", "sidebar.head.canvases"],
        ] {
            let tree = Loaded::new(&app, "sidebar", json!({ "hidden": hidden }), SIZE, cx).tree();
            for id in &listed {
                let hide = hidden.contains(&id.as_str())
                    || section_of(&app, id).is_some_and(|head| hidden.contains(&head.as_str()));
                assert_eq!(
                    shows(&tree, id),
                    !hide,
                    "{name}: `{id}` with {hidden:?} hidden:\n{tree}"
                );
            }
        }
    }
}

/// The id of the section `id` is a row or a control of.
fn section_of(app: &TestApp, id: &str) -> Option<String> {
    let sidebar = &state(app.app())["panels/sidebar"];
    (sidebar["sections"].as_array().into_iter().flatten())
        .find(|section| {
            (section["rows"].as_array().into_iter().flatten()).any(|row| row["id"] == id)
                || section["id"]
                    .as_str()
                    .is_some_and(|head| id.starts_with(&format!("{head}.")))
        })
        .and_then(|section| section["id"].as_str().map(str::to_owned))
}

#[gpui::test]
fn a_click_on_a_row_or_a_button_presses_it_through_the_host(cx: &mut TestAppContext) {
    let app = app_with_projects();
    for id in [
        "sidebar.canvas.2",
        "sidebar.canvas.3",
        "sidebar.project.2",
        "sidebar.project.1.reveal",
        "sidebar.project.1.git.commit",
        "sidebar.project.1.origin.1.remove",
        "sidebar.open",
        "sidebar.toggle",
    ] {
        let mut loaded = Loaded::new(&app, "sidebar", json!({}), SIZE, cx);
        let wanted = format!(r#"control.press {{"id":"{id}"}}"#);
        let runs = loaded.click_until(grid((256, 600), 4), &wanted);
        assert!(runs.contains(&wanted), "{id}: {runs:?}");
    }
}

#[gpui::test]
fn the_sidebar_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = app_with_projects();
    let mut loaded = timed_load(&app, "sidebar", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("sidebar"),
        ["look", "mods/sidebar/props", "panels/sidebar"]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    loaded.assert_camera_moves_nothing(&mut app);
    crate::support::assert_renders_inside_a_frame(
        &mut loaded,
        &mut app,
        "sidebar",
        300,
        |app, i| {
            // A selection marks a row of the Notes or Pages section, which is
            // in the key the mod reads.
            app.select(&[if i % 2 == 0 { "n1" } else { "p1" }]);
        },
    );
}
