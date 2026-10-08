//! Page anchoring on placement: an entity whose centre lands on a page's
//! body is hooked to that page and moves with it.

use specular_doc::{Entity, EntityId, Kind, PageAnchor, Rect};
use specular_interact::{Key, Tool};
use specular_testkit::{ALT, CMD, CTRL, TestApp, page, shape};

/// The page of `TestApp::with_pages(1)`.
const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

/// The entity the last placement left selected.
#[track_caller]
fn placed(app: &TestApp) -> &Entity {
    app.entity(app.selected().unwrap_or("nothing is selected"))
}

/// The page each drawing is anchored to, back-to-front.
fn drawing_anchors(app: &TestApp) -> Vec<Option<&str>> {
    app.document()
        .entities()
        .filter(|entity| matches!(entity.kind, Kind::Drawing(_)))
        .map(|entity| entity.anchor.as_ref().map(|a| a.page_id.as_str()))
        .collect()
}

#[test]
fn a_sticky_whose_centre_lands_on_a_page_is_anchored_and_moves_with_the_page() {
    let mut app = TestApp::with_pages(1);
    app.tool(Tool::AddSticky).click((200.0, 160.0));
    let id = placed(&app).id.clone();
    let anchor = placed(&app).anchor.clone().expect("an anchor");
    assert_eq!(anchor.page_id.as_str(), "p1");
    assert_eq!(anchor.page_url.as_deref(), Some("https://example.com/p1"));
    // It records the page's scroll, so the entity follows it.
    assert_eq!((anchor.scroll_x, anchor.scroll_y), (Some(0.0), Some(0.0)));

    // The sticky covers the press point, so the page is grabbed beside it.
    // An empty sticky would not outlive its edit.
    app.type_text("note")
        .key(Key::Escape)
        .key(Key::Escape)
        .drag((450.0, 380.0), (550.0, 480.0));
    assert_eq!(app.rect("p1"), P1.translated(100.0, 100.0));
    assert_eq!(
        app.rect(id.as_str()),
        Rect::new(300.0, 260.0, 200.0, 200.0),
        "the sticky went with it"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_shape_anchors_by_where_its_centre_is_not_where_the_drag_began() {
    let mut app = TestApp::with_pages(1);
    // Starts off the page, centre (140, 140) on it.
    app.tool(Tool::AddShape).drag((60.0, 60.0), (220.0, 220.0));
    assert_eq!(
        placed(&app).anchor.as_ref().map(|a| a.page_id.as_str()),
        Some("p1")
    );
    // Starts on the page, centre (560, 200) off it.
    app.tool(Tool::AddShape)
        .drag((480.0, 120.0), (640.0, 280.0));
    assert_eq!(placed(&app).anchor, None);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_stroke_drawn_on_a_page_is_anchored_to_it_and_one_on_the_canvas_is_free() {
    let mut app = TestApp::with_pages(1);
    app.tool(Tool::Draw)
        .drag((150.0, 150.0), (250.0, 250.0))
        .drag((700.0, 700.0), (800.0, 800.0));
    assert_eq!(drawing_anchors(&app), [Some("p1"), None]);
    app.assert_undo_returns_to_start();
}

// Re-anchoring when a move ends (ADR 0031: placement decides).

/// Pages `p1` and `p2` (400x300 at (100, 100) and (700, 100)), and a shape
/// `s` of 100x100 at `at`, hooked to `hooked` if given.
fn with_shape(at: (f64, f64), hooked: Option<&str>) -> TestApp {
    let mut s = shape("s", Rect::new(at.0, at.1, 100.0, 100.0));
    s.anchor = hooked.map(|id| PageAnchor {
        page_url: Some(format!("https://example.com/{id}")),
        scroll_x: Some(7.0),
        ..PageAnchor::new(EntityId::new(id))
    });
    TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        page("p2", Rect::new(700.0, 100.0, 400.0, 300.0)),
        s,
    ])
}

fn anchor_of(app: &TestApp) -> Option<&str> {
    app.entity("s").anchor.as_ref().map(|a| a.page_id.as_str())
}

#[test]
fn dragging_onto_a_page_hooks_to_it_in_the_same_step() {
    let mut app = with_shape((300.0, 600.0), None);
    app.drag((350.0, 650.0), (350.0, 250.0));
    assert_eq!(app.rect("s"), Rect::new(300.0, 200.0, 100.0, 100.0));
    assert_eq!(anchor_of(&app), Some("p1"));
    app.undo();
    assert!(!app.app().can_undo(), "the move and the hook are one step");
    assert_eq!(anchor_of(&app), None);
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn dragging_from_one_page_to_another_moves_the_hook() {
    let mut app = with_shape((200.0, 200.0), Some("p1"));
    app.drag((250.0, 250.0), (850.0, 250.0));
    assert_eq!(anchor_of(&app), Some("p2"));
    assert_eq!(
        app.entity("s")
            .anchor
            .as_ref()
            .and_then(|a| a.page_url.as_deref()),
        Some("https://example.com/p2")
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn command_or_control_at_the_release_keeps_every_anchor() {
    for held in [CMD, CTRL] {
        let mut app = with_shape((300.0, 600.0), None);
        app.press((350.0, 650.0)).drag_to((350.0, 250.0));
        app.hold(held).release().let_go();
        assert_eq!(anchor_of(&app), None, "dropped on a page, not hooked");
        app.assert_undo_returns_to_start();

        let mut app = with_shape((200.0, 200.0), Some("p1"));
        app.press((250.0, 250.0)).drag_to((250.0, 650.0));
        app.hold(held).release().let_go();
        assert_eq!(anchor_of(&app), Some("p1"), "dragged off, still hooked");
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn an_item_dragged_with_its_page_stays_hooked_even_under_a_page_in_front() {
    let mut s = shape("s", Rect::new(400.0, 160.0, 100.0, 100.0));
    s.anchor = Some(PageAnchor::new(EntityId::new("p1")));
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        page("p2", Rect::new(520.0, 100.0, 400.0, 300.0)),
        s,
    ]);
    app.select(&["p1", "s"])
        .drag((450.0, 210.0), (550.0, 210.0));
    assert_eq!(app.rect("s").x, 500.0);
    assert_eq!(anchor_of(&app), Some("p1"), "p2 is in front but p1 owns it");
    app.assert_undo_returns_to_start();
}

#[test]
fn an_option_drag_copy_is_hooked_to_the_page_it_lands_on() {
    let mut app = with_shape((200.0, 200.0), Some("p1"));
    app.hold(ALT).drag((250.0, 250.0), (850.0, 250.0)).let_go();
    let copy = app.selected().expect("the copy is selected").to_owned();
    assert_ne!(copy, "s");
    assert_eq!(
        app.entity(&copy)
            .anchor
            .as_ref()
            .map(|a| a.page_id.as_str()),
        Some("p2")
    );
    assert_eq!(anchor_of(&app), Some("p1"), "the original is untouched");
    app.assert_undo_returns_to_start();
}
