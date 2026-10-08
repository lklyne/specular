use super::Document;
use crate::{
    Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, Author, Color, Command,
    CommandError, Drawing, Edge, EdgeId, Entity, EntityId, FileRef, Group, History, ItemId,
    JsonMap, Kind, Page, PageAnchor, Point, Rect, Shape, ShapeKind, Stroke, Text, TextStyle,
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
fn a_refused_command_leaves_the_document_unchanged() {
    let mut before = fixture();
    before.apply(insert(group("g2"), 0)).unwrap();
    let set_parent = |id: &str, parent: &str| Command::SetParent {
        id: EntityId::new(id),
        parent: Some(EntityId::new(parent)),
    };
    before.apply(set_parent("g2", "g1")).unwrap();
    let invalid_parent = |id: &str, parent: &str| CommandError::InvalidParent {
        id: EntityId::new(id),
        parent: EntityId::new(parent),
    };
    let mut swapped_order = before.order().to_vec();
    swapped_order[0] = swapped_order[1].clone();

    let rows = vec![
        (
            "batch with a missing target, after a valid first step",
            Command::Batch(vec![
                Command::RemoveEntity(EntityId::new("p1")),
                Command::SetRect {
                    id: EntityId::new("missing"),
                    rect: Rect::default(),
                },
            ]),
            CommandError::UnknownEntity(EntityId::new("missing")),
        ),
        (
            "entity id taken by an edge",
            insert(page("e1"), 0),
            CommandError::DuplicateId("e1".to_owned()),
        ),
        (
            "edge id taken by an entity",
            Command::InsertEdge {
                edge: Box::new(Edge::new("p1", "t1", "sh1")),
                at: 0,
            },
            CommandError::DuplicateId("p1".to_owned()),
        ),
        (
            "insert past the end",
            insert(page("p2"), 9),
            CommandError::IndexOutOfRange { index: 9, len: 8 },
        ),
        (
            "kind changed by set kind",
            Command::SetKind {
                id: EntityId::new("p1"),
                kind: Box::new(Kind::Text(Text::default())),
            },
            CommandError::KindMismatch {
                id: EntityId::new("p1"),
                expected: "page",
                found: "text",
            },
        ),
        (
            "parent that is not a group",
            set_parent("t1", "p1"),
            invalid_parent("t1", "p1"),
        ),
        (
            "parent that is missing",
            set_parent("t1", "missing"),
            invalid_parent("t1", "missing"),
        ),
        (
            "group inside itself",
            set_parent("g1", "g1"),
            invalid_parent("g1", "g1"),
        ),
        (
            "group inside its own child",
            set_parent("g1", "g2"),
            invalid_parent("g1", "g2"),
        ),
        (
            "order that repeats an id",
            Command::SetOrder(swapped_order),
            CommandError::OrderMismatch,
        ),
    ];
    for (name, command, expected) in rows {
        let mut document = before.clone();
        assert_eq!(document.apply(command), Err(expected), "{name}");
        assert_eq!(document, before, "{name}: the document changed");
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
fn history_undoes_and_redoes_in_order() {
    let start = fixture();
    let mut document = start.clone();
    let mut history = History::new();

    history
        .apply(&mut document, Command::RemoveEdge(EdgeId::new("e1")))
        .unwrap();
    let after_first = document.clone();
    history
        .apply(&mut document, Command::RemoveEntity(EntityId::new("p1")))
        .unwrap();
    let after_second = document.clone();

    assert!(history.undo(&mut document).unwrap().is_some());
    assert_eq!(document, after_first);
    assert!(history.undo(&mut document).unwrap().is_some());
    assert_eq!(document, start);
    assert!(history.undo(&mut document).unwrap().is_none());

    assert!(history.redo(&mut document).unwrap().is_some());
    assert!(history.redo(&mut document).unwrap().is_some());
    assert_eq!(document, after_second);
    assert!(history.redo(&mut document).unwrap().is_none());

    {
        let mut document = fixture();
        let mut history = History::new();
        history
            .apply(&mut document, Command::RemoveEdge(EdgeId::new("e1")))
            .unwrap();
        history.undo(&mut document).unwrap();
        assert!(history.can_redo());

        history
            .apply(&mut document, Command::RemoveEntity(EntityId::new("sh1")))
            .unwrap();
        assert!(!history.can_redo());
    }

    {
        let mut document = fixture();
        let mut history = History::new();
        let refused = history.apply(
            &mut document,
            Command::RemoveEntity(EntityId::new("missing")),
        );
        assert!(refused.is_err());
        assert!(!history.can_undo());
    }
}

#[test]
fn a_step_carries_the_state_from_either_side_of_it() {
    let mut document = fixture();
    let mut history: History<&str> = History::default();
    let remove = Command::RemoveEntity(EntityId::new("sh1"));
    history.apply_from(&mut document, remove, "before").unwrap();
    assert!(history.is_open());
    history.settle("after");
    // A later settle is not this step's.
    history.settle("later");

    assert_eq!(history.undo(&mut document), Ok(Some("before")));
    assert_eq!(history.redo(&mut document), Ok(Some("after")));
    assert_eq!(history.undo(&mut document), Ok(Some("before")));
    assert_eq!(history.undo(&mut document), Ok(None));

    {
        let mut document = fixture();
        let mut history: History<u8> = History::default();
        let remove = |id: &str| Command::RemoveEntity(EntityId::new(id));
        history.apply_from(&mut document, remove("sh1"), 1).unwrap();
        history.settle(2);
        history.apply_from(&mut document, remove("d1"), 3).unwrap();
        assert_eq!(history.undo(&mut document), Ok(Some(3)));
        // The step just undone is closed, so this settle is not the earlier one's.
        history.settle(9);
        assert_eq!(history.undo(&mut document), Ok(Some(1)));
        assert_eq!(history.redo(&mut document), Ok(Some(2)));
    }
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
