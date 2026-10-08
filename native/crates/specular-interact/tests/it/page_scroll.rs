//! Page scroll: comments on a page are stored in its document and follow its
//! scroll, and an entity anchored to a page is seen, hit and moved where the
//! scroll has put it.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use glam::Vec2;
use specular_core::{CssSize, synthetic_element_at};
use specular_doc::{AnnotationAnchor, Entity, PageAnchor, Rect, RegionAnchor};
use specular_interact::{Hit, MarkShape, PageNotice, Tool, element_on_canvas, hit_test};
use specular_testkit::{TestApp, page, shape};

/// The page, 400x300 at (100, 100); its CSS pixels are canvas units.
const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

fn scrolled(app: &mut TestApp, y: f64) {
    app.page_reports("p1", PageNotice::Scrolled { x: 0.0, y });
}

/// A shape `s` at `rect`, anchored to `p1` at a scroll of `at`.
fn followed(rect: Rect, at: Option<f64>) -> Entity {
    let mut s = shape("s", rect);
    s.anchor = Some(PageAnchor {
        scroll_x: at.map(|_| 0.0),
        scroll_y: at,
        ..PageAnchor::new("p1".into())
    });
    s
}

fn app_with(entities: [Entity; 1]) -> TestApp {
    let [one] = entities;
    TestApp::with_entities([page("p1", P1), one])
}

/// The region of the only comment mark, on screen.
#[track_caller]
fn region_mark(app: &TestApp) -> Rect {
    let marks = app.app().comment_marks();
    let [mark] = marks.as_slice() else {
        panic!("one mark expected, found {marks:?}");
    };
    let MarkShape::Region(frame) = mark.shape else {
        panic!("a region expected: {mark:?}");
    };
    Rect::new(
        f64::from(frame.min.x),
        f64::from(frame.min.y),
        f64::from(frame.size.x),
        f64::from(frame.size.y),
    )
}

/// A comment on a region of `p1`, made with the page scrolled to `y`.
fn region_comment(y: f64) -> TestApp {
    let mut app = TestApp::with_pages(1);
    scrolled(&mut app, y);
    app.tool(Tool::Comment)
        .tick(86_400_000)
        .drag((150.0, 150.0), (250.0, 250.0))
        .answer_grab(&[1])
        .type_text("tighten")
        .key(specular_interact::Key::Enter);
    app
}

