//! Focusing a comment, resolving it and deleting it: each change is one undo
//! step, and the focus and the selection are never both set.

use specular_doc::{Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, EntityId, Rect};
use specular_interact::{Action, Key, MenuEntry, Tool, menus};
use specular_testkit::{
    TestApp, assert_doc_snapshot, comment, document, pages, shape, with_comment,
};

/// Where the pill of a comment on `p1`'s right edge, halfway down, is.
const PILL: (f32, f32) = (480.0, 250.0);

fn id(id: &str) -> AnnotationId {
    AnnotationId::new(id)
}

fn on_page(offset_y: f64) -> AnnotationAnchor {
    AnnotationAnchor::Page {
        page_id: EntityId::from("p1"),
        offset_x: 0.5,
        offset_y,
    }
}

fn element(selector: &str) -> AnnotationAnchor {
    AnnotationAnchor::Element {
        page_id: EntityId::from("p1"),
        selector: selector.to_owned(),
        element_path: None,
        bounding_box: Some(Rect::new(50.0, 40.0, 100.0, 30.0)),
    }
}

/// Two pages with `comments` on them, the first in the pill at [`PILL`].
fn app_with(comments: impl IntoIterator<Item = Annotation>) -> TestApp {
    let mut document = document(pages(2));
    for annotation in comments {
        document = with_comment(document, annotation);
    }
    TestApp::from_document(document)
}

fn one() -> TestApp {
    app_with([comment("a", on_page(0.5), "fix the header")])
}

fn statuses(app: &TestApp) -> Vec<(&str, AnnotationStatus)> {
    (app.document().annotations().iter())
        .map(|annotation| (annotation.id.as_str(), annotation.status))
        .collect()
}

fn menu_enabled(app: &TestApp, label: &str) -> bool {
    (menus(app.app()).into_iter())
        .flat_map(|menu| menu.entries)
        .find_map(|entry| match entry {
            MenuEntry::Item(item) if item.label == label => Some(item.enabled),
            MenuEntry::Item(_) | MenuEntry::Separator => None,
        })
        .unwrap_or(false)
}

#[test]
fn a_click_on_a_pill_focuses_its_comment_with_the_select_tool() {
    let mut app = one();
    app.click(PILL);
    assert_eq!(app.app().focused_comment(), Some(&id("a")));
    assert!(app.app().comment_marks()[0].focused);
    assert_eq!(app.session().gesture, None);
    // The page under the pill was not selected by the press.
    assert_eq!(app.selected(), None);
}

#[test]
fn a_click_on_a_pill_with_the_comment_tool_focuses_it_and_starts_no_comment() {
    let mut app = one();
    app.tool(Tool::Comment).take_effects();
    app.click(PILL);
    assert_eq!(app.app().focused_comment(), Some(&id("a")));
    assert!(app.app().comment_draft().is_none());
    assert_eq!(app.session().tool, Tool::Comment);
    assert_eq!(app.take_effects(), []);
}

#[test]
fn a_press_on_a_pill_with_another_tool_is_that_tools() {
    let mut app = one();
    app.tool(Tool::AddShape).click(PILL);
    assert_eq!(app.app().focused_comment(), None);
}

#[test]
fn focusing_a_comment_clears_the_selection_and_selecting_clears_the_focus() {
    let mut app = one();
    app.select(&["p2"]);
    app.click(PILL);
    assert_eq!(
        (app.selected_ids(), app.app().focused_comment()),
        (Vec::<&str>::new(), Some(&id("a")))
    );
    app.click((900.0, 300.0));
    assert_eq!(
        (app.selected_ids(), app.app().focused_comment()),
        (vec!["p2"], None)
    );
    app.act(Action::FocusComment(Some(id("a"))));
    assert_eq!(
        (app.selected_ids(), app.app().focused_comment()),
        (Vec::<&str>::new(), Some(&id("a")))
    );
    app.select(&["p1"]);
    assert_eq!(app.app().focused_comment(), None);
}

#[test]
fn focus_comment_ignores_ids_it_does_not_show_and_none_lets_go() {
    let mut hidden = comment("h", on_page(0.2), "hidden");
    hidden.status = AnnotationStatus::Resolved;
    let mut app = app_with([comment("a", on_page(0.5), "a"), hidden]);
    app.act(Action::FocusComment(Some(id("nope"))));
    app.act(Action::FocusComment(Some(id("h"))));
    assert_eq!(app.app().focused_comment(), None);
    app.act(Action::FocusComment(Some(id("a"))));
    assert_eq!(app.app().focused_comment(), Some(&id("a")));
    app.act(Action::FocusComment(None));
    assert_eq!(app.app().focused_comment(), None);
    assert!(!app.app().can_undo(), "focus is not in undo");
}

