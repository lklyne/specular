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

/// Applies `command`, checks its inverse restores the fixture exactly, and
/// that the inverse's inverse reproduces the change.
fn assert_round_trips(command: Command) -> Document {
    let before = fixture();
    let mut document = before.clone();
    let inverse = document.apply(command).unwrap();
    let after = document.clone();
    assert_ne!(after, before, "the command should change the document");

    let redo = document.apply(inverse).unwrap();
    assert_eq!(document, before, "the inverse should restore the document");

    document.apply(redo).unwrap();
    assert_eq!(
        document, after,
        "the inverse's inverse should redo the change"
    );
    after
}

#[test]
fn insert_entity_round_trips_and_lands_at_its_stack_index() {
    let after = assert_round_trips(insert(page("p2"), 1));
    assert_eq!(after.order()[1], ItemId::Entity(EntityId::new("p2")));
}

#[test]
fn remove_entity_round_trips_to_the_same_stack_index() {
    let after = assert_round_trips(Command::RemoveEntity(EntityId::new("p1")));
    assert!(after.entity(&EntityId::new("p1")).is_none());
    assert_eq!(after.stack_len(), 6);
}

#[test]
fn set_rect_round_trips() {
    let rect = Rect::new(10.5, -20.0, 300.0, 200.0);
    let after = assert_round_trips(Command::SetRect {
        id: EntityId::new("p1"),
        rect,
    });
    assert_eq!(after.entity(&EntityId::new("p1")).unwrap().rect, rect);
}

#[test]
fn set_label_round_trips() {
    assert_round_trips(Command::SetLabel {
        id: EntityId::new("g1"),
        label: Some("Header".to_owned()),
    });
}

#[test]
fn set_parent_round_trips_and_updates_children() {
    let after = assert_round_trips(Command::SetParent {
        id: EntityId::new("sh1"),
        parent: Some(EntityId::new("g1")),
    });
    let children: Vec<&str> = after
        .children(&EntityId::new("g1"))
        .map(|child| child.id.as_str())
        .collect();
    assert_eq!(children, ["t1", "sh1"]);
}

#[test]
fn set_anchor_round_trips() {
    assert_round_trips(Command::SetAnchor {
        id: EntityId::new("sh1"),
        anchor: Some(Box::new(PageAnchor::new(EntityId::new("p1")))),
    });
}

#[test]
fn set_kind_round_trips() {
    let kind = Kind::Text(Text {
        text: "edited".to_owned(),
        ..Text::default()
    });
    assert_round_trips(Command::SetKind {
        id: EntityId::new("t1"),
        kind: Box::new(kind),
    });
}

#[test]
fn edge_commands_round_trip() {
    assert_round_trips(Command::InsertEdge {
        edge: Box::new(Edge::new("e2", "p1", "sh1")),
        at: 0,
    });
    assert_round_trips(Command::RemoveEdge(EdgeId::new("e1")));
    let mut edge = Edge::new("e1", "t1", "p1");
    edge.label = Some("flows to".to_owned());
    assert_round_trips(Command::ReplaceEdge(Box::new(edge)));
}

#[test]
fn set_order_round_trips() {
    let mut order = fixture().order().to_vec();
    order.reverse();
    let after = assert_round_trips(Command::SetOrder(order));
    let ids: Vec<&str> = after.entities().map(|entity| entity.id.as_str()).collect();
    assert_eq!(ids, ["sh1", "d1", "f1", "t1", "p1", "g1"]);
}

#[test]
fn annotation_commands_round_trip() {
    assert_round_trips(Command::InsertAnnotation {
        annotation: Box::new(annotation("a2")),
        at: 0,
    });
    assert_round_trips(Command::RemoveAnnotation(AnnotationId::new("a1")));
    let mut resolved = annotation("a1");
    resolved.status = AnnotationStatus::Resolved;
    assert_round_trips(Command::ReplaceAnnotation(Box::new(resolved)));
}

