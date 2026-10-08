//! The space: several canvases in one app, each with its own document,
//! undo history, camera and selection.

use glam::Vec2;
use specular_core::Camera;
use specular_doc::{Document, EntityId, ItemId, Rect};
use specular_interact::{Action, CanvasAction, CanvasId, Effect, Event, TabRefError};
use specular_testkit::{TestApp, document, pages, space, sticky};

fn note(id: &str, text: &str) -> Document {
    document([sticky(id, Rect::new(100.0, 100.0, 200.0, 200.0), text)])
}

fn two() -> TestApp {
    TestApp::with_space([("Home", document(pages(2))), ("Notes", note("n1", "hello"))])
}

fn entity_ids(document: &Document) -> Vec<&str> {
    document
        .entities()
        .map(|entity| entity.id.as_str())
        .collect()
}

fn select(id: &str) -> Action {
    Action::Select(vec![ItemId::Entity(EntityId::from(id))])
}

#[test]
fn opening_a_space_shows_its_active_canvas_and_hosts_only_its_pages() {
    let mut app = TestApp::empty();
    let mut opened = space([("Home", document(pages(2))), ("Notes", note("n1", "hello"))]);
    opened.active = Some(CanvasId::new("tab_2"));
    app.send(Event::SpaceOpened(Box::new(opened)));
    assert_eq!(app.canvas_names(), ["Home", "Notes"]);
    assert_eq!(app.active_canvas(), "Notes");
    assert_eq!(entity_ids(app.document()), ["n1"]);
    let hosted = (app.effects().iter()).any(|effect| matches!(effect, Effect::CreatePage { .. }));
    assert!(!hosted, "a background canvas's pages are not hosted");
    assert_eq!(app.app().space().folder(), Some("/space"));
}

#[test]
fn a_space_with_no_canvas_gets_an_empty_first_one_which_is_written() {
    let mut app = TestApp::empty();
    app.send(Event::SpaceOpened(Box::new(space([]))));
    assert_eq!(app.canvas_names(), ["Canvas 1"]);
    let id = app.canvas_id("Canvas 1");
    assert_eq!(
        app.take_effects(),
        [
            Effect::WriteCanvas(id),
            Effect::SaveSpaceMeta,
            Effect::LoadThreads
        ]
    );
}