#[test]
fn escape_lets_go_of_the_focus_and_the_next_escape_does_what_it_did() {
    let mut app = one();
    app.tool(Tool::Comment).click(PILL);
    assert_eq!(app.app().focused_comment(), Some(&id("a")));
    app.key(Key::Escape);
    assert_eq!(
        (app.app().focused_comment(), app.session().tool),
        (None, Tool::Comment)
    );
    app.key(Key::Escape);
    assert_eq!(app.session().tool, Tool::Select);
}

#[test]
fn resolving_marks_the_comment_resolved_by_the_user_in_one_step() {
    let mut app = one();
    app.click(PILL).act(Action::ResolveComment(None));
    assert_eq!(statuses(&app), [("a", AnnotationStatus::Resolved)]);
    assert_eq!(app.app().focused_comment(), None);
    assert_eq!(app.app().comment_marks().len(), 0);
    assert_doc_snapshot!(app);
    app.undo();
    assert_eq!(statuses(&app), [("a", AnnotationStatus::Pending)]);
    assert!(!app.app().can_undo());
    assert_eq!(app.app().comment_marks().len(), 1);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn resolving_a_dismissed_comment_drops_its_reason() {
    let mut dismissed = comment("a", on_page(0.5), "a");
    dismissed.status = AnnotationStatus::Acknowledged;
    dismissed.metadata = Some(
        serde_json::from_str(r#"{"dismissReason":"stale","pageName":"Home"}"#).unwrap_or_default(),
    );
    let mut app = app_with([dismissed]);
    app.act(Action::ResolveComment(Some(id("a"))));
    let metadata = app.document().annotations()[0].metadata.clone();
    assert_eq!(
        serde_json::to_string(&metadata).unwrap_or_default(),
        r#"{"pageName":"Home","resolvedBy":"user"}"#
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn deleting_removes_the_comment_and_undo_puts_it_back_where_it_was() {
    let mut app = app_with([
        comment("a", on_page(0.2), "a"),
        comment("b", on_page(0.5), "b"),
        comment("c", on_page(0.8), "c"),
    ]);
    app.act(Action::DeleteComment(Some(id("b"))));
    let ids = |app: &TestApp| -> Vec<String> {
        (app.document().annotations().iter())
            .map(|annotation| annotation.id.as_str().to_owned())
            .collect()
    };
    assert_eq!(ids(&app), ["a", "c"]);
    app.undo();
    assert_eq!(ids(&app), ["a", "b", "c"]);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn none_acts_on_every_comment_of_the_focused_pill_as_one_step() {
    let mut app = app_with([
        comment("a", element("#cta"), "a"),
        comment("b", element("#cta"), "b"),
        comment("c", element("#nav"), "c"),
    ]);
    app.act(Action::FocusComment(Some(id("a"))));
    assert!(app.app().comment_marks().iter().any(|mark| mark.focused));
    app.act(Action::ResolveComment(None));
    assert_eq!(
        statuses(&app),
        [
            ("a", AnnotationStatus::Resolved),
            ("b", AnnotationStatus::Resolved),
            ("c", AnnotationStatus::Pending)
        ]
    );
    app.undo();
    assert_eq!(statuses(&app)[0].1, AnnotationStatus::Pending);
    assert!(!app.app().can_undo(), "both were one step");
    app.redo().act(Action::FocusComment(Some(id("c"))));
    app.act(Action::DeleteComment(None));
    assert_eq!(app.document().annotations().len(), 2);
    app.assert_undo_returns_to_start();
}

#[test]
fn deleting_a_shared_pill_removes_all_of_it_in_one_step() {
    let mut app = app_with([
        comment("a", element("#cta"), "a"),
        comment("b", element("#cta"), "b"),
    ]);
    app.act(Action::FocusComment(Some(id("b"))))
        .act(Action::DeleteComment(None));
    assert_eq!(app.document().annotations().len(), 0);
    app.undo();
    assert_eq!(app.document().annotations().len(), 2);
    assert!(!app.app().can_undo());
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn an_unknown_id_or_no_focus_does_nothing_and_records_nothing() {
    let mut app = one();
    app.act(Action::ResolveComment(None))
        .act(Action::DeleteComment(None))
        .act(Action::ResolveComment(Some(id("nope"))))
        .act(Action::DeleteComment(Some(id("nope"))));
    assert_eq!(statuses(&app), [("a", AnnotationStatus::Pending)]);
    assert!(!app.app().can_undo());
    // Resolving what is resolved already is nothing too.
    app.act(Action::ResolveComment(Some(id("a"))))
        .act(Action::ResolveComment(Some(id("a"))));
    app.undo();
    assert!(!app.app().can_undo());
}

#[test]
fn backspace_and_delete_remove_the_focused_comment() {
    for key in [Key::Backspace, Key::Delete] {
        let mut app = one();
        app.click(PILL).key(key);
        assert!(app.document().annotations().is_empty(), "{key:?}");
        assert_eq!(app.app().focused_comment(), None);
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn delete_with_a_selection_still_removes_the_selection() {
    let mut app = TestApp::from_document(with_comment(
        document([
            specular_testkit::page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
            shape("s", Rect::new(600.0, 500.0, 100.0, 100.0)),
        ]),
        comment("a", on_page(0.5), "a"),
    ));
    app.select(&["s"]).key(Key::Backspace);
    assert!(app.document().entity(&"s".into()).is_none());
    assert_eq!(app.document().annotations().len(), 1);
    app.assert_undo_returns_to_start();
}

#[test]
fn delete_does_nothing_to_a_comment_while_text_is_edited() {
    let mut app = one();
    app.click(PILL).tool(Tool::Comment);
    app.click((800.0, 700.0))
        .type_text("abc")
        .key(Key::Backspace);
    assert_eq!(app.editing_text(), "ab");
    assert_eq!(app.document().annotations().len(), 1);
}

#[test]
fn the_menu_items_are_enabled_only_with_something_to_act_on() {
    let mut app = one();
    let enabled = |app: &TestApp| {
        [
            menu_enabled(app, "Annotate selection"),
            menu_enabled(app, "Resolve comment"),
            menu_enabled(app, "Delete comment"),
        ]
    };
    assert_eq!(enabled(&app), [false, false, false]);
    app.select(&["p2"]);
    assert_eq!(enabled(&app), [true, false, false]);
    app.click(PILL);
    assert_eq!(enabled(&app), [false, true, true]);
    assert!(menu_enabled(&app, "Delete"), "Delete takes the comment too");
}

#[test]
fn the_comment_menu_runs_the_actions_and_has_no_keys() {
    let app = one();
    let all = menus(app.app());
    let comment_menu = all.iter().find(|menu| menu.title == "Comment");
    let items: Vec<_> = (comment_menu
        .map(|menu| menu.entries.clone())
        .unwrap_or_default())
    .into_iter()
    .filter_map(|entry| match entry {
        MenuEntry::Item(item) => Some((item.label.into_owned(), item.action, item.chord)),
        MenuEntry::Separator => None,
    })
    .collect();
    assert_eq!(
        items,
        [
            (
                "Annotate selection".to_owned(),
                Action::AnnotateSelection,
                None
            ),
            (
                "Resolve comment".to_owned(),
                Action::ResolveComment(None),
                None
            ),
            (
                "Delete comment".to_owned(),
                Action::DeleteComment(None),
                None
            ),
        ]
    );
}

#[test]
fn a_committed_comment_is_focused_with_nothing_selected() {
    let mut app = TestApp::with_pages(2);
    app.select(&["p2"])
        .tool(Tool::Comment)
        .click((900.0, 700.0));
    app.type_text("here").key(Key::Enter);
    let made = app.document().annotations()[0].id.clone();
    assert_eq!(
        (app.app().focused_comment(), app.selected_ids()),
        (Some(&made), Vec::<&str>::new())
    );
    app.undo();
    assert_eq!(app.app().focused_comment(), None, "the comment is gone");
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn annotating_a_selection_ends_with_the_new_comment_focused_and_nothing_selected() {
    let mut app = TestApp::with_pages(2);
    app.select(&["p1"]).act(Action::AnnotateSelection);
    app.type_text("check this").key(Key::Enter);
    let made = app.document().annotations()[0].id.clone();
    assert_eq!(
        (app.app().focused_comment(), app.selected_ids()),
        (Some(&made), Vec::<&str>::new())
    );
    app.undo();
    assert_eq!(app.app().focused_comment(), None);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn opening_another_document_forgets_the_focus() {
    let mut app = one();
    app.click(PILL);
    assert_eq!(app.app().focused_comment(), Some(&id("a")));
    app.open(document(pages(2)));
    assert_eq!(app.app().focused_comment(), None);
}
