//! The comment routes: list, read, create, respond to and delete
//! annotations, and comment on a selection. Each write is one undo step.

use serde_json::{Value, json};
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, Author, Command, Entity,
    EntityId, JsonMap, Kind, PageAnchor, RegionAnchor, Reply as ThreadReply,
};
use specular_interact::{ApiRun, App, iso8601, region_annotation, region_on_canvas};

use crate::ids::Ids;
use crate::reply::Reply;
use crate::{Request, Response, Screenshot, ShotArea, Step};

/// An annotation as the `.canvas` file holds it.
pub(crate) fn json(annotation: &Annotation) -> Value {
    serde_json::to_value(annotation).unwrap_or(Value::Null)
}

fn find<'a>(app: &'a App, id: &str) -> Result<&'a Annotation, Response> {
    (app.document().annotation(&AnnotationId::new(id)))
        .ok_or_else(|| Response::not_found(format!("Annotation not found: {id}")))
}

fn now(app: &App) -> String {
    iso8601(app.session().now_ms)
}

fn step(command: Command, reply: Reply) -> Step {
    let run = ApiRun::Apply {
        command,
        select: None,
    };
    Step::Run(run, reply)
}

/// `GET /annotations`, filtered by `status` (`unresolved`, `all` or one
/// status), `page_id` and `url`. Only a comment bound to a page matches a
/// page or URL filter.
pub(crate) fn list(app: &App, request: &Request) -> Value {
    let status = request.query("status").filter(|status| *status != "all");
    let page = request.query("page_id");
    let url = request.query("url").map(|url| url.trim_end_matches('/'));
    let kept: Vec<Value> = (app.document().annotations().iter())
        .filter(|annotation| match status {
            None => true,
            Some("unresolved") => matches!(
                annotation.status,
                AnnotationStatus::Pending | AnnotationStatus::Acknowledged
            ),
            Some(wanted) => json!(annotation.status) == json!(wanted),
        })
        .filter(|annotation| {
            let bound = annotation.page_anchor.as_ref();
            let on_page =
                page.is_none_or(|page| bound.is_some_and(|anchor| anchor.page_id.as_str() == page));
            let at_url = url.is_none_or(|url| {
                (bound.and_then(|anchor| anchor.page_url.as_deref()))
                    .is_some_and(|bound| bound.trim_end_matches('/') == url)
            });
            on_page && at_url
        })
        .map(json)
        .collect();
    json!({ "annotations": kept })
}

/// What a selection comment's request is about: each selected entity, a
/// group standing for everything inside it.
fn member(entity: &Entity) -> Value {
    let rect = entity.rect;
    let mut out = json!({
        "id": entity.id,
        "kind": entity.kind.name(),
        "bounds": { "x": rect.x, "y": rect.y, "width": rect.width, "height": rect.height },
    });
    if let Some(label) = &entity.label {
        out["label"] = json!(label);
    }
    match &entity.kind {
        Kind::Page(page) => {
            out["url"] = json!(page.url);
            out["pageName"] = json!(entity.label);
        }
        Kind::Text(text) => out["text"] = json!(text.text),
        Kind::Shape(shape) => out["text"] = json!(shape.text),
        Kind::File(file) => out["filePath"] = json!(file.file),
        Kind::Group(_) | Kind::Drawing(_) => {}
    }
    out
}

/// The entity ids a selection comment recorded.
fn selected(annotation: &Annotation) -> Vec<EntityId> {
    (annotation.metadata.as_ref())
        .and_then(|metadata| metadata.get("selectionEntityIds")?.as_array())
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(EntityId::from)
                .collect()
        })
        .unwrap_or_default()
}

/// `GET /annotations/<id>`: the annotation, for a selection comment the
/// entities it is about, and for a region comment a picture of the region as
/// the canvas shows it now. The Electron app takes that picture when the
/// comment is made and keeps it in the file; one taken on demand shows what
/// the comment is about today and keeps megabytes out of the `.canvas`.
pub(crate) fn detail(app: &App, id: &str) -> Result<Step, Response> {
    let annotation = find(app, id)?;
    let mut body = json(annotation);
    let ids = selected(annotation);
    if !ids.is_empty() {
        let document = app.document();
        let members: Vec<Value> = (app.scope_of(&ids).operands.iter())
            .filter_map(|id| document.entity(id))
            .map(member)
            .collect();
        body["selection"] = json!({ "members": members, "priorFeedback": [] });
    }
    let kept = body["metadata"]["regionScreenshot"].is_string();
    if let Some(rect) = region_on_canvas(app, annotation).filter(|_| !kept) {
        let shot = Screenshot {
            path: None,
            area: ShotArea::Canvas(rect),
        };
        return Ok(Step::Annotated(body, shot));
    }
    Ok(Step::Answer(body))
}

