//! Starting and ending a text edit: what starts one, the one undo step it
//! leaves, the empty text that is deleted, and the rect that follows the
//! text. Text is measured 10 units a character and 20 a line.

use glam::Vec2;
use specular_doc::{Entity, Kind, Rect};
use specular_interact::{Action, Cursor, Effect, Key, TextEdit, Tool};
use specular_testkit::{
    CMD, SHIFT, TestApp, assert_doc_snapshot, connected, document, labelled, page, plain_text,
    shape, sticky,
};

const NOTE: Rect = Rect::new(100.0, 100.0, 200.0, 200.0);
const INSIDE: (f32, f32) = (150.0, 250.0);

/// The entity's text or label, or nothing for a kind that has none.
fn text_of(entity: &Entity) -> &str {
    match &entity.kind {
        Kind::Text(text) => &text.text,
        Kind::Shape(shape) => &shape.text,
        _ => "",
    }
}

fn edited(app: &TestApp) -> Option<&str> {
    app.session()
        .editing
        .as_ref()
        .map(|edit| edit.entity().as_str())
}

#[test]
fn a_double_click_edits_a_sticky_with_all_of_its_text_selected() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "hello")]);
    app.click(INSIDE);
    assert_eq!(edited(&app), None, "one click only selects");
    app.take_effects();
    app.double_click(INSIDE);
    assert_eq!(edited(&app), Some("n"));
    assert_eq!(app.caret(), (5, 0));
    assert_eq!(app.session().gesture, None);
    assert_eq!(
        app.take_effects(),
        [
            Effect::SetImeAllowed(true),
            // The caret, after the fifth character of the text 8 units in.
            Effect::SetImeCursorArea {
                origin: Vec2::new(158.0, 108.0),
                size: Vec2::new(0.0, 20.0),
            },
            Effect::SetCursor(Cursor::Text),
        ]
    );
    assert_eq!(app.app().editing_text(&"n".into()), Some("hello"));
    assert_eq!(app.app().editing_text(&"other".into()), None);
}

#[test]
fn a_double_click_with_shift_held_or_on_a_page_edits_nothing() {
    let mut app = TestApp::with_entities([
        page("p", Rect::new(400.0, 100.0, 400.0, 300.0)),
        sticky("n", NOTE, "hello"),
    ]);
    app.hold(SHIFT).double_click(INSIDE).let_go();
    assert_eq!(edited(&app), None);
    app.double_click((500.0, 200.0));
    assert_eq!(edited(&app), None);
    assert_eq!(app.selected(), Some("p"));
}

#[test]
fn escape_ends_the_edit_as_one_undo_step_and_keeps_the_selection() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "hello")]);
    app.double_click(INSIDE)
        .key(Key::End)
        .type_text(" there")
        .key(Key::Enter);
    app.type_text("friend");
    assert_eq!(text_of(app.entity("n")), "hello", "the document waits");
    assert!(!app.app().can_undo());
    app.take_effects();

    app.key(Key::Escape);
    assert_eq!(edited(&app), None);
    assert_eq!(app.selected(), Some("n"));
    assert_eq!(text_of(app.entity("n")), "hello there\nfriend");
    assert_eq!(
        app.take_effects(),
        [
            Effect::SetImeAllowed(false),
            Effect::Save,
            Effect::SetCursor(Cursor::Default)
        ]
    );
    app.undo();
    assert_eq!(text_of(app.entity("n")), "hello");
    assert!(!app.app().can_undo(), "the whole session was one step");
    app.redo().assert_undo_returns_to_start();

    app.key(Key::Escape);
    assert_eq!(app.selected(), None, "a second Escape deselects");
}

#[test]
fn an_edit_that_changes_nothing_leaves_no_step() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "hello")]);
    app.double_click(INSIDE)
        .key(Key::End)
        .type_text("!")
        .key(Key::Backspace);
    app.take_effects();
    app.key(Key::Escape);
    assert!(!app.app().can_undo());
    assert!(!app.take_effects().contains(&Effect::Save));
    assert_eq!(app.entity("n").rect, NOTE);
}

