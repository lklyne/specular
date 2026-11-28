//! The sidebar mod against `panels/sidebar`: the projects, their canvases
//! and their cards, the sidebar's own toggle and its footer.

use gpui::TestAppContext;
use serde_json::{Value, json};
use specular_core::Camera;
use specular_doc::{Document, Rect};
use specular_interact::{
    Action, CanvasId, Event, OpenedCanvas, OpenedSpace, Project, ProjectAction, SidebarAction,
    run_named, state,
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

/// Two projects: a repo with two canvases and a site bound to it, and a
/// plain folder with one. The sidebar is open and holds
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
/// head and its +, and its canvases; the Canvases head and its rows for
/// canvases in no folder; the footer, and the sidebar's own toggle while it
/// holds the top. The Notes and Pages sections are their section mods',
/// and a project's card controls are the `project` mod's ([`card_ids`]).
///
/// GAP: `sidebar.add` (the model's `add`, a new canvas beside the Canvases
/// head) is not drawn by the mod, so canvases in no folder have no new
/// canvas button. A project's `details` id is not drawn either, as the
/// native sidebar does not draw it: pressed by name it opens the project's
/// card, which a right press on the head opens in the mod.
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

/// Every control id the model lists for a project's card: Show in Finder,
/// Close project, the git choice and each bound site's buttons.
fn card_ids(section: &Value) -> Vec<String> {
    let text = |value: &Value| value.as_str().map(str::to_owned);
    let mut ids: Vec<String> = Vec::new();
    ids.extend(text(&section["reveal"]));
    ids.extend(text(&section["close"]));
    ids.extend(text(&section["git"]["commit"]));
    ids.extend(text(&section["git"]["ignore"]));
    for origin in section["origins"].as_array().into_iter().flatten() {
        ids.extend(text(&origin["auto"]));
        ids.extend(text(&origin["remove"]));
    }
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
fn a_right_press_opens_a_canvas_menu_or_a_project_card_through_the_host(cx: &mut TestAppContext) {
    let mut app = app_with_projects();
    let sidebar = state(app.app())["panels/sidebar"].clone();
    let project = sidebar["sections"][0]["project"]
        .as_str()
        .unwrap_or_default();
    let rows: [(&str, Vec<String>); 3] = [
        (
            "canvas c2",
            vec![
                "menu.open {".into(),
                r#""target":"canvas""#.into(),
                r#""id":"c2""#.into(),
            ],
        ),
        (
            "canvas c3",
            vec!["menu.open {".into(), r#""id":"c3""#.into()],
        ),
        (
            "project",
            vec![
                "overlay.open {".into(),
                r#""mod":"project""#.into(),
                r#""kind":"popup""#.into(),
                format!(r#""project":"{project}""#),
            ],
        ),
    ];
    for (name, parts) in rows {
        let mut loaded = Loaded::new(&app, "sidebar", json!({}), SIZE, cx);
        let wanted = |run: &str| {
            parts.iter().all(|part| run.contains(part.as_str())) && run.contains(r#""at":["#)
        };
        let runs = loaded.right_press_until_run(grid((256, 600), 4), wanted);
        assert!(runs.iter().any(|run| wanted(run)), "{name}: {runs:?}");
    }

    // While a canvas's name is typed, its row holds the app's name field.
    app.act(Action::Canvas(
        specular_interact::CanvasAction::BeginRename(Some(CanvasId::new("c2"))),
    ));
    let tree = Loaded::new(&app, "sidebar", json!({}), SIZE, cx).tree();
    assert!(tree.contains("sidebar.canvas.2.name"), "{tree}");
}

/// The project card shows every control the model lists for it, opened
/// by the project's `details` id as by a right press on its head, and a
/// press on one of its buttons reaches the host.
#[gpui::test]
fn the_project_card_shows_every_control_the_model_lists_and_presses_them(cx: &mut TestAppContext) {
    let mut app = app_with_projects();
    let sidebar = state(app.app())["panels/sidebar"].clone();
    let section = &sidebar["sections"][0];
    let project = section["project"].as_str().unwrap_or_default();
    let listed = card_ids(section);
    for id in [
        "sidebar.project.1.reveal",
        "sidebar.project.1.close",
        "sidebar.project.1.git.commit",
        "sidebar.project.1.git.ignore",
        "sidebar.project.1.origin.1.auto",
        "sidebar.project.1.origin.1.remove",
    ] {
        assert!(listed.contains(&id.to_owned()), "`{id}` in {listed:?}");
    }

    let details = json!({ "id": "sidebar.project.1.details" });
    let opened = run_named(app.app(), "control.press", &details).expect("a named control");
    app.send(opened.into_event().expect("the app's"));
    let overlays = state(app.app())["overlays"].clone();
    assert_eq!(overlays[0]["mod"], "project", "{overlays}");
    assert_eq!(overlays[0]["kind"], "popup", "{overlays}");
    assert_eq!(overlays[0]["props"]["project"], project, "{overlays}");

    let props = json!({ "project": project, "overlay": "project" });
    let card = (300, 260);
    let size = (card.0 as f32, card.1 as f32);
    let tree = Loaded::new(&app, "project", props.clone(), size, cx).tree();
    for id in listed
        .iter()
        .map(String::as_str)
        .chain(["sidebar.project.1.bind"])
    {
        assert!(shows(&tree, id), "`{id}`:\n{tree}");
    }
    assert!(tree.contains("site"), "the project's name: {tree}");
    let press = |id: &str| format!(r#"control.press {{"id":"sidebar.project.1.{id}"}}"#);
    for (id, wanted) in [
        ("reveal", press("reveal")),
        ("git.commit", press("git.commit")),
        ("origin.1.remove", press("origin.1.remove")),
        ("bind", r#""command":"project.bind""#.to_owned()),
    ] {
        let mut loaded = Loaded::new(&app, "project", props.clone(), size, cx);
        let runs = loaded.click_until_run(grid(card, 4), |run| run.contains(wanted.as_str()));
        assert!(
            runs.iter().any(|run| run.contains(wanted.as_str())),
            "{id}: {runs:?}"
        );
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
