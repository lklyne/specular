//! Small documents to start a test from.

use specular_doc::{
    Command, Document, Drawing, Edge, Entity, EntityId, FileRef, Group, Kind, Page, Rect, Shape,
    ShapeKind, Text, TextStyle, WidthMode,
};

/// A page entity `id` at `rect`, showing `https://example.com/{id}`.
pub fn page(id: &str, rect: Rect) -> Entity {
    let page = Page {
        url: format!("https://example.com/{id}"),
        ..Page::default()
    };
    Entity::new(id, rect, Kind::Page(page))
}

/// A text entity `id` at `rect`, reading its own id.
pub fn text(id: &str, rect: Rect) -> Entity {
    let text = Text {
        text: id.to_owned(),
        ..Text::default()
    };
    Entity::new(id, rect, Kind::Text(text))
}

/// A sticky note `id` at `rect` reading `content`. Its text starts 8 units
/// in from the rect's corner and wraps 16 short of its width.
pub fn sticky(id: &str, rect: Rect, content: &str) -> Entity {
    let text = Text {
        text: content.to_owned(),
        style: Some(TextStyle::Sticky),
        ..Text::default()
    };
    Entity::new(id, rect, Kind::Text(text))
}

/// Plain text `id` at `rect` reading `content`, as wide as its lines. Its
/// text starts at the rect's corner and never wraps.
pub fn plain_text(id: &str, rect: Rect, content: &str) -> Entity {
    let text = Text {
        text: content.to_owned(),
        style: Some(TextStyle::Plain),
        width_mode: Some(WidthMode::Auto),
        ..Text::default()
    };
    Entity::new(id, rect, Kind::Text(text))
}

/// A rectangle shape `id` at `rect` with `content` as its label, which is
/// centred in the rect less 12 units at each side and 8 above and below.
pub fn labelled(id: &str, rect: Rect, content: &str) -> Entity {
    let shape = Shape {
        text: content.to_owned(),
        ..Shape::new(ShapeKind::Rectangle)
    };
    Entity::new(id, rect, Kind::Shape(shape))
}

/// A rectangle shape `id` at `rect`.
pub fn shape(id: &str, rect: Rect) -> Entity {
    Entity::new(id, rect, Kind::Shape(Shape::new(ShapeKind::Rectangle)))
}

/// A file entity `id` at `rect`, showing `{id}.png`.
pub fn file(id: &str, rect: Rect) -> Entity {
    let file = FileRef {
        file: format!("{id}.png"),
        ..FileRef::default()
    };
    Entity::new(id, rect, Kind::File(file))
}

/// A Document `id` at `rect`: a file entity showing the markdown file
/// `path`. Its text arrives with [`TestApp::note_text`](crate::TestApp::note_text).
pub fn note(id: &str, rect: Rect, path: &str) -> Entity {
    let file = FileRef {
        file: path.to_owned(),
        ..FileRef::default()
    };
    Entity::new(id, rect, Kind::File(file))
}

/// A drawing `id` with no strokes, whose box is `rect`.
pub fn drawing(id: &str, rect: Rect) -> Entity {
    Entity::new(id, rect, Kind::Drawing(Drawing::default()))
}

/// A group `id` at `rect`. Put entities in it with [`inside`].
pub fn group(id: &str, rect: Rect) -> Entity {
    Entity::new(id, rect, Kind::Group(Group::default()))
}

/// `entity` as a member of the group `parent`.
#[must_use]
pub fn inside(parent: &str, entity: Entity) -> Entity {
    Entity {
        parent: Some(EntityId::from(parent)),
        ..entity
    }
}

/// `count` 400x300 pages in a row, 200 apart: `p1` at (100, 100), `p2` at
/// (700, 100), `p3` at (1300, 100) and so on.
pub fn pages(count: usize) -> Vec<Entity> {
    (0..count)
        .map(|index| {
            let x = 100.0 + 600.0 * index as f64;
            page(
                &format!("p{}", index + 1),
                Rect::new(x, 100.0, 400.0, 300.0),
            )
        })
        .collect()
}

/// A document holding `entities`, back-to-front.
#[track_caller]
pub fn document(entities: impl IntoIterator<Item = Entity>) -> Document {
    let mut document = Document::new();
    for entity in entities {
        let id = entity.id.clone();
        let command = Command::InsertEntity {
            entity: Box::new(entity),
            at: document.stack_len(),
        };
        if let Err(error) = document.apply(command) {
            panic!("entity {id:?} could not be inserted: {error}");
        }
    }
    document
}

/// `document` with an edge `id` from the entity `from` to the entity `to`,
/// in front of everything.
#[track_caller]
#[must_use]
pub fn connected(document: Document, id: &str, from: &str, to: &str) -> Document {
    with_edge(document, Edge::new(id, from, to))
}

/// `document` with `edge`, in front of everything: for an edge that names
/// sides, a label or a style.
#[track_caller]
#[must_use]
pub fn with_edge(mut document: Document, edge: Edge) -> Document {
    let id = edge.id.clone();
    let command = Command::InsertEdge {
        edge: Box::new(edge),
        at: document.stack_len(),
    };
    if let Err(error) = document.apply(command) {
        panic!("edge {id:?} could not be inserted: {error}");
    }
    document
}
