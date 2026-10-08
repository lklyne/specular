//! Editing a Document: the markdown source in place, written to its file as
//! it changes, one undo step a session, and no text lost when the file
//! changes under the edit.
//!
//! Text is measured by the testkit's `FixedAdvance`: 10 units a character.
//! A Document's rows are 21 apart, its body line height.

use specular_doc::{Document, EntityId, Kind, Rect};
use specular_interact::{Effect, Event, Key, NoteNotice, NoteState, Tool};
use specular_testkit::{ALT, CMD, CMD_SHIFT, SHIFT, TestApp, document, note, sticky};

const BOX: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);
const FILE: &str = "plan.md";
/// The canvas point of the first character: the box's corner and 12 of
/// padding.
const TEXT: (f32, f32) = (112.0, 112.0);

fn opened(document: Document) -> TestApp {
    let mut app = TestApp::empty();
    app.open(document);
    app
}

/// A Document holding `source`, not yet edited.
fn shown(source: &str) -> TestApp {
    let mut app = opened(document([note("n", BOX, FILE)]));
    app.tick(1_000).note_text(FILE, source);
    app.take_effects();
    app
}

/// A Document holding `source`, double-clicked on its first character.
fn editing(source: &str) -> TestApp {
    let mut app = shown(source);
    app.double_click((TEXT.0 + 1.0, TEXT.1 + 5.0));
    app.take_effects();
    app
}

/// The note writes in `effects`, as `(file, text)`.
fn writes(effects: &[Effect]) -> Vec<(String, String)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::WriteNote { file, text } => Some((file.clone(), text.clone())),
            _ => None,
        })
        .collect()
}

fn write(text: &str) -> Vec<(String, String)> {
    vec![(FILE.to_owned(), text.to_owned())]
}

fn read(app: &TestApp, file: &str) -> Option<String> {
    match app.app().note(file) {
        Some(NoteState::Ready(text)) => Some(text.to_string()),
        _ => None,
    }
}

fn scroll(app: &TestApp) -> f32 {
    app.app().note_scroll(&EntityId::from("n"))
}

