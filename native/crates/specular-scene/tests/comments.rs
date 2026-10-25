//! Scene snapshots of comments: the marks at rest and focused, the markers
//! and composer of a comment being written, and a region drag in flight.
//! Text is measured by the testkit's `FixedAdvance`: 10 units a character.

use specular_doc::{Annotation, AnnotationAnchor, EntityId, PageAnchor, Rect, RegionAnchor};
use specular_testkit::{TestApp, assert_scene_snapshot, comment, document, pages, with_comment};

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

/// Two pages, `p1` at (100, 100) and `p2` at (700, 100), 400x300 each, with
/// `comments` in the document.
fn app_with(comments: impl IntoIterator<Item = Annotation>) -> TestApp {
    let mut document = document(pages(2));
    for annotation in comments {
        document = with_comment(document, annotation);
    }
    TestApp::from_document(document)
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
