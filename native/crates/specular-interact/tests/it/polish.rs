//! What the polish batch after the QA pass fixed, each kept as the test
//! that failed before its fix.

use glam::Vec2;
use specular_doc::{Entity, EntityId, Rect};
use specular_interact::{Event, Hit, Key, Tool, hit_test};
use specular_testkit::{CMD, TestApp, connected, document, group, shape, sticky};

/// Two shapes a long way apart with a sticky between them, and an edge from
/// one shape to the other that runs across the sticky, in front of it.
fn an_edge_across_a_sticky() -> TestApp {
    let entities = [
        shape("a", Rect::new(0.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(600.0, 100.0, 100.0, 100.0)),
        sticky("s", Rect::new(250.0, 50.0, 200.0, 200.0), "in the way"),
    ];
    TestApp::from_document(connected(document(entities), "e1", "a", "b"))
}

#[test]
fn a_drag_from_an_edge_where_it_crosses_an_entity_moves_the_entity() {
    let mut app = an_edge_across_a_sticky();
    // On the line, inside the sticky.
    let on_both = (350.0, 150.0);
    assert_eq!(
        hit_test(app.app(), Vec2::from(on_both)),
        Hit::Edge { edge: "e1".into() }
    );
    app.press(on_both).drag_to((350.0, 350.0)).release();
    assert_eq!(app.rect("s").y, 260.0, "the sticky went with the drag");
    assert_eq!(app.selected_ids(), ["s"]);
    app.assert_undo_returns_to_start();

    // A click in the same place selects the edge.
    app.click(on_both);
    assert_eq!(app.selected_ids(), ["e1"]);
}

/// A diagonal pen stroke from (100, 100) to (300, 300), unselected.
fn a_diagonal_stroke() -> (TestApp, EntityId) {
    let mut app = TestApp::empty();
    app.tool(Tool::Draw)
        .press((100.0, 100.0))
        .drag_to((200.0, 200.0))
        .drag_to((300.0, 300.0))
        .release()
        .key(Key::Escape);
    let drawn = app.document().entities().next_back();
    let id = drawn.map_or_else(|| "nothing drawn".into(), |entity| entity.id.clone());
    (app, id)
}

#[test]
fn a_drawing_is_hit_on_its_ink_and_not_in_the_empty_corner_of_its_box() {
    let (mut app, id) = a_diagonal_stroke();
    let body = Hit::EntityBody { entity: id.clone() };
    let at = |app: &TestApp, x, y| hit_test(app.app(), Vec2::new(x, y));
    assert_eq!(at(&app, 200.0, 200.0), body, "on the line");
    assert_eq!(at(&app, 204.0, 198.0), body, "a few pixels off it");
    assert_eq!(at(&app, 220.0, 180.0), Hit::Empty, "28 pixels off it");
    assert_eq!(at(&app, 280.0, 120.0), Hit::Empty, "the empty corner");

    app.pointer_move((280.0, 120.0));
    assert_eq!(app.session().hover, None);
    // A click in the corner is a click on empty canvas.
    app.click((280.0, 120.0));
    assert_eq!(app.selected_ids(), [] as [&str; 0]);

    // Selected, its whole box drags it.
    app.click((200.0, 200.0));
    assert_eq!(at(&app, 280.0, 120.0), body);
    let before = app.rect(id.as_str());
    app.drag((280.0, 120.0), (300.0, 120.0));
    assert_eq!(app.rect(id.as_str()).x, before.x + 20.0);
}

#[test]
fn what_is_under_the_empty_part_of_a_drawing_can_be_pressed() {
    let (mut app, _) = a_diagonal_stroke();
    // A shape goes in under the stroke's box, behind it.
    app.tool(Tool::Select)
        .chord(CMD, Key::Char('z'))
        .open(document([shape(
            "under",
            Rect::new(220.0, 100.0, 80.0, 80.0),
        )]));
    app.tool(Tool::Draw)
        .press((100.0, 100.0))
        .drag_to((300.0, 300.0))
        .release()
        .key(Key::Escape);
    app.click((270.0, 130.0));
    assert_eq!(app.selected_ids(), ["under"]);
}

#[test]
fn a_duplicate_that_lands_off_screen_is_brought_into_view() {
    // The viewport is filled, so the free spot is past its right edge.
    let wide = Rect::new(0.0, 0.0, 2000.0, 100.0);
    let mut app = TestApp::with_entities([shape("wide", wide)]);
    let viewport = Vec2::new(1000.0, 800.0);
    app.send(Event::ViewportResized(viewport));
    app.select(&["wide"]).chord(CMD, Key::Char('d'));
    let copy = app.selected().unwrap().to_owned();
    let rect = app.rect(&copy);
    let camera = app.session().camera;
    let corner = camera.world_to_screen(Vec2::new(rect.x as f32, rect.y as f32));
    assert!(
        corner.cmpge(Vec2::ZERO).all() && corner.cmplt(viewport).all(),
        "the copy's corner is at {corner} in a viewport of {viewport}"
    );
    assert_ne!(camera.pan, Vec2::ZERO);
}

#[test]
fn a_group_title_is_hit_where_it_is_drawn_when_zoomed_out() {
    let titled = Entity {
        label: Some("A long title for a small group".to_owned()),
        ..group("g", Rect::new(400.0, 400.0, 120.0, 80.0))
    };
    let short = Entity {
        label: Some("abcd".to_owned()),
        ..group("h", Rect::new(800.0, 400.0, 400.0, 80.0))
    };
    let mut app = TestApp::with_entities([titled, short]);
    let label = Hit::GroupLabel { group: "g".into() };
    let at = |app: &TestApp, x, y| hit_test(app.app(), Vec2::new(x, y));
    // At full size the title is a 20.5 px box above the corner, and never
    // wider than the group: 30 characters would run to 183 px.
    assert_eq!(at(&app, 510.0, 390.0), label);
    assert_eq!(at(&app, 560.0, 390.0), Hit::Empty);
    assert_eq!(
        at(&app, 510.0, 365.0),
        Hit::Empty,
        "and no taller than drawn"
    );
    // At a quarter zoom the group is at (100, 100) and 30 px wide, and the
    // title is half its size: about 10 px tall.
    app.zoom(0.25);
    assert_eq!(at(&app, 110.0, 95.0), label);
    assert_eq!(at(&app, 110.0, 92.0), label, "about 10 px above the corner");
    assert_eq!(at(&app, 110.0, 85.0), Hit::Empty);
    assert_eq!(at(&app, 140.0, 95.0), Hit::Empty);
    // A short title at a quarter zoom is about 12 px wide, under the group's
    // 100: its width follows the zoom, not only the group.
    let short = Hit::GroupLabel { group: "h".into() };
    assert_eq!(at(&app, 205.0, 97.0), short);
    assert_eq!(at(&app, 218.0, 97.0), Hit::Empty);
}
