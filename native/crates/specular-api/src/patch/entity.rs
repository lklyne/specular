//! One entity item of a patch: a create or an update.

use glam::DVec2;
use serde_json::{Value, json};
use specular_doc::{Command, Document, Entity, EntityId, JsonMap, Rect};
use specular_interact::{group_command, move_commands};

use super::Builder;
use super::fields::{Sort, misread, overlay};
use crate::http::strings;
use crate::{Response, presets};

/// What a group made without a `label` is called, as in the window.
const GROUP_LABEL: &str = "Group";

/// Refuses an item whose kind cannot be told: an update of an id the
/// document does not hold, or a create with a missing or unknown `kind`.
pub(super) fn check_kind(document: &Document, item: &JsonMap, at: &str) -> Result<(), Response> {
    let known = match item.get("id") {
        Some(id) => (id.as_str()).is_some_and(|id| document.entity(&EntityId::from(id)).is_some()),
        None => sort_of(item).is_some(),
    };
    if known {
        Ok(())
    } else {
        Err(Response::bad_request(format!(
            "{at}: missing or unknown kind"
        )))
    }
}

fn sort_of(item: &JsonMap) -> Option<Sort> {
    Sort::parse(item.get("kind")?.as_str()?)
}

fn unknown_kind(at: &str) -> Response {
    Response::bad_request(format!("{at}: missing or unknown kind"))
}

/// Whether `url` names a page on its own: a scheme, and a host unless it is
/// a file. The canvas holds pages from many origins, so a bare path such as
/// `/garden` is ambiguous.
fn is_full_url(url: &str) -> bool {
    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let named = !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric());
    named && (scheme == "file" || !host.is_empty())
}

fn check_url(item: &JsonMap, at: &str, required: bool) -> Result<(), Response> {
    match item.get("url").and_then(Value::as_str) {
        Some(url) if is_full_url(url) => Ok(()),
        None if !required => Ok(()),
        given => Err(Response::bad_request(format!(
            "{at}: url must be a full URL with a scheme and a host, such as \
             http://localhost:4321/garden (got {})",
            json!(given)
        ))),
    }
}

/// The size a page item asks for by `presetIndex` and `orientation`, or
/// `None` when it names neither.
fn page_size(
    item: &JsonMap,
    current: Option<u64>,
    at: &str,
) -> Result<Option<(f64, f64)>, Response> {
    let orientation = item.get("orientation").and_then(Value::as_str);
    let index = match item.get("presetIndex") {
        None | Some(Value::Null) if orientation.is_none() => return Ok(None),
        None | Some(Value::Null) => current.unwrap_or(u64::from(presets::LAPTOP)),
        Some(index) => (index.as_u64())
            .ok_or_else(|| Response::bad_request(format!("{at}.presetIndex: invalid value")))?,
    };
    let size = presets::size(index)
        .ok_or_else(|| Response::bad_request(format!("{at}.presetIndex: no preset {index}")))?;
    Ok(Some(presets::oriented(size, orientation)))
}

fn set_size(node: &mut JsonMap, (width, height): (f64, f64)) {
    node.insert("width".to_owned(), json!(width));
    node.insert("height".to_owned(), json!(height));
}

/// Reads `node` back as an entity, refusing a value the reader could not
/// type.
fn read(node: JsonMap, item: &JsonMap, sort: Sort, at: &str) -> Result<Entity, Response> {
    let entity = Entity::from_node(node).map_err(|_| {
        Response::bad_request(format!("{at}: could not be read as a {}", sort.name()))
    })?;
    match misread(&entity.extra, item, sort) {
        Some(field) => Err(Response::bad_request(format!(
            "{at}.{field}: invalid value"
        ))),
        None => Ok(entity),
    }
}

