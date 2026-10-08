//! A group's rect follows its members: whatever changes one's rect or
//! membership refits the groups above it in the same undo step, frame by
//! frame while a drag is in flight.

use specular_doc::{Entity, Rect};
use specular_interact::Key;
use specular_testkit::{TestApp, group, inside, shape};

const A: Rect = Rect::new(100.0, 100.0, 100.0, 100.0);
const B: Rect = Rect::new(300.0, 100.0, 100.0, 100.0);
/// The rect that hugs `a` and `b` with 24 units of room.
const HUG: Rect = Rect::new(76.0, 76.0, 348.0, 148.0);

fn named(group: Entity) -> Entity {
    Entity {
        label: Some("Group".to_owned()),
        ..group
    }
}

/// Group `g` hugging `a` and `b`, and a loose `c` below.
fn board() -> TestApp {
    TestApp::with_entities([
        inside("g", shape("a", A)),
        inside("g", shape("b", B)),
        named(group("g", HUG)),
        shape("c", Rect::new(100.0, 600.0, 100.0, 100.0)),
    ])
}

/// The rect that hugs `ids` with 24 units of room.
fn hugging(app: &TestApp, ids: &[&str]) -> Rect {
    let rects = ids.iter().map(|id| app.rect(id));
    let left = rects.clone().map(|r| r.x).fold(f64::MAX, f64::min);
    let top = rects.clone().map(|r| r.y).fold(f64::MAX, f64::min);
    let right = rects
        .clone()
        .map(|r| r.x + r.width)
        .fold(f64::MIN, f64::max);
    let bottom = rects.map(|r| r.y + r.height).fold(f64::MIN, f64::max);
    Rect::new(
        left - 24.0,
        top - 24.0,
        right - left + 48.0,
        bottom - top + 48.0,
    )
}