#[test]
fn switching_closes_the_pages_left_and_hosts_the_pages_entered() {
    let mut app = two();
    app.switch_to("Notes");
    assert_eq!(app.active_canvas(), "Notes");
    assert_eq!(entity_ids(app.document()), ["n1"]);
    let effects = app.take_effects();
    let closed: Vec<&str> = (effects.iter())
        .filter_map(|effect| match effect {
            Effect::ClosePage(page) => Some(page.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(closed, ["p1", "p2"]);
    assert!(effects.contains(&Effect::SaveSpaceMeta));
    assert!(
        !effects.contains(&Effect::Save),
        "showing a canvas does not change it"
    );

    app.switch_to("Home");
    let created: Vec<&str> = (app.effects().iter())
        .filter_map(|effect| match effect {
            Effect::CreatePage { page, .. } => Some(page.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(created, ["p1", "p2"]);
}

#[test]
fn each_canvas_keeps_its_own_undo_history_camera_and_selection() {
    let mut app = two();
    app.act(select("p1"))
        .act(Action::Nudge { dx: 40.0, dy: 0.0 });
    let moved = app.rect("p1");
    let home_camera = Camera::new(Vec2::new(-50.0, 20.0), 0.5);
    app.act(Action::SetCamera(home_camera));

    app.switch_to("Notes");
    assert!(
        !app.app().can_undo(),
        "the other canvas's steps are not here"
    );
    assert!(app.selection().is_empty());
    assert_eq!(app.session().camera, Camera::default());
    app.act(select("n1")).act(Action::Delete);
    assert_eq!(entity_ids(app.document()), [] as [&str; 0]);

    app.switch_to("Home");
    assert_eq!(app.session().camera, home_camera);
    assert_eq!(app.selected(), Some("p1"));
    assert_eq!(app.rect("p1"), moved);
    app.undo();
    assert_eq!(app.rect("p1"), Rect::new(100.0, 100.0, 400.0, 300.0));

    app.switch_to("Notes").undo();
    assert_eq!(entity_ids(app.document()), ["n1"]);
}

#[test]
fn a_text_edit_open_at_a_switch_is_kept_and_its_canvas_written() {
    let mut app = TestApp::with_space([("Notes", note("n1", "hello")), ("Home", document([]))]);
    app.double_click((200.0, 200.0)).type_text("bye");
    app.take_effects();
    app.switch_to("Home");
    let notes = app.canvas_id("Notes");
    assert!(app.effects().contains(&Effect::WriteCanvas(notes.clone())));
    assert!(app.session().editing.is_none());
    let kept = app.app().canvas_document(&notes).expect("a canvas");
    let text = kept.entities().find_map(|entity| match &entity.kind {
        specular_doc::Kind::Text(text) => Some(text.text.as_str()),
        _ => None,
    });
    assert_eq!(text, Some("bye"));
    app.switch_to("Notes").undo();
    assert!(app.doc_snapshot().contains("hello"));
}

#[test]
fn a_new_canvas_is_named_in_sequence_shown_and_written() {
    let mut app = two();
    app.act(Action::Canvas(CanvasAction::New));
    assert_eq!(app.canvas_names(), ["Home", "Notes", "Canvas 3"]);
    assert_eq!(app.active_canvas(), "Canvas 3");
    assert_eq!(entity_ids(app.document()), [] as [&str; 0]);
    let id = app.canvas_id("Canvas 3");
    let effects = app.take_effects();
    assert!(effects.contains(&Effect::WriteCanvas(id.clone())));
    assert!(effects.contains(&Effect::SaveSpaceMeta));
    let canvas = app.app().space().canvas(&id).expect("a canvas");
    let suffix: String = id
        .as_str()
        .trim_start_matches("tab_")
        .chars()
        .take(4)
        .collect();
    assert_eq!(canvas.file, format!("Canvas 3-{suffix}.canvas"));

    app.act(Action::Canvas(CanvasAction::New));
    assert_eq!(app.active_canvas(), "Canvas 4");
}

#[test]
fn renaming_moves_the_file_and_a_taken_or_empty_name_is_refused() {
    let mut app = TestApp::with_space([
        ("Home", document(pages(2))),
        (" Notes ", note("n1", "hello")),
    ]);
    let home = app.canvas_id("Home");
    let rename = |name: &str| {
        Action::Canvas(CanvasAction::Rename {
            canvas: None,
            name: name.to_owned(),
        })
    };
    app.act(rename("  Landing page "));
    assert_eq!(app.canvas_names(), ["Landing page", " Notes "]);
    assert_eq!(
        app.take_effects(),
        [
            Effect::RenameCanvasFile {
                canvas: home.clone(),
                from: "Home.canvas".to_owned(),
                to: "Landing page-1.canvas".to_owned(),
            },
            Effect::SaveSpaceMeta,
        ]
    );
    // Taken, empty, and the name it already has.
    for refused in ["Notes", " Notes ", "   ", "Landing page"] {
        app.act(rename(refused));
        assert_eq!(app.take_effects(), [], "{refused:?}");
    }
    assert_eq!(app.canvas_names(), ["Landing page", " Notes "]);
    // Two names that make the same file stem: the name changes, no file moves.
    app.act(rename("No/tes"));
    app.take_effects();
    app.act(rename("No:tes"));
    assert_eq!(app.canvas_names(), ["No:tes", " Notes "]);
    assert_eq!(app.take_effects(), [Effect::SaveSpaceMeta]);
}

#[test]
fn a_duplicate_sits_after_its_source_is_shown_and_starts_with_no_history() {
    let mut app = two();
    app.act(select("p1"))
        .act(Action::Nudge { dx: 40.0, dy: 0.0 });
    app.take_effects();
    app.act(Action::Canvas(CanvasAction::Duplicate(None)));
    assert_eq!(app.canvas_names(), ["Home", "Home Copy", "Notes"]);
    assert_eq!(app.active_canvas(), "Home Copy");
    assert_eq!(app.rect("p1").x, 140.0);
    assert!(!app.app().can_undo());
    let copy = app.canvas_id("Home Copy");
    assert!(app.effects().contains(&Effect::WriteCanvas(copy)));

    // The copy goes on from where it was; a copy of Home is Home as it is.
    app.act(select("p1"))
        .act(Action::Nudge { dx: 20.0, dy: 0.0 });
    let home = app.canvas_id("Home");
    app.act(Action::Canvas(CanvasAction::Duplicate(Some(home))));
    assert_eq!(
        app.canvas_names(),
        ["Home", "Home Copy 2", "Home Copy", "Notes"]
    );
    assert_eq!(app.active_canvas(), "Home Copy 2");
    assert_eq!(app.rect("p1").x, 140.0);

    let mut spaced = TestApp::with_space([(" Notes ", note("n1", "hello"))]);
    spaced.act(Action::Canvas(CanvasAction::Duplicate(None)));
    assert_eq!(spaced.canvas_names(), [" Notes ", "Notes Copy"]);
}

#[test]
fn deleting_the_active_canvas_shows_its_neighbour_and_trashes_the_file() {
    let mut app = TestApp::with_space([
        ("One", note("a", "1")),
        ("Two", note("b", "2")),
        ("Three", note("c", "3")),
        ("Four", note("d", "4")),
    ]);
    app.switch_to("Two").take_effects();
    let two = app.canvas_id("Two");
    app.act(Action::Canvas(CanvasAction::Delete(None)));
    assert_eq!(app.canvas_names(), ["One", "Three", "Four"]);
    assert_eq!(app.active_canvas(), "Three");
    assert_eq!(entity_ids(app.document()), ["c"]);
    let trashed = Effect::TrashCanvasFile {
        canvas: two,
        file: "Two.canvas".to_owned(),
    };
    assert!(app.take_effects().contains(&trashed));

    // The last in the list falls back to the one before it.
    app.switch_to("Four");
    app.act(Action::Canvas(CanvasAction::Delete(None)));
    assert_eq!(
        (app.canvas_names(), app.active_canvas()),
        (vec!["One", "Three"], "Three")
    );
    assert_eq!(entity_ids(app.document()), ["c"]);
}

#[test]
fn deleting_the_only_canvas_leaves_an_empty_canvas_1() {
    let mut app = TestApp::with_space([("Solo", document(pages(1)))]);
    app.act(Action::Canvas(CanvasAction::Delete(None)));
    assert_eq!(app.canvas_names(), ["Canvas 1"]);
    assert_eq!(entity_ids(app.document()), [] as [&str; 0]);
    let fresh = app.canvas_id("Canvas 1");
    let effects = app.take_effects();
    assert!(effects.contains(&Effect::WriteCanvas(fresh)));
    assert!(effects.contains(&Effect::ClosePage(EntityId::from("p1"))));
    assert!(effects.iter().any(
        |effect| matches!(effect, Effect::TrashCanvasFile { file, .. } if file == "Solo.canvas")
    ));
}

#[test]
fn a_background_file_change_replaces_that_canvas_and_clears_its_history() {
    let mut app = two();
    app.switch_to("Notes").act(select("n1")).act(Action::Delete);
    app.switch_to("Home").take_effects();
    let notes = app.canvas_id("Notes");
    app.send(Event::CanvasFileChanged {
        canvas: notes.clone(),
        document: Box::new(note("n2", "from outside")),
    });
    assert_eq!(app.take_effects(), [], "nothing on screen changed");
    assert_eq!(entity_ids(app.document()), ["p1", "p2"]);
    let held = app.app().canvas_document(&notes).map(entity_ids);
    assert_eq!(held, Some(vec!["n2"]));
    app.switch_to("Notes");
    assert!(!app.app().can_undo());
}

#[test]
fn a_tab_ref_is_an_id_or_an_exact_name_and_never_a_guess() {
    let app = TestApp::with_space([
        ("Home", document([])),
        ("tab_1", document([])),
        (" Notes ", document([])),
    ]);
    let space = app.app().space();
    let name = |tab_ref: &str| space.resolve(tab_ref).map(|canvas| canvas.name.as_str());
    // An id wins over a canvas that has it as its name.
    assert_eq!(name("tab_1"), Ok("Home"));
    assert_eq!(name(" Notes"), Ok(" Notes "));
    assert_eq!(name("  "), Err(TabRefError::Empty));
    let unknown = name("home").expect_err("names are exact");
    assert_eq!(
        unknown.to_string(),
        "unknown tab 'home' \u{2014} available: tab_1 (Home), tab_2 (tab_1), tab_3 ( Notes )"
    );
}
