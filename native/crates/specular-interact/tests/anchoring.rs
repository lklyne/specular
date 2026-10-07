//! Page anchoring on placement: an entity whose centre lands on a page's
//! body is hooked to that page and moves with it.

use specular_doc::{Entity, Kind, Rect};
use specular_interact::{Key, Tool};
use specular_testkit::TestApp;

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
    assert_eq!((anchor.scroll_x, anchor.scroll_y), (None, None));

    // The sticky covers the press point, so the page is grabbed beside it.
    app.key(Key::Escape)
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

#[test]
fn the_frontmost_page_under_the_centre_takes_the_anchor() {
    let mut app = TestApp::with_pages(1);
    // A second page over the first one's top-left corner.
    app.tool(Tool::AddPage).click((80.0, 80.0));
    let front = placed(&app).id.clone();
    assert_eq!(placed(&app).anchor, None, "a page never anchors");
    app.tool(Tool::AddShape)
        .drag((120.0, 120.0), (220.0, 220.0));
    assert_eq!(
        placed(&app).anchor.as_ref().map(|a| &a.page_id),
        Some(&front)
    );
    app.assert_undo_returns_to_start();
}
