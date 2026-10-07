//! Documents: `update` asks the shell for each markdown file, keeps the
//! text the shell sends, and scrolls a selected Document under the wheel.

use specular_doc::{Document, Entity, EntityId, FileRef, Kind, Rect};
use specular_interact::{Action, Effect, Event, NoteNotice, NoteState};
use specular_testkit::{CMD, TestApp, document, file, page};

const BOX: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);
/// A point inside [`BOX`] under the default camera.
const INSIDE: (f32, f32) = (300.0, 250.0);

fn note_at(id: &str, path: &str, rect: Rect) -> Entity {
    let file = FileRef {
        file: path.to_owned(),
        ..FileRef::default()
    };
    Entity::new(id, rect, Kind::File(file))
}

fn note(id: &str, path: &str) -> Entity {
    note_at(id, path, BOX)
}

fn opened(document: Document) -> TestApp {
    let mut app = TestApp::empty();
    app.open(document);
    app
}

/// The note effects in `effects`: `+file` for a load, `-file` for a drop.
fn note_effects(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::LoadNote { file } => Some(format!("+{file}")),
            Effect::DropNote { file } => Some(format!("-{file}")),
            _ => None,
        })
        .collect()
}

fn text(app: &mut TestApp, file: &str, text: &str) {
    app.send(Event::Note {
        file: file.to_owned(),
        notice: NoteNotice::Text(text.to_owned()),
    });
}

fn scroll(app: &TestApp, id: &str) -> f32 {
    app.app().note_scroll(&EntityId::from(id))
}

#[test]
fn opening_a_document_asks_for_each_markdown_file_once() {
    let mut app = opened(document([
        note("a", "notes/plan.md"),
        note("b", "notes/plan.md"),
        note("c", "/abs/README.MD"),
        file("d", BOX),
        page("p1", BOX),
    ]));
    assert_eq!(
        note_effects(&app.take_effects()),
        ["+notes/plan.md", "+/abs/README.MD"]
    );
    assert_eq!(app.app().note("notes/plan.md"), Some(&NoteState::Loading));
    assert_eq!(app.app().note("d.png"), None);
}

#[test]
fn the_text_the_shell_sends_is_kept_and_replaced_when_the_file_changes() {
    let mut app = opened(document([note("a", "a.md"), note("b", "b.md")]));
    text(&mut app, "a.md", "# one");
    app.send(Event::Note {
        file: "b.md".to_owned(),
        notice: NoteNotice::Missing,
    });
    assert_eq!(
        app.app().note("a.md"),
        Some(&NoteState::Ready("# one".into()))
    );
    assert_eq!(app.app().note("b.md"), Some(&NoteState::Missing));

    // The file was edited on disk, and the missing one appeared.
    text(&mut app, "a.md", "# two");
    text(&mut app, "b.md", "now here");
    assert_eq!(
        app.app().note("a.md"),
        Some(&NoteState::Ready("# two".into()))
    );
    assert_eq!(
        app.app().note("b.md"),
        Some(&NoteState::Ready("now here".into()))
    );
}

#[test]
fn text_for_a_file_no_longer_held_is_ignored() {
    let mut app = opened(document([note("a", "a.md")]));
    let before = app.session().clone();
    text(&mut app, "gone.md", "late");
    assert_eq!(app.session(), &before);
}

#[test]
fn opening_another_document_drops_what_it_does_not_show_and_keeps_the_rest() {
    let mut app = opened(document([note("a", "a.md"), note("b", "b.md")]));
    text(&mut app, "b.md", "kept");
    app.take_effects();
    app.open(document([note("b", "b.md"), note("c", "c.md")]));
    assert_eq!(note_effects(&app.take_effects()), ["-a.md", "+c.md"]);
    assert_eq!(app.app().note("a.md"), None);
    assert_eq!(
        app.app().note("b.md"),
        Some(&NoteState::Ready("kept".into()))
    );
}

#[test]
fn a_deleted_document_keeps_its_text_for_the_undo() {
    let mut app = opened(document([note("a", "a.md")]));
    text(&mut app, "a.md", "body");
    app.select(&["a"]).act(Action::Delete);
    app.take_effects();
    app.undo();
    assert_eq!(note_effects(&app.take_effects()), [] as [&str; 0]);
    assert_eq!(
        app.app().note("a.md"),
        Some(&NoteState::Ready("body".into()))
    );
}

#[test]
fn the_wheel_scrolls_the_selected_document_under_the_pointer() {
    let mut app = opened(document([note("a", "a.md")]));
    let camera = app.session().camera;
    app.select(&["a"]).pointer_move(INSIDE).wheel((0.0, -60.0));
    assert_eq!((scroll(&app, "a"), app.session().camera), (60.0, camera));
    // Back past the top stops at the top.
    app.wheel((0.0, 25.0));
    assert_eq!(scroll(&app, "a"), 35.0);
    app.wheel((0.0, 500.0));
    assert_eq!(scroll(&app, "a"), 0.0);
}

#[test]
fn the_wheel_scrolls_by_canvas_units_at_any_zoom() {
    let mut app = opened(document([note("a", "a.md")]));
    app.zoom(2.0).select(&["a"]);
    let centre = app
        .session()
        .camera
        .world_to_screen(glam::Vec2::new(300.0, 250.0));
    app.pointer_move(centre).wheel((0.0, -60.0));
    assert_eq!(scroll(&app, "a"), 30.0);
}

#[test]
fn the_wheel_pans_the_canvas_unless_the_document_is_the_whole_selection() {
    let far = Rect::new(900.0, 100.0, 400.0, 300.0);
    let mut app = opened(document([note("a", "a.md"), note_at("b", "b.md", far)]));
    let start = app.session().camera;
    // Not selected.
    app.pointer_move(INSIDE).wheel((0.0, -60.0));
    let panned = app.session().camera;
    assert!(scroll(&app, "a") == 0.0 && panned != start);
    // Selected with another.
    app.select(&["a", "b"]).wheel((0.0, -60.0));
    assert!(scroll(&app, "a") == 0.0 && app.session().camera != panned);
    // Selected, but the pointer is over empty canvas.
    let moved = app.session().camera;
    app.select(&["a"])
        .pointer_move((5.0, 5.0))
        .wheel((0.0, -60.0));
    assert!(scroll(&app, "a") == 0.0 && app.session().camera != moved);
}

#[test]
fn cmd_wheel_over_a_selected_document_still_zooms() {
    let mut app = opened(document([note("a", "a.md")]));
    let zoom = app.session().camera.zoom;
    app.select(&["a"]).pointer_move(INSIDE);
    app.hold(CMD).wheel((0.0, -60.0)).let_go();
    let zoomed = (app.session().camera.zoom - zoom).abs() > 0.001;
    assert!(scroll(&app, "a") == 0.0 && zoomed);
}

#[test]
fn scroll_is_forgotten_with_the_entity_when_another_document_opens() {
    let mut app = opened(document([note("a", "a.md"), note("b", "b.md")]));
    app.select(&["a"]).pointer_move(INSIDE).wheel((0.0, -60.0));
    app.open(document([note("b", "b.md")]));
    assert_eq!(scroll(&app, "a"), 0.0);
    // The same id back in a third document starts at the top.
    app.open(document([note("a", "a.md")]));
    assert_eq!(scroll(&app, "a"), 0.0);
}
