//! The sidebar model: the Canvases list and the Notes and Pages sections,
//! read off the app.

use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, Entity, EntityId, ItemId,
    PageAnchor, Rect,
};
use specular_interact::{Action, RowKind, RowTarget, SidebarRow, sidebar};
use specular_testkit::{
    TestApp, comment, document, drawing, group, inside, note, page, shape, sticky, with_comment,
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
fn the_canvases_list_names_every_canvas_and_switches_to_it() {
    let mut app = TestApp::with_space([
        ("Home", document([page("p1", BOX), page("p2", BOX)])),
        ("Notes", document([sticky("n1", BOX, "hi")])),
    ]);
    let rows = sidebar(app.app()).canvases;
    let read: Vec<_> = (rows.iter())
        .map(|row| (row.label.as_str(), row.active, row.entity_count))
        .collect();
    assert_eq!(read, [("Home", true, 2), ("Notes", false, 1)]);
    app.act(rows[1].action.clone());
    assert_eq!(app.active_canvas(), "Notes");
    let rows = sidebar(app.app()).canvases;
    assert_eq!((rows[0].active, rows[1].active), (false, true));
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
fn a_group_with_notes_and_pages_has_a_row_in_each_section() {
    let app = TestApp::with_entities([
        group("g", Rect::new(-24.0, -24.0, 500.0, 500.0)),
        inside("g", page("p1", BOX)),
        inside("g", sticky("s1", BOX, "inside")),
        inside("g", sticky("s2", BOX, "also inside")),
        sticky("s3", BOX, "outside"),
    ]);
    let model = sidebar(app.app());
    assert_eq!(
        outline(&model.notes),
        [
            "text outside",
            "group(2) Group",
            "  text also inside",
            "  text inside"
        ]
    );
    assert_eq!(
        outline(&model.pages),
        ["group(1) Group", "  page example.com"]
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
}

#[test]
fn a_row_selects_what_it_stands_for_and_shows_the_selection() {
    let mut app = TestApp::with_entities([
        page("p1", BOX),
        sticky("s1", Rect::new(600.0, 0.0, 200.0, 200.0), "note"),
    ]);
    let row = sidebar(app.app()).notes.remove(0);
    assert_eq!(
        row.action,
        Action::Select(vec![ItemId::Entity(EntityId::from("s1"))])
    );
    app.act(row.action);
    assert_eq!(app.selected(), Some("s1"));
    let model = sidebar(app.app());
    assert_eq!(outline(&model.notes), ["*text note"]);
    assert_eq!(outline(&model.pages), ["page example.com"]);
}
