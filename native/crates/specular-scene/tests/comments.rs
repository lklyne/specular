//! Scene snapshots of comments: the marks at rest and focused, the markers
//! and composer of a comment being written, and a region drag in flight.
//! Text is measured by the testkit's `FixedAdvance`: 10 units a character.

use specular_core::{PageElement, PixelRect};
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationStatus, EntityId, PageAnchor, Rect, RegionAnchor, Reply,
};
use specular_interact::{Action, Key, Tool};
use specular_testkit::{
    SHIFT, TestApp, assert_scene_snapshot, comment, document, pages, with_comment,
};

fn point(x: f64, y: f64) -> AnnotationAnchor {
    AnnotationAnchor::Canvas {
        canvas_x: x,
        canvas_y: y,
    }
}

fn region(rect: Rect) -> AnnotationAnchor {
    AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect: rect })
}

fn element(selector: &str) -> AnnotationAnchor {
    AnnotationAnchor::Element {
        page_id: EntityId::from("p1"),
        selector: selector.to_owned(),
        element_path: None,
        bounding_box: Some(Rect::new(50.0, 40.0, 100.0, 30.0)),
    }
}

fn bound(annotation: Annotation, url: Option<&str>) -> Annotation {
    Annotation {
        page_anchor: Some(PageAnchor {
            page_url: url.map(str::to_owned),
            ..PageAnchor::new(EntityId::from("p1"))
        }),
        ..annotation
    }
}

fn with_replies(mut annotation: Annotation, count: usize) -> Annotation {
    annotation.replies = (0..count)
        .map(|index| Reply {
            author: specular_doc::Author::Agent,
            text: format!("reply {index}"),
            timestamp: "2026-01-02T00:00:00.000Z".to_owned(),
            extra: specular_doc::JsonMap::new(),
        })
        .collect();
    annotation
}

/// Two pages, `p1` at (100, 100) and `p2` at (700, 100), 400x300 each, with
/// `comments` in the document.
fn app_with(comments: impl IntoIterator<Item = Annotation>) -> TestApp {
    let mut document = document(pages(2));
    for annotation in comments {
        document = with_comment(document, annotation);
    }
    TestApp::from_document(document)
}

fn focus(app: &mut TestApp, id: &str) {
    app.act(Action::FocusComment(Some(specular_doc::AnnotationId::new(
        id,
    ))));
}

fn drafting() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.tool(Tool::Comment);
    app
}

fn button(x: i32, y: i32, width: u32, height: u32) -> PageElement {
    PageElement {
        selector: "#buy".to_owned(),
        element_path: Some("body > button".to_owned()),
        bounding_box: PixelRect {
            x,
            y,
            width,
            height,
        },
    }
}

#[test]
fn every_mark_form_at_rest() {
    let app = app_with([
        comment("region", region(Rect::new(600.0, 500.0, 200.0, 100.0)), "r"),
        comment("point", point(950.0, 550.0), "p"),
        comment(
            "page",
            AnnotationAnchor::Page {
                page_id: EntityId::from("p1"),
                offset_x: 0.5,
                offset_y: 0.5,
            },
            "g",
        ),
        comment("element", element("#a"), "e"),
        bound(
            comment(
                "doc-region",
                AnnotationAnchor::Region(RegionAnchor::Document {
                    doc_rect: Rect::new(20.0, 200.0, 120.0, 60.0),
                }),
                "d",
            ),
            None,
        ),
    ]);
    assert_scene_snapshot!(app);
}

#[test]
fn a_focused_region() {
    let mut app = app_with([comment(
        "region",
        region(Rect::new(600.0, 500.0, 200.0, 100.0)),
        "r",
    )]);
    focus(&mut app, "region");
    assert_scene_snapshot!(app);
}

#[test]
fn a_focused_badge_has_a_ring() {
    let mut app = app_with([comment("point", point(600.0, 500.0), "p")]);
    focus(&mut app, "point");
    assert_scene_snapshot!(app);
}

