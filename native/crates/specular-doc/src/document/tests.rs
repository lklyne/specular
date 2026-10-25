use super::Document;
use crate::{
    Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, Author, Color, Command, Drawing,
    Edge, EdgeId, Entity, EntityId, FileRef, Group, ItemId, JsonMap, Kind, Page, PageAnchor, Point,
    Rect, Shape, ShapeKind, Stroke, Text, TextStyle,
};

fn entity(id: &str, kind: Kind) -> Entity {
    Entity::new(id, Rect::new(0.0, 0.0, 100.0, 50.0), kind)
}

fn page(id: &str) -> Entity {
    entity(
        id,
        Kind::Page(Page {
            url: "https://example.com/".to_owned(),
            ..Page::default()
        }),
    )
}

fn group(id: &str) -> Entity {
    entity(id, Kind::Group(Group::default()))
}

fn stroke() -> Stroke {
    Stroke {
        id: "s1".to_owned(),
        color: Color::parse("#112233"),
        width: 2.0,
        points: vec![Point::new(0.0, 0.0), Point::new(4.5, 9.0)],
        brush: None,
        extra: JsonMap::new(),
    }
}

fn annotation(id: &str) -> Annotation {
    Annotation {
        id: AnnotationId::new(id),
        anchor: AnnotationAnchor::Canvas {
            canvas_x: 1.0,
            canvas_y: 2.0,
        },
        author: Author::User,
        text: "note".to_owned(),
        status: AnnotationStatus::Pending,
        replies: Vec::new(),
        created_at: "2026-01-01T00:00:00.000Z".to_owned(),
        element_name: None,
        page_anchor: None,
        metadata: None,
        extra: JsonMap::new(),
    }
}

fn insert(entity: Entity, at: usize) -> Command {
    Command::InsertEntity {
        entity: Box::new(entity),
        at,
    }
}

/// One of each kind, an edge between two of them, and an annotation.
/// Stack order, back-to-front: g1, p1, t1, e1, f1, d1, sh1.
fn fixture() -> Document {
    let mut text = entity(
        "t1",
        Kind::Text(Text {
            text: "hello".to_owned(),
            style: Some(TextStyle::Plain),
            ..Text::default()
        }),
    );
    text.parent = Some(EntityId::new("g1"));
    let drawing = Kind::Drawing(Drawing {
        strokes: vec![stroke()],
    });
    let file = Kind::File(FileRef {
        file: "notes.md".to_owned(),
        ..FileRef::default()
    });
    let commands = vec![
        insert(group("g1"), 0),
        insert(page("p1"), 1),
        insert(text, 2),
        Command::InsertEdge {
            edge: Box::new(Edge::new("e1", "p1", "t1")),
            at: 3,
        },
        insert(entity("f1", file), 4),
        insert(entity("d1", drawing), 5),
        insert(
            entity("sh1", Kind::Shape(Shape::new(ShapeKind::Diamond))),
            6,
        ),
        Command::InsertAnnotation {
            annotation: Box::new(annotation("a1")),
            at: 0,
        },
    ];
    let mut document = Document::new();
    document.apply(Command::Batch(commands)).unwrap();
    document
}

fn moved() -> Rect {
    Rect::new(10.5, -20.0, 300.0, 200.0)
}

/// Applies `command`, checks its inverse restores the fixture exactly, and
/// that the inverse's inverse reproduces the change.
fn assert_round_trips(name: &str, command: Command) -> Document {
    let before = fixture();
    let mut document = before.clone();
    let inverse = document.apply(command).unwrap();
    let after = document.clone();
    assert_ne!(
        after, before,
        "{name}: the command should change the document"
    );

    let redo = document.apply(inverse).unwrap();
    assert_eq!(
        document, before,
        "{name}: the inverse should restore the document"
    );

    document.apply(redo).unwrap();
    assert_eq!(
        document, after,
        "{name}: the inverse's inverse should redo the change"
    );
    after
}

type Check = fn(&Document) -> bool;

