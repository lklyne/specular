//! Input-method composition in the text editor. The sticky's text starts at
//! (108, 108), 10 units a character and 20 a line.

use glam::Vec2;
use specular_core::ImeEvent;
use specular_doc::Rect;
use specular_interact::{Effect, Event, Key, TextEdit};
use specular_testkit::{CMD, TestApp, sticky};

const NOTE: Rect = Rect::new(100.0, 100.0, 200.0, 200.0);

/// A sticky reading `content`, being edited with the caret at its end.
fn editing(content: &str) -> TestApp {
    let mut app = TestApp::with_entities([sticky("n", NOTE, content)]);
    app.double_click((150.0, 250.0)).key(Key::End);
    app.take_effects();
    app
}

fn composition(app: &TestApp) -> Option<std::ops::Range<usize>> {
    app.app().text_edit().and_then(TextEdit::composition)
}

#[test]
fn a_composition_sits_in_the_text_marked_until_it_is_committed() {
    let mut app = editing("ab");
    app.compose("に");
    assert_eq!(
        (app.editing_text(), composition(&app)),
        ("abに", Some(2..5))
    );
    assert_eq!(app.caret(), (5, 5));
    app.compose("にほ");
    assert_eq!(
        (app.editing_text(), composition(&app)),
        ("abにほ", Some(2..8))
    );
    assert_eq!(
        app.app().composition_rects(),
        [Rect::new(128.0, 108.0, 20.0, 20.0)]
    );
    // The candidate window goes beside the caret.
    assert_eq!(
        app.take_effects().last(),
        Some(&Effect::SetImeCursorArea {
            origin: Vec2::new(148.0, 108.0),
            size: Vec2::new(0.0, 20.0),
        })
    );

    app.commit("日本");
    assert_eq!((app.editing_text(), composition(&app)), ("ab日本", None));
    assert_eq!(app.caret(), (8, 8));
    assert_eq!(app.app().composition_rects(), []);

    app.chord(CMD, Key::Char('z'));
    assert_eq!(
        app.editing_text(),
        "ab",
        "the whole composition was one step"
    );
}

#[test]
fn a_cancelled_composition_leaves_the_text_and_the_undo_as_they_were() {
    let mut app = editing("ab");
    app.type_text("c").compose("x").compose("xy");
    app.send(Event::Ime(ImeEvent::Cancel));
    assert_eq!((app.editing_text(), composition(&app)), ("abc", None));
    app.chord(CMD, Key::Char('z'));
    assert_eq!(
        app.editing_text(),
        "ab",
        "undo reaches the typing, with no step between"
    );
}

#[test]
fn the_preedit_is_cleared_before_a_commit_as_macos_sends_it() {
    let mut app = editing("ab");
    app.compose("に")
        .send(Event::Ime(ImeEvent::Cancel))
        .commit("日");
    assert_eq!(app.editing_text(), "ab日");
    app.chord(CMD, Key::Char('z'));
    assert_eq!(app.editing_text(), "ab");
    app.chord(CMD, Key::Char('z'));
    assert_eq!(app.editing_text(), "ab", "one step, not two");
}

#[test]
fn a_composition_replaces_the_selection_and_owns_the_keys_while_it_runs() {
    let mut app = editing("hello");
    app.chord(CMD, Key::Char('a')).compose("k");
    assert_eq!((app.editing_text(), composition(&app)), ("k", Some(0..1)));
    app.key(Key::ArrowLeft).key(Key::Backspace).type_text("z");
    assert_eq!((app.editing_text(), app.caret()), ("k", (1, 1)));
    app.commit("か");
    assert_eq!(app.editing_text(), "か");
}

#[test]
fn the_composition_caret_is_counted_in_utf16_units() {
    let mut app = editing("é");
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
