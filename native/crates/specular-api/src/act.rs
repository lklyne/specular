//! The act routes: the ones that map onto an [`Action`] the window's keys
//! and menus already run. Each selects what it acts on first, as a user
//! would.

use glam::DVec2;
use serde_json::{Value, json};
use specular_doc::{EdgeId, EntityId, ItemId, Kind, Rect};
use specular_interact::{Action, ApiRun, App, Showing, fit_camera, is_note_file};

use crate::reply::Reply;
use crate::{Request, Response, Step, unported};

/// The item `id` names: an entity, else an edge.
pub(crate) fn item(app: &App, id: &str) -> Option<ItemId> {
    let document = app.document();
    if document.entity(&EntityId::from(id)).is_some() {
        Some(ItemId::Entity(EntityId::from(id)))
    } else if document.edge(&EdgeId::from(id)).is_some() {
        Some(ItemId::Edge(EdgeId::from(id)))
    } else {
        None
    }
}

fn is_group(app: &App, id: &EntityId) -> bool {
    matches!(
        app.document().entity(id).map(|entity| &entity.kind),
        Some(Kind::Group(_))
    )
}

/// `GET /selection`: a lone group as `selectedGroupId`, anything else as
/// `selectedEntityIds` with the first repeated as `selectedEntityId`.
pub(crate) fn selection(app: &App) -> Value {
    let selection = &app.session().selection;
    if let Some(id) = selection.single_entity()
        && is_group(app, id)
    {
        return json!({ "selectedGroupId": id });
    }
    let entities: Vec<&EntityId> = selection.entities().collect();
    let edges: Vec<&str> = (selection.items().iter())
        .filter_map(|item| match item {
            ItemId::Edge(id) => Some(id.as_str()),
            ItemId::Entity(_) => None,
        })
        .collect();
    let mut out = json!({});
    if let Some(first) = entities.first() {
        out["selectedEntityId"] = json!(first);
        out["selectedEntityIds"] = json!(entities);
    }
    if !edges.is_empty() {
        out["selectedEdgeIds"] = json!(edges);
    }
    out
}

/// `POST /selection/<verb>`: replace the selection.
pub(crate) fn select(app: &App, request: &Request, verb: &str) -> Result<Step, Response> {
    let one = |key: &str| {
        (request.text(key).map(|id| vec![id]))
            .ok_or_else(|| Response::bad_request(format!("{key} is required")))
    };
    let ids = match verb {
        "deselect" => Vec::new(),
        "select-entities" => request.strings("entityIds").unwrap_or_default(),
        "select-page" => one("pageId")?,
        "select-entity" => one("entityId")?,
        "select-group" => one("groupId")?,
        _ => return unported::answer(request, &["selection", verb]),
    };
    let items: Vec<ItemId> = ids.iter().filter_map(|id| item(app, id)).collect();
    let ok = items.len() == ids.len() && (verb == "deselect" || !items.is_empty());
    let run = ApiRun::Act {
        on: None,
        action: Action::Select(items),
    };
    Ok(Step::Run(run, Reply::Selection { ok }))
}

/// Whether `id` can be shown alone, in a tab of its own: a page or a
/// Document.
fn has_a_tab(app: &App, id: &EntityId) -> bool {
    match app.document().entity(id).map(|entity| &entity.kind) {
        Some(Kind::Page(_)) => true,
        Some(Kind::File(file)) => is_note_file(&file.file),
        Some(Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_)) | None => false,
    }
}