/// Adds the entity `item` describes, in front of everything.
pub(super) fn create(builder: &mut Builder<'_>, at: &str, item: &JsonMap) -> Result<(), Response> {
    let sort = sort_of(item).ok_or_else(|| unknown_kind(at))?;
    let id = builder.fresh_id(sort.name());
    if sort == Sort::Group {
        return create_group(builder, at, item, id);
    }
    let mut node = JsonMap::new();
    node.insert("id".to_owned(), json!(id));
    node.insert("type".to_owned(), json!(sort.node_type()));
    node.insert("x".to_owned(), json!(0));
    node.insert("y".to_owned(), json!(0));
    set_size(&mut node, sort.default_size());
    match sort {
        Sort::Page => {
            check_url(item, at, true)?;
            node.insert("presetIndex".to_owned(), json!(presets::LAPTOP));
            node.insert("source".to_owned(), json!("generated"));
        }
        Sort::Text => {
            node.insert("text".to_owned(), json!(""));
            // A sticky's color is its card, yellow unless told; plain
            // text's is its ink, which follows the theme.
            let plain = item.get("textStyle").and_then(Value::as_str) == Some("plain");
            let color = if plain { "neutral" } else { "3" };
            let style = if plain { "plain" } else { "sticky" };
            overlay(
                &mut node,
                &defaults(&[("color", color), ("textStyle", style)]),
                sort,
            );
        }
        Sort::Shape => {
            node.insert("shapeKind".to_owned(), json!("rectangle"));
            node.insert("text".to_owned(), json!(""));
        }
        Sort::File | Sort::Drawing | Sort::Group => {}
    }
    overlay(&mut node, item, sort);
    if sort == Sort::Page {
        let size = page_size(item, None, at)?.unwrap_or(sort.default_size());
        set_size(&mut node, size);
    }
    let entity = read(node, item, sort, at)?;
    let front = builder.trial.stack_len();
    builder.push(
        Command::InsertEntity {
            entity: Box::new(entity),
            at: front,
        },
        at,
    )?;
    builder.created.push(id);
    Ok(())
}

fn defaults(pairs: &[(&str, &str)]) -> JsonMap {
    (pairs.iter())
        .map(|(key, value)| ((*key).to_owned(), json!(value)))
        .collect()
}

/// A group is made around entities the document already holds, so it reads
/// `entityIds` and a `label` and takes its place and size from its members.
fn create_group(
    builder: &mut Builder<'_>,
    at: &str,
    item: &JsonMap,
    id: String,
) -> Result<(), Response> {
    let members = (item.get("entityIds").and_then(strings))
        .filter(|members| !members.is_empty())
        .ok_or_else(|| Response::bad_request(format!("{at}: entityIds is required")))?;
    let members: Vec<EntityId> = members.into_iter().map(EntityId::from).collect();
    if let Some(missing) = (members.iter()).find(|id| builder.trial.entity(id).is_none()) {
        return Err(Response::bad_request(format!(
            "{at}: unknown entity '{missing}'"
        )));
    }
    let label = item.get("label").and_then(Value::as_str);
    let label = label.unwrap_or(GROUP_LABEL).to_owned();
    let command = group_command(
        &builder.trial,
        &members,
        &EntityId::from(id.as_str()),
        label,
    )
    .ok_or_else(|| Response::bad_request(format!("{at}: nothing to group")))?;
    builder.push(command, at)?;
    builder.created.push(id);
    Ok(())
}

/// Changes the entity `id` to what `item` gives. A new position moves it as
/// a drag would: with what is inside its group and hooked to its page, and
/// a drawing's points with it.
pub(super) fn update(
    builder: &mut Builder<'_>,
    at: &str,
    item: &JsonMap,
    id: &str,
) -> Result<(), Response> {
    let id = EntityId::from(id);
    let before = (builder.trial.entity(&id).cloned()).ok_or_else(|| unknown_kind(at))?;
    let sort = Sort::of(&before.kind);
    let mut node = before
        .to_node()
        .map_err(|error| Response::error(500, error.to_string()))?;
    overlay(&mut node, item, sort);
    if sort == Sort::Page {
        check_url(item, at, false)?;
        let current = node.get("presetIndex").and_then(Value::as_u64);
        if let Some(size) = page_size(item, current, at)? {
            set_size(&mut node, size);
        }
    }
    let after = read(node, item, sort, at)?;

    if after.label != before.label {
        let label = after.label.clone();
        builder.push(
            Command::SetLabel {
                id: id.clone(),
                label,
            },
            at,
        )?;
    }
    if after.kind != before.kind {
        let kind = Box::new(after.kind.clone());
        builder.push(
            Command::SetKind {
                id: id.clone(),
                kind,
            },
            at,
        )?;
    }
    let delta = DVec2::new(after.rect.x - before.rect.x, after.rect.y - before.rect.y);
    if delta != DVec2::ZERO {
        for command in move_commands(&builder.trial, std::slice::from_ref(&id), delta) {
            builder.push(command, at)?;
        }
    }
    let resized = (after.rect.width, after.rect.height) != (before.rect.width, before.rect.height);
    if resized {
        let rect = Rect::new(
            after.rect.x,
            after.rect.y,
            after.rect.width,
            after.rect.height,
        );
        builder.push(
            Command::SetRect {
                id: id.clone(),
                rect,
            },
            at,
        )?;
    }
    builder.updated.push(id.as_str().to_owned());
    Ok(())
}
