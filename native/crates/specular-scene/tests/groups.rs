//! Scene snapshots for groups: the drop-target ring while items are dragged
//! over one, a group's title while it is renamed, and the reorder dots and
//! gap bars of an auto-layout row.

use specular_doc::{Entity, Rect};
use specular_testkit::{TestApp, assert_scene_snapshot, group, inside, shape};

fn named(group: Entity, label: &str) -> Entity {
    Entity {
        label: Some(label.to_owned()),
        ..group
    }
}

fn board() -> TestApp {
    TestApp::with_entities([
        inside("g", shape("a", Rect::new(140.0, 140.0, 100.0, 100.0))),
        named(
            group("g", Rect::new(100.0, 100.0, 400.0, 300.0)),
            "Moodboard",
        ),
        shape("c", Rect::new(100.0, 600.0, 100.0, 100.0)),
    ])
}

#[test]
fn the_group_under_a_drag_gets_a_selection_ring() {
    let mut app = board();
    app.press((150.0, 650.0)).drag_to((300.0, 300.0));
    assert_scene_snapshot!(app);
}

#[test]
fn a_title_being_renamed_shows_the_caret_after_what_was_typed() {
    let mut app = board();
    app.double_click((110.0, 90.0)).type_text("Ref");
    assert_scene_snapshot!(app);
}

#[test]
fn an_entered_group_has_a_dashed_ring_round_it() {
    let mut app = board();
    app.double_click((300.0, 380.0));
    assert_scene_snapshot!(app);
}

#[test]
fn a_group_that_follows_its_dragged_member_is_drawn_at_its_live_size() {
    let mut app = board();
    app.press((190.0, 190.0)).drag_to((220.0, 220.0));
    assert_scene_snapshot!(app);
}

#[test]
fn a_selected_auto_layout_row_shows_a_dot_on_each_member_and_a_bar_in_each_gap() {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(240.0, 100.0, 100.0, 100.0)),
        shape("c", Rect::new(380.0, 100.0, 100.0, 100.0)),
    ]);
    app.select(&["a", "b", "c"])
        .act(specular_interact::Action::AutoLayout);
    assert_scene_snapshot!(app);
}

#[test]
fn a_box_being_reordered_floats_at_half_strength_over_the_line_as_it_would_be() {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(240.0, 100.0, 100.0, 100.0)),
        shape("c", Rect::new(380.0, 100.0, 100.0, 100.0)),
    ]);
    app.select(&["a", "b", "c"]);
    // The pointer rests on the dot of `b`, which fills out, then drags `a`.
    app.pointer_move((290.0, 150.0));
    assert_scene_snapshot!("reorder_dot_under_the_pointer", app);
    app.press((150.0, 150.0)).drag_to((450.0, 190.0));
    assert_scene_snapshot!("reorder_in_flight", app);
}

#[test]
fn a_gap_bar_under_the_pointer_is_drawn_at_full_strength() {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(240.0, 100.0, 100.0, 100.0)),
        shape("c", Rect::new(380.0, 100.0, 100.0, 100.0)),
    ]);
    app.select(&["a", "b", "c"]);
    app.pointer_move((220.0, 150.0));
    let scene = app.scene_snapshot();
    assert!(
        scene.contains("screen rect 219,138 2x24 fill=#ec4899 stroke=#ffffff/1/outside\n"),
        "{scene}"
    );
    assert!(
        scene.contains(
            "screen rect 359,138 2x24 fill=#ec4899 stroke=#ffffff/1/outside opacity=0.6\n"
        ),
        "{scene}"
    );
}
