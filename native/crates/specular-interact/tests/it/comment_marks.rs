//! The marks that stand for comments: which comments have one, where it
//! sits for each anchor, how comments share a pill, and what a press hits.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationStatus, EntityId, PageAnchor, Rect, RegionAnchor,
};
use specular_interact::{CommentMark, Hit, Key, MarkShape, ScreenRect, Tool, hit_test};
use specular_testkit::{TestApp, comment, document, page, pages, shape, with_comment};

/// `p1` is at (100, 100), 400x300, so its right edge is at x = 500.
const P1: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

fn page_point(offset_y: f64) -> AnnotationAnchor {
    AnnotationAnchor::Page {
        page_id: EntityId::from("p1"),
        offset_x: 0.5,
        offset_y,
    }
}

fn element(selector: &str, bounding_box: Option<Rect>) -> AnnotationAnchor {
    AnnotationAnchor::Element {
        page_id: EntityId::from("p1"),
        selector: selector.to_owned(),
        element_path: None,
        bounding_box,
    }
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

/// `annotation` bound to `page`'s document as it was at `url`.
fn bound(annotation: Annotation, page: &str, url: Option<&str>) -> Annotation {
    let binding = PageAnchor {
        page_url: url.map(str::to_owned),
        ..PageAnchor::new(EntityId::from(page))
    };
    Annotation {
        page_anchor: Some(binding),
        ..annotation
    }
}

fn app_with(comments: impl IntoIterator<Item = Annotation>) -> TestApp {
    let mut document = document(pages(2));
    for annotation in comments {
        document = with_comment(document, annotation);
    }
    TestApp::from_document(document)
}

fn marks(app: &TestApp) -> Vec<CommentMark> {
    app.app().comment_marks()
}

/// The pill of the only mark, as `(x, y, width, height)`.
fn pill(app: &TestApp) -> (f32, f32, f32, f32) {
    let marks = marks(app);
    let [mark] = marks.as_slice() else {
        panic!("expected one mark, got {marks:?}");
    };
    match mark.shape {
        MarkShape::Badge(rect) => numbers(rect),
        MarkShape::Region(_) => panic!("expected a pill, got {mark:?}"),
    }
}

fn numbers(rect: ScreenRect) -> (f32, f32, f32, f32) {
    (rect.min.x, rect.min.y, rect.size.x, rect.size.y)
}

fn hit_comment(app: &TestApp, at: (f32, f32)) -> Option<String> {
    match hit_test(app.app(), at.into()) {
        Hit::Comment { annotation } => Some(annotation.as_str().to_owned()),
        _ => None,
    }
}

#[test]
fn a_pill_sits_by_its_anchor_and_stays_inside_its_page() {
    let bounds = Rect::new(50.0, 40.0, 100.0, 30.0);
    let wide = Rect::new(300.0, 0.0, 400.0, 50.0);
    let below = Rect::new(0.0, 500.0, 50.0, 50.0);
    let left_of = Rect::new(-200.0, 40.0, 50.0, 30.0);
    let above = Rect::new(50.0, -100.0, 100.0, 30.0);
    let cases = [
        ("page, halfway down", page_point(0.5), (466.0, 237.0)),
        ("page, top", page_point(0.0), (466.0, 97.0)),
        ("page, bottom", page_point(1.0), (466.0, 377.0)),
        ("element's box", element("#a", Some(bounds)), (216.0, 148.0)),
        (
            "element wider than the page",
            element("#a", Some(wide)),
            (466.0, 108.0),
        ),
        (
            "element below the page",
            element("#a", Some(below)),
            (116.0, 392.0),
        ),
        (
            "element left of the page",
            element("#a", Some(left_of)),
            (82.0, 148.0),
        ),
        (
            "element above the page",
            element("#a", Some(above)),
            (216.0, 108.0),
        ),
        ("element with no box", element("#a", None), (466.0, 108.0)),
    ];
    for (name, anchor, (x, y)) in cases {
        let app = app_with([comment("a", anchor, "a")]);
        assert_eq!(pill(&app), (x, y, 26.0, 26.0), "{name}");
    }
}

#[test]
fn a_comment_on_a_point_has_a_pill_centred_on_it() {
    let mut app = app_with([comment("a", canvas_point(600.0, 500.0), "a")]);
    assert_eq!(pill(&app), (587.0, 487.0, 26.0, 26.0));
    // The pill is chrome: it keeps its size at any zoom.
    app.zoom(2.0);
    let (_, _, width, height) = pill(&app);
    assert_eq!((width, height), (26.0, 26.0));
}

#[test]
fn a_region_is_its_frame_and_never_smaller_than_four_pixels() {
    let app = app_with([
        comment(
            "a",
            canvas_region(Rect::new(600.0, 500.0, 100.0, 80.0)),
            "a",
        ),
        comment("b", canvas_region(Rect::new(800.0, 500.0, 1.0, 1.0)), "b"),
    ]);
    let shapes: Vec<_> = marks(&app)
        .into_iter()
        .map(|mark| match mark.shape {
            MarkShape::Region(frame) => numbers(frame),
            MarkShape::Badge(_) => panic!("a region is a frame"),
        })
        .collect();
    // Back to front: the newer of two made together is in front.
    assert_eq!(
        shapes,
        [(800.0, 500.0, 4.0, 4.0), (600.0, 500.0, 100.0, 80.0)]
    );
}

#[test]
fn a_region_is_hit_on_its_edge_and_not_in_its_inside() {
    let app = app_with([comment(
        "a",
        canvas_region(Rect::new(600.0, 500.0, 100.0, 80.0)),
        "a",
    )]);
    let hit = |at| hit_comment(&app, at).is_some();
    // Six pixels either side of the edge.
    assert!(hit((600.0, 540.0)));
    assert!(hit((595.0, 540.0)));
    assert!(hit((605.0, 540.0)));
    assert!(hit((650.0, 505.0)));
    assert!(hit((650.0, 584.0)));
    assert!(!hit((650.0, 540.0)), "the inside stays reachable");
    // The band reaches six pixels in as well.
    assert!(hit((605.0, 540.0)));
    assert!(!hit((607.0, 540.0)));
    assert!(!hit((593.0, 540.0)));
    assert!(!hit((650.0, 587.0)));
}

#[test]
fn a_pill_is_hit_anywhere_on_it_and_hides_what_is_under_it() {
    let app = app_with([comment("a", page_point(0.5), "a")]);
    assert_eq!(hit_comment(&app, (480.0, 250.0)).as_deref(), Some("a"));
    assert_eq!(hit_comment(&app, (467.0, 238.0)).as_deref(), Some("a"));
    assert_eq!(hit_comment(&app, (460.0, 250.0)), None);
    // Just off each side of the 26 by 26 pill at (466, 237).
    for at in [
        (464.0, 250.0),
        (494.0, 250.0),
        (480.0, 235.0),
        (480.0, 265.0),
    ] {
        assert_eq!(hit_comment(&app, at), None, "{at:?}");
    }
    assert!(matches!(
        hit_test(app.app(), (460.0, 250.0).into()),
        Hit::PageContent { .. }
    ));
}

#[test]
fn where_pills_overlap_a_press_takes_the_newest() {
    let older = comment("old", canvas_point(600.0, 500.0), "old");
    let mut newer = comment("new", canvas_point(610.0, 500.0), "new");
    newer.created_at = "2026-02-01T00:00:00.000Z".to_owned();
    let app = app_with([older, newer]);
    assert_eq!(hit_comment(&app, (605.0, 500.0)).as_deref(), Some("new"));
    assert_eq!(hit_comment(&app, (590.0, 500.0)).as_deref(), Some("old"));
}

#[test]
fn a_resolved_or_dismissed_comment_has_no_mark_and_cannot_be_hit() {
    let mut resolved = comment("a", page_point(0.5), "a");
    resolved.status = AnnotationStatus::Resolved;
    let mut dismissed = comment("b", page_point(0.2), "b");
    dismissed.status = AnnotationStatus::Dismissed;
    let mut acknowledged = comment("c", canvas_point(600.0, 500.0), "c");
    acknowledged.status = AnnotationStatus::Acknowledged;
    let app = app_with([resolved, dismissed, acknowledged]);
    let ids: Vec<_> = marks(&app)
        .iter()
        .map(|mark| mark.annotation.as_str().to_owned())
        .collect();
    assert_eq!(ids, ["c"], "open means pending or acknowledged");
    assert_eq!(hit_comment(&app, (480.0, 250.0)), None);
}

#[test]
fn a_comment_bound_to_a_document_the_page_no_longer_shows_has_no_mark() {
    let url = |url| {
        let made = comment("a", page_point(0.5), "a");
        app_with([bound(made, "p1", Some(url))])
    };
    assert_eq!(marks(&url("https://example.com/other")).len(), 0);
    assert_eq!(marks(&url("https://example.com/p1?tab=2")).len(), 0);
    // Only the hash differs: the same document.
    assert_eq!(marks(&url("https://example.com/p1#top")).len(), 1);
    // Recorded with no URL: it matches whatever the page shows.
    let made = comment("a", page_point(0.5), "a");
    assert_eq!(marks(&app_with([bound(made, "p1", None)])).len(), 1);
}

#[test]
fn a_comment_bound_to_a_page_that_is_gone_has_no_mark() {
    let region = canvas_region(Rect::new(600.0, 500.0, 100.0, 80.0));
    let made = bound(comment("a", region, "a"), "p9", None);
    assert_eq!(marks(&app_with([made])).len(), 0);
    let made = comment("a", element("#a", None), "a");
    let mut app = app_with([made]);
    assert_eq!(marks(&app).len(), 1);
    app.select(&["p1"]).key(Key::Backspace);
    assert_eq!(marks(&app).len(), 0);
    app.undo();
    assert_eq!(marks(&app).len(), 1);
    app.assert_undo_returns_to_start();
}

#[test]
fn marks_follow_their_page_when_it_moves() {
    let page_region = Annotation {
        anchor: AnnotationAnchor::Region(RegionAnchor::Document {
            doc_rect: Rect::new(50.0, 40.0, 100.0, 30.0),
        }),
        ..bound(comment("r", canvas_point(0.0, 0.0), "r"), "p1", None)
    };
    let mut app = app_with([
        comment(
            "e",
            element("#a", Some(Rect::new(50.0, 40.0, 100.0, 30.0))),
            "e",
        ),
        page_region,
    ]);
    let before: Vec<_> = marks(&app).into_iter().map(|mark| mark.shape).collect();
    app.drag((300.0, 350.0), (500.0, 450.0));
    assert_eq!(app.rect("p1"), P1.translated(200.0, 100.0));
    let moved = |shape: MarkShape| match shape {
        MarkShape::Badge(rect) => MarkShape::Badge(shifted(rect)),
        MarkShape::Region(rect) => MarkShape::Region(shifted(rect)),
    };
    let after: Vec<_> = marks(&app).into_iter().map(|mark| mark.shape).collect();
    assert_eq!(after, before.into_iter().map(moved).collect::<Vec<_>>());
    app.assert_undo_returns_to_start();
}

fn shifted(rect: ScreenRect) -> ScreenRect {
    ScreenRect {
        min: rect.min + glam::Vec2::new(200.0, 100.0),
        ..rect
    }
}

#[test]
fn comments_on_one_element_share_a_pill_counting_every_message() {
    let bounds = Some(Rect::new(50.0, 40.0, 100.0, 30.0));
    let older = comment("old", element("#a", bounds), "old");
    let mut newer = comment("new", element("#a", bounds), "new");
    newer.created_at = "2026-02-01T00:00:00.000Z".to_owned();
    newer.replies = vec![specular_doc::Reply {
        author: specular_doc::Author::Agent,
        text: "on it".to_owned(),
        timestamp: "2026-02-01T00:01:00.000Z".to_owned(),
        extra: specular_doc::JsonMap::new(),
    }];
    let other = comment("other", element("#b", bounds), "other");
    // The same selector with another box is another element's place.
    let moved = comment(
        "moved",
        element("#a", Some(Rect::new(50.0, 200.0, 100.0, 30.0))),
        "moved",
    );
    // Points and regions never share a pill, even on the same spot.
    let spot_a = comment("spot-a", canvas_point(600.0, 500.0), "spot-a");
    let spot_b = comment("spot-b", canvas_point(600.0, 500.0), "spot-b");
    let app = app_with([older, newer, other, moved, spot_a, spot_b]);
    let marks = marks(&app);
    let shared = marks
        .iter()
        .find(|mark| mark.members.len() == 2)
        .unwrap_or_else(|| panic!("no shared pill in {marks:?}"));
    assert_eq!(
        shared.annotation.as_str(),
        "new",
        "the newest speaks for it"
    );
    assert_eq!(
        (
            shared
                .members
                .iter()
                .map(specular_doc::AnnotationId::as_str)
                .collect::<Vec<_>>(),
            shared.count,
            marks.len()
        ),
        (vec!["new", "old"], 3, 5)
    );
}

#[test]
fn the_comment_being_written_is_not_a_mark() {
    let mut app = TestApp::with_entities([
        page("p1", P1),
        shape("s", Rect::new(600.0, 500.0, 50.0, 50.0)),
    ]);
    app.tool(Tool::Comment).click((800.0, 700.0));
    assert!(app.app().comment_draft().is_some());
    assert_eq!(marks(&app).len(), 0);
}