fn lines(count: usize) -> String {
    (0..count)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_double_click_edits_the_source_with_the_caret_where_it_landed() {
    let mut app = shown("# Plan\n\nbody text");
    // The fourth character of the third row, under a heading row 1.4 times
    // as tall as a body row.
    app.double_click((TEXT.0 + 42.0, TEXT.1 + 21.0 * 2.4 + 5.0));
    assert_eq!(app.editing_text(), "# Plan\n\nbody text");
    assert_eq!(app.caret(), (12, 12));
    assert_eq!(app.selected(), Some("n"));
    app.type_text("!");
    assert_eq!(app.editing_text(), "# Plan\n\nbody! text");
}

#[test]
fn the_file_is_written_a_third_of_a_second_after_the_last_change() {
    let mut app = editing("one");
    app.type_text("a").tick(1_200).type_text("b").tick(1_400);
    assert_eq!(writes(&app.take_effects()), []);
    app.tick(1_549);
    assert_eq!(writes(&app.take_effects()), []);
    app.tick(1_550);
    assert_eq!(writes(&app.take_effects()), write("abone"));
    assert_eq!(read(&app, FILE).as_deref(), Some("abone"));
    app.tick(5_000);
    assert_eq!(writes(&app.take_effects()), [], "written once");
}

#[test]
fn ending_the_edit_writes_what_is_unsaved_and_is_one_undo_step() {
    let mut app = editing("one");
    app.type_text("a").type_text("b").key(Key::Escape);
    let effects = app.take_effects();
    assert_eq!(writes(&effects), write("abone"));
    assert!(effects.contains(&Effect::Save));
    assert!(app.app().text_edit().is_none());
    assert_eq!(app.document().note(FILE), Some("abone"));

    app.undo();
    assert_eq!(writes(&app.take_effects()), write("one"));
    assert_eq!(read(&app, FILE).as_deref(), Some("one"));
    assert!(!app.app().can_undo(), "one step for the whole session");
    app.redo();
    assert_eq!(writes(&app.take_effects()), write("abone"));
    assert_eq!(read(&app, FILE).as_deref(), Some("abone"));
}

#[test]
fn an_edit_that_changes_nothing_is_no_step_and_no_write() {
    let mut app = editing("one");
    app.type_text("a")
        .key(Key::Backspace)
        .key(Key::ArrowRight)
        .tick(9_000)
        .key(Key::Escape);
    // Typed and deleted again: the file is told, the history is not.
    assert_eq!(writes(&app.take_effects()), write("one"));
    assert!(!app.app().can_undo());
}

#[test]
fn undo_inside_the_edit_is_the_editors_own() {
    let mut app = editing("one");
    app.type_text("ab").chord(CMD, Key::Char('z'));
    assert_eq!(app.editing_text(), "one");
    app.chord(CMD_SHIFT, Key::Char('z'));
    assert_eq!(app.editing_text(), "abone");
    assert!(!app.app().can_undo());
}

#[test]
fn a_change_on_disk_while_not_editing_reloads_and_undo_goes_back_from_it() {
    let mut app = editing("one");
    app.type_text("a").key(Key::Escape).take_effects();
    app.note_text(FILE, "theirs");
    assert_eq!(read(&app, FILE).as_deref(), Some("theirs"));
    assert_eq!(writes(&app.take_effects()), []);
    // Undo puts back the text the edit started from, and redo the text the
    // undo replaced: the one from outside.
    app.undo();
    assert_eq!(writes(&app.take_effects()), write("one"));
    app.redo();
    assert_eq!(writes(&app.take_effects()), write("theirs"));
}

#[test]
fn a_change_on_disk_while_editing_keeps_both_texts() {
    let mut app = editing("one");
    app.type_text("mine ").note_text(FILE, "theirs");
    let written = writes(&app.take_effects());
    let [(copy, copied), ours] = written.as_slice() else {
        panic!("a copy and then the file: {written:?}");
    };
    assert!(
        copy.starts_with("plan (conflict ") && copy.ends_with(").md"),
        "{copy}"
    );
    assert_eq!(copied, "theirs");
    assert_eq!(*ours, (FILE.to_owned(), "mine one".to_owned()));
    assert_eq!(app.editing_text(), "mine one", "the edit goes on");

    // The copy has a Document of its own beside the edited one.
    let beside = (app.document().entities())
        .find(|entity| entity.id != EntityId::from("n"))
        .expect("a second Document");
    assert_eq!(beside.rect, Rect::new(520.0, 100.0, 400.0, 300.0));
    assert!(matches!(&beside.kind, Kind::File(file) if file.file == *copy));
    app.tick(9_000);
    assert_eq!(writes(&app.take_effects()), [], "nothing left to write");
}

#[test]
fn the_same_text_from_disk_is_not_a_conflict() {
    let mut app = editing("one");
    app.type_text("a").tick(2_000);
    assert_eq!(writes(&app.take_effects()), write("aone"));
    // The watcher reads our own write back, and an editor saves the same.
    app.note_text(FILE, "aone")
        .type_text("b")
        .note_text(FILE, "aone");
    assert_eq!(writes(&app.take_effects()), []);
    assert_eq!(app.document().entities().count(), 1);
    assert_eq!(app.editing_text(), "abone");
}

#[test]
fn a_refused_write_keeps_both_texts_with_no_edit_open() {
    let mut app = shown("one");
    app.send(Event::Note {
        file: FILE.to_owned(),
        notice: NoteNotice::Refused {
            disk: "theirs".to_owned(),
            ours: "ours".to_owned(),
        },
    });
    let written = writes(&app.take_effects());
    assert_eq!(written.len(), 2);
    assert_eq!(written[0].1, "theirs");
    assert_eq!(written[1], (FILE.to_owned(), "ours".to_owned()));
    assert_eq!(app.document().entities().count(), 2);
    assert_eq!(read(&app, FILE).as_deref(), Some("ours"));
}

#[test]
fn the_document_tool_asks_for_a_file_then_places_and_edits_it() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddDocument).click((205.0, 95.0));
    let rect = Rect::new(200.0, 100.0, 300.0, 300.0);
    assert!(app.take_effects().contains(&Effect::CreateNote { rect }));
    assert_eq!(app.session().tool, Tool::Select);
    assert_eq!(app.document().entities().count(), 0);

    let file = "Untitled Note.md".to_owned();
    app.send(Event::NoteCreated {
        file: file.clone(),
        rect,
    });
    let effects = app.take_effects();
    assert!(effects.contains(&Effect::LoadNote { file: file.clone() }));
    assert!(effects.contains(&Effect::Save));
    let placed = app.document().entities().next().expect("the Document");
    assert_eq!(placed.rect, rect);
    assert!(matches!(&placed.kind, Kind::File(shown) if shown.file == file));
    assert_eq!(app.selected_ids().len(), 1);
    assert_eq!(app.editing_text(), "");

    // The shell's first read of the empty file changes nothing.
    app.type_text("# Hi").note_text(&file, "").key(Key::Escape);
    assert_eq!(
        writes(&app.take_effects()),
        [(file.clone(), "# Hi".to_owned())]
    );
    assert_eq!(app.document().entities().count(), 1);
    app.undo().undo();
    assert_eq!(app.document().entities().count(), 0);
    assert!(!app.app().can_undo());
}