#[test]
fn pressing_anywhere_else_ends_the_edit_and_then_does_what_it_does() {
    let mut app = TestApp::with_entities([
        page("p", Rect::new(400.0, 100.0, 400.0, 300.0)),
        sticky("n", NOTE, "hello"),
    ]);
    app.double_click(INSIDE)
        .type_text("bye")
        .click((500.0, 200.0));
    assert_eq!(edited(&app), None);
    assert_eq!(app.selected(), Some("p"));
    assert_eq!(text_of(app.entity("n")), "bye");

    app.double_click(INSIDE)
        .type_text("gone")
        .click((900.0, 900.0));
    assert_eq!((edited(&app), app.selected()), (None, None));
    assert_eq!(text_of(app.entity("n")), "gone");
    app.assert_undo_returns_to_start();
}

#[test]
fn a_tool_change_a_verb_or_a_new_selection_ends_the_edit_first() {
    let mut app = TestApp::with_entities([
        sticky("n", NOTE, "a"),
        shape("s", NOTE.translated(400.0, 0.0)),
    ]);
    app.double_click(INSIDE).type_text("b").tool(Tool::Draw);
    assert_eq!((edited(&app), text_of(app.entity("n"))), (None, "b"));
    assert_eq!(app.session().tool, Tool::Draw);

    app.tool(Tool::Select)
        .double_click(INSIDE)
        .type_text("c")
        .select(&["s"]);
    assert_eq!((edited(&app), text_of(app.entity("n"))), (None, "c"));

    app.double_click(INSIDE)
        .type_text("d")
        .act(Action::Nudge { dx: 5.0, dy: 0.0 });
    assert_eq!(text_of(app.entity("n")), "d");
    assert_eq!(app.rect("n").x, 105.0);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_text_left_empty_is_deleted_with_its_edges() {
    let entities = [
        sticky("n", NOTE, "hello"),
        shape("s", NOTE.translated(400.0, 0.0)),
    ];
    let mut app = TestApp::from_document(connected(document(entities), "e1", "n", "s"));
    app.double_click(INSIDE)
        .key(Key::Backspace)
        .type_text("  ")
        .key(Key::Escape);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"s","type":"shape","x":500,"y":100,"width":200,"height":200,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["s"]}
    "#);
    assert_eq!(app.selected(), None);
    app.undo();
    assert!(!app.app().can_undo(), "one step took the text and its edge");
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn a_shape_label_is_edited_the_same_way_and_an_empty_one_keeps_its_shape() {
    let mut app = TestApp::with_entities([labelled("s", NOTE, "old")]);
    app.double_click((200.0, 200.0))
        .type_text("New label")
        .key(Key::Escape);
    assert_eq!(text_of(app.entity("s")), "New label");
    assert_eq!(app.rect("s"), NOTE);

    app.double_click((200.0, 200.0))
        .key(Key::Backspace)
        .key(Key::Escape);
    assert_eq!(text_of(app.entity("s")), "");
    app.assert_undo_returns_to_start();
}

#[test]
fn a_text_placed_and_typed_into_is_one_step_that_undoes_to_nothing() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddSticky).click((100.0, 100.0));
    assert!(!app.app().can_undo(), "the placement is not a step yet");
    app.take_effects();
    app.type_text("note").key(Key::Escape);
    let id = app.selected().map(str::to_owned).unwrap_or_default();
    assert_eq!(text_of(app.entity(&id)), "note");
    assert!(app.take_effects().contains(&Effect::Save));
    app.undo();
    assert_eq!(app.document().entities().count(), 0);
    assert!(!app.app().can_undo());
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn a_text_placed_and_left_empty_leaves_no_entity_and_no_step() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddText).click((100.0, 100.0)).take_effects();
    assert_eq!(app.document().entities().count(), 1);
    app.type_text("x").key(Key::Backspace).click((600.0, 600.0));
    assert_eq!(app.document().entities().count(), 0);
    assert_eq!((app.selected(), edited(&app)), (None, None));
    assert!(!app.app().can_undo() && !app.app().can_redo());
    assert!(!app.take_effects().contains(&Effect::Save));
}