#[test]
#[expect(clippy::too_many_lines, reason = "one table, a row per command")]
fn every_command_changes_the_document_and_its_inverse_restores_it() {
    let reversed_order = {
        let mut order = fixture().order().to_vec();
        order.reverse();
        order
    };
    let mut edge = Edge::new("e1", "t1", "p1");
    edge.label = Some("flows to".to_owned());
    let mut resolved = annotation("a1");
    resolved.status = AnnotationStatus::Resolved;
    let edited = Kind::Text(Text {
        text: "edited".to_owned(),
        ..Text::default()
    });
    let id = EntityId::new;
    let rows: Vec<(&str, Command, Check)> = vec![
        ("insert entity", insert(page("p2"), 1), |after| {
            after.order()[1] == ItemId::Entity(EntityId::new("p2"))
        }),
        ("remove entity", Command::RemoveEntity(id("p1")), |after| {
            after.entity(&EntityId::new("p1")).is_none() && after.stack_len() == 6
        }),
        (
            "set rect",
            Command::SetRect {
                id: id("p1"),
                rect: moved(),
            },
            |after| {
                after
                    .entity(&EntityId::new("p1"))
                    .is_some_and(|e| e.rect == moved())
            },
        ),
        (
            "set label",
            Command::SetLabel {
                id: id("g1"),
                label: Some("Header".to_owned()),
            },
            |after| {
                after
                    .entity(&EntityId::new("g1"))
                    .is_some_and(|e| e.label.as_deref() == Some("Header"))
            },
        ),
        (
            "set parent",
            Command::SetParent {
                id: id("sh1"),
                parent: Some(id("g1")),
            },
            |after| {
                let children: Vec<&str> = after
                    .children(&EntityId::new("g1"))
                    .map(|child| child.id.as_str())
                    .collect();
                children == ["t1", "sh1"]
            },
        ),
        (
            "set anchor",
            Command::SetAnchor {
                id: id("sh1"),
                anchor: Some(Box::new(PageAnchor::new(id("p1")))),
            },
            |after| {
                after
                    .entity(&EntityId::new("sh1"))
                    .is_some_and(|e| e.anchor.is_some())
            },
        ),
        (
            "set kind",
            Command::SetKind {
                id: id("t1"),
                kind: Box::new(edited),
            },
            |after| {
                after
                    .entity(&EntityId::new("t1"))
                    .is_some_and(|e| matches!(&e.kind, Kind::Text(text) if text.text == "edited"))
            },
        ),
        (
            "insert edge",
            Command::InsertEdge {
                edge: Box::new(Edge::new("e2", "p1", "sh1")),
                at: 0,
            },
            |after| after.edge(&EdgeId::new("e2")).is_some(),
        ),
        (
            "remove edge",
            Command::RemoveEdge(EdgeId::new("e1")),
            |after| after.edge(&EdgeId::new("e1")).is_none(),
        ),
        (
            "replace edge",
            Command::ReplaceEdge(Box::new(edge)),
            |after| {
                after
                    .edge(&EdgeId::new("e1"))
                    .is_some_and(|e| e.label.as_deref() == Some("flows to"))
            },
        ),
        ("set order", Command::SetOrder(reversed_order), |after| {
            let ids: Vec<&str> = after.entities().map(|e| e.id.as_str()).collect();
            ids == ["sh1", "d1", "f1", "t1", "p1", "g1"]
        }),
        (
            "insert annotation",
            Command::InsertAnnotation {
                annotation: Box::new(annotation("a2")),
                at: 0,
            },
            |after| after.annotations().len() == 2,
        ),
        (
            "remove annotation",
            Command::RemoveAnnotation(AnnotationId::new("a1")),
            |after| after.annotations().is_empty(),
        ),
        (
            "replace annotation",
            Command::ReplaceAnnotation(Box::new(resolved)),
            |after| after.annotations()[0].status == AnnotationStatus::Resolved,
        ),
        (
            "batch",
            Command::Batch(vec![
                Command::RemoveEdge(EdgeId::new("e1")),
                Command::RemoveEntity(id("t1")),
                Command::RemoveEntity(id("g1")),
            ]),
            |after| after.stack_len() == 4,
        ),
    ];
    for (name, command, check) in rows {
        let after = assert_round_trips(name, command);
        assert!(check(&after), "{name}: the change is not what it names");
    }
}

#[test]
fn ancestors_stops_at_a_cycle_loaded_from_a_file() {
    let mut a = group("a");
    a.parent = Some(EntityId::new("b"));
    let mut b = group("b");
    b.parent = Some(EntityId::new("a"));
    let mut document = Document::new();
    document.apply(insert(a, 0)).unwrap();
    document.apply(insert(b, 1)).unwrap();
    assert_eq!(document.ancestors(&EntityId::new("a")).count(), 2);
}

#[test]
fn set_note_round_trips_and_is_not_saved() {
    let mut doc = Document::new();
    let seed = Command::SetNote {
        file: "plan.md".to_owned(),
        text: Some("# Plan".to_owned()),
    };
    let forget = doc.apply(seed).unwrap();
    assert_eq!(doc.note("plan.md"), Some("# Plan"));
    let saved = doc.to_canvas_string().unwrap();
    assert!(!saved.contains("Plan"), "{saved}");

    let edit = Command::SetNote {
        file: "plan.md".to_owned(),
        text: Some("# Plan\n\n- one".to_owned()),
    };
    let back = doc.apply(edit).unwrap();
    assert_eq!(doc.note("plan.md"), Some("# Plan\n\n- one"));
    doc.apply(back).unwrap();
    assert_eq!(doc.note("plan.md"), Some("# Plan"));
    doc.apply(forget).unwrap();
    assert_eq!(doc.note("plan.md"), None);
    assert_eq!(doc, Document::new());
}