/// The anchor a create asks for. `viewport` is the middle of what the
/// window shows, and a `page` anchor with no offsets is the page's corner.
fn anchor_of(app: &App, anchor: &Value) -> Result<AnnotationAnchor, Response> {
    let mut anchor = anchor.clone();
    match anchor["type"].as_str() {
        Some("viewport") => {
            let session = app.session();
            let middle = session.camera.screen_to_world(session.viewport / 2.0);
            return Ok(AnnotationAnchor::Canvas {
                canvas_x: f64::from(middle.x).round(),
                canvas_y: f64::from(middle.y).round(),
            });
        }
        Some("page") => {
            for key in ["offsetX", "offsetY"] {
                if anchor[key].is_null() {
                    anchor[key] = json!(0);
                }
            }
        }
        _ => {}
    }
    serde_json::from_value(anchor)
        .map_err(|error| Response::bad_request(format!("anchor: {error}")))
}

/// The page an anchor is on, which must be a page of this canvas.
fn page_of(app: &App, anchor: &AnnotationAnchor) -> Result<Option<PageAnchor>, Response> {
    let id = match anchor {
        AnnotationAnchor::Page { page_id, .. } | AnnotationAnchor::Element { page_id, .. } => {
            page_id
        }
        AnnotationAnchor::Canvas { .. } | AnnotationAnchor::Region(_) => return Ok(None),
    };
    match app.document().entity(id).map(|entity| &entity.kind) {
        Some(Kind::Page(page)) => Ok(Some(PageAnchor {
            page_url: Some(page.url.clone()),
            ..PageAnchor::new(id.clone())
        })),
        _ => Err(Response::not_found(format!("Page not found: {id}"))),
    }
}

fn author(request: &Request, fallback: Author) -> Author {
    match request.text("author") {
        Some("agent") => Author::Agent,
        Some("user") => Author::User,
        _ => fallback,
    }
}

fn fresh_id(ids: &mut Ids, app: &App) -> AnnotationId {
    let document = app.document();
    AnnotationId::new(ids.fresh("ann", |id| {
        document.annotation(&AnnotationId::new(id)).is_some()
    }))
}

fn insert(app: &App, annotation: Annotation) -> Command {
    Command::InsertAnnotation {
        annotation: Box::new(annotation),
        at: app.document().annotations().len(),
    }
}

/// `POST /annotations`.
pub(crate) fn create(ids: &mut Ids, app: &App, request: &Request) -> Result<Step, Response> {
    let (Some(anchor), Some(text)) = (request.body.get("anchor"), request.text("text")) else {
        return Err(Response::bad_request("anchor and text are required"));
    };
    let anchor = anchor_of(app, anchor)?;
    let page_anchor = page_of(app, &anchor)?;
    let id = fresh_id(ids, app);
    let annotation = Annotation {
        id: id.clone(),
        anchor,
        author: author(request, Author::User),
        text: text.to_owned(),
        status: AnnotationStatus::Pending,
        replies: Vec::new(),
        created_at: now(app),
        element_name: None,
        page_anchor,
        metadata: request
            .body
            .get("metadata")
            .and_then(Value::as_object)
            .cloned(),
        extra: JsonMap::new(),
    };
    Ok(step(insert(app, annotation), Reply::Annotation(id)))
}

