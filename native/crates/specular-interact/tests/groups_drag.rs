//! Dragging into and out of groups: the group under the pointer when the
//! button comes up takes the dragged items, and leaving every group frees
//! them. Membership, the stack, page anchors and the move are one undo step.

use specular_doc::{Entity, Rect};
use specular_testkit::{ALT, CMD, TestApp, group, inside, page, shape};

/// Group `g` over (100, 100) to (500, 400) holding `a`; group `h` beside it
/// holding `b`; a shape `c` below them.
fn board() -> TestApp {
    TestApp::with_entities([
        inside("g", shape("a", Rect::new(140.0, 140.0, 100.0, 100.0))),
        named(group("g", Rect::new(100.0, 100.0, 400.0, 300.0))),
        inside("h", shape("b", Rect::new(740.0, 140.0, 100.0, 100.0))),
        named(group("h", Rect::new(700.0, 100.0, 300.0, 300.0))),
        shape("c", Rect::new(100.0, 600.0, 100.0, 100.0)),
    ])
}

fn named(group: Entity) -> Entity {
    let label = format!("Group {}", group.id.as_str());
    Entity {
        label: Some(label),
        ..group
    }
}

fn order(app: &TestApp) -> Vec<&str> {
    (app.document().order().iter())
        .map(specular_doc::ItemId::as_str)
        .collect()
}

fn parent<'a>(app: &'a TestApp, id: &str) -> Option<&'a str> {
    app.entity(id)
        .parent
        .as_ref()
        .map(specular_doc::EntityId::as_str)
}

fn target(app: &TestApp) -> Option<&str> {
    app.app()
        .group_drop_target()
        .map(specular_doc::EntityId::as_str)
}

#[test]
fn dropping_an_item_on_a_group_makes_it_a_member() {
    let mut app = board();
    app.press((150.0, 650.0)).drag_to((300.0, 300.0));
    assert_eq!(target(&app), Some("g"));
    app.release();
    assert_eq!(target(&app), None);
    assert_eq!(parent(&app, "c"), Some("g"));
    assert_eq!(app.rect("c"), Rect::new(260.0, 260.0, 100.0, 100.0));
    // The run gathers where its frontmost member was.
    assert_eq!(order(&app), ["b", "h", "a", "c", "g"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn the_target_is_the_innermost_group_under_the_pointer() {
    let mut app = TestApp::with_entities([
        inside(
            "outer",
            named(group("inner", Rect::new(200.0, 200.0, 200.0, 200.0))),
        ),
        named(group("outer", Rect::new(100.0, 100.0, 600.0, 500.0))),
        shape("c", Rect::new(800.0, 800.0, 100.0, 100.0)),
    ]);
    app.press((850.0, 850.0)).drag_to((300.0, 300.0));
    assert_eq!(target(&app), Some("inner"));
    app.drag_to((600.0, 500.0));
    assert_eq!(target(&app), Some("outer"));
    app.drag_to((900.0, 900.0));
    assert_eq!(target(&app), None);
    app.drag_to((600.0, 500.0)).release();
    assert_eq!(parent(&app, "c"), Some("outer"));
    app.assert_undo_returns_to_start();
}

#[test]
fn releasing_outside_every_group_frees_the_member() {
    let mut app = board();
    app.press((190.0, 190.0)).drag_to((190.0, 790.0)).release();
    assert_eq!(parent(&app, "a"), None);
    assert_eq!(app.rect("a"), Rect::new(140.0, 740.0, 100.0, 100.0));
    assert_eq!(order(&app), ["a", "g", "b", "h", "c"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn command_keeps_membership_as_it_was() {
    let mut app = board();
    app.press((150.0, 650.0)).hold(CMD).drag_to((300.0, 300.0));
    assert_eq!(target(&app), None);
    app.release().let_go();
    assert_eq!(parent(&app, "c"), None);
    assert_eq!(app.rect("c"), Rect::new(260.0, 260.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();

    let mut app = board();
    app.press((190.0, 190.0))
        .hold(CMD)
        .drag_to((190.0, 790.0))
        .release()
        .let_go();
    assert_eq!(
        parent(&app, "a"),
        Some("g"),
        "it stays a member, spatially outside"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn an_option_drag_copies_and_changes_no_membership() {
    let mut app = board();
    app.press((150.0, 650.0)).hold(ALT).drag_to((300.0, 300.0));
    assert_eq!(target(&app), None);
    app.release().let_go();
    assert_eq!(parent(&app, "c"), None);
    assert_eq!(app.rect("c"), Rect::new(100.0, 600.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_group_dragged_onto_another_joins_it_with_its_members() {
    let mut app = board();
    app.select(&["g"])
        .press((110.0, 90.0))
        .drag_to((810.0, 90.0));
    assert_eq!(target(&app), None, "the pointer is above both groups");
    app.drag_to((810.0, 180.0));
    assert_eq!(target(&app), Some("h"));
    app.release();
    assert_eq!(parent(&app, "g"), Some("h"));
    assert_eq!(parent(&app, "a"), Some("g"));
    assert_eq!(app.rect("a"), Rect::new(840.0, 240.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_grouped_item_has_no_page_anchor_and_a_freed_one_gains_it() {
    let mut app = TestApp::with_entities([
        page("p", Rect::new(1000.0, 100.0, 400.0, 300.0)),
        inside("g", shape("a", Rect::new(140.0, 140.0, 100.0, 100.0))),
        named(group("g", Rect::new(100.0, 100.0, 400.0, 300.0))),
        shape("c", Rect::new(1100.0, 600.0, 100.0, 100.0)),
    ]);
    // Out of the group onto the page: freed, and hooked to the page.
    app.press((190.0, 190.0)).drag_to((1190.0, 250.0)).release();
    assert_eq!(parent(&app, "a"), None);
    assert!(app.entity("a").anchor.is_some());
    app.assert_undo_returns_to_start();

    // From the page into a group: hooked no more.
    let mut app = TestApp::with_entities([
        page("p", Rect::new(1000.0, 100.0, 400.0, 300.0)),
        named(group("g", Rect::new(100.0, 100.0, 400.0, 300.0))),
        shape("c", Rect::new(1100.0, 200.0, 100.0, 100.0)),
    ]);
    assert!(app.entity("c").anchor.is_none());
    app.press((1150.0, 250.0))
        .drag_to((1250.0, 300.0))
        .release();
    assert!(
        app.entity("c").anchor.is_some(),
        "hooked to the page it landed on"
    );
    app.press((1250.0, 300.0)).drag_to((300.0, 300.0)).release();
    assert_eq!(parent(&app, "c"), Some("g"));
    assert!(app.entity("c").anchor.is_none());
    app.undo().undo();
}
