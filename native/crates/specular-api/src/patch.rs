//! `POST /canvas/apply`: the one declarative door (ADR 0019). A patch is
//!
//! ```json
//! { "entities": [ {"kind": "text", ...}, {"id": "text_7", ...} ],
//!   "edges":    [ {"fromEntityId": "a", "toEntityId": "b"} ],
//!   "delete":   ["text_3", "edge_9"] }
//! ```
//!
//! An entity with no `id` is created and one with an `id` is updated, its
//! kind read from the document. The whole patch becomes one
//! [`Command::Batch`], so it is one undo step, and a patch with a bad item
//! changes nothing.
//!
//! The commands are built against a copy of the document that each one is
//! applied to as it is made, so a later item sees what an earlier one did
//! and a refused command is reported against the item that made it.

mod edge;
mod entity;
mod fields;

use serde_json::{Value, json};
use specular_doc::{Command, Document, EdgeId, EntityId, JsonMap};
use specular_interact::{ApiRun, App, delete_commands};

use crate::http::strings;
use crate::ids::Ids;
use crate::reply::Reply;
use crate::{Request, Response, Step};

/// A patch being turned into commands.
pub(crate) struct Builder<'a> {
    ids: &'a mut Ids,
    /// The document as the commands so far leave it.
    trial: Document,
    commands: Vec<Command>,
    created: Vec<String>,
    updated: Vec<String>,
    deleted: Vec<String>,
    edges: Vec<String>,
}

impl<'a> Builder<'a> {
    fn new(ids: &'a mut Ids, app: &App) -> Self {
        Self {
            ids,
            trial: app.document().clone(),
            commands: Vec::new(),
            created: Vec::new(),
            updated: Vec::new(),
            deleted: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Adds `command` to the patch. `at` names the item it came from, for
    /// the error when the document refuses it.
    fn push(&mut self, command: Command, at: &str) -> Result<(), Response> {
        self.trial
            .apply(command.clone())
            .map_err(|error| Response::bad_request(format!("{at}: {error}")))?;
        self.commands.push(command);
        Ok(())
    }

    /// An id with this prefix that nothing in the document uses.
    fn fresh_id(&mut self, prefix: &str) -> String {
        let trial = &self.trial;
        self.ids.fresh(prefix, |id| {
            trial.entity(&EntityId::from(id)).is_some() || trial.edge(&EdgeId::from(id)).is_some()
        })
    }

    fn entities(&mut self, items: &[Value]) -> Result<(), Response> {
        let items = objects(items, "entities")?;
        // Every item's kind is checked before anything is built, so the
        // error for a bad one is the same wherever it sits in the patch.
        for (index, item) in items.iter().enumerate() {
            entity::check_kind(&self.trial, item, &format!("entities[{index}]"))?;
        }
        for (index, item) in items.iter().enumerate() {
            let at = format!("entities[{index}]");
            match item.get("id").and_then(Value::as_str) {
                Some(id) => entity::update(self, &at, item, id)?,
                None => entity::create(self, &at, item)?,
            }
        }
        Ok(())
    }

    fn edge_items(&mut self, items: &[Value]) -> Result<(), Response> {
        for (index, item) in objects(items, "edges")?.into_iter().enumerate() {
            edge::upsert(self, &format!("edges[{index}]"), item)?;
        }
        Ok(())
    }

    /// Removes what each id names: an entity with everything inside its
    /// group and hooked to its page, else an edge. An id that names nothing
    /// is skipped and not reported as deleted.
    fn delete(&mut self, ids: &[&str], edges_only: bool) -> Result<(), Response> {
        for id in ids {
            let at = format!("delete '{id}'");
            let entity = EntityId::from(*id);
            let edge = EdgeId::from(*id);
            let commands = if !edges_only && self.trial.entity(&entity).is_some() {
                delete_commands(&self.trial, &[entity], &[])
            } else if self.trial.edge(&edge).is_some() {
                vec![Command::RemoveEdge(edge)]
            } else {
                continue;
            };
            for command in commands {
                self.push(command, &at)?;
            }
            self.deleted.push((*id).to_owned());
        }
        Ok(())
    }

    /// The patch as one step, answered with `reply`'s view of what it did.
    fn finish(self, reply: impl FnOnce(&Self) -> Value) -> Step {
        let body = reply(&self);
        if self.commands.is_empty() {
            return Step::Answer(body);
        }
        let run = ApiRun::Apply {
            command: Command::Batch(self.commands),
            select: None,
        };
        Step::Run(run, Reply::Fixed(body))
    }
}

/// `items` as objects, or a `400` naming the first that is not one.
fn objects<'v>(items: &'v [Value], name: &str) -> Result<Vec<&'v JsonMap>, Response> {
    (items.iter().enumerate())
        .map(|(index, item)| {
            (item.as_object()).ok_or_else(|| {
                Response::bad_request(format!("{name}[{index}]: expected an object"))
            })
        })
        .collect()
}

/// The patch's array `key`. Missing or `null` is empty.
fn array<'v>(patch: &'v Value, key: &str) -> Result<&'v [Value], Response> {
    match patch.get(key) {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(Response::bad_request(format!("{key}: expected an array"))),
    }
}

/// `POST /canvas/apply`.
pub(crate) fn apply(ids: &mut Ids, app: &App, patch: &Value) -> Result<Step, Response> {
    if !patch.is_object() {
        return Err(Response::bad_request("patch: expected an object"));
    }
    let delete = array(patch, "delete")?;
    let delete = (strings(&Value::Array(delete.to_vec())).is_some())
        .then(|| delete.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .ok_or_else(|| Response::bad_request("delete: expected an array of ids"))?;
    let mut builder = Builder::new(ids, app);
    builder.entities(array(patch, "entities")?)?;
    builder.edge_items(array(patch, "edges")?)?;
    builder.delete(&delete, false)?;
    Ok(builder.finish(|done| {
        json!({
            "created": done.created,
            "updated": done.updated,
            "deleted": done.deleted,
            "edges": done.edges,
        })
    }))
}

/// `POST /edges/create`.
pub(crate) fn create_edges(ids: &mut Ids, app: &App, body: &Value) -> Result<Step, Response> {
    let mut builder = Builder::new(ids, app);
    builder.edge_items(array(body, "edges")?)?;
    Ok(builder.finish(|done| json!({ "edgeIds": done.edges })))
}

/// `POST /edges/delete`.
pub(crate) fn delete_edges(ids: &mut Ids, app: &App, request: &Request) -> Result<Step, Response> {
    let edges =
        (request.strings("edgeIds")).ok_or_else(|| Response::bad_request("edgeIds is required"))?;
    let mut builder = Builder::new(ids, app);
    builder.delete(&edges, true)?;
    Ok(builder.finish(|done| json!({ "deletedEdgeIds": done.deleted })))
}

/// `POST /groups/create`.
pub(crate) fn create_group(ids: &mut Ids, app: &App, body: &Value) -> Result<Step, Response> {
    let mut item = body.as_object().cloned().unwrap_or_default();
    if item
        .get("entityIds")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        return Err(Response::bad_request("entityIds is required"));
    }
    item.insert("kind".to_owned(), json!("group"));
    let mut builder = Builder::new(ids, app);
    entity::create(&mut builder, "group", &item)?;
    Ok(builder
        .finish(|done| json!({ "id": done.created.first(), "entityIds": item.get("entityIds") })))
}
