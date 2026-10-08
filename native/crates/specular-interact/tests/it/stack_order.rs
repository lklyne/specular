//! Bring forward, send backward, bring to front and send to back (ADR 0014):
//! each moves the selection in the one stack order as one undo step, and a
//! group's members stay in one run.

use specular_doc::Rect;
use specular_interact::{Action, Key};
use specular_testkit::{CMD, CMD_SHIFT, TestApp, group, inside, shape};

const R: Rect = Rect::new(100.0, 100.0, 100.0, 100.0);

fn four() -> TestApp {
    TestApp::with_entities([shape("a", R), shape("b", R), shape("c", R), shape("d", R)])
}

fn order(app: &TestApp) -> Vec<&str> {
    (app.document().order().iter())
        .map(specular_doc::ItemId::as_str)
        .collect()
}

#[test]
fn forward_and_backward_move_a_block_past_the_next_item() {
    let mut app = four();
    app.select(&["b", "c"]).act(Action::BringForward);
    assert_eq!(order(&app), ["a", "d", "b", "c"]);
    app.assert_undo_returns_to_start();

    let mut app = four();
    app.select(&["b", "c"]).act(Action::SendBackward);
    assert_eq!(order(&app), ["b", "c", "a", "d"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn to_front_and_to_back_keep_the_selection_in_its_own_order() {
    let mut app = four();
    app.select(&["d", "b"]).act(Action::BringToFront);
    assert_eq!(order(&app), ["a", "c", "b", "d"]);
    app.assert_undo_returns_to_start();

    let mut app = four();
    app.select(&["b", "d"]).act(Action::SendToBack);
    assert_eq!(order(&app), ["b", "d", "a", "c"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_group_moves_with_its_members_as_one_unit() {
    let mut app = TestApp::with_entities([
        inside("g", shape("m", R)),
        inside("g", shape("n", R)),
        group("g", R),
        shape("x", R),
    ]);
    assert_eq!(order(&app), ["m", "n", "g", "x"]);
    app.select(&["g"]).act(Action::BringForward);
    assert_eq!(order(&app), ["x", "m", "n", "g"]);
    app.act(Action::SendToBack);
    assert_eq!(order(&app), ["m", "n", "g", "x"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_member_that_would_leave_its_group_drags_the_run_with_it() {
    // The frontmost member stepping past the group's own slot would split the
    // run; contiguity gathers it back, so the member stays inside.
    let mut app = TestApp::with_entities([
        shape("x", R),
        inside("g", shape("m", R)),
        inside("g", shape("n", R)),
        group("g", R),
    ]);
    app.select(&["n"]).act(Action::BringForward);
    assert_eq!(order(&app), ["x", "m", "n", "g"]);
    assert!(!app.app().can_undo(), "nothing changed, so no step");
}

#[test]
fn the_keys_run_the_verbs() {
    let mut app = four();
    app.select(&["a"]);
    app.chord(CMD, Key::Char(']'));
    assert_eq!(order(&app), ["b", "a", "c", "d"]);
    app.hold(CMD_SHIFT).key(Key::Char(']')).let_go();
    assert_eq!(order(&app), ["b", "c", "d", "a"]);
    app.chord(CMD, Key::Char('['));
    assert_eq!(order(&app), ["b", "c", "a", "d"]);
    app.hold(CMD_SHIFT).key(Key::Char('[')).let_go();
    assert_eq!(order(&app), ["a", "b", "c", "d"]);
    app.undo().undo().undo().undo();
    assert!(!app.app().can_undo());
}
