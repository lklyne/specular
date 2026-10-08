//! Scene snapshots for edges: the dot on the anchor under the pointer, the
//! rubber band while an edge is dragged, the edge that follows a dragged
//! entity, and a label being edited.

use specular_doc::{Edge, EdgeSide, Rect};
use specular_testkit::{TestApp, assert_scene_snapshot, document, shape, with_edge};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(500.0, 100.0, 200.0, 100.0);

fn pair() -> TestApp {
    TestApp::with_entities([shape("a", A), shape("b", B)])
}

fn linked() -> TestApp {
    let edge = Edge {
        from_side: Some(EdgeSide::Right),
        to_side: Some(EdgeSide::Left),
        ..Edge::new("e", "a", "b")
    };
    TestApp::from_document(with_edge(document([shape("a", A), shape("b", B)]), edge))
}

#[test]
fn a_hovered_entity_shows_the_dot_of_the_anchor_under_the_pointer() {
    let mut app = pair();
    app.pointer_move((200.0, 150.0));
    let quiet = app.scene_snapshot();
    app.pointer_move((315.0, 150.0));
    assert_ne!(quiet, app.scene_snapshot(), "the right dot appears");
    assert_scene_snapshot!(app);
}

#[test]
fn a_selected_entity_shows_a_dot_once_the_pointer_is_over_an_anchor() {
    let mut app = pair();
    app.click((200.0, 150.0)).pointer_move((200.0, 85.0));
    assert_scene_snapshot!(app);
}

#[test]
fn the_rubber_band_runs_from_the_anchor_to_the_pointer() {
    let mut app = pair();
    app.pointer_move((200.0, 150.0))
        .press((315.0, 150.0))
        .drag_to((400.0, 300.0));
    assert_scene_snapshot!(app);
}

#[test]
fn the_rubber_band_snaps_to_an_anchor_with_a_ringed_dot() {
    let mut app = pair();
    app.pointer_move((200.0, 150.0))
        .press((315.0, 150.0))
        .drag_to((470.0, 160.0));
    assert_scene_snapshot!(app);
}

#[test]
fn the_edge_being_re_routed_is_left_out_for_the_rubber_band() {
    let mut app = linked();
    app.pointer_move((600.0, 150.0))
        .press((490.0, 150.0))
        .drag_to((400.0, 300.0));
    assert_scene_snapshot!(app);
}

#[test]
fn an_edge_follows_an_entity_dragged_down() {
    let mut app = linked();
    app.press((600.0, 130.0)).drag_to((600.0, 330.0));
    assert_scene_snapshot!(app);
}

#[test]
fn a_label_being_edited_is_drawn_where_the_committed_one_is() {
    let mut app = linked();
    app.double_click((400.0, 150.0)).type_text("uses");
    assert_scene_snapshot!(app);
}
