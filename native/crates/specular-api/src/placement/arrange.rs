//! The arrange and auto-layout routes: tidy a set of items in place or pack
//! them at a gap, make a group manage its members, and move a member in a
//! managed group's sequence. Each is one undo step.

use glam::DVec2;
use serde_json::{Value, json};
use specular_doc::{Entity, EntityId, ItemId, Kind};
use specular_interact::{
    ApiRun, App, ArrangeMode, arrange_command, auto_layout_command, place_command, reorder_command,
};

use super::layout::{self, Mode, Size};
use super::{GUTTER, number};
use crate::ids::Ids;
use crate::reply::Reply;
use crate::{Request, Response, Step};

/// The grid pitch, which a reading-order band is at least as tall as.
const GRID: f64 = 20.0;

fn changed(command: Option<specular_doc::Command>) -> Step {
    match command {
        Some(command) => Step::Run(
            ApiRun::Apply {
                command,
                select: None,
            },
            Reply::Fixed(json!({ "changed": true })),
        ),
        None => Step::Answer(json!({ "changed": false })),
    }
}

/// `POST /selection/arrange`: `{mode, entityIds?, gap?, cols?}`. With no
/// gap the items keep their footprint and the spacing inside it is evened
/// (ADR 0026). With one they are packed at it from their top-left, in
/// reading order.
pub(crate) fn arrange(app: &App, request: &Request) -> Result<Step, Response> {
    let body = &request.body;
    let directive = json!({ "kind": body["mode"], "gap": body["gap"], "cols": body["cols"] });
    let mode = layout::validate(&directive).map_err(Response::bad_request)?;
    let ids: Vec<EntityId> = match request.strings("entityIds") {
        Some(ids) => ids.into_iter().map(EntityId::from).collect(),
        None => app.selection_scope().members,
    };
    if body.get("gap").is_none_or(Value::is_null) {
        let mode = match mode {
            Mode::Row => ArrangeMode::Row,
            Mode::Column => ArrangeMode::Column,
            Mode::Grid => ArrangeMode::Grid,
        };
        return Ok(changed(arrange_command(app, &ids, mode)));
    }
    let mut found: Vec<&Entity> = (ids.iter())
        .filter_map(|id| app.document().entity(id))
        .collect();
    if found.len() < 2 {
        return Ok(Step::Answer(json!({ "changed": false })));
    }
    // Reading order, in bands loose enough that a rough row does not sort
    // by its exact tops.
    let least_height = (found.iter())
        .map(|entity| entity.rect.height)
        .fold(f64::INFINITY, f64::min);
    let band = GRID.max(least_height / 2.0);
    found.sort_by(|a, b| {
        if (a.rect.y - b.rect.y).abs() > band {
            a.rect.y.total_cmp(&b.rect.y)
        } else {
            a.rect.x.total_cmp(&b.rect.x)
        }
    });
    let sizes: Vec<Size> = (found.iter())
        .map(|entity| Size {
            width: entity.rect.width,
            height: entity.rect.height,
        })
        .collect();
    let corner = found
        .iter()
        .fold((f64::INFINITY, f64::INFINITY), |at, entity| {
            (at.0.min(entity.rect.x), at.1.min(entity.rect.y))
        });
    let gap = layout::spacing(&body["gap"], GUTTER);
    let cols = (body["cols"].as_u64()).and_then(|cols| usize::try_from(cols).ok());
    let targets: Vec<(EntityId, DVec2)> = (found.iter())
        .zip(layout::positions(&sizes, mode, gap, gap, corner, cols))
        .map(|(entity, (x, y))| (entity.id.clone(), DVec2::new(x, y)))
        .collect();
    Ok(changed(place_command(app, &targets)))
}

/// `POST /groups/auto-layout`: `{groupId?, entityIds?, label?, gap?}`.
/// Makes the group, or a new one round the entities, manage its members as
/// a row or a column, and answers with the group.
pub(crate) fn auto_layout(ids: &mut Ids, app: &App, request: &Request) -> Result<Step, Response> {
    let named: Vec<EntityId> = match request.text("groupId") {
        Some(group) => vec![EntityId::from(group)],
        None => (request.strings("entityIds").unwrap_or_default())
            .into_iter()
            .map(EntityId::from)
            .collect(),
    };
    if named.is_empty() {
        return Err(Response::bad_request("groupId or entityIds is required"));
    }
    let document = app.document();
    let id =
        EntityId::from(ids.fresh("group", |id| document.entity(&EntityId::from(id)).is_some()));
    let label = request.text("label").unwrap_or("Auto-layout");
    let gap = number(&request.body, "gap");
    let is_group = |id: &EntityId| {
        matches!(
            document.entity(id).map(|entity| &entity.kind),
            Some(Kind::Group(_))
        )
    };
    if request.text("groupId").is_some() && !named.iter().all(is_group) {
        return Err(Response::not_found("Nothing to manage"));
    }
    let Some((group, command)) = auto_layout_command(app, &named, &id, label, gap) else {
        return Err(Response::not_found("Nothing to manage"));
    };
    let run = ApiRun::Apply {
        command,
        select: Some(vec![ItemId::Entity(group.clone())]),
    };
    Ok(Step::Run(run, Reply::Group(group)))
}

/// `POST /groups/reorder-child`: `{groupId, childId, toIndex}`. Moves a
/// member to a slot of its managed group's sequence.
pub(crate) fn reorder_child(app: &App, request: &Request) -> Result<Step, Response> {
    let (Some(group), Some(child), Some(to)) = (
        request.text("groupId"),
        request.text("childId"),
        request.body["toIndex"].as_f64(),
    ) else {
        return Err(Response::bad_request(
            "groupId, childId and toIndex are required",
        ));
    };
    let to = to.max(0.0) as usize;
    Ok(changed(reorder_command(
        app,
        &EntityId::from(group),
        &EntityId::from(child),
        to,
    )))
}

/// A group as the routes answer with it: its rect, its layout and its
/// direct members.
pub(crate) fn group_json(app: &App, id: &EntityId) -> Option<Value> {
    let entity = app.document().entity(id)?;
    let Kind::Group(group) = &entity.kind else {
        return None;
    };
    let members: Vec<&str> = (app.document().children(id))
        .map(|child| child.id.as_str())
        .collect();
    let mut out = json!({
        "id": id,
        "canvasX": entity.rect.x,
        "canvasY": entity.rect.y,
        "width": entity.rect.width,
        "height": entity.rect.height,
        "layoutMode": group.layout_mode,
        "managedLayout": group.managed_layout.unwrap_or(false),
        "entityIds": members,
    });
    if let Some(label) = &entity.label {
        out["label"] = json!(label);
    }
    if let Some(gap) = group.layout_gap {
        out["layoutGap"] = json!(gap);
    }
    Some(out)
}
