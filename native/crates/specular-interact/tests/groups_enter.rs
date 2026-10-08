//! Stepping into a group by double click, and renaming one by double
//! clicking its title. A double click on a group's interior selects what is
//! directly inside it, one level at a time, and enters it; Escape steps out
//! one level before it deselects.

use specular_doc::{Entity, ItemId, Rect};
use specular_interact::{Action, Cursor, Effect, Key};
use specular_testkit::{CMD, SHIFT, TestApp, group, inside, shape};

fn named(group: Entity, label: &str) -> Entity {
    Entity {
        label: Some(label.to_owned()),
        ..group
    }
}

/// `outer` holds `inner` (which holds `a` and `b`) and `c`.
fn nested() -> TestApp {
    TestApp::with_entities([
        inside("inner", shape("a", Rect::new(220.0, 220.0, 100.0, 100.0))),
        inside("inner", shape("b", Rect::new(360.0, 220.0, 100.0, 100.0))),
        inside(
            "outer",
            named(
                group("inner", Rect::new(200.0, 200.0, 300.0, 160.0)),
                "Inner",
            ),
        ),
        inside("outer", shape("c", Rect::new(560.0, 220.0, 100.0, 100.0))),
        named(
            group("outer", Rect::new(100.0, 100.0, 700.0, 400.0)),
            "Outer",
        ),
    ])
}

// Stepping in.

#[test]
fn a_double_click_on_a_group_selects_what_is_directly_inside() {
    let mut app = nested();
    app.double_click((150.0, 450.0));
    assert_eq!(app.selected_ids(), ["inner", "c"]);
    app.double_click((350.0, 340.0));
    assert_eq!(app.selected_ids(), ["a", "b"], "one level at a time");
    assert!(!app.app().can_undo());
}

#[test]
fn a_click_on_a_member_selects_it_without_selecting_its_group() {
    let mut app = nested();
    app.click((270.0, 270.0));
    assert_eq!(app.selected_ids(), ["a"]);
}

#[test]
fn a_click_on_a_groups_interior_selects_the_group() {
    let mut app = nested();
    app.click((150.0, 450.0));
    assert_eq!(app.selected_ids(), ["outer"]);
}

#[test]
fn a_double_click_on_an_empty_group_leaves_it_selected() {
    let mut app = TestApp::with_entities([named(
        group("g", Rect::new(100.0, 100.0, 300.0, 200.0)),
        "Empty",
    )]);
    app.double_click((250.0, 200.0));
    assert_eq!(app.selected_ids(), ["g"]);
}

