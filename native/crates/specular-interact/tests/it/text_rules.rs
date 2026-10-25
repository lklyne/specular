//! The text rules a scenario cannot pin to round numbers: where wrapped
//! lines break, how a word grows under a dragged double click, and the
//! input method's caret counted in UTF-16 units. The editor's keys, the
//! pointer and composition are in `fixtures/scenarios/text-editing.txt`.
//!
//! Text is measured 10 units a character and 20 a line. The sticky under
//! edit is 200 wide, so its text wraps at 18 characters and starts at
//! (108, 108).

use specular_core::ImeEvent;
use specular_doc::Rect;
use specular_interact::{Event, Key, TextEdit};
use specular_testkit::{CMD, SHIFT, TestApp, sticky};

const NOTE: Rect = Rect::new(100.0, 100.0, 200.0, 200.0);

/// A sticky reading `content`, double-clicked, with the caret put at the
/// start of its text.
fn editing(content: &str) -> TestApp {
    let mut app = TestApp::with_entities([sticky("n", NOTE, content)]);
    app.double_click((150.0, 250.0)).chord(CMD, Key::ArrowUp);
    app
}

/// The composition's range in the text being edited.
fn composition(app: &TestApp) -> Option<std::ops::Range<usize>> {
    app.app().text_edit().and_then(TextEdit::composition)
}

/// The middle of the character at `column` of the first line.
fn at(column: u8) -> (f32, f32) {
    (110.0 + f32::from(column) * 10.0, 115.0)
}

#[test]
fn line_keys_follow_the_wrapped_lines() {
    // 18 characters fit, so the lines are "aaaa bbbb cccc " and "dddd eeee".
    let mut app = editing("aaaa bbbb cccc dddd eeee");
    assert_eq!(app.chord(CMD, Key::ArrowRight).caret(), (14, 14));
    assert_eq!(app.key(Key::ArrowDown).caret(), (24, 24));
    assert_eq!(app.chord(CMD, Key::ArrowLeft).caret(), (15, 15));
    assert_eq!(app.key(Key::ArrowUp).caret(), (0, 0));
    assert_eq!(app.key(Key::ArrowDown).key(Key::End).caret(), (24, 24));
    assert_eq!(app.key(Key::ArrowUp).caret(), (9, 9));
    app.chord(CMD, Key::Delete);
    assert_eq!(
        app.editing_text(),
        "aaaa bbbb dddd eeee",
        "to the end of the wrapped line"
    );
}

#[test]
fn a_double_click_takes_a_word_and_a_drag_from_it_grows_by_words() {
    let mut app = editing("one two three four");
    app.double_click(at(5));
    assert_eq!(app.caret(), (7, 4));

    // A click, then the second press held and dragged.
    app.click(at(5)).send(press_again(at(5), 2)).drag_to(at(10));
    assert_eq!(app.caret(), (13, 4), "all of the word under the pointer");
    app.drag_to(at(1)).release();
    assert_eq!(app.caret(), (0, 7), "backwards, keeping the first word");

    // Shift on a double click still takes the word instead of extending.
    app.click(at(0)).hold(SHIFT).double_click(at(5)).let_go();
    assert_eq!(app.caret(), (7, 4));

    // Dragged back to exactly where the word starts: the word stays whole.
    app.double_click(at(5));
    app.click(at(5)).send(press_again(at(5), 2)).drag_to(at(4));
    assert_eq!(app.caret(), (7, 4));
}

#[test]
fn the_composition_caret_is_counted_in_utf16_units() {
    let mut app = editing("é");
    app.key(Key::End);
    app.send(Event::Ime(ImeEvent::SetComposition {
        text: "😀ab".to_owned(),
        selection: 2..3,
        replacement: None,
    }));
    // Past "é" (2 bytes), the emoji is 4 bytes and 2 units.
    assert_eq!(app.caret(), (7, 6));
    app.send(Event::Ime(ImeEvent::FinishComposing {
        keep_selection: false,
    }));
    assert_eq!((app.editing_text(), composition(&app)), ("é😀ab", None));
    assert_eq!(app.caret(), (8, 8));
}

fn press_again(at: (f32, f32), click_count: u8) -> specular_interact::Event {
    specular_interact::Event::Pointer(specular_interact::PointerInput {
        kind: specular_core::PointerEventKind::Down {
            button: specular_core::PointerButton::Left,
            click_count,
        },
        screen: at.into(),
        modifiers: specular_core::Modifiers::default(),
    })
}
