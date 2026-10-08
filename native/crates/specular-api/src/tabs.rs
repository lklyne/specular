//! The tab routes, behind `specular tab`: list the canvases of the space,
//! make one, delete one, and show one.
//!
//! `POST /tabs` and `POST /tabs/delete` leave the user where they are, so
//! an agent that makes a canvas can tidy it up again. `POST /tabs/switch`
//! is the one verb that moves the user.

use serde_json::{Value, json};
use specular_interact::{Action, ApiRun, App, Canvas, CanvasAction, CanvasId};

use crate::reply::Reply;
use crate::{Request, Response, Step};

/// Every canvas of the space and the one the user is looking at.
pub(crate) fn identity(app: &App) -> Value {
    let space = app.space();
    let tabs: Vec<Value> = (space.canvases().iter())
        .map(|canvas| {
            let entities =
                (app.canvas_document(&canvas.id)).map_or(0, |document| document.entities().count());
            json!({ "id": canvas.id.as_str(), "name": canvas.name, "entityCount": entities })
        })
        .collect();
    json!({ "activeTab": named(space.active()), "tabs": tabs })
}

fn named(canvas: &Canvas) -> Value {
    json!({ "id": canvas.id.as_str(), "name": canvas.name })
}

/// The canvas the request body's `ref` names.
fn resolved<'a>(app: &'a App, request: &Request) -> Result<&'a Canvas, Response> {
    let tab_ref = request.text("ref").unwrap_or_default();
    (app.space().resolve(tab_ref)).map_err(|error| Response::bad_request(error.to_string()))
}

/// `POST /tabs`: a new empty canvas, in the background.
pub(crate) fn create(request: &Request) -> Step {
    let name = request.text("name").unwrap_or_default().trim().to_owned();
    let reply = Reply::NewTab { name: name.clone() };
    Step::Run(ApiRun::NewCanvas { name }, reply)
}

/// `POST /tabs/delete`: remove a canvas. The last one cannot go, so it is
/// reset to an empty default canvas, and the answer says which happened.
pub(crate) fn delete(app: &App, request: &Request) -> Result<Step, Response> {
    let canvas = resolved(app, request)?;
    let run = ApiRun::Act {
        on: None,
        action: Action::Canvas(CanvasAction::Delete(Some(canvas.id.clone()))),
    };
    let reply = Reply::DeletedTab {
        deleted: named(canvas),
        reset: app.space().canvases().len() == 1,
    };
    Ok(Step::Run(run, reply))
}

/// `POST /tabs/switch`: show a canvas.
pub(crate) fn switch(app: &App, request: &Request) -> Result<Step, Response> {
    let canvas = resolved(app, request)?;
    let run = ApiRun::Act {
        on: None,
        action: Action::Canvas(CanvasAction::Switch(canvas.id.clone())),
    };
    Ok(Step::Run(
        run,
        Reply::Fixed(json!({ "activeTab": named(canvas) })),
    ))
}

/// The canvas a `--tab` ref names, or the reason it names none.
pub(crate) fn target(app: &App, tab_ref: &str) -> Result<CanvasId, Response> {
    (app.space().resolve(tab_ref))
        .map(|canvas| canvas.id.clone())
        .map_err(|error| Response::bad_request(error.to_string()))
}
