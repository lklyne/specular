//! The pointer in the text being edited. The sticky's text starts at
//! (108, 108), 10 units a character and 20 a line.

use specular_doc::Rect;
use specular_testkit::{SHIFT, TestApp, sticky};

const NOTE: Rect = Rect::new(100.0, 100.0, 200.0, 200.0);

/// The point over character `column` of line `line`, a little into it.
fn at(column: u8, line: u8) -> (f32, f32) {
    (
        110.0 + f32::from(column) * 10.0,
        115.0 + f32::from(line) * 20.0,
    )
}

fn editing(content: &str) -> TestApp {
    let mut app = TestApp::with_entities([sticky("n", NOTE, content)]);
    app.double_click((150.0, 250.0));
    app
}

#[test]
fn a_click_puts_the_caret_at_the_nearest_boundary() {
    let mut app = editing("hello world\nfoo");
    assert_eq!(app.click(at(3, 0)).caret(), (3, 3));
    assert_eq!(
        app.click((146.0, 115.0)).caret(),
        (4, 4),
        "past a glyph's middle"
    );
    assert_eq!(
        app.click(at(9, 1)).caret(),
        (15, 15),
        "past the end of a line"
    );
    assert_eq!(
        app.click((110.0, 290.0)).caret(),
        (12, 12),
        "below the last line"
    );
    assert!(app.session().editing.is_some());
    assert_eq!(app.session().gesture, None);
}

#[test]
fn a_drag_selects_from_the_press_to_the_pointer() {
    let mut app = editing("hello world\nfoo");
    app.press(at(2, 0)).drag_to(at(7, 0));
    assert_eq!(app.caret(), (7, 2));
    app.drag_to(at(1, 1));
    assert_eq!(app.caret(), (13, 2), "onto the next line");
    app.drag_to(at(0, 0)).release();
    assert_eq!(app.caret(), (0, 2), "and back past the press");
    assert_eq!(app.session().gesture, None);
    assert_eq!(app.rect("n"), NOTE, "the drag did not move the sticky");

    app.type_text("X");
    assert_eq!(app.editing_text(), "Xllo world\nfoo");
}

#[test]
fn a_shift_click_extends_the_selection_from_its_anchor() {
    let mut app = editing("hello world");
    app.click(at(2, 0)).hold(SHIFT).click(at(8, 0));
    assert_eq!(app.caret(), (8, 2));
    app.click(at(0, 0)).let_go();
    assert_eq!(app.caret(), (0, 2));
}

#[test]
fn a_double_click_takes_a_word_and_a_drag_from_it_grows_by_words() {
    let mut app = editing("one two three four");
    app.double_click(at(5, 0));
    assert_eq!(app.caret(), (7, 4));

    // A click, then the second press held and dragged.
    app.click(at(5, 0))
        .send(press_again(at(5, 0), 2))
        .drag_to(at(10, 0));
    assert_eq!(app.caret(), (13, 4), "all of the word under the pointer");
    app.drag_to(at(1, 0)).release();
    assert_eq!(app.caret(), (0, 7), "backwards, keeping the first word");
}

#[test]
fn a_triple_click_takes_the_paragraph() {
    let mut app = editing("one\ntwo words\nthree");
    app.triple_click(at(5, 1));
    assert_eq!(app.caret(), (13, 4));
    app.type_text("2");
    assert_eq!(app.editing_text(), "one\n2\nthree");
}

/// A left press with a click count, with no release after it.
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
