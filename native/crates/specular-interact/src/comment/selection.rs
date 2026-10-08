//! Annotating the selection: one region draft over everything selected,
//! which records what was selected and the artifact the comment is about.

use serde_json::{Value, json};
use specular_doc::{Document, EntityId, ItemId, JsonMap, Kind};

use super::{create, draft};
use crate::{App, Effect, geometry, groups};

/// Opens a draft on the region the selected entities span. Edges are left
/// out, and with no entity selected nothing happens. The region is the
/// canvas's, however much of a page it covers.
pub(crate) fn annotate(app: &mut App, effects: &mut Vec<Effect>) {
    let ids: Vec<EntityId> = app.session.selection.entities().cloned().collect();
    let bounds = (ids.iter())
        .filter_map(|id| app.document.entity(id))
        .map(|entity| crate::scroll_follow::placed_rect(app, entity))
        .reduce(geometry::union);
    let Some(bounds) = bounds else {
        return;
    };
    let mut made = create::canvas_region(app, bounds);
    made.metadata = Some(selection_metadata(&app.document, &ids));
    draft::open(app, made, effects);
}

/// The metadata of a comment on the entities `ids`: `selectionEntityIds`,
/// the ids as given, and `selectionTarget` when the comment is about one
/// artifact.
pub fn selection_metadata(document: &Document, ids: &[EntityId]) -> JsonMap {
    let mut metadata = JsonMap::new();
    let ids_written = ids.iter().map(|id| json!(id.as_str())).collect();
    metadata.insert("selectionEntityIds".to_owned(), Value::Array(ids_written));
    if let Some(target) = selection_target(document, ids) {
        metadata.insert("selectionTarget".to_owned(), target);
    }
    metadata
}

/// The one artifact a comment on `ids` is about: the page, when exactly one
/// is selected, and with no page the file, when exactly one is. A selection
/// spanning several has no target, because naming one would be a guess.
///
/// A group stands for what is inside it, so selecting the group that holds
/// one page reads the same as selecting that page.
fn selection_target(document: &Document, ids: &[EntityId]) -> Option<Value> {
    let mut candidates: Vec<EntityId> = Vec::new();
    let mut add = |id: &EntityId| {
        if !candidates.contains(id) {
            candidates.push(id.clone());
        }
    };
    for id in ids {
        match document.entity(id).map(|entity| &entity.kind) {
            Some(Kind::Group(_)) => {
                for item in groups::descendants(document, id) {
                    match item {
                        ItemId::Entity(inside) => add(&inside),
                        ItemId::Edge(_) => {}
                    }
                }
            }
            Some(
                Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Drawing(_) | Kind::Shape(_),
            ) => add(id),
            None => {}
        }
    }
    let (mut pages, mut files) = (Vec::new(), Vec::new());
    for id in &candidates {
        match document.entity(id).map(|entity| &entity.kind) {
            Some(Kind::Page(page)) => pages.push((id, page)),
            Some(Kind::File(file)) => files.push((id, file)),
            Some(Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_)) | None => {}
        }
    }
    let mut target = JsonMap::new();
    match (pages.as_slice(), files.as_slice()) {
        ([(id, page)], _) => {
            target.insert("entityId".to_owned(), json!(id.as_str()));
            target.insert("kind".to_owned(), json!("page"));
            if !page.url.is_empty() {
                target.insert("url".to_owned(), json!(page.url));
            }
        }
        ([], [(id, file)]) => {
            target.insert("entityId".to_owned(), json!(id.as_str()));
            target.insert("kind".to_owned(), json!("file"));
            if !file.file.is_empty() {
                target.insert("filePath".to_owned(), json!(file.file));
            }
        }
        _ => return None,
    }
    Some(Value::Object(target))
}
