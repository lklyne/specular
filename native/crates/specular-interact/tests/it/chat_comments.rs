//! Comments and chat are one conversation: a comment placed with a right
//! panel is finished in the panel, queued into a thread, and saved with that
//! thread's id.

use specular_core::{CssSize, synthetic_element_at};
use specular_doc::{Document, Entity};
use specular_interact::{Action, ChatAction, DraftKind, Effect, Key, Tool, toolbar};
use specular_testkit::{TestApp, assert_chat_snapshot, document, pages};

/// An app showing `entities` in a one-canvas space with a folder, the right
/// panel present and the comment tool armed.
fn panel_app(entities: impl IntoIterator<Item = Entity>) -> TestApp {
    let mut app = TestApp::with_space([("Home", Document::new())]);
    app.open(document(entities)).with_chat_panel();
    app.tool(Tool::Comment).take_effects();
    app
}

fn thread_id_of(app: &TestApp, annotation: usize) -> String {
    let annotation = &app.document().annotations()[annotation];
    let id = annotation.metadata.as_ref().and_then(|m| m.get("threadId"));
    id.and_then(|id| id.as_str()).unwrap_or_default().to_owned()
}

#[test]
fn placing_a_comment_opens_a_passive_draft_and_send_queues_it_without_a_run() {
    let mut app = panel_app([]);
    app.tick(86_400_000).click((600.0, 500.0));
    assert!(
        app.session().editing.is_none() && app.app().comment_composer().is_none(),
        "the field is the panel's, so the canvas has no text edit and no card"
    );
    assert_chat_snapshot!(app, @r#"
    chat available=yes visible=yes width=400 title="Threads"
    controls back=no close=no
    list
    draft Point "Canvas point"
    composer placeholder="Add a comment…" send-empty=no running=no pill=Canvas "Home" folder=Some("space")
    "#);
    app.send_chat("  move this ");
    let thread = app.chat_thread_id().expect("a draft thread was started");
    assert_eq!(thread_id_of(&app, 0), thread.as_str());
    let effects = app.take_effects();
    assert!(effects.contains(&Effect::WriteThread(thread)));
    assert!(effects.contains(&Effect::WriteThreadIndex));
    assert!(!effects.iter().any(|e| matches!(e, Effect::RunAgent(_))));
    assert_chat_snapshot!(app, @r#"
    chat available=yes visible=yes width=400 title="move this"
    thread [x] "move this" draft 1970-01-02T00:00:00.000Z
    controls back=yes close=yes
    hint "Comment on the canvas to queue a draft, or type below and send."
    queued "move this" pin=e220a8397b1dcdaf
    composer placeholder="Add or edit…" send-empty=yes running=no pill=Comment "move this" folder=Some("space")
    "#);
    app.undo();
    assert_eq!(app.document().annotations().len(), 0);
    assert_eq!(
        app.app().threads().all().len(),
        1,
        "chat is not undoable: the thread keeps its message"
    );
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn a_draft_with_nothing_to_send_stays_open() {
    let mut app = panel_app([]);
    app.click((600.0, 500.0)).send_chat("   ");
    assert_eq!(
        (
            app.app().comment_draft().is_some(),
            app.document().annotations().len()
        ),
        (true, 0)
    );
}

#[test]
fn a_second_comment_joins_the_draft_and_the_next_after_a_run_starts_another() {
    let mut app = panel_app([]);
    app.click((600.0, 500.0)).send_chat("one");
    app.click((700.0, 500.0)).send_chat("two");
    assert_eq!(thread_id_of(&app, 0), thread_id_of(&app, 1));
    assert_eq!(app.app().threads().all()[0].messages.len(), 2);
    app.send_chat("")
        .agent_says(specular_interact::Notice::Finished {
            text: "Done".into(),
        });
    app.click((800.0, 500.0)).send_chat("three");
    assert_ne!(thread_id_of(&app, 2), thread_id_of(&app, 0));
    assert_eq!(app.chat().threads.len(), 2);
}

#[test]
fn an_element_comment_names_its_anchor_in_the_draft_chip() {
    let mut app = panel_app(pages(1));
    app.click((200.0, 200.0))
        .answer_element(synthetic_element_at(
            CssSize::new(400, 300),
            (100.0, 100.0).into(),
        ));
    let draft = app.chat().composer.draft.expect("a draft chip");
    assert_eq!(
        (draft.kind, draft.label.as_str()),
        (
            DraftKind::Element,
            "div.cell[data-col=\"0\"][data-row=\"2\"]"
        )
    );
}

#[test]
fn escape_and_the_chip_drop_a_draft_and_leaving_the_tool_keeps_only_a_selection_draft() {
    let mut app = panel_app(pages(2));
    app.click((600.0, 500.0)).key(Key::Escape);
    assert!(app.app().comment_draft().is_none(), "Escape drops it");
    app.click((600.0, 500.0))
        .act(Action::Chat(ChatAction::DropDraft));
    assert!(app.app().comment_draft().is_none(), "so does its chip");
    app.click((600.0, 500.0)).tool(Tool::Select);
    assert!(
        app.app().comment_draft().is_none(),
        "a point draft is the tool's"
    );
    app.select(&["p1", "p2"])
        .act(Action::AnnotateSelection)
        .tool(Tool::Comment);
    app.tool(Tool::Select);
    let draft = app
        .chat()
        .composer
        .draft
        .expect("the selection draft stays");
    assert_eq!(
        (draft.kind, draft.label.as_str()),
        (DraftKind::Selection, "2 items")
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn switching_canvas_clears_the_draft() {
    let mut app = TestApp::with_space([("Home", Document::new()), ("Notes", Document::new())]);
    app.with_chat_panel()
        .tool(Tool::Comment)
        .click((600.0, 500.0));
    assert!(app.app().comment_draft().is_some());
    app.switch_to("Notes");
    assert!(app.app().comment_draft().is_none());
}

#[test]
fn without_a_panel_the_card_on_the_canvas_still_commits_into_a_thread() {
    let mut app = TestApp::with_space([("Home", Document::new())]);
    app.tool(Tool::Comment).click((600.0, 500.0));
    assert!(app.app().comment_composer().is_some());
    app.take_effects();
    app.type_text("move this").key(Key::Enter);
    let thread = app.chat_thread_id().expect("queued into a draft thread");
    assert_eq!(thread_id_of(&app, 0), thread.as_str());
    assert!(app.take_effects().contains(&Effect::WriteThread(thread)));
    assert!(!app.app().chat_view().available());
}

#[test]
fn focusing_a_comment_opens_its_thread_and_marks_its_bubble() {
    for focus in [
        |id| Action::FocusComment(Some(id)),
        |id| Action::RevealComment(id),
    ] {
        let mut app = panel_app([]);
        app.click((600.0, 500.0)).send_chat("one");
        let first = app.chat_thread_id().expect("first thread");
        let pin = app.document().annotations()[0].id.clone();
        app.send_chat("")
            .agent_says(specular_interact::Notice::Finished { text: "ok".into() });
        app.act(Action::Chat(ChatAction::NewThread));
        assert_ne!(app.chat_thread_id(), Some(first.clone()));
        app.act(focus(pin.clone()));
        assert_eq!(app.chat_thread_id(), Some(first));
        let bubble = &app.chat().transcript.expect("the thread is open").messages[0];
        assert_eq!(
            (
                bubble.annotation.as_ref(),
                bubble.focused,
                bubble.focus.is_some()
            ),
            (Some(&pin), true, true)
        );
    }
}

#[test]
fn resolve_comments_closes_the_threads_sent_comments_as_one_step() {
    let mut app = panel_app([]);
    app.click((600.0, 500.0)).send_chat("one");
    app.click((700.0, 500.0)).send_chat("two");
    assert!(
        app.chat().composer.open_comments.is_none(),
        "queued comments are not sent yet"
    );
    app.send_chat("")
        .agent_says(specular_interact::Notice::Finished { text: "ok".into() });
    let open = app
        .chat()
        .composer
        .open_comments
        .expect("two sent comments are open");
    assert_eq!(open.count, 2);
    app.act(open.resolve);
    assert!(app.chat().composer.open_comments.is_none());
    app.undo();
    assert_eq!(app.chat().composer.open_comments.map(|o| o.count), Some(2));
    app.redo();
    assert!(
        app.document()
            .annotations()
            .iter()
            .all(|a| a.status == specular_doc::AnnotationStatus::Resolved)
    );
}

#[test]
fn the_toolbar_has_the_panels_toggle_only_when_there_is_a_panel() {
    let mut app = TestApp::with_pages(1);
    assert!(toolbar(app.app()).chat.is_none());
    app.with_chat_panel();
    let button = toolbar(app.app()).chat.expect("the panel's toggle");
    assert_eq!(
        (button.label.as_ref(), button.open),
        ("Expand right panel", false)
    );
    app.act(button.action)
        .act(Action::Chat(ChatAction::Resize(5000.0)));
    let model = app.chat();
    assert_eq!((model.visible, model.width), (true, 960.0));
    app.act(Action::Chat(ChatAction::Resize(10.0)));
    assert_eq!(app.chat().width, 280.0);
    assert_eq!(toolbar(app.app()).chat.map(|b| b.open), Some(true));
}
