//! A comment made here is written as the Electron app writes one, and reads
//! back as it was made.
#![expect(
    clippy::unwrap_used,
    reason = "a test fails by panicking on a document that cannot be written or read"
)]

use serde_json::{Value, json};
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, Command, Document, EntityId, PageAnchor, Rect,
    RegionAnchor,
};

const CREATED: &str = "2026-01-01T00:00:00.000Z";

fn comment(id: &str, anchor: AnnotationAnchor, text: &str) -> Annotation {
    Annotation {
        text: text.to_owned(),
        ..Annotation::new(AnnotationId::new(id), anchor, CREATED.to_owned())
    }
}

fn on_page(annotation: Annotation) -> Annotation {
    let page_anchor = PageAnchor {
        page_url: Some("https://example.com/p1".to_owned()),
        ..PageAnchor::new(EntityId::from("p1"))
    };
    Annotation {
        page_anchor: Some(page_anchor),
        ..annotation
    }
}

/// One comment of each form the comment tool makes.
fn comments() -> Vec<Annotation> {
    let point = AnnotationAnchor::Canvas {
        canvas_x: 600.0,
        canvas_y: 500.5,
    };
    let element = AnnotationAnchor::Element {
        page_id: EntityId::from("p1"),
        selector: "div.cell[data-col=\"0\"][data-row=\"2\"]".to_owned(),
        element_path: Some("body > div.cell".to_owned()),
        bounding_box: Some(Rect::new(0.0, 96.0, 160.0, 48.0)),
    };
    let on_canvas = AnnotationAnchor::Region(RegionAnchor::Canvas {
        canvas_rect: Rect::new(550.0, 500.0, 100.0, 60.0),
    });
    let in_document = AnnotationAnchor::Region(RegionAnchor::Document {
        doc_rect: Rect::new(100.0, 100.0, 100.0, 50.0),
    });
    let mut selection = comment("a5", on_canvas.clone(), "tighten these");
    selection.metadata = json!({
        "selectionEntityIds": ["p1"],
        "selectionTarget": {"entityId": "p1", "kind": "page", "url": "https://example.com/p1"},
    })
    .as_object()
    .cloned();
    vec![
        comment("a1", point, "move this"),
        on_page(comment("a2", element, "too small")),
        comment("a3", on_canvas, "group these"),
        on_page(comment("a4", in_document, "fix the header")),
        selection,
    ]
}

fn with_comments() -> Document {
    let mut document = Document::new();
    for (at, annotation) in comments().into_iter().enumerate() {
        let annotation = Box::new(annotation);
        document
            .apply(Command::InsertAnnotation { annotation, at })
            .unwrap();
    }
    document
}

#[test]
fn each_form_is_written_in_the_electron_shape() {
    let written = with_comments().to_canvas_value().unwrap();
    let page_anchor = json!({"pageId": "p1", "pageUrl": "https://example.com/p1"});
    let thread = |id: &str, anchor: Value, text: &str| {
        json!({
            "id": id, "anchor": anchor, "author": "user", "text": text,
            "status": "pending", "replies": [], "createdAt": CREATED,
        })
    };
    let with = |mut thread: Value, key: &str, value: Value| {
        thread[key] = value;
        thread
    };
    let region = json!({
        "type": "region", "canvasRect": {"x": 550, "y": 500, "width": 100, "height": 60},
    });
    let expected = json!([
        thread(
            "a1",
            json!({"type": "canvas", "canvasX": 600, "canvasY": 500.5}),
            "move this"
        ),
        with(
            thread(
                "a2",
                json!({
                    "type": "element", "pageId": "p1",
                    "selector": "div.cell[data-col=\"0\"][data-row=\"2\"]",
                    "elementPath": "body > div.cell",
                    "boundingBox": {"x": 0, "y": 96, "width": 160, "height": 48},
                }),
                "too small",
            ),
            "pageAnchor",
            page_anchor.clone(),
        ),
        thread("a3", region.clone(), "group these"),
        with(
            thread(
                "a4",
                json!({
                    "type": "region",
                    "docRect": {"x": 100, "y": 100, "width": 100, "height": 50},
                }),
                "fix the header",
            ),
            "pageAnchor",
            page_anchor,
        ),
        with(
            thread("a5", region, "tighten these"),
            "metadata",
            json!({
                "selectionEntityIds": ["p1"],
                "selectionTarget":
                    {"entityId": "p1", "kind": "page", "url": "https://example.com/p1"},
            }),
        ),
    ]);
    // As text, so the order of the keys counts too.
    assert_eq!(written["annotations"].to_string(), expected.to_string());
}

#[test]
fn what_was_written_loads_back_as_it_was_made() {
    let text = with_comments().to_canvas_string().unwrap();
    let loaded = Document::from_canvas_str(&text).unwrap();
    assert_eq!(loaded.annotations(), comments());
}
