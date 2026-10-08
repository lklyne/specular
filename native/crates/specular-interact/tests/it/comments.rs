//! The comment tool: a click comments on a point or on a page's element, a
//! drag on a region, and each ends as one annotation and one undo step.

use glam::Vec2;
use specular_core::{CssSize, synthetic_element_at};
use specular_doc::{AnnotationAnchor, EntityId, Rect, RegionAnchor};
use specular_interact::{Effect, Gesture, Key, PageRegion, Tool};
use specular_testkit::{TestApp, assert_doc_snapshot, page, shape, sticky};

const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

/// Two pages with the comment tool armed: `p1` at (100, 100) and `p2` at
/// (700, 100), each 400x300.
fn armed() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.tool(Tool::Comment).take_effects();
    app
}

/// What the comment being written is on.
fn draft_anchor(app: &TestApp) -> AnnotationAnchor {
    app.comment_draft().anchor.clone()
}

/// The page the comment being written is bound to.
fn draft_page(app: &TestApp) -> Option<&str> {
    (app.comment_draft().page_anchor.as_ref()).map(|anchor| anchor.page_id.as_str())
}

fn canvas_point(x: f64, y: f64) -> AnnotationAnchor {
    AnnotationAnchor::Canvas {
        canvas_x: x,
        canvas_y: y,
    }
}

fn canvas_region(rect: Rect) -> AnnotationAnchor {
    AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect: rect })
}

fn covered(page: &str, rect: Rect) -> PageRegion {
    PageRegion {
        page: EntityId::from(page),
        rect,
    }
}

