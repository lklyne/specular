//! Small documents to start a test from.

use specular_doc::{Command, Document, Entity, Kind, Page, Rect};

/// A page entity `id` at `rect`, showing `https://example.com/{id}`.
pub fn page(id: &str, rect: Rect) -> Entity {
    let page = Page {
        url: format!("https://example.com/{id}"),
        ..Page::default()
    };
    Entity::new(id, rect, Kind::Page(page))
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
