//! What a page's scroll does to what is drawn on it: anchored entities and
//! comments are shifted, clipped to the page, and hidden once out of it.

use specular_doc::{PageAnchor, Rect};
use specular_interact::{Key, PageNotice, Tool};
use specular_testkit::{TestApp, assert_scene_snapshot, page, shape};

const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

fn followed(rect: Rect, at: Option<f64>) -> TestApp {
    let mut s = shape("s", rect);
    s.anchor = Some(PageAnchor {
        scroll_x: at.map(|_| 0.0),
        scroll_y: at,
        ..PageAnchor::new("p1".into())
    });
    TestApp::with_entities([page("p1", P1), s])
}

fn scroll(app: &mut TestApp, y: f64) {
    app.page_reports("p1", PageNotice::Scrolled { x: 0.0, y });
}

#[test]
fn an_anchored_shape_draws_shifted_by_the_scroll() {
    let mut app = followed(Rect::new(200.0, 200.0, 100.0, 100.0), Some(0.0));
    scroll(&mut app, 40.0);
    assert_scene_snapshot!(app);
}

#[test]
fn a_shape_leaving_the_page_is_clipped_to_it() {
    let mut app = followed(Rect::new(200.0, 120.0, 100.0, 100.0), Some(0.0));
    scroll(&mut app, 60.0);
    assert_scene_snapshot!(app);
}

#[test]
fn a_shape_scrolled_out_of_the_page_is_not_drawn() {
    let mut app = followed(Rect::new(200.0, 120.0, 100.0, 60.0), Some(0.0));
    scroll(&mut app, 200.0);
    assert_scene_snapshot!(app);
}

#[test]
fn a_shape_with_no_recorded_scroll_stays_pinned() {
    let mut app = followed(Rect::new(200.0, 200.0, 100.0, 100.0), None);
    scroll(&mut app, 40.0);
    assert_scene_snapshot!(app);
}

#[test]
fn the_selection_of_a_shifted_shape_wraps_where_it_is_seen() {
    let mut app = followed(Rect::new(200.0, 200.0, 100.0, 100.0), Some(0.0));
    scroll(&mut app, 40.0);
    app.select(&["s"]);
    assert_scene_snapshot!(app);
}

#[test]
fn a_region_comment_on_a_page_follows_the_scroll_and_is_clipped_to_the_page() {
    let mut app = TestApp::with_entities([page("p1", P1)]);
    app.tool(Tool::Comment)
        .tick(86_400_000)
        .drag((150.0, 150.0), (250.0, 250.0))
        .answer_grab(&[1])
        .type_text("tighten")
        .key(Key::Enter);
    scroll(&mut app, 80.0);
    assert_scene_snapshot!(app);
}
