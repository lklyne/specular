//! Scene snapshots of text being edited in place: the working text, the
//! selection behind it, the caret and its blink, the input method's
//! underline, and the handles that go away. Text is measured by the
//! testkit's `FixedAdvance`: 10 units a character, 20 a line.

use specular_doc::Rect;
use specular_interact::Key;
use specular_testkit::{TestApp, assert_scene_snapshot, labelled, plain_text, sticky};

const NOTE: Rect = Rect::new(100.0, 100.0, 200.0, 200.0);

/// A sticky reading "ab" with the caret after the "b".
fn editing_note() -> TestApp {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "ab")]);
    app.double_click((150.0, 250.0)).key(Key::ArrowRight);
    app
}

#[test]
fn a_selected_sticky_has_handles_and_no_caret() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "ab")]);
    app.click((150.0, 250.0));
    assert_scene_snapshot!(app);
}

#[test]
fn an_edited_sticky_draws_the_working_text_and_a_caret_and_no_handles() {
    let mut app = editing_note();
    app.type_text("cd");
    assert_scene_snapshot!(app);
}

#[test]
fn selected_text_is_a_rect_behind_the_glyphs_and_hides_the_caret() {
    let mut app = TestApp::with_entities([sticky("n", NOTE, "ab\ncd")]);
    // Starting an edit selects everything: one rect a line.
    app.double_click((150.0, 250.0));
    assert_scene_snapshot!(app);
}

#[test]
fn the_caret_blinks_off_after_half_a_second_and_typing_brings_it_back() {
    let mut app = editing_note();
    let carets = |app: &TestApp| {
        app.scene_snapshot()
            .matches("screen rect 128,108 1x20")
            .count()
    };
    assert_eq!(carets(&app), 1, "{}", app.scene_snapshot());
    app.tick(499);
    assert_eq!(carets(&app), 1, "still in the shown half");
    app.tick(500);
    assert_eq!(carets(&app), 0, "the hidden half");
    app.tick(1000);
    assert_eq!(carets(&app), 1, "shown again");
    app.tick(1700).type_text("c");
    assert!(app.app().caret_visible(), "typing restarts the blink");
    app.tick(2199);
    assert!(app.app().caret_visible());
    app.tick(2200);
    assert!(!app.app().caret_visible());
}

#[test]
fn a_composition_is_underlined_under_its_glyphs() {
    let mut app = editing_note();
    app.compose("にほ");
    assert_scene_snapshot!(app);
}

#[test]
fn the_caret_and_underline_keep_a_pixel_when_zoomed_out() {
    let mut app = editing_note();
    app.compose("に").zoom(0.25);
    assert_scene_snapshot!(app);
}

#[test]
fn an_empty_plain_text_shows_its_prompt_behind_the_caret() {
    let mut app =
        TestApp::with_entities([plain_text("t", Rect::new(100.0, 100.0, 64.0, 20.0), "x")]);
    // The middle, clear of the handles the first click brings up.
    app.double_click((132.0, 110.0)).key(Key::Backspace);
    assert_scene_snapshot!(app);
}

#[test]
fn a_shape_label_is_edited_centred_in_its_box() {
    let mut app = TestApp::with_entities([labelled("s", NOTE, "ab")]);
    app.double_click((200.0, 200.0))
        .key(Key::ArrowRight)
        .type_text("c");
    assert_scene_snapshot!(app);
}
