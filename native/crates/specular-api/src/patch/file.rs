//! A `file` create: the path is made to point inside the space folder, and
//! an image is sized from its pixels, as a drop of the file would.

use serde_json::{Value, json};
use specular_doc::{EntityId, JsonMap};
use specular_interact::{dropped_size, shown_path};

use super::Builder;
use crate::Response;
use crate::facts::Found;

/// The path a `file` create item names, when it is one for the disk: a URL
/// is left to the renderer.
pub(crate) fn file_path(item: &JsonMap) -> Option<&str> {
    let named = item.get("kind").and_then(Value::as_str) == Some("file");
    let path = item.get("file").and_then(Value::as_str)?;
    (named && !item.contains_key("id") && !path.contains("://")).then_some(path)
}

/// `item` with its `file` pointing into the space folder, and a size when
/// it gave none. A file outside the folder is copied into `assets/` by an
/// effect queued on `builder`. A path the host found no file at is a 400.
pub(super) fn resolve(
    builder: &mut Builder<'_>,
    at: &str,
    item: &JsonMap,
    id: &str,
) -> Result<JsonMap, Response> {
    let mut resolved = item.clone();
    let Some(path) = file_path(item) else {
        return Ok(resolved);
    };
    let found = builder
        .facts
        .map_or(Found::Unasked, |facts| facts.file(path));
    let file = match found {
        Found::Unasked => return Ok(resolved),
        Found::Missing => {
            return Err(Response::bad_request(format!(
                "{at}: file not found: {path}"
            )));
        }
        Found::File(file) => file,
    };
    let (shown, copy) = shown_path(&EntityId::from(id), file).ok_or_else(|| {
        Response::bad_request(format!(
            "{at}: {path} is outside the space folder and has no extension to copy"
        ))
    })?;
    builder.effects.extend(copy);
    resolved.insert("file".to_owned(), json!(shown));
    let sized = ["width", "height"]
        .iter()
        .any(|key| item.get(*key).is_some_and(Value::is_number));
    if let (false, Some(size)) = (sized, dropped_size(file)) {
        resolved.insert("width".to_owned(), json!(size.x));
        resolved.insert("height".to_owned(), json!(size.y));
    }
    Ok(resolved)
}
