//! `hit_test` over every kind, the selection's handles and anchors, and
//! edges. The camera is at zoom 1 with no pan unless a test says otherwise,
//! so canvas and screen coordinates are the same numbers.

use glam::Vec2;
use specular_doc::{EdgeId, EdgeSide, Entity, EntityId, Rect};
use specular_interact::{Corner, Handle, HandleOwner, Hit, hit_test};
use specular_testkit::{TestApp, connected, document, drawing, group, inside, page, shape, text};

const PAGE: Rect = Rect::new(200.0, 200.0, 400.0, 300.0);
const GROUP: Rect = Rect::new(100.0, 100.0, 600.0, 500.0);

fn at(app: &TestApp, x: f32, y: f32) -> Hit {
    hit_test(app.app(), Vec2::new(x, y))
}

fn body(id: &str) -> Hit {
    Hit::EntityBody {
        entity: EntityId::from(id),
    }
}

fn page_content(id: &str, local: (f32, f32)) -> Hit {
    Hit::PageContent {
        page: EntityId::from(id),
        local: Vec2::from(local),
    }
}

fn handle_of(id: &str, handle: Handle) -> Hit {
    Hit::Handle {
        owner: HandleOwner::Entity(EntityId::from(id)),
        handle,
    }
}

fn selection_handle(corner: Corner) -> Hit {
    Hit::Handle {
        owner: HandleOwner::Selection,
        handle: Handle::Corner(corner),
    }
}

fn anchor(id: &str, side: EdgeSide) -> Hit {
    Hit::Anchor {
        entity: EntityId::from(id),
        side,
    }
}

fn selected(entities: impl IntoIterator<Item = Entity>, ids: &[&str]) -> TestApp {
    let mut app = TestApp::with_entities(entities);
    app.select(ids);
    app
}

// Anchors: the top one is a 68x32 box 4 px above the page, the right one a
// 32x68 box 4 px to its right.

#[test]
fn the_top_anchor_of_the_selected_page_is_hit_above_its_edge() {
    let app = selected([page("f1", PAGE)], &["f1"]);
    assert_eq!(at(&app, 400.0, 185.0), anchor("f1", EdgeSide::Top));
}

#[test]
fn the_hovered_page_shows_its_anchors_without_being_selected() {
    let mut app = TestApp::with_entities([page("f1", PAGE)]);
    app.pointer_move((400.0, 250.0));
    assert_eq!(at(&app, 400.0, 185.0), anchor("f1", EdgeSide::Top));
    // Moving onto the anchor keeps the page hovered.
    app.pointer_move((400.0, 185.0));
    assert_eq!(app.session().hover.as_ref(), Some(&EntityId::from("f1")));
}

#[test]
fn anchors_shrink_with_the_canvas_down_to_a_floor() {
    let mut app = selected([page("f1", PAGE)], &["f1"]);
    // At zoom 0.1 the page's top edge runs from (20, 20) to (60, 20). The
    // anchor is 35% size: 23.8 wide, 11.2 deep, 4 px above the edge.
    app.zoom(0.1);
    assert_eq!(
        (
            at(&app, 40.0, 10.0),
            at(&app, 40.0, 3.0),
            at(&app, 54.0, 10.0)
        ),
        (anchor("f1", EdgeSide::Top), Hit::Empty, Hit::Empty)
    );
}

// Resize handles: 12 px squares on the corners of the outline 1 px outside
// the entity, and 12 px strips along its sides.

#[test]
fn a_corner_handle_wins_over_the_page_body() {
    let app = selected([page("f1", PAGE)], &["f1"]);
    assert_eq!(
        at(&app, 600.0, 200.0),
        handle_of("f1", Handle::Corner(Corner::TopRight))
    );
}

#[test]
fn a_press_a_few_pixels_past_the_corner_still_hits_the_handle() {
    // The top-right hit square spans x 595..607, y 193..205.
    let app = selected([page("f1", PAGE)], &["f1"]);
    assert_eq!(
        (at(&app, 606.0, 194.0), at(&app, 608.0, 194.0)),
        (
            handle_of("f1", Handle::Corner(Corner::TopRight)),
            Hit::Empty
        )
    );
}

#[test]
fn a_side_handle_runs_the_whole_side() {
    let app = selected([page("f1", PAGE)], &["f1"]);
    assert_eq!(
        (at(&app, 230.0, 195.0), at(&app, 603.0, 480.0)),
        (
            handle_of("f1", Handle::Side(EdgeSide::Top)),
            handle_of("f1", Handle::Side(EdgeSide::Right))
        )
    );
}

#[test]
fn every_kind_gets_handles_when_selected() {
    let rect = Rect::new(100.0, 100.0, 200.0, 100.0);
    let kinds = [
        page("e", rect),
        text("e", rect),
        shape("e", rect),
        drawing("e", rect),
        group("e", rect),
        specular_testkit::file("e", rect),
    ];
    for entity in kinds {
        let name = entity.kind.name();
        let app = selected([entity], &["e"]);
        assert_eq!(
            at(&app, 300.0, 200.0),
            handle_of("e", Handle::Corner(Corner::BottomRight)),
            "{name}"
        );
    }
}

