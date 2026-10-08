//! Drawing an edge from an anchor, moving its end, and deleting it by
//! dragging the end away. Shapes `a`, `b` and `c` stand on the canvas at
//! zoom 1, so screen and canvas coordinates agree.
//!
//! a: (100,100)-(300,200)   b: (500,100)-(700,200)   c: (100,400)-(300,500)
//! A side's dot sits 8 outside its middle: a's right dot is (308,150), b's
//! left (492,150), c's top (200,392).

use specular_doc::{Edge, EdgeEnd, EdgeKind, EdgeSide, ItemId, Rect};
use specular_interact::{Action, Effect, Key};
use specular_testkit::{TestApp, assert_doc_snapshot, document, shape, with_edge};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(500.0, 100.0, 200.0, 100.0);
const C: Rect = Rect::new(100.0, 400.0, 200.0, 100.0);

fn three() -> TestApp {
    TestApp::with_entities([shape("a", A), shape("b", B), shape("c", C)])
}

/// `a` to `b`, from a's right to b's left.
fn linked() -> TestApp {
    let edge = Edge {
        from_side: Some(EdgeSide::Right),
        to_side: Some(EdgeSide::Left),
        ..Edge::new("e", "a", "b")
    };
    let start = document([shape("a", A), shape("b", B), shape("c", C)]);
    TestApp::from_document(with_edge(start, edge))
}

fn saved(app: &mut TestApp) -> bool {
    app.take_effects()
        .iter()
        .any(|effect| matches!(effect, Effect::Save))
}

fn edge_count(app: &TestApp) -> usize {
    app.document().edges().count()
}

// Creating.