#[test]
fn a_region_made_on_a_scrolled_page_stores_document_coordinates() {
    let mut app = region_comment(200.0);
    let annotation = &app.document().annotations()[0];
    assert_eq!(
        annotation.anchor,
        AnnotationAnchor::Region(RegionAnchor::Document {
            doc_rect: Rect::new(50.0, 250.0, 100.0, 100.0)
        })
    );
    // It is drawn where it was made.
    assert_eq!(region_mark(&app), Rect::new(150.0, 150.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_region_scrolled_out_of_its_page_is_not_shown_and_comes_back() {
    let mut app = region_comment(200.0);
    // Its top is above the page: it still shows, clipped to the page.
    scrolled(&mut app, 300.0);
    assert_eq!(region_mark(&app), Rect::new(150.0, 50.0, 100.0, 100.0));
    let clip = app.app().comment_marks()[0].clip.expect("on a page");
    assert_eq!(
        (clip.min, clip.size),
        (Vec2::new(100.0, 100.0), Vec2::new(400.0, 300.0))
    );
    // Wholly above it, it is gone and cannot be pressed.
    scrolled(&mut app, 400.0);
    assert_eq!(app.app().comment_marks().len(), 0);
    scrolled(&mut app, 200.0);
    assert_eq!(region_mark(&app), Rect::new(150.0, 150.0, 100.0, 100.0));
}

#[test]
fn an_element_comment_follows_the_scroll_it_was_made_at() {
    let mut app = TestApp::with_pages(1);
    app.tool(Tool::Comment)
        .tick(86_400_000)
        .click((200.0, 200.0));
    let element = synthetic_element_at(CssSize::new(400, 300), Vec2::new(100.0, 100.0));
    app.answer_element(element)
        .type_text("small")
        .key(specular_interact::Key::Enter);
    let annotation = app.document().annotations()[0].clone();
    let at = |app: &TestApp| element_on_canvas(app.app(), &annotation).expect("a box");
    assert_eq!(at(&app), Rect::new(100.0, 196.0, 160.0, 48.0));
    scrolled(&mut app, 30.0);
    assert_eq!(at(&app), Rect::new(100.0, 166.0, 160.0, 48.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn an_element_comment_with_no_recorded_scroll_stays_pinned() {
    let json = r#"{"nodes":[{"id":"p1","type":"link","url":"https://example.com/p1","x":100,"y":100,"width":400,"height":300}],
        "edges":[],
        "annotations":[{"id":"a","anchor":{"type":"element","pageId":"p1","selector":"div","boundingBox":{"x":0,"y":96,"width":160,"height":48}},
        "author":"user","text":"x","status":"pending","replies":[],"createdAt":"2026-01-01T00:00:00.000Z","pageAnchor":{"pageId":"p1"}}]}"#;
    let mut app = TestApp::from_canvas(json);
    let annotation = app.document().annotations()[0].clone();
    scrolled(&mut app, 80.0);
    assert_eq!(
        element_on_canvas(app.app(), &annotation),
        Some(Rect::new(100.0, 196.0, 160.0, 48.0))
    );
}

#[test]
fn an_anchored_entity_is_seen_and_hit_where_the_scroll_put_it() {
    let mut app = app_with([followed(Rect::new(200.0, 200.0, 100.0, 100.0), Some(0.0))]);
    scrolled(&mut app, 40.0);
    // Its rect is as stored; it is seen 40 up.
    assert_eq!(app.rect("s"), Rect::new(200.0, 200.0, 100.0, 100.0));
    let at = |app: &TestApp, x, y| hit_test(app.app(), Vec2::new(x, y));
    assert_eq!(
        at(&app, 250.0, 180.0),
        Hit::EntityBody { entity: "s".into() }
    );
    assert!(matches!(at(&app, 250.0, 280.0), Hit::PageContent { .. }));
    // Selected, its outline and handles are around where it is seen.
    app.select(&["s"]);
    assert_eq!(
        app.app().handles().map(|(_, rect)| rect),
        Some(Rect::new(200.0, 160.0, 100.0, 100.0))
    );
}

#[test]
fn a_marquee_takes_what_it_sees() {
    let mut app = app_with([followed(Rect::new(200.0, 200.0, 100.0, 100.0), Some(0.0))]);
    scrolled(&mut app, 40.0);
    // Across the old place only: the page is taken, the shape is not.
    app.drag((50.0, 270.0), (310.0, 290.0));
    assert_eq!(app.selected_ids(), ["p1"]);
    app.drag((50.0, 150.0), (310.0, 170.0));
    assert!(app.selected_ids().contains(&"s"));
}

#[test]
fn a_drag_folds_the_scroll_into_the_rect_and_restamps_the_anchor() {
    let mut app = app_with([followed(Rect::new(200.0, 200.0, 100.0, 100.0), Some(0.0))]);
    scrolled(&mut app, 40.0);
    // Seen at y 160..260. Pressed there, moved 40 right.
    app.drag((250.0, 200.0), (290.0, 200.0));
    // Stored where it is seen, 40 to the right.
    assert_eq!(app.rect("s"), Rect::new(240.0, 160.0, 100.0, 100.0));
    let anchor = app.entity("s").anchor.clone().expect("still anchored");
    assert_eq!((anchor.scroll_x, anchor.scroll_y), (Some(0.0), Some(40.0)));
    // And it does not move until the page does.
    assert_eq!(
        hit_test(app.app(), Vec2::new(290.0, 200.0)),
        Hit::EntityBody { entity: "s".into() }
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_new_entity_records_the_scroll_it_was_placed_at() {
    let mut app = TestApp::with_pages(1);
    scrolled(&mut app, 120.0);
    app.tool(Tool::AddShape)
        .drag((200.0, 200.0), (300.0, 300.0));
    let anchor = app.entity(app.selected().expect("placed")).anchor.clone();
    assert_eq!(
        anchor.map(|a| (a.page_id, a.scroll_x, a.scroll_y)),
        Some(("p1".into(), Some(0.0), Some(120.0)))
    );
    // Seen where it was placed, then following.
    let id = app.selected().expect("placed").to_owned();
    assert_eq!(
        hit_test(app.app(), Vec2::new(250.0, 250.0)),
        Hit::EntityBody {
            entity: id.as_str().into()
        }
    );
    app.assert_undo_returns_to_start();
}