#[test]
fn a_selection_of_several_has_handles_on_its_bounds_only() {
    // The two rects span (100, 100) to (280, 240).
    let entities = [
        text("t1", Rect::new(100.0, 100.0, 50.0, 50.0)),
        text("t2", Rect::new(200.0, 200.0, 80.0, 40.0)),
    ];
    let app = selected(entities, &["t1", "t2"]);
    assert_eq!(
        (
            at(&app, 280.0, 240.0),
            at(&app, 100.0, 100.0),
            // t1's own bottom-right corner is inside the bounds, not on them.
            // A pixel in from it, clear of the gap strip the pair shows.
            at(&app, 149.0, 149.0),
        ),
        (
            selection_handle(Corner::BottomRight),
            selection_handle(Corner::TopLeft),
            body("t1")
        )
    );
}

#[test]
fn a_group_selected_with_a_sibling_gets_handles_around_both() {
    // The group, its child and the sibling span (100, 50) to (900, 650).
    let entities = [
        group("g1", GROUP),
        inside("g1", text("c1", Rect::new(150.0, 150.0, 100.0, 100.0))),
        text("t1", Rect::new(800.0, 50.0, 100.0, 600.0)),
    ];
    let app = selected(entities, &["g1", "t1"]);
    assert_eq!(
        (at(&app, 900.0, 650.0), at(&app, 700.0, 600.0)),
        (
            selection_handle(Corner::BottomRight),
            // The group's own corner has no handle of its own.
            Hit::GroupBorder {
                group: EntityId::from("g1")
            }
        )
    );
}

// Bodies.

#[test]
fn a_page_body_reports_the_point_in_the_pages_pixels() {
    let app = TestApp::with_entities([page("f1", PAGE)]);
    assert_eq!(at(&app, 400.0, 350.0), page_content("f1", (200.0, 150.0)));
}

#[test]
fn a_member_is_hit_before_its_group_whatever_the_stack_order() {
    let member = || inside("g1", text("t1", Rect::new(300.0, 300.0, 100.0, 40.0)));
    let behind = TestApp::with_entities([group("g1", GROUP), member()]);
    let in_front = TestApp::with_entities([member(), group("g1", GROUP)]);
    assert_eq!(
        (at(&behind, 350.0, 320.0), at(&in_front, 350.0, 320.0)),
        (body("t1"), body("t1"))
    );
}

#[test]
fn a_groups_interior_and_its_border_are_different_hits() {
    let app = TestApp::with_entities([
        group("g1", GROUP),
        inside("g1", text("t1", Rect::new(300.0, 300.0, 100.0, 40.0))),
    ]);
    let border = Hit::GroupBorder {
        group: EntityId::from("g1"),
    };
    assert_eq!(
        (
            at(&app, 600.0, 550.0),
            at(&app, 104.0, 300.0),
            at(&app, 400.0, 596.0),
            at(&app, 109.0, 300.0),
        ),
        (body("g1"), border.clone(), border, body("g1"))
    );
}

#[test]
fn the_front_of_two_overlapping_entities_is_hit() {
    let rect = Rect::new(200.0, 200.0, 200.0, 200.0);
    let app = TestApp::with_entities([text("t1", rect), text("t2", rect)]);
    let reversed = TestApp::with_entities([text("t2", rect), text("t1", rect)]);
    assert_eq!(
        (at(&app, 300.0, 300.0), at(&reversed, 300.0, 300.0)),
        (body("t2"), body("t1"))
    );
}

#[test]
fn a_drawing_in_front_of_a_page_wins_only_where_it_is() {
    let app = TestApp::with_entities([
        page("p1", PAGE),
        drawing("d1", Rect::new(220.0, 170.0, 100.0, 80.0)),
    ]);
    assert_eq!(
        (
            at(&app, 260.0, 185.0),
            at(&app, 260.0, 215.0),
            at(&app, 260.0, 350.0)
        ),
        (body("d1"), body("d1"), page_content("p1", (60.0, 150.0)))
    );
}

#[test]
fn a_flat_line_is_widened_enough_to_grab() {
    let app = TestApp::with_entities([
        page("p1", PAGE),
        drawing("line", Rect::new(220.0, 260.0, 200.0, 0.0)),
    ]);
    assert_eq!(
        (at(&app, 300.0, 263.0), at(&app, 300.0, 267.0)),
        (body("line"), page_content("p1", (100.0, 67.0)))
    );
}

// Group titles: a 20.5 px tall box above the group's top-left corner, as
// wide as its text is estimated to be (6.1 px a character).

fn titled(label: Option<&str>) -> Entity {
    Entity {
        label: label.map(str::to_owned),
        ..group("g1", GROUP)
    }
}

#[test]
fn a_group_title_is_hit_above_the_groups_corner() {
    let app = TestApp::with_entities([titled(Some("Group"))]);
    let title = Hit::GroupLabel {
        group: EntityId::from("g1"),
    };
    assert_eq!(
        (at(&app, 110.0, 90.0), at(&app, 140.0, 90.0)),
        (title, Hit::Empty)
    );
}

// Edges.

fn linked() -> TestApp {
    // The edge leaves t1's right side at (208, 150) and reaches t2's left
    // side at (492, 150), in a straight line because the two are level.
    let entities = [
        text("t1", Rect::new(100.0, 100.0, 100.0, 100.0)),
        text("t2", Rect::new(500.0, 100.0, 100.0, 100.0)),
    ];
    TestApp::from_document(connected(document(entities), "e1", "t1", "t2"))
}

#[test]
fn an_edge_is_hit_within_seven_pixels_of_its_line() {
    let app = linked();
    let edge = Hit::Edge {
        edge: EdgeId::from("e1"),
    };
    assert_eq!(
        (at(&app, 350.0, 156.0), at(&app, 350.0, 158.0)),
        (edge, Hit::Empty)
    );
}