/// `POST /annotations/<id>/<verb>`: acknowledge, resolve, dismiss or reply.
pub(crate) fn respond(
    app: &App,
    request: &Request,
    id: &str,
    verb: &str,
) -> Result<Step, Response> {
    let status = match verb {
        "acknowledge" => Some(AnnotationStatus::Acknowledged),
        "resolve" => Some(AnnotationStatus::Resolved),
        "dismiss" => Some(AnnotationStatus::Dismissed),
        "reply" => None,
        _ => return crate::unported::answer(request, &["annotations", id, verb]),
    };
    let mut annotation = find(app, id)?.clone();
    if let Some(status) = status {
        annotation.status = status;
        let mut metadata = annotation.metadata.take().unwrap_or_default();
        // A reason belongs to a dismissal and leaves with it.
        match request.text("reason").filter(|reason| !reason.is_empty()) {
            Some(reason) => {
                metadata.insert("dismissReason".to_owned(), json!(reason));
            }
            None if status != AnnotationStatus::Dismissed => {
                metadata.shift_remove("dismissReason");
            }
            None => {}
        }
        annotation.metadata = (!metadata.is_empty()).then_some(metadata);
    } else {
        let text = (request.text("text").filter(|text| !text.is_empty()))
            .ok_or_else(|| Response::bad_request("text is required"))?;
        annotation.replies.push(ThreadReply {
            author: author(request, Author::Agent),
            text: text.to_owned(),
            timestamp: now(app),
            extra: JsonMap::new(),
        });
    }
    let id = annotation.id.clone();
    let command = Command::ReplaceAnnotation(Box::new(annotation));
    Ok(step(command, Reply::Annotation(id)))
}

/// `DELETE /annotations/<id>`.
pub(crate) fn delete(app: &App, id: &str) -> Result<Step, Response> {
    let id = find(app, id)?.id.clone();
    let command = Command::RemoveAnnotation(id);
    Ok(step(command, Reply::Fixed(json!({ "ok": true }))))
}

/// The one page, else the one file, a selection is about. A selection
/// spanning several has no target: naming one would be a guess.
fn selection_target(app: &App, operands: &[EntityId]) -> Option<Value> {
    let document = app.document();
    let entities = || operands.iter().filter_map(|id| document.entity(id));
    let pages: Vec<Value> = entities()
        .filter_map(|entity| {
            let Kind::Page(page) = &entity.kind else {
                return None;
            };
            Some(json!({ "entityId": entity.id, "kind": "page", "url": page.url }))
        })
        .collect();
    let files: Vec<Value> = entities()
        .filter_map(|entity| {
            let Kind::File(file) = &entity.kind else {
                return None;
            };
            Some(json!({ "entityId": entity.id, "kind": "file", "filePath": file.file }))
        })
        .collect();
    match (pages.as_slice(), files.as_slice()) {
        ([page], _) => Some(page.clone()),
        ([], [file]) => Some(file.clone()),
        _ => None,
    }
}

/// `POST /selection/annotate`: one region comment over the union of the
/// named entities, or of the selection, carrying their ids so a reader gets
/// the whole request at once.
pub(crate) fn annotate_selection(
    ids: &mut Ids,
    app: &App,
    request: &Request,
) -> Result<Step, Response> {
    let text = (request.text("text").map(str::trim))
        .filter(|text| !text.is_empty())
        .ok_or_else(|| Response::bad_request("text is required"))?;
    let named: Vec<EntityId> = (request.strings("entityIds").unwrap_or_default())
        .into_iter()
        .map(EntityId::from)
        .collect();
    let scope = if named.is_empty() {
        app.selection_scope()
    } else {
        app.scope_of(&named)
    };
    if scope.members.is_empty() {
        return Err(Response::bad_request("No entities selected"));
    }
    let canvas_rect =
        (scope.bounds).ok_or_else(|| Response::bad_request("Selection has no bounds"))?;
    let target = selection_target(app, &scope.operands);
    let mut metadata = JsonMap::new();
    metadata.insert("selectionEntityIds".to_owned(), json!(scope.members));
    if let Some(target) = &target {
        metadata.insert("selectionTarget".to_owned(), target.clone());
    }
    let id = fresh_id(ids, app);
    let annotation = Annotation {
        text: text.to_owned(),
        author: author(request, Author::User),
        metadata: Some(metadata),
        ..region_annotation(
            id.clone(),
            now(app),
            RegionAnchor::Canvas { canvas_rect },
            None,
        )
    };
    let body = json!({
        "id": id,
        "anchor": annotation.anchor,
        "selectionEntityIds": scope.members,
        "selectionTarget": target,
    });
    Ok(step(insert(app, annotation), Reply::Fixed(body)))
}