#[test]
fn a_moved_member_grows_the_group_and_follows_the_drag() {
    let mut app = board();
    // The pointer stays inside the group, so the member stays in it.
    app.press((390.0, 150.0)).drag_to((420.0, 150.0));
    assert!(app.rect("b").x > 300.0);
    assert_eq!(app.rect("g"), hugging(&app, &["a", "b"]), "live");
    assert!(app.rect("g").width > HUG.width);
    app.release();
    assert_eq!(app.rect("g"), hugging(&app, &["a", "b"]));
    assert!(app.rect("g").width > HUG.width);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_moved_member_can_shrink_the_group() {
    let mut app = board();
    app.press((350.0, 150.0)).drag_to((250.0, 150.0)).release();
    assert_eq!(app.rect("b").x, 200.0);
    assert_eq!(app.rect("g"), Rect::new(76.0, 76.0, 248.0, 148.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_nudged_member_refits_its_group() {
    let mut app = board();
    app.select(&["b"]).key(Key::ArrowRight);
    assert_eq!(app.rect("g"), Rect::new(76.0, 76.0, 353.0, 148.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_resized_member_refits_its_group() {
    let mut app = board();
    app.select(&["b"])
        .press((401.0, 201.0))
        .drag_to((501.0, 301.0));
    let live = app.rect("g");
    app.release();
    assert!(app.rect("b").width > 100.0);
    assert_eq!(
        app.rect("g"),
        live,
        "the release keeps what the drag showed"
    );
    let (a, b) = (app.rect("a"), app.rect("b"));
    assert_eq!(
        app.rect("g"),
        Rect::new(
            a.x - 24.0,
            a.y - 24.0,
            b.x + b.width - a.x + 48.0,
            b.y + b.height - a.y + 48.0
        )
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_deleted_member_refits_its_group() {
    let mut app = board();
    app.select(&["b"]).key(Key::Backspace);
    assert_eq!(app.rect("g"), Rect::new(76.0, 76.0, 148.0, 148.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_group_left_with_no_members_keeps_its_rect() {
    let mut app = TestApp::with_entities([inside("g", shape("a", A)), named(group("g", HUG))]);
    app.select(&["a"]).key(Key::Backspace);
    assert_eq!(app.rect("g"), HUG);
    app.assert_undo_returns_to_start();
}

#[test]
fn an_item_dropped_in_grows_the_group() {
    let mut app = board();
    app.press((150.0, 650.0)).drag_to((300.0, 200.0)).release();
    assert_eq!(
        app.entity("c")
            .parent
            .as_ref()
            .map(specular_doc::EntityId::as_str),
        Some("g")
    );
    assert_eq!(app.rect("g"), hugging(&app, &["a", "b", "c"]));
    app.assert_undo_returns_to_start();
}

#[test]
fn an_item_dropped_out_leaves_the_group_fitted_round_the_rest() {
    let mut app = board();
    app.press((150.0, 150.0)).drag_to((150.0, 750.0)).release();
    assert_eq!(app.entity("a").parent, None);
    assert_eq!(app.rect("g"), Rect::new(276.0, 76.0, 148.0, 148.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn the_drop_target_is_read_from_the_groups_as_the_drag_began() {
    let mut app = board();
    // The group grows under the drag to reach the pointer, but the pointer
    // is outside the group it began in.
    app.press((350.0, 150.0)).drag_to((550.0, 150.0));
    assert_eq!(app.app().group_drop_target(), None);
    app.release();
    assert_eq!(app.entity("b").parent, None);
    assert_eq!(app.rect("g"), Rect::new(76.0, 76.0, 148.0, 148.0));
    app.assert_undo_returns_to_start();

    let mut app = board();
    app.press((350.0, 150.0)).drag_to((410.0, 150.0));
    assert_eq!(
        app.app()
            .group_drop_target()
            .map(specular_doc::EntityId::as_str),
        Some("g")
    );
    app.release();
    assert_eq!(
        app.entity("b")
            .parent
            .as_ref()
            .map(specular_doc::EntityId::as_str),
        Some("g")
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn nested_groups_are_refitted_inside_out() {
    let mut app = TestApp::with_entities([
        inside("inner", shape("a", A)),
        inside("inner", shape("b", B)),
        inside("outer", named(group("inner", HUG))),
        named(group("outer", Rect::new(52.0, 52.0, 396.0, 196.0))),
    ]);
    app.press((390.0, 150.0)).drag_to((420.0, 150.0)).release();
    let inner = hugging(&app, &["a", "b"]);
    assert_eq!(app.rect("inner"), inner);
    assert!(inner.width > HUG.width);
    assert_eq!(app.rect("outer"), hugging(&app, &["inner"]));
    app.assert_undo_returns_to_start();
}

#[test]
fn escape_mid_drag_puts_the_group_back() {
    let mut app = board();
    app.press((350.0, 150.0)).drag_to((550.0, 150.0));
    app.key(Key::Escape);
    assert_eq!(app.rect("g"), HUG);
    app.release();
    assert!(!app.app().can_undo());
}

#[test]
fn a_loosely_sized_group_is_left_alone_until_a_member_changes() {
    let loose = Rect::new(0.0, 0.0, 1000.0, 800.0);
    let mut app = TestApp::with_entities([
        inside("g", shape("a", A)),
        named(group("g", loose)),
        shape("c", Rect::new(1200.0, 100.0, 100.0, 100.0)),
    ]);
    assert_eq!(app.rect("g"), loose);
    assert!(!app.app().can_undo());
    app.select(&["c"]).key(Key::ArrowRight);
    assert_eq!(
        app.rect("g"),
        loose,
        "an unrelated change does not touch it"
    );
    app.undo();
    app.press((150.0, 150.0)).drag_to((250.0, 150.0)).release();
    assert_eq!(
        app.rect("g"),
        Rect::new(76.0, 76.0, 148.0, 148.0).translated(100.0, 0.0)
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn moving_a_group_whole_does_not_refit_it() {
    let loose = Rect::new(0.0, 0.0, 1000.0, 800.0);
    let mut app = TestApp::with_entities([inside("g", shape("a", A)), named(group("g", loose))]);
    app.select(&["g"])
        .press((10.0, -10.0))
        .drag_to((110.0, -10.0))
        .release();
    assert_eq!(app.rect("g"), loose.translated(100.0, 0.0));
    assert_eq!(app.rect("a"), A.translated(100.0, 0.0));
    app.assert_undo_returns_to_start();

    let mut app = board();
    app.select(&["g"]).key(Key::ArrowDown);
    assert_eq!(app.rect("g"), HUG.translated(0.0, 5.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn resizing_a_group_by_its_handle_keeps_the_size_it_was_given() {
    let mut app = board();
    app.select(&["g"])
        .press((425.0, 225.0))
        .drag_to((525.0, 325.0))
        .release();
    let rect = app.rect("g");
    assert!(rect.width > HUG.width && rect.height > HUG.height);
    app.assert_undo_returns_to_start();
}