#[test]
fn dragging_from_an_anchor_to_another_anchor_makes_an_edge() {
    let mut app = three();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((480.0, 150.0))
        .release();
    let edge = app.document().edges().next().unwrap().clone();
    assert_eq!(
        (
            edge.from.as_str(),
            edge.to.as_str(),
            edge.from_side,
            edge.to_side
        ),
        ("a", "b", Some(EdgeSide::Right), Some(EdgeSide::Left))
    );
    assert_eq!(
        (edge.to_end, edge.kind),
        (Some(EdgeEnd::Arrow), Some(EdgeKind::Connection))
    );
    assert_eq!(
        app.document().order().last(),
        Some(&ItemId::Edge(edge.id.clone())),
        "the new edge is on top"
    );
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
    app.undo();
    assert_eq!(edge_count(&app), 0);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn the_edge_leaves_the_selection_as_it_was_when_it_starts_at_a_selected_entity() {
    let mut app = three();
    app.click((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((492.0, 150.0))
        .release();
    assert_eq!(app.selected_ids(), ["a"]);
    assert_eq!(edge_count(&app), 1);
}

#[test]
fn an_edge_started_at_an_unselected_entity_clears_the_selection() {
    let mut app = three();
    app.click((600.0, 150.0))
        .pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((492.0, 150.0))
        .release();
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
}

#[test]
fn releasing_on_another_entitys_body_connects_to_its_facing_side() {
    let mut app = three();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((600.0, 150.0))
        .release();
    let edge = app.document().edges().next().unwrap();
    assert_eq!(
        (edge.to.as_str(), edge.to_side),
        ("b", Some(EdgeSide::Left))
    );
    // c is below a, so its top faces a.
    let mut app = three();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((200.0, 445.0))
        .release();
    let edge = app.document().edges().next().unwrap();
    assert_eq!((edge.to.as_str(), edge.to_side), ("c", Some(EdgeSide::Top)));
    app.assert_undo_returns_to_start();
}

#[test]
fn releasing_on_empty_canvas_makes_nothing_and_saves_nothing() {
    let mut app = three();
    app.pointer_move((200.0, 150.0)).press((310.0, 150.0));
    app.take_effects();
    app.drag_to((400.0, 300.0)).release();
    assert_eq!(edge_count(&app), 0);
    assert!(!saved(&mut app));
    assert!(!app.app().can_undo());
    app.assert_undo_returns_to_start();
}

#[test]
fn releasing_on_the_entity_it_started_at_makes_nothing() {
    let mut app = three();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((250.0, 150.0))
        .release();
    assert_eq!(edge_count(&app), 0);
}

#[test]
fn escape_during_a_create_leaves_the_document_alone() {
    let mut app = three();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((492.0, 150.0));
    app.take_effects();
    app.key(Key::Escape);
    assert!(app.session().gesture.is_none());
    assert_eq!(edge_count(&app), 0);
    assert!(!saved(&mut app));
    app.assert_undo_returns_to_start();
}

#[test]
fn the_snap_is_forty_eight_pixels_at_zoom_one_and_less_zoomed_out() {
    // 40 px from b's left dot (492,150).
    let mut app = three();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((452.0, 150.0));
    assert!(app.app().edge_preview().is_some_and(|p| p.snap.is_some()));
    app.release().undo();

    // Zoomed to 0.5 the reach is 24 screen pixels around the same dot.
    let mut app = three();
    app.zoom(0.5);
    let dot = |x: f32, y: f32| (x * 0.5, y * 0.5);
    app.pointer_move(dot(200.0, 150.0))
        .press((154.0 + 8.0, 75.0))
        .drag_to((246.0 - 30.0, 75.0));
    assert!(app.app().edge_preview().is_some_and(|p| p.snap.is_none()));
    app.drag_to((246.0 - 10.0, 75.0));
    assert!(app.app().edge_preview().is_some_and(|p| p.snap.is_some()));
}

// Moving an end.

#[test]
fn dragging_an_edges_end_to_another_entity_re_routes_it() {
    let mut app = linked();
    app.pointer_move((600.0, 150.0))
        .press((490.0, 150.0))
        .drag_to((200.0, 392.0))
        .release();
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"shape","x":100,"y":100,"width":200,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"b","type":"shape","x":500,"y":100,"width":200,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"c","type":"shape","x":100,"y":400,"width":200,"height":100,"shapeKind":"rectangle","text":""}
    edges:
      {"id":"e","fromNode":"a","toNode":"c","fromSide":"right","toSide":"top"}
    specular: {"entityOrder":["a","b","c","e"]}
    "#);
    app.undo();
    assert_eq!(app.document().edges().next().unwrap().to.as_str(), "b");
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn dragging_the_from_end_moves_the_from_end() {
    let mut app = linked();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((200.0, 392.0))
        .release();
    let edge = app.document().edges().next().unwrap();
    assert_eq!(
        (edge.from.as_str(), edge.from_side, edge.to.as_str()),
        ("c", Some(EdgeSide::Top), "b")
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn the_edge_being_moved_is_named_for_the_scene_to_hide() {
    let mut app = linked();
    app.pointer_move((600.0, 150.0))
        .press((490.0, 150.0))
        .drag_to((400.0, 300.0));
    assert_eq!(
        app.app().rerouting().map(specular_doc::EdgeId::as_str),
        Some("e")
    );
    let preview = app.app().edge_preview().unwrap();
    assert_eq!(
        (preview.origin.x, preview.origin.y, preview.snap),
        (308.0, 150.0, None)
    );
}

#[test]
fn dropping_a_moved_end_on_nothing_deletes_the_edge() {
    let mut app = linked();
    app.pointer_move((600.0, 150.0))
        .press((490.0, 150.0))
        .drag_to((400.0, 700.0))
        .release();
    assert_eq!(edge_count(&app), 0);
    assert!(saved(&mut app));
    app.undo();
    assert_eq!(edge_count(&app), 1, "undo brings the edge back");
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn escape_during_a_re_route_deletes_the_edge() {
    let mut app = linked();
    app.pointer_move((600.0, 150.0))
        .press((490.0, 150.0))
        .drag_to((200.0, 392.0));
    app.take_effects();
    app.key(Key::Escape);
    assert_eq!(edge_count(&app), 0, "cancelling an edit discards the edge");
    assert!(saved(&mut app));
    app.undo();
    assert_eq!(edge_count(&app), 1);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn clicking_an_edges_end_without_dragging_leaves_it_as_it_is() {
    let mut app = linked();
    app.pointer_move((600.0, 150.0))
        .press((490.0, 150.0))
        .release();
    assert_eq!(edge_count(&app), 1);
    assert!(!app.app().can_undo());
}

// Following and deleting.

#[test]
fn an_edge_follows_the_entity_it_is_drawn_to() {
    let mut app = linked();
    let before = app.app().edge_curve(&"e".into()).unwrap();
    let edge = app.document().edge(&"e".into()).cloned();
    app.press((600.0, 150.0)).drag_to((600.0, 350.0));
    let during = app.app().edge_curve(&"e".into()).unwrap();
    assert_eq!(before.from, during.from);
    assert_eq!(during.to.y, before.to.y + 200.0);
    app.release();
    assert_eq!(app.document().edge(&"e".into()).cloned(), edge);
    app.assert_undo_returns_to_start();
}

#[test]
fn deleting_an_entity_deletes_its_edges_in_the_same_step() {
    let mut app = linked();
    app.select(&["b"]).act(Action::Delete);
    assert_eq!(edge_count(&app), 0);
    app.undo();
    assert_eq!(edge_count(&app), 1);
    assert!(!app.app().can_undo(), "one step undid both");
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn cutting_an_entity_deletes_its_edges_in_the_same_step() {
    let mut app = linked();
    app.select(&["a"]).act(Action::Cut);
    assert_eq!(edge_count(&app), 0);
    app.undo();
    assert_eq!(edge_count(&app), 1);
    app.redo().assert_undo_returns_to_start();
}

// Anchors.

#[test]
fn the_hovered_entity_offers_its_four_anchors_and_lights_the_one_under_the_pointer() {
    let mut app = three();
    app.pointer_move((200.0, 150.0));
    let anchors = app.app().anchors();
    assert_eq!(anchors.len(), 4);
    assert!(anchors.iter().all(|anchor| anchor.entity.as_str() == "a"));
    assert!(anchors.iter().all(|anchor| !anchor.active));
    app.pointer_move((315.0, 150.0));
    let lit: Vec<_> = app
        .app()
        .anchors()
        .into_iter()
        .filter(|anchor| anchor.active)
        .map(|anchor| (anchor.side, anchor.point.x, anchor.point.y))
        .collect();
    assert_eq!(lit, [(EdgeSide::Right, 308.0, 150.0)]);
}

#[test]
fn the_selected_entity_offers_anchors_but_a_selection_of_several_does_not() {
    let mut app = three();
    app.click((200.0, 150.0)).pointer_move((900.0, 900.0));
    assert_eq!(app.app().anchors().len(), 4);
    app.select(&["a", "b"]);
    assert_eq!(app.app().anchors().len(), 0);
}

#[test]
fn no_anchors_during_a_move_or_while_text_is_edited() {
    let mut app = three();
    app.click((200.0, 150.0))
        .press((200.0, 150.0))
        .drag_to((260.0, 190.0));
    assert_eq!(app.app().anchors().len(), 0);
    app.release();
    app.double_click((260.0, 190.0));
    assert!(app.app().text_edit().is_some());
    assert_eq!(app.app().anchors().len(), 0);
}

#[test]
fn every_entity_offers_anchors_while_an_edge_is_dragged() {
    let mut app = three();
    app.pointer_move((200.0, 150.0))
        .press((310.0, 150.0))
        .drag_to((400.0, 300.0));
    let anchors = app.app().anchors();
    assert_eq!(anchors.len(), 12);
    assert!(anchors.iter().all(|anchor| anchor.active));
}

#[test]
fn a_press_on_an_anchor_does_not_start_a_move() {
    let mut app = three();
    app.pointer_move((200.0, 150.0)).press((310.0, 150.0));
    assert!(matches!(
        app.session().gesture,
        Some(specular_interact::Gesture::EdgeDrag(_))
    ));
    app.drag_to((400.0, 300.0)).release();
    assert_eq!(app.rect("a"), A);
}
