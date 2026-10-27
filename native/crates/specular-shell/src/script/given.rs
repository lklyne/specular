//! `given` lines: a starting document written in the script itself.
//!
//! ```text
//! given page p1 100 100 400 300 https://example.com
//! given sticky s1 600 100 200 200 Ship the beta
//! given text t1 600 400 200 40 A caption
//! given shape r1 100 500 200 120 Draft
//! given file f1 400 500 200 200 plan.md
//! given drawing d1 700 500 100 100
//! given group g1 0 0 1000 800
//! given inside g1 sticky s2 40 40 200 200 In the group
//! given edge e1 s1 r1
//! ```
//!
//! The four numbers are x, y, width and height in canvas units, and the
//! rest of the line is the text, label, path or address. Entities stack in
//! the order they are given, the first at the back.

use anyhow::{Context as _, bail};
use specular_doc::{
    Command, Document, Drawing, Edge, Entity, EntityId, FileRef, Group, Kind, Page, Rect, Shape,
    ShapeKind, Text, TextStyle, WidthMode,
};

use super::words::number;

/// One `given` line.
#[derive(Debug, Clone, PartialEq)]
pub enum Given {
    /// An entity, in the group named first if there is one.
    Entity(Option<String>, Box<Entity>),
    /// An edge from one entity to another.
    Edge(Box<Edge>),
}

/// The `given` line whose words follow `given`.
pub(super) fn parse(rest: &str) -> anyhow::Result<Given> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    match words.as_slice() {
        ["edge", id, from, to] => Ok(Given::Edge(Box::new(Edge::new(*id, *from, *to)))),
        ["inside", group, ..] => {
            let tail = after(rest, 2);
            match parse(tail)? {
                Given::Entity(_, entity) => Ok(Given::Entity(Some((*group).to_owned()), entity)),
                Given::Edge(_) => bail!("an edge is not inside a group"),
            }
        }
        [kind, id, x, y, width, height, ..] => {
            let rect = Rect::new(
                f64::from(number(x)?),
                f64::from(number(y)?),
                f64::from(number(width)?),
                f64::from(number(height)?),
            );
            let text = after(rest, 6).to_owned();
            Ok(Given::Entity(
                None,
                Box::new(Entity::new(*id, rect, kind_of(kind, id, text)?)),
            ))
        }
        _ => bail!(
            "`given KIND ID X Y W H [TEXT]`, `given inside GROUP ..` or `given edge ID FROM TO`"
        ),
    }
}

/// `text` after its first `count` words.
fn after(text: &str, count: usize) -> &str {
    let mut rest = text.trim_start();
    for _ in 0..count {
        rest = rest
            .split_once(char::is_whitespace)
            .map_or("", |(_, tail)| tail.trim_start());
    }
    rest
}

fn kind_of(kind: &str, id: &str, text: String) -> anyhow::Result<Kind> {
    Ok(match kind {
        "page" => Kind::Page(Page {
            url: if text.is_empty() {
                format!("https://example.com/{id}")
            } else {
                text
            },
            ..Page::default()
        }),
        "sticky" => Kind::Text(Text {
            text,
            style: Some(TextStyle::Sticky),
            ..Text::default()
        }),
        "text" => Kind::Text(Text {
            text,
            style: Some(TextStyle::Plain),
            width_mode: Some(WidthMode::Auto),
            ..Text::default()
        }),
        "shape" => Kind::Shape(Shape {
            text,
            ..Shape::new(ShapeKind::Rectangle)
        }),
        "file" => Kind::File(FileRef {
            file: text,
            ..FileRef::default()
        }),
        "drawing" => Kind::Drawing(Drawing::default()),
        "group" => Kind::Group(Group::default()),
        other => bail!("unknown kind `{other}`"),
    })
}

/// The document the `given` lines describe.
pub(super) fn document(given: &[Given]) -> anyhow::Result<Document> {
    let mut document = Document::new();
    for line in given {
        let command = match line.clone() {
            Given::Entity(group, entity) => Command::InsertEntity {
                entity: Box::new(Entity {
                    parent: group.map(EntityId::from),
                    ..*entity
                }),
                at: document.stack_len(),
            },
            Given::Edge(edge) => Command::InsertEdge {
                edge,
                at: document.stack_len(),
            },
        };
        document
            .apply(command)
            .with_context(|| format!("given {line:?}"))?;
    }
    Ok(document)
}
