//! Mod shapes (ADR 0060): a mod's change to the outline and values of shapes
//! it gave, as one undo step.

use serde_json::Value;
use specular_doc::{Command, CustomShape, EntityId, JsonMap, Kind};

use crate::App;

/// What `shape.set` changes on each shape it names.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeChange {
    /// The shapes.
    pub ids: Vec<EntityId>,
    /// Keys of their values to set; a `null` takes its key away.
    pub params: JsonMap,
    /// Their new outline, when it changes.
    pub path: Option<String>,
}

/// The command that makes `change` to every mod shape it names, or `None`
/// when none would change. A shape that is no mod shape, or whose path the
/// new one does not read as, is left alone.
pub(crate) fn set_shapes(app: &App, change: &ShapeChange) -> Option<Command> {
    let steps: Vec<Command> = (change.ids.iter())
        .filter_map(|id| {
            let entity = app.document.entity(id)?;
            let Kind::Shape(shape) = &entity.kind else {
                return None;
            };
            let custom = shape.custom.as_ref()?;
            let mut params = custom.params.clone();
            for (key, value) in &change.params {
                match value {
                    Value::Null => params.shift_remove(key),
                    value => params.insert(key.clone(), value.clone()),
                };
            }
            let path = change.path.as_deref().unwrap_or(custom.path());
            let next = CustomShape::new(custom.name.clone(), params, path).ok()?;
            (next != *custom).then(|| {
                let mut shape = shape.clone();
                shape.custom = Some(next);
                Command::SetKind {
                    id: id.clone(),
                    kind: Box::new(Kind::Shape(shape)),
                }
            })
        })
        .collect();
    match steps.len() {
        0 => None,
        1 => steps.into_iter().next(),
        _ => Some(Command::Batch(steps)),
    }
}