#[test]
fn children_selected_by_stepping_in_move_together() {
    let mut app = nested();
    app.double_click((350.0, 340.0));
    app.double_click((350.0, 340.0));
    app.press((270.0, 270.0)).drag_to((270.0, 370.0)).release();
    assert_eq!(app.rect("a"), Rect::new(220.0, 320.0, 100.0, 100.0));
    assert_eq!(app.rect("b"), Rect::new(360.0, 320.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

fn entered(app: &TestApp) -> Option<&str> {
    app.app()
        .entered_group()
        .map(specular_doc::EntityId::as_str)
}

#[test]
fn a_double_click_enters_the_group_and_clicks_on_its_members_keep_it() {
    let mut app = nested();
    app.double_click((150.0, 450.0));
    assert_eq!(entered(&app), Some("outer"));
    app.click((580.0, 260.0));
    assert_eq!(app.selected_ids(), ["c"]);
    assert_eq!(entered(&app), Some("outer"), "a click on a member keeps it");
    app.hold(SHIFT).click((250.0, 250.0)).let_go();
    assert_eq!(app.selected_ids(), ["c", "a"]);
    assert_eq!(entered(&app), Some("outer"));
    app.drag((700.0, 340.0), (600.0, 300.0));
    assert_eq!(app.selected_ids(), ["c"]);
    assert_eq!(entered(&app), Some("outer"), "a marquee inside keeps it");
}

#[test]
fn escape_steps_out_one_level_at_a_time_and_then_deselects() {
    let mut app = nested();
    app.double_click((150.0, 450.0))
        .double_click((350.0, 340.0));
    assert_eq!(entered(&app), Some("inner"));
    assert_eq!(app.selected_ids(), ["a", "b"]);
    app.key(Key::Escape);
    assert_eq!(app.selected_ids(), ["inner"]);
    assert_eq!(entered(&app), Some("outer"));
    app.key(Key::Escape);
    assert_eq!(app.selected_ids(), ["outer"]);
    assert_eq!(entered(&app), None);
    app.key(Key::Escape);
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
    assert!(!app.app().can_undo());
}

#[test]
fn a_drag_or_an_edit_is_backed_out_of_before_the_group() {
    let mut app = nested();
    app.double_click((150.0, 450.0));
    app.press((580.0, 260.0))
        .drag_to((580.0, 360.0))
        .key(Key::Escape);
    assert_eq!(entered(&app), Some("outer"));
    app.release();
    assert_eq!(app.rect("c"), Rect::new(560.0, 220.0, 100.0, 100.0));
    app.key(Key::Escape);
    assert_eq!(entered(&app), None);
}

#[test]
fn a_click_outside_the_group_leaves_it() {
    let mut app = nested();
    app.double_click((150.0, 450.0));
    app.click((900.0, 900.0));
    assert_eq!(entered(&app), None);

    let mut app = nested();
    app.double_click((150.0, 450.0));
    app.click((150.0, 450.0));
    assert_eq!(app.selected_ids(), ["outer"]);
    assert_eq!(entered(&app), None, "the selection is the group itself");
}

#[test]
fn a_group_that_goes_away_is_no_longer_entered() {
    let mut app = nested();
    app.double_click((150.0, 450.0));
    app.act(Action::Select(vec![ItemId::Entity("outer".into())]));
    app.key(Key::Backspace);
    assert_eq!(entered(&app), None);

    let mut app = TestApp::with_entities([
        inside("g", shape("a", Rect::new(140.0, 140.0, 100.0, 100.0))),
        named(group("g", Rect::new(100.0, 100.0, 400.0, 300.0)), "G"),
    ]);
    app.select(&["a", "g"]).act(Action::Group);
    let outer = app.selected().expect("a group").to_owned();
    app.double_click((88.0, 300.0));
    assert_eq!(entered(&app), Some(outer.as_str()));
    app.undo();
    assert_eq!(entered(&app), None);
}

#[test]
fn opening_a_document_clears_it() {
    let mut app = nested();
    app.double_click((150.0, 450.0));
    app.open(specular_doc::Document::new());
    assert_eq!(entered(&app), None);
}

#[test]
fn a_marquee_inside_a_group_picks_its_members_one_by_one() {
    let mut app = nested();
    app.drag((230.0, 230.0), (300.0, 300.0));
    assert_eq!(app.selected_ids(), ["a"]);
}

#[test]
fn command_click_on_a_group_marquees_instead_of_stepping_in() {
    let mut app = nested();
    app.hold(CMD).double_click((150.0, 450.0)).let_go();
    assert_eq!(app.selected_ids(), ["outer"]);
}

// Renaming.

const TITLE: (f32, f32) = (110.0, 90.0);

fn label(app: &TestApp, id: &str) -> Option<String> {
    app.entity(id).label.clone()
}

fn editing(app: &TestApp) -> Option<&str> {
    app.session()
        .editing
        .as_ref()
        .map(|edit| edit.entity().as_str())
}

#[test]
fn a_double_click_on_the_title_edits_it_with_all_of_it_selected() {
    let mut app = nested();
    app.double_click(TITLE);
    assert_eq!(editing(&app), Some("outer"));
    assert_eq!(app.editing_text(), "Outer");
    assert_eq!(app.caret(), (5, 0));
    assert_eq!(app.selected_ids(), ["outer"]);
    assert_eq!(app.session().gesture, None);
}

#[test]
fn typing_and_enter_rename_the_group_in_one_step() {
    let mut app = nested();
    app.double_click(TITLE)
        .type_text("Moodboard")
        .key(Key::Enter);
    assert_eq!(editing(&app), None);
    assert_eq!(label(&app, "outer").as_deref(), Some("Moodboard"));
    assert_eq!(app.selected_ids(), ["outer"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn enter_never_breaks_the_line() {
    let mut app = nested();
    app.double_click(TITLE).type_text("ab").key(Key::Enter);
    assert_eq!(label(&app, "outer").as_deref(), Some("ab"));
    app.double_click(TITLE).paste("one\ntwo");
    assert_eq!(app.editing_text(), "one two");
}

#[test]
fn escape_abandons_the_rename() {
    let mut app = nested();
    app.double_click(TITLE)
        .type_text("Moodboard")
        .key(Key::Escape);
    assert_eq!(editing(&app), None);
    assert_eq!(label(&app, "outer").as_deref(), Some("Outer"));
    assert!(!app.app().can_undo());
}

#[test]
fn a_press_elsewhere_commits_the_rename() {
    let mut app = nested();
    app.double_click(TITLE)
        .type_text("Moodboard")
        .click((900.0, 900.0));
    assert_eq!(label(&app, "outer").as_deref(), Some("Moodboard"));
    app.assert_undo_returns_to_start();
}

#[test]
fn an_empty_or_unchanged_name_keeps_the_label_and_records_nothing() {
    let mut app = nested();
    app.double_click(TITLE).key(Key::Backspace).key(Key::Enter);
    assert_eq!(label(&app, "outer").as_deref(), Some("Outer"));
    app.double_click(TITLE).type_text("  ").key(Key::Enter);
    assert_eq!(label(&app, "outer").as_deref(), Some("Outer"));
    app.double_click(TITLE).key(Key::Enter);
    assert!(!app.app().can_undo());
    assert_eq!(
        app.take_effects()
            .iter()
            .filter(|e| **e == Effect::Save)
            .count(),
        0
    );
}

#[test]
fn the_name_is_trimmed() {
    let mut app = nested();
    app.double_click(TITLE)
        .type_text("  Moodboard ")
        .key(Key::Enter);
    assert_eq!(label(&app, "outer").as_deref(), Some("Moodboard"));
}

#[test]
fn the_caret_sits_on_the_titles_line_after_the_text() {
    let mut app = nested();
    app.double_click(TITLE).type_text("Hi");
    let caret = app.app().caret_rect().expect("a caret");
    // Two characters at 10 units, from the group's left edge.
    assert_eq!(caret.x, 120.0);
    // The line is centred in the 16.5 above the 4 gap over the group's top.
    assert_eq!(caret.y + caret.height / 2.0, 100.0 - 4.0 - 16.5 / 2.0);
}

#[test]
fn the_frame_follows_the_zoom_so_the_title_keeps_its_pixel_size() {
    let mut app = nested();
    app.zoom(2.0);
    // The title is at (200, 200 - 20.5) on screen at twice the zoom.
    app.double_click((210.0, 190.0)).type_text("Hi");
    let frame = app.app().edit_frame().expect("a frame");
    assert_eq!(frame.spec.size, 5.5);
    let caret = app.app().caret_rect().expect("a caret");
    assert_eq!(caret.x, 120.0);
}

#[test]
fn the_pointer_over_a_title_being_edited_is_a_text_cursor() {
    let mut app = nested();
    app.double_click(TITLE).pointer_move(TITLE);
    assert_eq!(app.session().cursor, Cursor::Text);
}

#[test]
fn a_click_in_the_title_places_the_caret() {
    let mut app = nested();
    app.double_click(TITLE).click((126.0, 90.0));
    assert_eq!(editing(&app), Some("outer"));
    assert_eq!(app.caret(), (3, 3));
}