#[test]
fn a_click_on_empty_canvas_writes_a_comment_on_that_point() {
    let mut app = armed();
    app.tick(86_400_000).click((600.0, 500.0));
    assert_eq!(
        (draft_anchor(&app), app.document().annotations().len()),
        (canvas_point(600.0, 500.0), 0)
    );
    app.type_text("move this").key(Key::Enter);
    let id = app.document().annotations()[0].id.clone();
    assert_eq!(
        (app.app().comment_draft(), app.app().focused_comment()),
        (None, Some(&id))
    );
    assert_doc_snapshot!(app);
    app.undo();
    assert_eq!(
        (app.document().annotations().len(), app.app().can_undo()),
        (0, false),
        "the comment is one step"
    );
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn a_click_on_a_sticky_or_a_shape_is_a_canvas_point_bound_to_nothing() {
    // The sticky lies on the page: what the click is on decides, not what is
    // under that.
    let mut app = TestApp::with_entities([
        page("p1", P1),
        sticky("s", Rect::new(200.0, 150.0, 100.0, 100.0), "note"),
        shape("r", Rect::new(600.0, 500.0, 100.0, 100.0)),
    ]);
    app.tool(Tool::Comment).take_effects();
    app.click((250.0, 200.0));
    assert_eq!(
        (draft_anchor(&app), draft_page(&app)),
        (canvas_point(250.0, 200.0), None)
    );
    app.click((650.0, 550.0));
    assert_eq!(
        (draft_anchor(&app), draft_page(&app), app.selected()),
        (canvas_point(650.0, 550.0), None, None)
    );
    let asked = (app.effects().iter()).any(|effect| matches!(effect, Effect::QueryElement { .. }));
    assert!(!asked, "no page is asked about a click that is not on one");
}

#[test]
fn a_click_on_a_page_asks_for_the_element_there_and_comments_on_it() {
    let mut app = armed();
    app.tick(86_400_000).click((200.0, 200.0));
    let point = Vec2::new(100.0, 100.0);
    assert_eq!(
        (app.effects(), app.app().comment_draft(), app.selected()),
        (
            &[Effect::QueryElement {
                page: "p1".into(),
                point
            }][..],
            None,
            None
        ),
        "the page is asked in its own pixels, and is neither selected nor entered"
    );
    app.answer_element(synthetic_element_at(CssSize::new(400, 300), point));
    assert_eq!(
        (draft_anchor(&app), draft_page(&app)),
        (
            AnnotationAnchor::Element {
                page_id: "p1".into(),
                selector: "div.cell[data-col=\"0\"][data-row=\"2\"]".to_owned(),
                element_path: Some("body > div.cell".to_owned()),
                bounding_box: Some(Rect::new(0.0, 96.0, 160.0, 48.0)),
            },
            Some("p1")
        )
    );
    app.type_text("too small").key(Key::Enter);
    assert_doc_snapshot!(app);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_page_with_no_element_under_the_click_gets_a_canvas_point() {
    let mut app = armed();
    app.click((200.0, 200.0)).answer_element(None);
    assert_eq!(
        (draft_anchor(&app), draft_page(&app)),
        (canvas_point(200.0, 200.0), None)
    );
}

#[test]
fn a_drag_on_empty_canvas_writes_a_comment_on_that_region_of_the_canvas() {
    let mut app = armed();
    app.tick(86_400_000).drag((650.0, 560.0), (550.0, 500.0));
    let asked =
        (app.effects().iter()).any(|effect| matches!(effect, Effect::QueryRegionGrab { .. }));
    assert_eq!(
        (draft_anchor(&app), draft_page(&app), asked),
        (
            canvas_region(Rect::new(550.0, 500.0, 100.0, 60.0)),
            None,
            false
        ),
        "no page is under it, so none is asked"
    );
    app.type_text("group these").key(Key::Enter);
    assert_doc_snapshot!(app);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_drag_over_pages_asks_each_what_it_grabbed_front_to_back() {
    let mut app = armed();
    app.drag((450.0, 150.0), (750.0, 250.0));
    assert_eq!(
        (app.effects(), app.app().comment_draft()),
        (
            &[Effect::QueryRegionGrab {
                region: Rect::new(450.0, 150.0, 300.0, 100.0),
                pages: vec![
                    covered("p2", Rect::new(0.0, 50.0, 50.0, 100.0)),
                    covered("p1", Rect::new(350.0, 50.0, 50.0, 100.0)),
                ],
            }][..],
            None
        )
    );
}

#[test]
fn a_region_is_bound_to_the_first_page_it_grabbed_from() {
    let mut app = armed();
    // It lies over `p2` and grabs nothing there.
    app.tick(86_400_000)
        .drag((450.0, 150.0), (750.0, 250.0))
        .answer_grab(&[0, 2]);
    assert_eq!(
        (draft_anchor(&app), draft_page(&app)),
        (
            AnnotationAnchor::Region(RegionAnchor::Document {
                doc_rect: Rect::new(350.0, 50.0, 300.0, 100.0)
            }),
            Some("p1")
        )
    );
    app.type_text("fix the header").key(Key::Enter);
    assert_doc_snapshot!(app);
    app.assert_undo_returns_to_start();

    // A region that grabbed nothing from either page stays on the canvas.
    let mut app = armed();
    app.drag((450.0, 150.0), (750.0, 250.0))
        .answer_grab(&[0, 0]);
    assert_eq!(
        (draft_anchor(&app), draft_page(&app)),
        (canvas_region(Rect::new(450.0, 150.0, 300.0, 100.0)), None)
    );
}

#[test]
fn a_press_is_a_click_until_it_has_travelled_four_pixels() {
    let mut app = armed();
    app.press((600.0, 500.0)).drag_to((603.5, 500.0));
    assert_eq!(app.session().comment_preview(), None);
    app.release();
    assert_eq!(draft_anchor(&app), canvas_point(600.0, 500.0));

    app.press((800.0, 700.0)).drag_to((804.0, 700.0));
    assert!(matches!(app.session().gesture, Some(Gesture::Comment(_))));
    assert_eq!(
        app.session().comment_preview(),
        Some(Rect::new(800.0, 700.0, 4.0, 0.0))
    );
    // Coming back does not make it a click again.
    app.drag_to((801.0, 701.0)).release();
    assert_eq!(
        draft_anchor(&app),
        canvas_region(Rect::new(800.0, 700.0, 1.0, 1.0))
    );
}

#[test]
fn an_answer_that_finds_a_drag_in_flight_is_dropped() {
    let mut app = armed();
    app.click((200.0, 200.0)).press((600.0, 500.0));
    app.answer_element(None);
    assert_eq!(app.app().comment_draft(), None);
    app.drag_to((700.0, 600.0)).release();
    assert_eq!(
        draft_anchor(&app),
        canvas_region(Rect::new(600.0, 500.0, 100.0, 100.0))
    );
}
