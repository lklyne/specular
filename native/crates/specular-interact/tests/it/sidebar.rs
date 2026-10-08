//! The sidebar model: the Canvases list and the Notes and Pages sections,
//! read off the app.

use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, Entity, EntityId, PageAnchor,
    Rect,
};
use specular_interact::{RowKind, RowTarget, SidebarRow, sidebar};
use specular_testkit::{
    TestApp, comment, document, drawing, note, page, shape, sticky, with_comment,
};

const BOX: Rect = Rect {
    x: 0.0,
    y: 0.0,
    width: 200.0,
    height: 200.0,
};

/// `entity` hooked to `page` as it was on the document at `url`.
fn hooked(page: &str, url: &str, entity: Entity) -> Entity {
    Entity {
        anchor: Some(PageAnchor {
            page_url: Some(url.to_owned()),
            ..PageAnchor::new(EntityId::from(page))
        }),
        ..entity
    }
}

/// A comment bound to `page` as it was on the document at `url`.
fn page_comment(id: &str, page: &str, url: &str, text: &str, at: &str) -> Annotation {
    let anchor = AnnotationAnchor::Page {
        page_id: EntityId::from(page),
        offset_x: 10.0,
        offset_y: 10.0,
    };
    Annotation {
        page_anchor: Some(PageAnchor {
            page_url: Some(url.to_owned()),
            ..PageAnchor::new(EntityId::from(page))
        }),
        created_at: at.to_owned(),
        ..comment(id, anchor, text)
    }
}

/// One line a row: indent, a mark for selected (`*`) and dimmed (`~`), the
/// kind, the label.
fn outline(rows: &[SidebarRow]) -> Vec<String> {
    fn walk(rows: &[SidebarRow], depth: usize, out: &mut Vec<String>) {
        for row in rows {
            let kind = match row.kind {
                RowKind::Group { entity_count } => format!("group({entity_count})"),
                RowKind::Page => "page".to_owned(),
                RowKind::Text => "text".to_owned(),
                RowKind::File => "file".to_owned(),
                RowKind::Drawing { strokes } => format!("drawing({strokes})"),
                RowKind::Shape(_) => "shape".to_owned(),
                RowKind::Comment { messages } => format!("comment({messages})"),
            };
            let marks = format!(
                "{}{}",
                if row.selected { "*" } else { "" },
                if row.dimmed { "~" } else { "" }
            );
            out.push(format!("{}{marks}{kind} {}", "  ".repeat(depth), row.label));
            walk(&row.children, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    walk(rows, 0, &mut out);
    out
}

#[test]
fn notes_and_pages_are_listed_front_of_the_stack_first() {
    let app = TestApp::with_entities([
        page("p1", BOX),
        sticky("s1", BOX, "first line\nsecond"),
        page("p2", BOX),
        note("d1", BOX, "notes/Plan.md"),
        drawing("w1", BOX),
        shape("r1", BOX),
    ]);
    let model = sidebar(app.app());
    assert_eq!(
        outline(&model.notes),
        [
            "shape Rectangle",
            "drawing(0) Drawing (0 strokes)",
            "file Plan",
            "text first line",
        ]
    );
    assert_eq!(
        outline(&model.pages),
        ["page example.com", "page example.com"]
    );
}

#[test]
fn what_is_hooked_to_a_page_nests_under_it_with_its_open_comments() {
    let url = "https://example.com/p1";
    let entities = document([
        page("p1", BOX),
        hooked("p1", url, sticky("s1", BOX, "on the page")),
        hooked(
            "p1",
            "https://example.com/elsewhere",
            sticky("s2", BOX, "left behind"),
        ),
        hooked("gone", url, sticky("s3", BOX, "its page is gone")),
    ]);
    let older = page_comment("c1", "p1", url, "tighten this", "2026-01-01T00:00:00.000Z");
    let newer = Annotation {
        element_name: Some("button.cta".to_owned()),
        replies: vec![specular_doc::Reply {
            author: specular_doc::Author::Agent,
            text: "done".to_owned(),
            timestamp: "2026-01-03T00:00:00.000Z".to_owned(),
            extra: specular_doc::JsonMap::new(),
        }],
        ..page_comment(
            "c2",
            "p1",
            "https://example.com/elsewhere",
            "",
            "2026-01-02T00:00:00.000Z",
        )
    };
    let resolved = Annotation {
        status: AnnotationStatus::Resolved,
        ..page_comment("c3", "p1", url, "done with", "2026-01-04T00:00:00.000Z")
    };
    let on_canvas = comment(
        "c4",
        AnnotationAnchor::Canvas {
            canvas_x: 0.0,
            canvas_y: 0.0,
        },
        "not on a page",
    );
    let entities = [older, newer, resolved, on_canvas]
        .into_iter()
        .fold(entities, with_comment);
    let mut app = TestApp::from_document(entities);

    let model = sidebar(app.app());
    assert_eq!(outline(&model.notes), ["text its page is gone"]);
    assert_eq!(
        outline(&model.pages),
        [
            "page example.com",
            "  ~text left behind",
            "  text on the page",
            "  ~comment(2) button.cta",
            "  comment(1) tighten this",
        ]
    );

    // A comment row focuses its comment, and shows it.
    let comment_row = &model.pages[0].children[3];
    assert_eq!(
        comment_row.target,
        RowTarget::Comment(AnnotationId::new("c1"))
    );
    app.act(comment_row.action.clone());
    assert_eq!(
        outline(&sidebar(app.app()).pages)[4],
        "  *comment(1) tighten this"
    );
    // Each comment row asks for its own comment.
    assert_eq!(
        model.pages[0].children[2].action,
        specular_interact::Action::RevealComment(AnnotationId::new("c2"))
    );
}