#[test]
fn a_grouped_badge_counts_every_message_in_two_digits() {
    let app = app_with([
        with_replies(comment("a", element("#a"), "a"), 5),
        with_replies(comment("b", element("#a"), "b"), 4),
    ]);
    assert_scene_snapshot!(app);
}

#[test]
fn resolved_dismissed_and_off_url_comments_draw_nothing() {
    let mut resolved = comment("a", point(600.0, 500.0), "a");
    resolved.status = AnnotationStatus::Resolved;
    let mut dismissed = comment("b", region(Rect::new(600.0, 500.0, 50.0, 50.0)), "b");
    dismissed.status = AnnotationStatus::Dismissed;
    let off_url = bound(
        comment("c", region(Rect::new(700.0, 500.0, 50.0, 50.0)), "c"),
        Some("https://elsewhere.test/"),
    );
    let app = app_with([resolved, dismissed, off_url]);
    assert_scene_snapshot!(app);
}

#[test]
fn a_page_region_and_an_element_badge_follow_their_page() {
    let mut app = app_with([
        bound(
            comment(
                "doc-region",
                AnnotationAnchor::Region(RegionAnchor::Document {
                    doc_rect: Rect::new(20.0, 200.0, 120.0, 60.0),
                }),
                "d",
            ),
            None,
        ),
        comment("element", element("#a"), "e"),
    ]);
    assert_scene_snapshot!("before", app);
    app.drag((300.0, 350.0), (500.0, 450.0));
    assert_scene_snapshot!("after", app);
}

#[test]
fn a_point_draft_with_an_empty_composer_shows_the_placeholder() {
    let mut app = drafting();
    app.click((600.0, 500.0));
    assert_scene_snapshot!(app);
}

#[test]
fn a_point_draft_with_text_shows_the_text_and_a_caret() {
    let mut app = drafting();
    app.click((600.0, 500.0)).type_text("ship it");
    assert_scene_snapshot!(app);
}

#[test]
fn a_canvas_region_draft_with_an_empty_composer() {
    let mut app = drafting();
    app.drag((600.0, 500.0), (800.0, 580.0));
    assert_scene_snapshot!(app);
}

#[test]
fn a_canvas_region_draft_with_text() {
    let mut app = drafting();
    app.drag((600.0, 500.0), (800.0, 580.0));
    app.type_text("tighten this");
    assert_scene_snapshot!(app);
}

#[test]
fn an_element_draft_with_an_empty_composer() {
    let mut app = drafting();
    app.click((200.0, 200.0));
    app.answer_element(Some(button(40, 60, 120, 40)));
    assert_scene_snapshot!(app);
}

#[test]
fn an_element_draft_with_text() {
    let mut app = drafting();
    app.click((200.0, 200.0));
    app.answer_element(Some(button(40, 60, 120, 40)));
    app.type_text("wrong colour");
    assert_scene_snapshot!(app);
}

#[test]
fn a_composer_wraps_over_a_second_line_and_underlines_composing_text() {
    let mut app = drafting();
    app.click((600.0, 500.0));
    app.type_text("0123456789012345678901234 more");
    app.compose("に");
    assert_scene_snapshot!(app);
}

#[test]
fn selected_composer_text_is_a_rect_behind_it() {
    let mut app = drafting();
    app.click((600.0, 500.0)).type_text("abc");
    app.key(Key::Home);
    app.chord(SHIFT, Key::ArrowRight);
    assert_scene_snapshot!(app);
}

#[test]
fn a_region_drag_in_flight_looks_like_the_resting_draft_region() {
    let mut app = drafting();
    app.press((600.0, 500.0)).drag_to((800.0, 580.0));
    assert_scene_snapshot!("in_flight", app);
    app.release();
    assert_scene_snapshot!("resting", app);
}

#[test]
fn the_composer_keeps_its_pixel_size_zoomed_in() {
    let mut app = drafting();
    app.click((600.0, 500.0)).type_text("hi");
    app.zoom(2.0);
    assert_scene_snapshot!(app);
}
