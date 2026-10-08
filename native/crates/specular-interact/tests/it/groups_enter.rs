//! Stepping into a group by double click, and renaming one by double
//! clicking its title. A double click on a group's interior selects what is
//! directly inside it, one level at a time, and enters it; Escape steps out
//! one level before it deselects.

use specular_doc::{Entity, ItemId, Rect};
use specular_interact::{Action, Effect, Key};
use specular_testkit::{SHIFT, TestApp, group, inside, shape};

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
