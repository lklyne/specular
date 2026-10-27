//! The comment rules a scenario cannot reach in one place: where a pill
//! sits for each kind of anchor, and which item a comment on a selection
//! names as its target. The tool, the draft, the marks, focusing,
//! resolving and deleting are in `fixtures/scenarios/comment-tool.txt` and
//! `comment-marks.txt`.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::{Annotation, AnnotationAnchor, EntityId, Kind, Rect};
use specular_interact::{MarkShape, selection_metadata};
use specular_testkit::{
    TestApp, comment, document, file, group, inside, page, pages, sticky, with_comment,
};

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

/// Two pages, `p1` at (100, 100) and `p2` at (700, 100), with `comments`.
fn app_with(comments: impl IntoIterator<Item = Annotation>) -> TestApp {
    let mut document = document(pages(2));
    for annotation in comments {
        document = with_comment(document, annotation);
    }
    TestApp::from_document(document)
}

/// The one mark's pill, as x, y, width and height.
fn pill(app: &TestApp) -> (f32, f32, f32, f32) {
    let marks = app.app().comment_marks();
    let [mark] = marks.as_slice() else {
        panic!("expected one mark, got {marks:?}");
    };
    match mark.shape {
        MarkShape::Badge(rect) => (rect.min().x, rect.min().y, rect.extent().x, rect.extent().y),
        MarkShape::Region(_) => panic!("expected a pill, got {mark:?}"),
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
fn the_target_is_the_one_page_selected_or_else_the_one_file() {
    let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let document = document([
        group("g", rect),
        inside("g", page("in-group", rect)),
        page("p", rect),
        file("f", rect),
        file("f2", rect),
        sticky("s", rect, "note"),
    ]);
    let mut blank_page = page("blank-page", rect);
    if let Kind::Page(fields) = &mut blank_page.kind {
        fields.url.clear();
    }
    let mut blank_file = file("blank-file", rect);
    if let Kind::File(fields) = &mut blank_file.kind {
        fields.file.clear();
    }
    let document = {
        let mut entities: Vec<_> = document.entities().cloned().collect();
        entities.extend([blank_page, blank_file]);
        specular_testkit::document(entities)
    };
    let target = |ids: &[&str]| {
        let ids: Vec<EntityId> = ids.iter().map(|id| EntityId::from(*id)).collect();
        (selection_metadata(&document, &ids).get("selectionTarget")).map(ToString::to_string)
    };
    let page = |id: &str| {
        Some(format!(
            r#"{{"entityId":"{id}","kind":"page","url":"https://example.com/{id}"}}"#
        ))
    };
    assert_eq!(
        target(&["p", "s", "f"]),
        page("p"),
        "a page comes before a file"
    );
    assert_eq!(
        target(&["g"]),
        page("in-group"),
        "a group stands for its page"
    );
    assert_eq!(target(&["g", "p"]), None, "two pages");
    assert_eq!(
        target(&["f", "s"]).as_deref(),
        Some(r#"{"entityId":"f","kind":"file","filePath":"f.png"}"#)
    );
    assert_eq!(target(&["f", "f2"]), None, "two files");
    assert_eq!(target(&["s"]), None, "neither");
    assert_eq!(
        target(&["g", "in-group"]),
        page("in-group"),
        "a page selected with its group is one page"
    );
    assert_eq!(
        target(&["p", "in-group", "f"]),
        None,
        "two pages are not settled by a file"
    );
    assert_eq!(
        target(&["blank-page"]).as_deref(),
        Some(r#"{"entityId":"blank-page","kind":"page"}"#),
        "no address, no url"
    );
    assert_eq!(
        target(&["blank-file"]).as_deref(),
        Some(r#"{"entityId":"blank-file","kind":"file"}"#),
        "no path, no filePath"
    );
}