#[test]
fn the_wheel_stops_at_the_end_once_the_renderer_reports_the_height() {
    let mut app = shown(&lines(30));
    app.click((300.0, 250.0)).pointer_move((300.0, 250.0));
    app.wheel((0.0, -500.0));
    assert_eq!(scroll(&app), 500.0, "no height known yet");
    // 400 of rows in a window of 300 less the padding: 124 is the end.
    app.send(Event::NoteHeights(vec![(EntityId::from("n"), 400.0)]));
    assert_eq!(scroll(&app), 124.0);
    app.wheel((0.0, -500.0));
    assert_eq!(scroll(&app), 124.0);
    app.wheel((0.0, 24.0));
    assert_eq!(scroll(&app), 100.0, "no dead travel on the way back");
}

#[test]
fn the_caret_is_kept_in_view_and_page_keys_move_by_the_window() {
    let mut app = editing(&lines(30));
    app.chord(CMD, Key::ArrowDown);
    // The last of 30 rows 21 apart ends at 629, in a window 276 tall.
    assert_eq!(scroll(&app), 353.0);
    app.key(Key::PageUp);
    assert_eq!(app.caret().0, lines(15).len() + 1 + "line 15".len());
    assert_eq!(scroll(&app), 315.0, "the caret's row is at the top");
    app.key(Key::PageDown).key(Key::PageDown);
    assert_eq!(app.caret().0, lines(30).len());
    app.chord(CMD, Key::ArrowUp);
    assert_eq!(scroll(&app), 0.0);
    // While edited, the wheel stops at the end of the source's own layout.
    app.pointer_move((300.0, 250.0)).wheel((0.0, -5_000.0));
    assert_eq!(scroll(&app), 353.0);
}

#[test]
fn a_document_takes_every_format_and_tab_types_an_indent() {
    let mut app = editing("word here");
    app.hold(SHIFT).chord(ALT, Key::ArrowRight).let_go();
    app.chord(CMD, Key::Char('e'));
    assert_eq!(app.editing_text(), "`word` here");
    app.chord(CMD, Key::Char('b'));
    assert_eq!(app.editing_text(), "`**word**` here");
    app.chord(CMD_SHIFT, Key::Char('7'));
    assert_eq!(app.editing_text(), "1. `**word**` here");
    app.chord(CMD_SHIFT, Key::Char('9'));
    assert_eq!(app.editing_text(), "- [ ] `**word**` here");
    let cmd_alt = specular_core::Modifiers { alt: true, ..CMD };
    app.chord(CMD_SHIFT, Key::Char('9'))
        .chord(cmd_alt, Key::Char('2'));
    assert_eq!(app.editing_text(), "## `**word**` here");
    app.chord(cmd_alt, Key::Char('0'));
    assert_eq!(app.editing_text(), "`**word**` here");
    app.chord(CMD, Key::ArrowUp).key(Key::Tab);
    assert_eq!(app.editing_text(), "  `**word**` here");
}

#[test]
fn a_sticky_takes_bold_italic_strike_and_bullets_only() {
    let rect = Rect::new(100.0, 100.0, 200.0, 200.0);
    let mut app = TestApp::with_entities([sticky("s", rect, "word")]);
    app.double_click((150.0, 150.0));
    app.chord(CMD, Key::Char('e'))
        .chord(CMD_SHIFT, Key::Char('7'));
    assert_eq!(app.editing_text(), "word");
    app.chord(CMD, Key::Char('b'));
    assert_eq!(app.editing_text(), "**word**");
    app.chord(CMD, Key::Char('i'))
        .chord(CMD_SHIFT, Key::Char('x'));
    assert_eq!(app.editing_text(), "***~~word~~***");
    app.chord(CMD_SHIFT, Key::Char('8'));
    assert_eq!(app.editing_text(), "- ***~~word~~***");
    app.key(Key::Tab);
    assert_eq!(app.editing_text(), "  - ***~~word~~***", "Tab only nests");
}