#[test]
fn batch_round_trips_as_one_command() {
    let after = assert_round_trips(Command::Batch(vec![
        Command::RemoveEdge(EdgeId::new("e1")),
        Command::RemoveEntity(EntityId::new("t1")),
        Command::RemoveEntity(EntityId::new("g1")),
    ]));
    assert_eq!(after.stack_len(), 4);
}

#[test]
fn failed_batch_leaves_the_document_unchanged() {
    let before = fixture();
    let mut document = before.clone();
    let error = document
        .apply(Command::Batch(vec![
            Command::RemoveEntity(EntityId::new("p1")),
            Command::SetRect {
                id: EntityId::new("missing"),
                rect: Rect::default(),
            },
        ]))
        .unwrap_err();
    assert_eq!(error, CommandError::UnknownEntity(EntityId::new("missing")));
    assert_eq!(document, before);
}

#[test]
fn entity_and_edge_ids_share_one_namespace() {
    let mut document = fixture();
    let error = document.apply(insert(page("e1"), 0)).unwrap_err();
    assert_eq!(error, CommandError::DuplicateId("e1".to_owned()));
    let error = document
        .apply(Command::InsertEdge {
            edge: Box::new(Edge::new("p1", "t1", "sh1")),
            at: 0,
        })
        .unwrap_err();
    assert_eq!(error, CommandError::DuplicateId("p1".to_owned()));
}

#[test]
fn insert_past_the_front_of_the_stack_is_refused() {
    let error = fixture().apply(insert(page("p2"), 8)).unwrap_err();
    assert_eq!(error, CommandError::IndexOutOfRange { index: 8, len: 7 });
}

#[test]
fn set_kind_cannot_change_the_variant() {
    let error = fixture()
        .apply(Command::SetKind {
            id: EntityId::new("p1"),
            kind: Box::new(Kind::Text(Text::default())),
        })
        .unwrap_err();
    assert!(matches!(
        error,
        CommandError::KindMismatch {
            expected: "page",
            found: "text",
            ..
        }
    ));
}

#[test]
fn set_parent_refuses_non_groups_missing_groups_and_cycles() {
    let mut document = fixture();
    document.apply(insert(group("g2"), 0)).unwrap();
    let set_parent = |id: &str, parent: &str| Command::SetParent {
        id: EntityId::new(id),
        parent: Some(EntityId::new(parent)),
    };
    document.apply(set_parent("g2", "g1")).unwrap();

    for (id, parent) in [("t1", "p1"), ("t1", "missing"), ("g1", "g1"), ("g1", "g2")] {
        let error = document.apply(set_parent(id, parent)).unwrap_err();
        assert!(
            matches!(error, CommandError::InvalidParent { .. }),
            "{id} -> {parent}: {error}"
        );
    }
}

#[test]
fn set_order_must_be_a_permutation() {
    let mut document = fixture();
    let mut order = document.order().to_vec();
    order[0] = order[1].clone();
    assert_eq!(
        document.apply(Command::SetOrder(order)).unwrap_err(),
        CommandError::OrderMismatch
    );
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
fn edges_touching_finds_both_ends() {
    let document = fixture();
    for id in ["p1", "t1"] {
        assert_eq!(document.edges_touching(&EntityId::new(id)).count(), 1);
    }
    assert_eq!(document.edges_touching(&EntityId::new("sh1")).count(), 0);
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

    assert!(history.undo(&mut document).unwrap());
    assert_eq!(document, after_first);
    assert!(history.undo(&mut document).unwrap());
    assert_eq!(document, start);
    assert!(!history.undo(&mut document).unwrap());

    assert!(history.redo(&mut document).unwrap());
    assert!(history.redo(&mut document).unwrap());
    assert_eq!(document, after_second);
    assert!(!history.redo(&mut document).unwrap());
}

#[test]
fn a_new_command_clears_the_redo_stack() {
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

#[test]
fn a_refused_command_is_not_an_undo_step() {
    let mut document = fixture();
    let mut history = History::new();
    let refused = history.apply(
        &mut document,
        Command::RemoveEntity(EntityId::new("missing")),
    );
    assert!(refused.is_err());
    assert!(!history.can_undo());
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