/// `POST /camera/focus`: bring what is named into attention. One page or
/// Document is shown alone, in its tab (ADR 0045). Anything else, several
/// entities or a canvas rect is shown centred and as large as fits, with the
/// first entity named selected.
pub(crate) fn focus(app: &App, request: &Request) -> Step {
    let ids: Vec<EntityId> = ["groupIds", "pageIds"]
        .iter()
        .flat_map(|key| request.strings(key).unwrap_or_default())
        .map(EntityId::from)
        .filter(|id| app.document().entity(id).is_some())
        .collect();
    let number = |key: &str| request.body["bounds"][key].as_f64();
    let given = match (number("x"), number("y"), number("width"), number("height")) {
        (Some(x), Some(y), Some(width), Some(height)) => Some(Rect::new(x, y, width, height)),
        _ => None,
    };
    if let ([only], None) = (ids.as_slice(), given)
        && has_a_tab(app, only)
    {
        let run = ApiRun::Act {
            on: None,
            action: Action::Show(Showing::Item(only.clone())),
        };
        return Step::Run(run, Reply::Fixed(json!({ "focused": true })));
    }
    let Some(bounds) = given.or_else(|| app.scope_of(&ids).bounds) else {
        return Step::Answer(json!({ "focused": false }));
    };
    let viewport = app.session().viewport;
    let camera = fit_camera(bounds, DVec2::new(viewport.x.into(), viewport.y.into()));
    let on = (ids.first().cloned()).map(|first| vec![ItemId::Entity(first)]);
    let run = ApiRun::Act {
        on: on.filter(|_| given.is_none()),
        action: Action::SetCamera(camera),
    };
    Step::Run(run, Reply::Fixed(json!({ "focused": true })))
}

/// `POST /stack-order/<verb>`: move the named items in the stack order.
pub(crate) fn stack_order(app: &App, request: &Request, verb: &str) -> Result<Step, Response> {
    let action = match verb {
        "bring-forward" => Action::BringForward,
        "send-backward" => Action::SendBackward,
        "bring-to-front" => Action::BringToFront,
        "send-to-back" => Action::SendToBack,
        _ => return unported::answer(request, &["stack-order", verb]),
    };
    let ids = (request.text("id").map(|id| vec![id]))
        .or_else(|| request.strings("ids"))
        .filter(|ids| !ids.is_empty())
        .ok_or_else(|| Response::bad_request("id or ids is required"))?;
    let unknown: Vec<&str> = (ids.iter().copied())
        .filter(|id| item(app, id).is_none())
        .collect();
    if !unknown.is_empty() {
        return Err(Response {
            status: 404,
            body: json!({ "error": "Unknown stack-order id", "unknownIds": unknown }),
        });
    }
    let on = Some(ids.iter().filter_map(|id| item(app, id)).collect());
    Ok(Step::Run(ApiRun::Act { on, action }, Reply::StackOrder))
}

/// `POST /groups/ungroup`: take a group apart, leaving its members.
pub(crate) fn ungroup(app: &App, request: &Request) -> Result<Step, Response> {
    let id = (request.text("groupId").map(EntityId::from))
        .ok_or_else(|| Response::bad_request("groupId is required"))?;
    if !is_group(app, &id) {
        return Err(Response::not_found("Group not found"));
    }
    let run = ApiRun::Act {
        on: Some(vec![ItemId::Entity(id)]),
        action: Action::Ungroup,
    };
    Ok(Step::Run(run, Reply::Ungrouped))
}

/// `POST /history/undo` and `/history/redo`: one step of the same history
/// Cmd+Z walks.
pub(crate) fn history(app: &App, request: &Request, verb: &str) -> Result<Step, Response> {
    let (action, possible) = match verb {
        "undo" => (Action::Undo, app.can_undo()),
        "redo" => (Action::Redo, app.can_redo()),
        _ => return unported::answer(request, &["history", verb]),
    };
    if !possible {
        return Ok(Step::Answer(history_state(app, false)));
    }
    let run = ApiRun::Act { on: None, action };
    Ok(Step::Run(run, Reply::History))
}

/// What the history can do now. `ok` is whether a step was just taken.
pub(crate) fn history_state(app: &App, ok: bool) -> Value {
    json!({ "ok": ok, "canUndo": app.can_undo(), "canRedo": app.can_redo() })
}