#[test]
fn a_sticky_grows_downward_with_its_text_and_undo_shrinks_it_back() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "top")]);
    app.double_click(INSIDE).key(Key::End);
    for _ in 0..8 {
        app.key(Key::Enter);
    }
    assert_eq!(
        app.rect("n"),
        NOTE,
        "nine lines and the padding still fit in 200"
    );
    app.key(Key::Enter).key(Key::Enter);
    // Eleven lines of 20, with 8 above and below.
    assert_eq!(app.rect("n"), Rect::new(100.0, 100.0, 200.0, 236.0));
    assert!(!app.app().can_undo(), "the rect is live, as in a drag");
    app.chord(CMD, Key::Char('z'));
    assert_eq!(
        app.rect("n").height,
        216.0,
        "the editor's undo shrinks it too"
    );

    app.type_text("end").key(Key::Escape);
    assert_eq!(app.rect("n").height, 216.0);
    app.undo();
    assert_eq!((app.rect("n"), text_of(app.entity("n"))), (NOTE, "top"));
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn plain_text_hugs_its_lines_and_an_empty_one_fits_its_prompt() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddText).click((100.0, 100.0));
    let id = app.selected().map(str::to_owned).unwrap_or_default();
    // "Add text" is 80 wide, plus the 8 kept clear for the caret.
    assert_eq!(app.rect(&id), Rect::new(100.0, 100.0, 88.0, 20.0));
    app.type_text("hello world");
    assert_eq!(app.rect(&id), Rect::new(100.0, 100.0, 118.0, 20.0));
    app.type_text("\nmore");
    assert_eq!(app.rect(&id), Rect::new(100.0, 100.0, 118.0, 40.0));
    app.chord(CMD, Key::Char('a')).type_text("hi");
    assert_eq!(
        app.rect(&id),
        Rect::new(100.0, 100.0, 64.0, 20.0),
        "never under 64 wide"
    );
    app.key(Key::Escape).assert_undo_returns_to_start();

    let mut app = TestApp::with_entities([plain_text("t", Rect::new(0.0, 0.0, 64.0, 20.0), "hi")]);
    app.double_click((10.0, 10.0))
        .key(Key::End)
        .type_text(" there, you");
    assert_eq!(app.rect("t"), Rect::new(0.0, 0.0, 138.0, 20.0));
    app.key(Key::Escape).assert_undo_returns_to_start();
}

#[test]
fn opening_another_document_drops_the_edit() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "hello")]);
    app.double_click(INSIDE).type_text("lost");
    app.open(document([sticky("n", NOTE, "theirs")]));
    assert_eq!(edited(&app), None);
    assert_eq!(text_of(app.entity("n")), "theirs");
    assert!(!app.app().can_undo());
}

#[test]
fn the_accessors_give_the_caret_and_the_selection_in_canvas_space() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "one two\nthree")]);
    assert_eq!(app.app().caret_rect(), None);
    assert_eq!(app.app().selection_rects(), []);
    app.double_click(INSIDE);
    // Line 1 with a little extra for the line break it takes, then line 2.
    assert_eq!(
        app.app().selection_rects(),
        [
            Rect::new(108.0, 108.0, 76.0, 20.0),
            Rect::new(108.0, 128.0, 50.0, 20.0)
        ]
    );
    assert_eq!(
        app.app().caret_rect(),
        Some(Rect::new(158.0, 128.0, 0.0, 20.0))
    );
    app.key(Key::ArrowLeft);
    assert_eq!(app.app().selection_rects(), []);
    assert_eq!(
        app.app().caret_rect(),
        Some(Rect::new(108.0, 108.0, 0.0, 20.0))
    );
    assert_eq!(
        app.app().text_edit().map(TextEdit::text),
        Some("one two\nthree")
    );

    // A label is centred both ways in its box: 176 by 184 after padding.
    let mut app = TestApp::with_entities([labelled("s", NOTE, "ab")]);
    app.double_click((200.0, 200.0));
    assert_eq!(
        app.app().selection_rects(),
        [Rect::new(190.0, 190.0, 20.0, 20.0)]
    );
    assert_eq!(
        app.app().caret_rect(),
        Some(Rect::new(210.0, 190.0, 0.0, 20.0))
    );
}
