//! An HTML file node is a page like a link node: hosted at its file's URL
//! in the space folder, entered, commented on and shown in its own tab, and
//! still a `file` node in the `.canvas` whatever its page does.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_core::{CssSize, synthetic_element_at};
use specular_doc::{Document, EntityId, Kind};
use specular_interact::{Action, Effect, Event, Focus, Key, PageNotice, Showing, Tool, settings};
use specular_testkit::TestApp;

/// Inside h1.
const ON_H1: (f32, f32) = (200.0, 150.0);
const URL: &str = "file:///space/assets/demo.html";

fn id(name: &str) -> EntityId {
    EntityId::new(name)
}

const H1: &str = r#"{"id":"h1","type":"file","file":"assets/demo.html",
    "x":100,"y":100,"width":400,"height":300}"#;

/// A canvas of `nodes` opened on its own from the folder `/space`.
fn opened_from_space(nodes: &[&str]) -> TestApp {
    let canvas = format!(r#"{{"nodes":[{}],"edges":[]}}"#, nodes.join(","));
    let document = Document::from_canvas_str(&canvas).unwrap_or_default();
    let mut app = TestApp::empty();
    app.send(Event::DocumentFolder("/space".to_owned()));
    app.open(document);
    app
}

/// The file node h1 at (100, 100), 400x300, in the folder `/space`.
fn html_app() -> TestApp {
    opened_from_space(&[H1])
}

fn file_kind(app: &TestApp) -> Kind {
    match &app.entity("h1").kind {
        kind @ Kind::File(_) => kind.clone(),
        other => panic!("h1 is not a file node: {other:?}"),
    }
}

fn hosting(effects: &[Effect]) -> Vec<&Effect> {
    (effects.iter())
        .filter(|effect| matches!(effect, Effect::CreatePage { .. } | Effect::ClosePage(_)))
        .collect()
}

#[test]
fn an_html_file_is_hosted_at_its_file_url_and_stays_a_file() {
    let mut app = html_app();
    let file = file_kind(&app);
    let create = Effect::CreatePage {
        page: id("h1"),
        url: URL.to_owned(),
        viewport: CssSize::new(400, 300),
    };
    assert_eq!(hosting(&app.take_effects()), [&create]);
    // Select-first, as a page is: the second click enters it.
    app.click(ON_H1).click(ON_H1);
    assert_eq!(app.session().focus, Focus::Page(id("h1")));
    // Where the page goes from its file is not written back.
    app.take_effects();
    app.page_reports("h1", PageNotice::Url("https://example.com/".to_owned()));
    assert_eq!(
        (file_kind(&app), app.take_effects()),
        (file.clone(), vec![])
    );
    app.key(Key::Escape).key(Key::Backspace);
    assert_eq!(hosting(&app.take_effects()), [&Effect::ClosePage(id("h1"))]);
    app.undo();
    assert_eq!(hosting(&app.take_effects()), [&create]);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_comment_on_an_html_file_is_bound_to_its_page_and_shown_in_its_tab() {
    let mut app = html_app();
    app.tool(Tool::Comment).click(ON_H1);
    app.answer_element(synthetic_element_at(
        CssSize::new(400, 300),
        glam::Vec2::new(100.0, 50.0),
    ));
    app.type_text("tighten this").key(Key::Enter);
    let binding = app.document().annotations()[0].page_anchor.clone();
    // Recorded by its path in the folder, so the `.canvas` holds no
    // absolute path and the comment survives the folder moving.
    let bound = binding.map(|anchor| (anchor.page_id, anchor.page_url));
    assert_eq!(bound, Some((id("h1"), Some("assets/demo.html".to_owned()))));
    assert_eq!(app.app().comment_marks().len(), 1, "the comment is shown");
    app.send(Event::DocumentFolder("/moved".to_owned()));
    assert_eq!(
        app.app().comment_marks().len(),
        1,
        "and still is once moved"
    );
    // Its own tab fills the view with it, laid out at the tab's size, and
    // the comment comes along.
    app.key(Key::Escape).key(Key::Escape).take_effects();
    app.act(Action::Show(Showing::Item(id("h1"))));
    assert_eq!(app.app().shown_item(), Some(&id("h1")));
    let resized = (app.take_effects().into_iter())
        .any(|effect| matches!(effect, Effect::SetPageViewport { page, .. } if page == id("h1")));
    assert!(resized, "the page is laid out at the tab's size");
    assert_eq!(app.app().comment_marks().len(), 1);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_canvas_opened_on_its_own_loads_its_files_from_its_folder_but_opens_no_space() {
    let app = html_app();
    assert_eq!(
        app.app().page(&id("h1")).map(|page| page.url.clone()),
        Some(URL.to_owned())
    );
    assert_eq!(settings(app.app()).general.space, None);
}

#[test]
fn a_sync_set_is_links_or_html_files_never_both() {
    let link = r#"{"id":"p1","type":"link","url":"https://example.com/",
        "x":600,"y":100,"width":400,"height":300}"#;
    let mut app = opened_from_space(&[H1, link]);
    app.select(&["h1", "p1"]).act(Action::ToggleSync);
    assert_eq!(app.app().selection_synced(), None);
    assert!(!app.app().is_synced(&id("h1")) && !app.app().is_synced(&id("p1")));
}
